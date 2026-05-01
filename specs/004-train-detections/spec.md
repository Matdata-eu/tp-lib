# Feature Specification: Absolute Train Position Detections (Punctual & Linear)

**Feature Branch**: `004-train-detections`  
**Created**: May 1, 2026  
**Status**: Draft  
**Input**: User description: "Allow the user to also supply data that contains absolute singular/punctual or linear positioning information of the train. (1) singular/punctual: an external detector at a specific known location has detected the train at a certain timestamp, or the train drove over a signaling beacon of known topological position. (2) linear: a train can also be detected on a linearelement, with a timestamp from and to."

## Clarifications

### Session 2026-05-01

- Q: How should `intrinsic` / `start_intrinsic` / `end_intrinsic` fields be treated in this feature, given FR-014 was removed? → A: Keep the fields in the input schema as informational metadata only — system validates and records them in provenance output, but does not use them to constrain path calculation in this feature (reserved for a later feature).
- Q: What must the webapp support for detections in this feature, given the Assumptions section now lists the webapp UI as in scope? → A: Display + per-detection details panel — punctual detections rendered as map markers, linear detections as highlighted netelements on the existing path-review map; clicking a marker or highlighted segment opens a panel showing source, timestamp(s), applied/discarded status, and discard reason. No upload, no filtering, no editing in this feature.
- Q: Which file format(s) must the detection input files support, and how is the format selected? → A: Both GeoJSON and CSV; format is auto-detected per file by extension (`.geojson` / `.json` → GeoJSON; `.csv` → CSV). Schemas for both formats are documented in this spec / contracts.
- Q: What are the units of `--cutoff-distance-detections` and why is it separate from the existing `--cutoff-distance`? → A: Meters; kept separate because detector locations are surveyed with high accuracy whereas GNSS samples are noisy with meter-scale error. Reusing the GNSS cutoff would accept clearly-wrong detector resolutions. Default 2.5 m is appropriate for surveyed detectors.
- Q: How should the system handle multiple punctual detections sharing the same timestamp but referencing different netelements? → A: Reject as a fatal input error. Two physically distinct detectors firing at exactly the same timestamp on different netelements indicates a data-quality problem (duplicate, clock issue, or wrong association); the run aborts with a clear error identifying the conflicting detections. Same-timestamp detections that agree on netelement are not a conflict and are retained (deduplicated).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Anchor Path Calculation with Punctual Detections (Priority: P1)

A railway engineer has access to data from trackside detectors and signaling beacons that recorded the train passing a known topological location at a specific moment. They supply this data alongside the GNSS log so that ambiguous map-matching choices (parallel tracks, junctions, tunnels) are resolved deterministically — the calculated train path is forced to pass through every supplied detection point.

**Why this priority**: Punctual detections from balises, RFID tags, or trackside detectors are the highest-value ground-truth available in railway operations. A single confirmed beacon read can rescue an otherwise ambiguous or incorrect path. This is the core capability of the feature.

**Independent Test**: Can be fully tested by providing GNSS data over a parallel-track section together with one punctual detection on the correct track at a known time, and verifying that the calculated path traverses the detected netelement instead of the parallel one.

**Acceptance Scenarios**:

1. **Given** a GNSS log spanning several minutes and a punctual detection identifying a specific netelement at a timestamp inside that range, **When** the user runs path calculation with the detections file supplied, **Then** the calculated train path includes the detected netelement at the corresponding position in the sequence.
2. **Given** a punctual detection that contradicts the GNSS-derived best candidate, **When** path calculation runs, **Then** the detection overrides the GNSS-derived candidate and the path is forced through the detected netelement.
3. **Given** a punctual detection whose timestamp lies before the first GNSS sample or after the last GNSS sample, **When** path calculation runs, **Then** the detection is discarded and a warning is reported; the path is still produced from the remaining inputs.

---

### User Story 2 - Anchor Path Calculation with Linear Detections (Priority: P1)

A railway engineer has access to track-circuit occupation logs or axle-counter section records that report the train was present on a specific netelement between a `t_from` and `t_to`. They supply this data so the calculated path is constrained to that netelement during the corresponding interval — particularly valuable in tunnels or other zones where GNSS is unreliable.

**Why this priority**: Linear detections (track circuits, block sections) are the dominant absolute-positioning data source in modern signaling systems. Like punctual detections, they are essential for path correctness in GNSS-degraded environments.

**Independent Test**: Can be fully tested by providing a GNSS log that crosses a tunnel where positions drift, plus a linear detection covering the tunnelled netelement during the corresponding time window, and verifying that the calculated path includes that netelement for the full window.

**Acceptance Scenarios**:

