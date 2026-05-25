# API Contracts: ERA RINF Topology Retrieval

**Phase**: Phase 1 — Design & Contracts  
**Date**: 2026-05-13  
**Feature**: `006-download-rinf-topology`

This document specifies the public contracts for automatic topology retrieval across the external SPARQL boundary and the repo's user-facing integration surfaces.

---

## 1. External SPARQL Contract

### Endpoint

- `POST https://graph.data.era.europa.eu/repositories/rinf-plus`
- Request content type: `application/sparql-query` or standard form-encoded `query=` payload
- Response content type: `application/sparql-results+json`

### Query A: Netelements by Search Polygon

**Inputs**:
- `search_polygon_wkt: string` in WGS84 lon/lat order

**Result columns**:
- `netelement`
- `netelement_wkt`

**Contract requirements**:
- Returns every `era:LinearElement` whose geometry intersects the search polygon.
- Returned `netelement_wkt` must be parseable into a LineString.

### Query B: Netrelations by Retrieved Elements

**Inputs**:
- `seed element IRIs` from Query A
- current date for validity filtering

**Result columns**:
- `netrelation`
- `netelementA`
- `netelementB`
- `isOnOriginOfElementA`
- `isOnOriginOfElementB`
- `navigability`

**Contract requirements**:
- Only currently valid netrelations are returned.
- Each row must reference two netelements that can be mapped into the retrieved topology bundle.

---

## 2. CLI Contract (`tp-cli`)

Topology-dependent commands continue to support manual topology input and gain automatic retrieval when `--network` is omitted.

### Default / `calculate-path`

```text
tp-cli calculate-path --gnss <FILE> [--network <FILE>] [--rinf-endpoint <URL>] [--rinf-buffer-meters <N>]
```

**Behavior**:
- If `--network` is provided, CLI uses supplied topology and does not contact RINF.
- If `--network` is omitted, CLI derives a search polygon from GNSS, retrieves RINF topology, validates it, and then runs path calculation.

**Failure outcomes**:
- Invalid GNSS input: non-zero exit, stderr explains GNSS input is empty/invalid.
- Missing coverage: non-zero exit, stderr explains no topology was available for the area.
- Incomplete topology: non-zero exit, stderr explains coarse geometry or missing netrelations.
- Endpoint failure: non-zero exit, stderr explains retrieval failed upstream.

### `simple-projection` and other topology-dependent commands

Same source-selection rule applies: omit `--network` to trigger automatic RINF retrieval.

---

## 3. Python Binding Contract (`tp-py`)

### Retrieval options class

```python
class RinfRetrievalOptions:
    endpoint_url: str = "https://graph.data.era.europa.eu/repositories/rinf-plus"
    buffer_meters: float = 1000.0
```

### Projection

```python
project_gnss(
    gnss_file: str,
    gnss_crs: str,
    network_file: str | None = None,
    network_crs: str | None = None,
    target_crs: str | None = None,
    config: ProjectionConfig | None = None,
    rinf_options: RinfRetrievalOptions | None = None,
)
```

### Path calculation

```python
calculate_train_path(
    gnss_file: str,
    gnss_crs: str,
    network_file: str | None = None,
    network_crs: str | None = None,
    config: PathConfig | None = None,
    rinf_options: RinfRetrievalOptions | None = None,
)
```

### Detections preparation

```python
prepare_detections(
    gnss_file: str,
    detections_file: str,
    network_file: str | None = None,
    rinf_options: RinfRetrievalOptions | None = None,
)
```

**Behavior**:
- `network_file is not None`: supplied topology is authoritative; no retrieval.
- `network_file is None`: bindings invoke Rust retrieval/validation logic using `rinf_options` (or defaults).
- Missing coverage and endpoint failures surface as typed Python exceptions
  (`InvalidGnssInputError`, `RinfMissingCoverageError`,
  `RinfIncompleteTopologyError`, `RinfRetrievalFailedError`).

---

## 4. .NET Contract (`tp-net`)

Because C# overload resolution cannot distinguish overloads that differ only
in nullable-reference annotations, the auto-retrieval entry points use the
`*Auto` suffix.

### Projection

```csharp
public static IReadOnlyList<ProjectedPosition> Projection.ProjectGnssAuto(
    NetworkInput? network,
    GnssInput gnss,
    ProjectionConfig? config = null,
    RinfRetrievalOptions? rinfOptions = null);
```

### Path calculation

```csharp
public static PathResult PathCalculation.CalculateTrainPathAuto(
    NetworkInput? network,
    GnssInput gnss,
    PathConfig? config = null,
    PreparedDetections? detections = null,
    RinfRetrievalOptions? rinfOptions = null);
```

### Retrieval options type

```csharp
public sealed class RinfRetrievalOptions
{
    public string EndpointUrl { get; set; } = "https://graph.data.era.europa.eu/repositories/rinf-plus";
    public double BufferMeters { get; set; } = 1000.0;
}
```

**Behavior**:
- `network != null`: no RINF retrieval is attempted.
- `network == null`: the wrapper calls the Rust retrieval workflow before
  invoking the existing algorithms.
- Failures surface as distinct typed exceptions:
  `TpLibInvalidGnssInputException`, `TpLibRinfMissingCoverageException`,
  `TpLibRinfIncompleteTopologyException`, `TpLibRinfRetrievalFailedException`.

---

## 5. Shared Outcome Contract

All surfaces must preserve the same semantic outcome categories:

| Outcome | Meaning |
|---|---|
| `success` | Topology was supplied or retrieved and validated successfully |
| `invalid_input` | GNSS data was empty or unusable before any retrieval attempt |
| `missing_coverage` | No suitable topology could be retrieved for the search region |
| `incomplete_topology` | Retrieved topology failed validation, including coarse geometry or zero netrelations |
| `endpoint_failure` | The external SPARQL request failed or returned an unusable response |

No interface is allowed to collapse these categories into a generic failure message.