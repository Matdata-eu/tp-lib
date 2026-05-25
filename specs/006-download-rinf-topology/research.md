# Research: ERA RINF Network Download

**Phase**: Phase 0 — Outline & Research  
**Date**: 2026-05-13  
**Feature**: `006-download-rinf-topology`

---

## Decision 1: Retrieval Region Geometry

**Decision**: Use a single GNSS-derived search polygon built from the dataset envelope and expanded by 1 km in every direction. The first implementation uses an axis-aligned bounding box polygon in WGS84 rather than a convex hull or buffered line corridor.

**Rationale**:
- The specification explicitly accepts a simple min/max X/Y bounding box as sufficient.
- A single rectangular polygon is easy to generate deterministically from all supported GNSS inputs.
- It reduces implementation risk for the first release and keeps the SPARQL query builder simple and testable.
- Downloading more topology than strictly required is acceptable for this feature.

**How the 1 km expansion works**:
- Parse GNSS positions into WGS84 longitude/latitude first.
- Compute `min_lon`, `max_lon`, `min_lat`, `max_lat` across usable points.
- Expand latitude bounds by `1000 / 111_320` degrees.
- Expand longitude bounds by `1000 / (111_320 * cos(latitude_of_bbox_center))` degrees.
- Emit the expanded rectangle as a closed WKT polygon in lon/lat order.

**Alternatives considered**:
- **Convex hull + 1 km buffer**: More spatially selective, but adds geometry-buffering complexity for limited practical gain in the first release.
- **Polyline corridor around the GNSS path**: Most precise, but couples retrieval to path-shape heuristics before topology exists.
- **Multiple clusters / multiple polygons**: Rejected because the clarified spec explicitly allows one larger retrieval region.

---

## Decision 2: SPARQL Access Pattern

**Decision**: Query the RINF knowledge graph via two tabular `SELECT` queries against `https://graph.data.era.europa.eu/repositories/rinf-plus`, one for netelements and one for netrelations, and request JSON SPARQL results over HTTPS.

**Rationale**:
- The user explicitly confirmed that splitting the construct query into two `SELECT` queries yields tabular results for both entity types.
- Tabular JSON is cheaper to parse and validate than an RDF `CONSTRUCT` graph.
- The repository already standardizes on typed Rust models; tabular rows map directly into those structs.
- Two queries are easier to test independently: geometry/query correctness for netelements and topology completeness/validity for netrelations.

**Operational choice**:
- Use a synchronous HTTP client in the library path so CLI, Python, and .NET can reuse the same Rust retrieval logic without introducing async boundary changes.
- Send the SPARQL query to the endpoint with `Accept: application/sparql-results+json`.

**Alternatives considered**:
- **Single `CONSTRUCT` query**: More faithful to RDF semantics, but requires graph parsing and more complex mapping code.
- **SPARQL client library with RDF model objects**: Heavier dependency surface than needed for two bounded query shapes.
- **Async-only client**: Would force Tokio or executor concerns into call sites that are currently synchronous.

---

## Decision 3: Spatial Predicate and Query Shape

**Decision**: Filter netelements with a polygon WKT and a GeoSPARQL intersection predicate so any netelement partly inside the retrieval area is eligible. The smoke-test polygon from the prompt becomes the fixed integration test region.

**Rationale**:
- The clarified feature rule says any netelement partly inside the expanded search box must be downloaded.
- `sfContains` is too strict for boundary-crossing lines; the desired semantics are intersection-based.
- The prompt already supplies a polygon known to return valid netelements and netrelations, making it suitable as the feature's endpoint-backed smoke test.

**Selected query shapes**:

Netelements query:

```sparql
PREFIX era: <http://data.europa.eu/949/>
PREFIX gsp: <http://www.opengis.net/ont/geosparql#>
PREFIX geof: <http://www.opengis.net/def/function/geosparql/>

SELECT ?netelement ?netelement_wkt
WHERE {
  ?netelement a era:LinearElement ;
              gsp:hasGeometry/gsp:asWKT ?netelement_wkt .
  FILTER(geof:sfIntersects(
    ?netelement_wkt,
    "POLYGON((...closed search polygon...))"^^gsp:wktLiteral
  ))
}
```

Netrelations query:

```sparql
PREFIX era: <http://data.europa.eu/949/>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
PREFIX time: <http://www.w3.org/2006/time#>

SELECT ?netrelation ?netelementA ?netelementB ?isOnOriginOfElementA ?isOnOriginOfElementB ?navigability
WHERE {
  VALUES ?seed_element { <...netelement IRIs returned by query 1...> }
  {
    BIND(?seed_element AS ?netelementA)
    ?netrelation a era:NetRelation ;
                 era:elementA ?netelementA ;
                 era:elementB ?netelementB ;
                 era:isOnOriginOfElementA ?isOnOriginOfElementA ;
                 era:isOnOriginOfElementB ?isOnOriginOfElementB ;
                 era:navigability ?navigability ;
                 era:validity/time:hasBeginning/time:inXSDDate ?valid_from_date .
    OPTIONAL {
      ?netrelation era:validity/time:hasEnd/time:inXSDDate ?valid_to_date .
      FILTER (xsd:date(now()) >= ?valid_to_date)
    }
    FILTER (xsd:date(now()) >= ?valid_from_date && !BOUND(?valid_to_date))
  }
  UNION
  {
    BIND(?seed_element AS ?netelementB)
    ?netrelation a era:NetRelation ;
                 era:elementA ?netelementA ;
                 era:elementB ?netelementB ;
                 era:isOnOriginOfElementA ?isOnOriginOfElementA ;
                 era:isOnOriginOfElementB ?isOnOriginOfElementB ;
                 era:navigability ?navigability ;
                 era:validity/time:hasBeginning/time:inXSDDate ?valid_from_date .
    OPTIONAL {
      ?netrelation era:validity/time:hasEnd/time:inXSDDate ?valid_to_date .
      FILTER (xsd:date(now()) >= ?valid_to_date)
    }
    FILTER (xsd:date(now()) >= ?valid_from_date && !BOUND(?valid_to_date))
  }
}
```

**Smoke-test polygon**:

```text
POLYGON((10.99113464355469 59.939604892689715,
11.035079956054688 59.521503830930165,
11.82060241699219 59.443399524042945,
12.90275573730469 60.3443471917804,
12.106246948242188 60.668873461039,
11.51847839355469 60.57185665386768,
10.99113464355469 59.939604892689715))
```

**Alternatives considered**:
- **Continue using the provided `CONSTRUCT` query verbatim**: Good for smoke testing, but less convenient for typed row parsing.
- **Filter netrelations spatially rather than by retrieved seed netelements**: Harder to guarantee consistency with the loaded element set.

---

## Decision 4: Topology Validation Rules

**Decision**: Validate the retrieved topology before any downstream workflow uses it. Fail fast on coarse geometry and missing relations.

**Validation rules**:
- If the GNSS dataset is empty or has no usable coordinates, fail before any HTTP request.
- If no netelements are returned, report missing coverage.
- If netelements are returned but no netrelations are returned, report incomplete topology.
- For every returned netelement longer than 250 m, fail validation if its WKT contains 2 or fewer coordinates.

**Rationale**:
- The prompt identifies macro-topology migration as a real data-quality risk.
- The workflow should not continue on coarse or relationless topology because that produces misleading path results.
- These checks are deterministic and can be covered by unit tests and smoke tests.

**Alternatives considered**:
- **Warn but continue**: Rejected because the feature specification explicitly requires fail-fast behavior for incomplete coverage.
- **Auto-fallback to topology-free logic**: Rejected because topology-dependent workflows would silently change semantics.

---

## Decision 5: Integration Architecture

**Decision**: Add automatic retrieval as a new high-level topology-source layer in `tp-core`, while leaving the low-level `project_gnss`, `calculate_train_path`, and `prepare_detections` algorithms unchanged. CLI and bindings call the new orchestration layer only when topology is absent.

**Rationale**:
- Existing algorithms already assume a validated `Netelement`/`NetRelation` graph.
- Keeping retrieval separate from core matching logic reduces regression risk and preserves current tests.
- This architecture makes manual topology and auto-retrieved topology equivalent once converted into core models.

**Integration consequences**:
- `tp-cli`: `--network` becomes optional for topology-dependent commands; omission triggers retrieval.
- `tp-py`: add API entry points or overloads that accept GNSS-only input and optional retrieval options.
- `tp-net`: add overloads or nullable-network entry points with matching retrieval options.

**Alternatives considered**:
- **Inject HTTP retrieval directly into existing path/projection functions**: Too much branching inside algorithm code.
- **Retrieve topology separately in each binding**: Would fragment behavior and duplicate SPARQL logic across languages.

---

## Decision 6: First-Release Scope for Caching and Endpoint Configuration

**Decision**: Do not add persistent topology caching in the first release. Use the production endpoint by default, but make the endpoint URL overridable for tests and advanced callers.

**Rationale**:
- The specification is about automatic retrieval and validation, not offline caching.
- Keeping retrieval stateless avoids cache invalidation questions during an active topology migration.
- A configurable endpoint is necessary for smoke tests, staging, and reproducible integration tests.

**Alternatives considered**:
- **Disk cache keyed by bbox**: Useful later, but out of scope for the first feature slice.
- **Hard-coded endpoint with no override**: Simpler, but harder to test and debug.

---

## Resolved NEEDS CLARIFICATION

All planning-level unknowns for this feature are resolved:

| Unknown | Resolution |
|---|---|
| Retrieval geometry | Single envelope polygon expanded by 1 km |
| Endpoint | `https://graph.data.era.europa.eu/repositories/rinf-plus` |
| Query format | Two JSON `SELECT` queries |
| Spatial predicate | Intersection semantics for partial overlap |
| Validation rules | Coarse-geometry and zero-netrelation failures are mandatory |
| Workflow scope | All existing topology-dependent workflows |
| Manual-topology precedence | Manual input remains authoritative |
| Empty GNSS behavior | Fail validation before retrieval |