1. **Given** a linear detection `(netelement_id, t_from, t_to)` covering a netelement and a GNSS log overlapping that interval, **When** path calculation runs, **Then** GNSS observations whose timestamps fall within `[t_from, t_to]` are forced onto the constrained netelement.
2. **Given** a linear detection where the supplied time window is broader than the actual train presence (a known characteristic of track-circuit data), **When** path calculation runs, **Then** the system still produces a continuous, navigable path that enters and exits the constrained netelement within the window without forcing the train to remain stationary on it for the full duration.
3. **Given** a linear detection whose `t_from`–`t_to` window lies entirely outside the GNSS time range, **When** path calculation runs, **Then** the detection is discarded and a warning is reported.
4. **Given** a linear detection where only one of `t_from` or `t_to` falls within the GNSS time range, **When** path calculation runs, **Then** the detection is discarded and a warning is reported (partial-overlap detections are not honored).

---

### User Story 3 - Coordinate-Only Punctual Detections (Priority: P2)

A railway engineer has data from a fixed-position trackside detector for which the topological location (`netelement_id`) is not pre-recorded — only the geographic coordinates of the device are known. They supply these coordinates and the system resolves them to the nearest netelement at load time, then treats the result identically to a topologically-known punctual detection.

**Why this priority**: Many real detector inventories store only `(lat, lon)`. Supporting coordinate-only input avoids forcing the user to pre-process their data, but it depends on the core punctual-anchor mechanism (US1) being in place first.

**Independent Test**: Can be fully tested by supplying a punctual detection with `(lat, lon, crs)` (no `netelement_id`) located within the network coverage, and verifying it is resolved to the correct netelement and applied as an anchor exactly like a topological detection.

**Acceptance Scenarios**:

1. **Given** a punctual detection with only `(lat, lon, crs)` supplied and a network containing a netelement near those coordinates, **When** the detections file is loaded, **Then** the detection is resolved to the nearest netelement (with intrinsic computed from perpendicular projection) before path calculation begins.
2. **Given** a punctual detection whose coordinates cannot be resolved to any netelement within a reasonable distance (no candidate within the configured cutoff), **When** the detections file is loaded, **Then** the detection is discarded and a warning is reported; path calculation proceeds with the remaining detections.
3. **Given** a coordinate-only detection that resolves successfully, **When** path calculation completes, **Then** the result is indistinguishable from supplying the same detection in topological form — same anchored netelement, same intrinsic.

---

### Edge Cases

- **Train length offset**: GNSS positions originate from one fixed point on the train (e.g. roof antenna), while detections fire from the leading bow in the direction of travel. The timestamp-based correlation between a GNSS sample and a detection is therefore approximate. The system MUST tolerate a temporal mismatch on the order of seconds without failing or producing incorrect anchoring; it MUST NOT assume the GNSS sample nearest in time to a detection is at the same location as the detection.
- **Linear window broader than presence**: Track-circuit windows typically extend before train arrival and after departure (signaling release delays). The system MUST NOT require the train to remain on the constrained netelement for the entire `[t_from, t_to]` window — only that it visits that netelement during the window.
- **Detection outside GNSS time range**: Discard with warning (applies to both punctual and linear; linear requires both endpoints inside the GNSS range).
- **Coordinate-only punctual unresolvable to a netelement**: Discard with warning.
- **Multiple punctual detections at the same timestamp on different netelements**: Treated as a fatal input error (fail-fast); the system aborts with a clear message identifying the conflicting detections. Same-timestamp detections that agree on netelement are retained (and deduplicated).
- **Detection referencing a `netelement_id` that does not exist in the network**: Treated as a fatal input error (fail-fast, consistent with existing CRS/network validation).
- **Anchored netelements unreachable from each other under network topology**: A warning is emitted; the existing path-calculation gap-filling / bridge-insertion logic determines whether a continuous path can still be produced.
- **Empty detections file**: A warning is emitted; Path calculation proceeds exactly as today (no detections supplied is a valid case).
- **Detections supplied without GNSS data**: Out of scope — the path-calculation engine continues to require GNSS input.

## Requirements *(mandatory)*

### Functional Requirements

#### Inputs

