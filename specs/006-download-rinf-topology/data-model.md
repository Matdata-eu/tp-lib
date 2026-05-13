# Data Model: ERA RINF Network Download

**Phase**: Phase 1 — Design & Data Model  
**Date**: 2026-05-13  
**Feature**: `006-download-rinf-topology`

---

## 1. RetrievalArea

Represents the spatial search region sent to the RINF SPARQL endpoint.

**Fields**:
- `min_longitude: f64`
- `max_longitude: f64`
- `min_latitude: f64`
- `max_latitude: f64`
- `expansion_meters: f64` default `1000.0`
- `polygon_wkt: String` closed WGS84 polygon
- `source_crs: String` expected `EPSG:4326` after normalization

**Validation rules**:
- At least one usable GNSS coordinate is required to create the area.
- `min_* <= max_*` for both axes.
- `polygon_wkt` must be closed and contain 5 coordinate pairs for a rectangle.

**Relationships**:
- Derived from `AutoTopologyRequest.gnss_positions`.
- Consumed by `RinfRetrievalRequest`.

---

## 2. AutoTopologyRequest

Represents a workflow invocation that may need automatic topology retrieval.

**Fields**:
- `workflow_kind: enum { projection, path_calculation, detection_preparation, path_review }`
- `gnss_positions: Vec<GnssPosition>`
- `supplied_topology_present: bool`
- `rinf_endpoint_url: String`
- `retrieval_area: Option<RetrievalArea>`
- `requested_at: DateTime<Utc>`

**Validation rules**:
- If `supplied_topology_present == true`, automatic retrieval is skipped.
- If `supplied_topology_present == false`, `gnss_positions` must contain at least one usable coordinate.

**Relationships**:
- Produces either a `RetrievedTopology` or a terminal `RetrievalOutcome`.

---

## 3. RinfNetelementRow

Typed representation of one row returned by the netelement SPARQL query.

**Fields**:
- `netelement_iri: String`
- `netelement_id: String` derived stable identifier used by tp-lib
- `wkt: String`
- `geometry_point_count: usize`
- `length_meters: f64`

**Validation rules**:
- `wkt` must parse into a LineString.
- `geometry_point_count >= 2`.
- If `length_meters > 250.0`, then `geometry_point_count > 2`.

**Relationships**:
- Maps to existing core `Netelement`.
- Referenced by `RinfNetrelationRow.element_a_id` and `.element_b_id`.

---

## 4. RinfNetrelationRow

Typed representation of one row returned by the netrelation SPARQL query.

**Fields**:
- `netrelation_iri: String`
- `element_a_id: String`
- `element_b_id: String`
- `is_on_origin_of_element_a: bool`
- `is_on_origin_of_element_b: bool`
- `navigability: enum { both, AB, BA, none }`
- `valid_on_date: NaiveDate`

**Validation rules**:
- Both endpoint element IDs must be non-empty.
- `navigability` must map to a supported tp-lib direction model.
- The referenced elements must exist in the retrieved netelement set after mapping.

**Relationships**:
- Maps to existing core `NetRelation`.
- Belongs to `RetrievedTopology.netrelations`.

---

## 5. RetrievedTopology

Normalized topology bundle prepared for downstream workflows.

**Fields**:
- `netelements: Vec<Netelement>`
- `netrelations: Vec<NetRelation>`
- `retrieval_area: RetrievalArea`
- `endpoint_url: String`
- `retrieved_at: DateTime<Utc>`
- `validation_report: TopologyValidationReport`

**Validation rules**:
- `netelements.len() > 0` for a successful retrieval.
- `netrelations.len() > 0` for a topology-valid success.
- `validation_report.status == valid` before downstream use.

**Relationships**:
- Output of `AutoTopologyRequest` when retrieval and validation succeed.
- Input to all existing topology-dependent workflow functions.

---

## 6. TopologyValidationReport

Explains whether the downloaded topology is usable.

**Fields**:
- `status: enum { valid, missing_coverage, incomplete_topology, invalid_input, endpoint_failure }`
- `netelement_count: usize`
- `netrelation_count: usize`
- `coarse_geometry_ids: Vec<String>`
- `uncovered_gnss_indices: Vec<usize>`
- `message: String`

**Validation rules**:
- `status == valid` implies `coarse_geometry_ids` is empty.
- `status == incomplete_topology` when `netelement_count > 0 && netrelation_count == 0`.
- `status == invalid_input` when retrieval never starts because GNSS input is unusable.

**Relationships**:
- Embedded in `RetrievedTopology`.
- Converted into binding/CLI-facing `RetrievalOutcome` diagnostics.

---

## 7. RetrievalOutcome

Caller-visible outcome for source selection and validation.

**Fields**:
- `source_used: enum { supplied_topology, era_rinf }`
- `status: enum { success, invalid_input, missing_coverage, incomplete_topology, endpoint_failure }`
- `detail_message: String`
- `diagnostic_area_wkt: Option<String>`
- `affected_gnss_indices: Vec<usize>`

**Validation rules**:
- `source_used == supplied_topology` bypasses all RINF-specific failure states.
- `status == success` requires a non-empty topology bundle.

**Relationships**:
- Returned or surfaced by CLI, Python, and .NET adapters.

---

## State Transitions

```text
AutoTopologyRequest
  -> invalid_input
  -> retrieval_area_built
  -> queried_endpoint
  -> parsed_rows
  -> validation_failed(missing_coverage | incomplete_topology | endpoint_failure)
  -> validated_topology
  -> success
```

Rules:
- `supplied_topology_present == true` short-circuits directly to `success` with `source_used = supplied_topology`.
- Any invalid-input failure occurs before `queried_endpoint`.
- Any topology-dependent workflow consumes only the `validated_topology` state.