- **FR-001**: System MUST accept an optional file containing punctual train detections alongside the existing GNSS and network inputs.
- **FR-002**: System MUST accept an optional file containing linear train detections alongside the existing GNSS and network inputs.
- **FR-002a**: For each detection input file, the system MUST auto-detect the file format from the file extension: `.geojson` and `.json` MUST be parsed as GeoJSON; `.csv` MUST be parsed as CSV. Any other extension MUST be rejected with a clear error.
- **FR-002b**: System MUST accept both GeoJSON and CSV serializations of punctual and linear detections; both serializations MUST express the same schema (FR-003 / FR-004) and MUST be interchangeable for the same logical input.
- **FR-003**: A punctual detection MUST contain a timestamp (with timezone) and a location specified in either topological form (`netelement_id` plus optional `intrinsic`) or geographic form (`latitude`, `longitude`, `crs`).
- **FR-004**: A linear detection MUST contain `t_from`, `t_to` (both with timezone) and a `netelement_id`; it MAY additionally specify `start_intrinsic` and `end_intrinsic` (each in `[0.0, 1.0]`, with `start_intrinsic ≤ end_intrinsic`).
- **FR-005**: System MUST validate that `t_from ≤ t_to` for every linear detection and reject the file with a clear error otherwise.
- **FR-006**: System MUST validate that every supplied `netelement_id` exists in the supplied network and reject the file with a clear error otherwise.
- **FR-007**: System MUST validate that supplied `intrinsic`, `start_intrinsic`, and `end_intrinsic` values lie in `[0.0, 1.0]` and reject the file with a clear error otherwise.
- **FR-007a**: System MUST reject, with a fatal error identifying the conflicting detections, any input where two or more punctual detections share the same timestamp but reference different `netelement_id`s. Same-timestamp punctual detections that reference the same `netelement_id` MUST be retained and deduplicated.

#### Coordinate-Only Resolution (Punctual)

- **FR-008**: When a punctual detection supplies coordinates instead of a `netelement_id`, the system MUST resolve those coordinates to the nearest netelement once at load time (not during path calculation).
- **FR-009**: When coordinate resolution fails (no netelement within `--cutoff-distance-detections` meters), the system MUST discard the detection and emit a warning identifying the discarded detection by id (or by timestamp + coordinates if no id was supplied).

#### Time-Range Filtering

- **FR-010**: System MUST discard, with a warning, any punctual detection whose timestamp lies strictly before the first GNSS sample or strictly after the last GNSS sample.
- **FR-011**: System MUST discard, with a warning, any linear detection where `t_from` or `t_to` lies outside the GNSS time range (only fully-contained windows are accepted).

#### Path Calculation Influence

- **FR-012**: Punctual detections retained after filtering MUST act as hard anchors: the calculated train path MUST traverse the anchored netelement at the position implied by the detection, overriding any conflicting GNSS-derived candidate.
- **FR-013**: Linear detections retained after filtering MUST constrain path calculation such that the train is on the anchored netelement at some point during `[t_from, t_to]`; the system MUST NOT require continuous occupation across the entire window.
- **FR-014**: Supplied `intrinsic`, `start_intrinsic`, and `end_intrinsic` values MUST be validated (per FR-007) and recorded in the anchor application provenance output (per FR-017), but MUST NOT influence path calculation in this feature; anchoring operates at netelement granularity only. (These fields are reserved for use by a later feature.)
- **FR-015**: System MUST NOT assume that the GNSS sample nearest in time to a detection corresponds to the same physical location as the detection (the train length introduces a position-vs-time offset between the two data sources).
- **FR-016**: Detections MUST influence path calculation only. The subsequent projection of GNSS coordinates onto the calculated path MUST behave identically to the existing behavior and MUST NOT be modified by the presence of detections.

#### Output & Provenance

- **FR-017**: The path-calculation result MUST record which detections were applied as anchors and which were discarded (with reasons), preserving data provenance.
- **FR-018**: System MUST emit a warning, but still produce a path, when supplied anchors require traversing netelements that are not reachable from each other under the network topology (the existing gap-filling mechanism handles continuity).

#### CLI

- **FR-019**: The CLI MUST expose options to supply a punctual-detections file and a linear-detections file independently; supplying neither, either, or both MUST all be valid invocations.
- **FR-020**: The CLI MUST emit, for every supplied detections file, a summary stating how many detections were loaded, how many were applied as anchors, and how many were discarded (broken down by reason).

#### Webapp

- **FR-021**: The path-review webapp MUST render every retained punctual detection as a map marker at its anchored location and every retained linear detection as a highlighted segment over its anchored netelement.
- **FR-022**: The path-review webapp MUST render every discarded detection visually distinguishable from retained detections (e.g. greyed-out / different marker style) so the user can see what was discarded.
- **FR-023**: The path-review webapp MUST provide a per-detection details panel, opened by clicking a marker or highlighted segment, that displays at minimum: detection id (if any), source, timestamp(s), applied-or-discarded status, and — when discarded — the reason.
- **FR-024**: The path-review webapp MUST NOT provide upload, edit, delete, or filter controls for detections in this feature; detections are loaded from the same files supplied to the path-calculation run being reviewed.

### Key Entities

- **Punctual Detection**: A single-instant ground-truth observation that the train was at a known location at a known timestamp. Location is expressed either topologically (`netelement_id` + optional `intrinsic`) or geographically (`latitude`, `longitude`, `crs`); geographic locations are resolved to topological form at load time. The `intrinsic` field, when supplied, is informational metadata only in this feature (see FR-014). Optional metadata: id, source (e.g. "beacon", "detector"), free-form key/value fields.
- **Linear Detection**: An interval observation that the train was somewhere on a known netelement between a `t_from` and `t_to`. Optional `start_intrinsic` and `end_intrinsic`, when supplied, are informational metadata only in this feature (see FR-014). Optional metadata: id, source (e.g. "track_circuit", "axle_counter"), free-form key/value fields.
- **Anchor Application Record** (provenance): For each supplied detection, the system records whether it was applied, discarded (with reason), or resolved (in the coordinate-only case), so that the path's lineage can be inspected after the fact.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For test scenarios over parallel-track sections where GNSS-only path calculation chooses the wrong track, supplying a single correctly-placed punctual detection results in the calculated path traversing the correct track in 100% of cases.
- **SC-002**: For test scenarios crossing GNSS-degraded zones (tunnels) where GNSS-only path calculation produces gaps or wrong netelements, supplying a linear detection covering the degraded zone results in the calculated path including the correct netelement during the corresponding interval in 100% of cases.
- **SC-003**: When detections fall outside the GNSS time range, the system completes path calculation successfully (no failure) and reports each discarded detection in a user-visible warning that identifies the detection.
- **SC-004**: When a coordinate-only punctual detection cannot be resolved to a netelement, the system completes path calculation successfully and reports the discarded detection in a user-visible warning.
- **SC-005**: For a typical workload (≤ 10,000 GNSS positions and ≤ 1,000 detections), path calculation with detections completes in no more than 20% additional wall-clock time compared to the same workload without detections.
- **SC-006**: For a given GNSS log and resulting calculated path, the projected coordinates output is identical between (a) a run without detections and (b) a run with detections that yield the same calculated path — confirming detections do not leak into projection.
- **SC-007**: When a supplied detection references a `netelement_id` that does not exist in the network, the system fails fast with an error message that names the offending detection and the missing netelement id.
- **SC-008**: Users running the CLI with detection files supplied receive, for every input file, a summary line stating how many detections were loaded, how many were applied as anchors, and how many were discarded (broken down by reason).
- **SC-009**: In the path-review webapp, every retained detection appears on the map at its anchored location and every discarded detection is visually distinguishable from retained detections in 100% of cases.
- **SC-010**: In the path-review webapp, clicking any detection marker or highlighted linear segment opens a details panel displaying its id (if any), source, timestamp(s), applied/discarded status, and discard reason (when applicable) in 100% of cases.
- **SC-011**: When two or more punctual detections share the same timestamp but reference different netelements, the system aborts with an error that names the conflicting detections in 100% of cases.

## Assumptions

- The GNSS data and the detection data describe the same train run; the user is responsible for time-aligning their data sources before supplying them.
- Train length introduces an approximate temporal offset between GNSS samples and detections; this offset is handled implicitly by treating linear-detection windows as broader than actual presence and by not assuming exact time-correlation between the two data sources. No explicit train-length parameter is introduced in this feature.
- Linear detection windows from track-circuit or axle-counter systems are consistently broader than (never narrower than) the actual train presence on the netelement.
- All detection timestamps include timezone information, consistent with existing GNSS timestamp requirements (Constitution VI).
- All detection coordinates include CRS information, consistent with existing position requirements (Constitution VII).
- Each detection references at most one netelement. Occupations spanning multiple netelements must be supplied as multiple linear detections.
- Detection file formats are custom CSV/GeoJSON formats defined for this project; no external canonical layout is required to be matched. Both serializations are supported and selected per file by extension (see FR-002a / FR-002b).
- The webapp UI for visualizing detections is in scope for this feature.
- A "soft" / uncertainty-weighted anchoring mode (where detections influence rather than override GNSS-derived candidates) is out of scope and may be added later.
- A "detections-only" path calculation mode (without GNSS input) is out of scope and not planned.

## Configuration Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `--punctual-detections <FILE>` | (none) | Optional input file with punctual detections. |
| `--linear-detections <FILE>` | (none) | Optional input file with linear detections. |
| `--cutoff-distance-detections <DECIMAL>` | 2.5 | Maximum distance in **meters** between a coordinate-only punctual detection and its nearest netelement for resolution to succeed. Detector locations are typically surveyed and accurate, so this cutoff is intentionally tighter than the GNSS-projection `--cutoff-distance`; resolutions beyond this threshold are treated as referring to a detector outside the supplied network and discarded with a warning. |
