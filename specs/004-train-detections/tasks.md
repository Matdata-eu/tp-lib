# Tasks: Absolute Train Position Detections (Punctual & Linear)

**Feature**: `004-train-detections`
**Input**: `specs/004-train-detections/`
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, contracts/ ✓, quickstart.md ✓

---

## Format: `[ID] [P?] [Story] Description with file path`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: User story label — US1, US2, US3, US4 (setup/foundational/polish phases carry no label)
- **TDD Rule**: All test tasks MUST be written first, confirmed failing, then implemented against

---

## Phase 1: Setup (Module Registration)

**Purpose**: Declare every new module in the Rust workspace so `cargo check` resolves paths before
any implementation starts. No logic — stubs only.

- [X] T001 Register `pub mod detections` in `tp-core/src/lib.rs`; create stub `tp-core/src/detections.rs` and directory `tp-core/src/detections/` with empty `error.rs`, `load.rs`, `validate.rs`, `filter.rs`, `resolve.rs`
- [X] T002 [P] Register `pub mod detection` and `pub mod detection_record` in `tp-core/src/models.rs`; create stub files `tp-core/src/models/detection.rs` and `tp-core/src/models/detection_record.rs`
- [X] T003 [P] Create stub `tp-core/src/io/csv/detections.rs` and `tp-core/src/io/geojson/detections.rs`; add `pub mod detections;` in the parent `csv` and `geojson` module files respectively

**Checkpoint**: `cargo check -p tp-lib-core` passes with all new modules visible but empty.

---

## Phase 2: Foundational (Data Types + Parsers + Shared Pipeline)

**Purpose**: Define all shared types and implement the load/validate/filter pipeline that blocks
both US1 and US2. No user story can begin until this phase is complete.

**⚠️ CRITICAL**: Complete before any user story implementation.

### Tests (TDD — write FIRST, run `cargo test` to confirm FAILURE)

- [X] T004 [P] Write integration tests for detection loading (FR-001, FR-002, FR-002a, FR-002b, FR-005..FR-007a) in `tp-core/tests/detections_load.rs` — cover: extension dispatch (`.csv`, `.geojson`, `.json`, unsupported), valid punctual CSV round-trip, valid linear GeoJSON round-trip, schema errors (`InvalidSchema`), invalid timestamp (`InvalidTimestamp`), conflicting punctual detections (`ConflictingDetections`), duplicate deduplication; confirm `cargo test --test detections_load` FAILS
- [X] T005 [P] Write integration tests for time-range filtering (FR-010, FR-011) in `tp-core/tests/detections_filter.rs` — cover: punctual strictly before GNSS window discarded, punctual strictly after discarded, linear window not fully contained discarded, linear window fully contained accepted, `DiscardReason::OutOfTimeRange` fields correct; confirm `cargo test --test detections_filter` FAILS

### Implementation

- [X] T006 Implement `DetectionError` enum (11 variants: `UnsupportedExtension`, `InvalidSchema`, `Parse`, `InvalidTimestamp`, `InvalidIntrinsic`, `MissingCrs`, `ConflictingDetections`, `InvalidTimeRange`, `UnknownNetelement`, `DuplicateResolution`, `Io`) with `thiserror::Error` in `tp-core/src/detections/error.rs`
- [X] T007 [P] Implement `Detection`, `PunctualDetection`, `LinearDetection`, `TopologicalLocation`, `GeographicLocation`, and `ResolvedAnchor` (both variants) in `tp-core/src/models/detection.rs` — all fields from data-model.md; derive `Debug`, `Clone`, `PartialEq`, `serde::Serialize`, `serde::Deserialize`
- [X] T008 [P] Implement `DetectionRecord`, `DetectionKind`, `TimestampOrRange`, `DetectionStatus`, and `DiscardReason` (5 variants) in `tp-core/src/models/detection_record.rs` — all fields from data-model.md; derive `Debug`, `Clone`, `PartialEq`, `serde::Serialize`, `serde::Deserialize`
- [X] T009 Implement CSV parser for punctual and linear detections in `tp-core/src/io/csv/detections.rs` — required columns per `contracts/detections-csv.md`; timezone-aware timestamp parsing via `chrono::DateTime<FixedOffset>` (reject naive → `InvalidTimestamp`); `MissingCrs` when coordinate row has no `crs`; unknown columns captured in `metadata`; return `Vec<Detection>`
- [X] T010 [P] Implement GeoJSON parser for punctual and linear detections in `tp-core/src/io/geojson/detections.rs` — `FeatureCollection` input per `contracts/detections-geojson.md`; `properties.kind` dispatch (`"punctual"` / `"linear"`); optional `Point` geometry for coordinate-only punctual; `MissingCrs` when geometry present but no `crs` property; unknown properties captured in `metadata`; return `Vec<Detection>`
- [X] T011 Implement extension-based format dispatch in `tp-core/src/detections/load.rs` — `.csv` → CSV parser; `.geojson` / `.json` → GeoJSON parser; any other extension → `DetectionError::UnsupportedExtension`; pass `source_file` and `source_row` through to returned `Detection` records for provenance (D1)
- [X] T012 Implement validation pipeline in `tp-core/src/detections/validate.rs` — FR-005: `t_from ≤ t_to` or `InvalidTimeRange`; FR-006: `netelement_id` exists in supplied network or `UnknownNetelement` (fatal); FR-007: `intrinsic` / `start_intrinsic` / `end_intrinsic` ∈ [0, 1] or `InvalidIntrinsic`; FR-007a: same timestamp + same netelement → silently keep first and record `DuplicateOfPriorDetection`; same timestamp + different netelement → `ConflictingDetections` (fatal, D4)
- [X] T013 Implement time-range filter in `tp-core/src/detections/filter.rs` — FR-010: discard punctual if `timestamp < gnss_first || timestamp > gnss_last`; FR-011: discard linear if `t_to < gnss_first || t_from > gnss_last` (no clipping, D5); discarded detections produce `DetectionRecord` with `DetectionStatus::Discarded { reason: DiscardReason::OutOfTimeRange { gnss_first, gnss_last } }`; emit warning per discarded detection
- [X] T014 Extend `PathConfig` with two additive fields in `tp-core/src/path.rs`: `pub anchors: Vec<ResolvedAnchor>` (default `vec![]`) and `pub detection_cutoff_distance: f64` (default `2.5`); add `Default` impl values; existing callers using `..Default::default()` must be unaffected (backward-compatible)
- [X] T015 Extend `PathResult` with additive field `pub detection_provenance: Vec<DetectionRecord>` (default empty) in `tp-core/src/path.rs`; update all `PathResult` struct literals and constructors throughout `tp-core/src/` to include the new field

**Checkpoint**: `cargo test --test detections_load` and `cargo test --test detections_filter` now pass. `cargo check --workspace` clean.

---

## Phase 3: User Story 1 — Anchor Path with Punctual Detections (Priority: P1) 🎯 MVP

**Goal**: A topological punctual detection (`netelement_id` + `timestamp`) forces the Viterbi path
through the specified netelement at the nearest GNSS index, overriding any GNSS-derived candidate.
Every detection (applied and discarded) appears in `PathResult.detection_provenance`.

**Independent Test**: Supply a GNSS log over a parallel-track section and one punctual detection on
the correct track at an in-window timestamp. Verify the calculated path uses the detected netelement
(SC-001 parallel-track disambiguation).

### Tests (TDD — write FIRST, run `cargo test` to confirm FAILURE)

- [X] T016 [P] [US1] Write integration tests for punctual anchor injection (SC-001, FR-012) in `tp-core/tests/detections_anchor.rs` — cover: topological punctual applied → correct netelement chosen over GNSS-conflicting candidate, out-of-window punctual discarded + path unaffected, multiple punctual anchors sorted correctly; confirm `cargo test --test detections_anchor` FAILS
- [X] T017 [P] [US1] Write integration tests for provenance output (FR-017, D9) in `tp-core/tests/detections_provenance.rs` — cover: applied detection produces `DetectionStatus::Applied { netelement_id }` record with correct `source_file`/`source_row`/`kind`/`timestamp`, discarded detection produces `DetectionStatus::Discarded { reason }` record, `detection_provenance` length equals total input count; confirm `cargo test --test detections_provenance` FAILS

### Implementation

- [X] T018 [US1] Implement topological punctual → `ResolvedAnchor::Punctual` conversion in `tp-core/src/detections/resolve.rs` — find `gnss_index` as argmin of `|gnss[i].timestamp − detection.timestamp|` per D7; store `netelement_id` and `intrinsic`; return `(ResolvedAnchor, DetectionRecord)` pair
- [X] T019 [US1] Implement forced-state anchor injection for `ResolvedAnchor::Punctual` in `tp-core/src/path/viterbi.rs` (FR-012) — at each anchored `gnss_index`: replace candidate set with a single state for `netelement_id`; set emission probability to `1.0` for that state, prune all others; forward variable initialised exclusively from the forced state at that step
- [X] T020 [US1] Implement `prepare_detections` public entry point in `tp-core/src/detections.rs` — orchestrate: `load::load_detections` → `validate::validate` → `filter::filter_by_time_range` → `resolve::resolve_topological_punctual` → sort anchors by `gnss_index`; return `(Vec<ResolvedAnchor>, Vec<DetectionRecord>)`
- [X] T021 [US1] Add `--punctual-detections <FILE>` CLI flag in `tp-cli/src/main.rs` — call `prepare_detections` with the network and GNSS observations, populate `PathConfig.anchors` and `PathResult.detection_provenance`; emit summary line to stderr per FR-020: `"detections: N applied, M discarded (breakdown)"` (D10)
- [X] T022 [P] [US1] Write CLI contract tests for `--punctual-detections` flag and stderr summary in `tp-cli/tests/cli_detections.rs` — cover: flag absent (no detections, normal run), flag with valid CSV applies anchor, flag with out-of-window detection produces discard summary line (SC-008, FR-020)

**Checkpoint**: `cargo test --test detections_anchor` (punctual cases), `cargo test --test detections_provenance`, and `cargo test --test cli_detections` (punctual cases) all pass. MVP deliverable functional.

---

## Phase 4: User Story 2 — Anchor Path with Linear Detections (Priority: P1)

**Goal**: A linear detection (`netelement_id`, `t_from`, `t_to`) restricts Viterbi candidates to the
given netelement for every GNSS index whose timestamp falls within `[t_from, t_to]`, without
requiring continuous occupation across the entire window.

**Independent Test**: Supply a GNSS log crossing a tunnel and a linear detection covering the tunnel
netelement during that time window. Verify the path includes the correct netelement for the window
(SC-002 tunnel/GNSS-degraded zone).

### Tests (TDD — write FIRST, run `cargo test` to confirm FAILURE on new cases)

- [X] T023 [P] [US2] Extend `tp-core/tests/detections_anchor.rs` with linear anchor cases (SC-002, FR-013) — cover: linear window active → only anchored netelement in candidate set, linear window broader than presence → succeeds (D5), linear out-of-window discarded, linear and punctual anchors applied simultaneously; confirm new test cases FAIL
- [X] T024 [P] [US2] Extend `tp-core/tests/detections_filter.rs` with linear filter edge cases — cover: `t_from` inside window but `t_to` outside → discarded, `t_to` inside but `t_from` outside → discarded, both endpoints on boundary → accepted; confirm new test cases FAIL

### Implementation

- [X] T025 [US2] Implement window-restricted candidate filtering for `ResolvedAnchor::Linear` in `tp-core/src/path/candidate.rs` (FR-013) — function that, given a `gnss_index` and a `ResolvedAnchor::Linear`, returns `true` if the index is within `gnss_range` and the candidate netelement matches `netelement_id`; used by the Viterbi step
- [X] T026 [US2] Wire `ResolvedAnchor::Linear` into the Viterbi step loop in `tp-core/src/path/viterbi.rs` — at each step, check all linear anchors whose `gnss_range` contains the current index; filter candidate set to only those on the anchored netelement using `candidate.rs` helper; normal Viterbi scoring proceeds on the filtered set
- [X] T027 [US2] Extend `prepare_detections` in `tp-core/src/detections.rs` to produce `ResolvedAnchor::Linear` — map `t_from`/`t_to` to `gnss_range: RangeInclusive<usize>` (all GNSS indices `i` where `gnss[i].timestamp ∈ [t_from, t_to]` per D7); merge with punctual anchors into single `Vec<ResolvedAnchor>` sorted by first index
- [X] T028 [US2] Add `--linear-detections <FILE>` CLI flag in `tp-cli/src/main.rs`; merge linear anchors into `PathConfig.anchors` alongside punctual anchors; extend summary line to cover both files (FR-019, FR-020)

**Checkpoint**: `cargo test --test detections_anchor` (all cases) and `cargo test --test detections_filter` (all cases) pass. `--punctual-detections` and `--linear-detections` work independently and in combination. `cargo test --workspace` clean.

---

## Phase 5: User Story 3 — Coordinate-Only Punctual Detections (Priority: P2)

**Goal**: A punctual detection with only `(lat, lon, crs)` (no `netelement_id`) is resolved at load
time to the nearest netelement via the R-tree. If within `--cutoff-distance-detections` metres,
it is treated identically to a topological punctual detection. If not, it is discarded with a
warning.

**Independent Test**: Supply a punctual detection file with a `(lat, lon, crs)` point surveyed near
a known netelement. Verify it resolves to that netelement and anchors correctly. Supply a second
detection farther than the cutoff; verify it is discarded with `OutOfReach` (SC-004, FR-009).

### Tests (TDD — write FIRST, run `cargo test` to confirm FAILURE)

- [X] T029 [P] [US3] Write integration tests for coordinate resolution (FR-008, FR-009, SC-004) in `tp-core/tests/detections_resolve.rs` — cover: point within cutoff → resolves to correct netelement + `DetectionStatus::Resolved { netelement_id, distance_m }` in provenance → anchor applied; point beyond cutoff → `DiscardReason::OutOfReach { nearest_distance_m, cutoff_m }`; missing `crs` field → `DetectionError::MissingCrs`; resolved result produces same path as topologically-equivalent detection; confirm `cargo test --test detections_resolve` FAILS

### Implementation

- [X] T030 [US3] Implement coordinate-to-netelement resolution in `tp-core/src/detections/resolve.rs` (FR-008, FR-009) — reproject `(lat, lon)` from `crs` to the network CRS using `proj4rs`; call `RailwayNetwork::find_nearest()` (R-tree, O(log n), D3); if perpendicular distance ≤ `detection_cutoff_distance` → return `ResolvedAnchor::Punctual` with resolved `netelement_id` and computed `intrinsic`, record `DetectionStatus::Resolved { netelement_id, distance_m }` in provenance before converting to anchor; if distance > cutoff → record `DiscardReason::OutOfReach { nearest_distance_m, cutoff_m }` and emit warning
- [X] T031 [US3] Add `--cutoff-distance-detections <DECIMAL>` CLI flag (default `2.5`) in `tp-cli/src/main.rs`; pass value into `PathConfig.detection_cutoff_distance` (FR-003b)
- [X] T032 [US3] Wire coordinate resolution into `prepare_detections` in `tp-core/src/detections.rs` — after `validate`, inspect each `Detection::Punctual` with `coordinates.is_some()` and `location.is_none()`; call `resolve::resolve_coordinate_punctual`; on `OutOfReach` emit warning and add `Discarded` record; on success, convert to `ResolvedAnchor::Punctual` identical to topological path

**Checkpoint**: `cargo test --test detections_resolve` passes. Coordinate-only detections produce identical anchoring to topological equivalent. `cargo test --workspace` clean.

---

## Phase 6: User Story 4 — Webapp Detection Visualization (Priority: P2)

**Goal**: The path-review webapp renders every retained detection on the map (circle marker for
punctual, highlighted segment for linear) and every discarded detection in a visually distinct
style. Clicking any detection opens a read-only details panel.

**Independent Test**: Load a path result JSON (containing both applied and discarded detections in
`detection_provenance`) in the webapp. Verify detections appear on the map and clicking a marker
opens the details panel showing correct id, source, timestamp(s), status, and reason (SC-009,
SC-010, FR-021..FR-024).

### Tests (TDD — write FIRST, run `cargo test` to confirm FAILURE)

- [X] T033 [P] [US4] Write contract tests for `GET /api/detections` per `contracts/webapp-detections-api.md` in `tp-webapp/tests/api_detections.rs` — cover: response shape `{ punctual: [...], linear: [...], discarded: [...] }`, applied punctual has `netelement_id`/`timestamp`/`status`/`id`/`source`, discarded has `reason`, `404` when no path result loaded, `200` with empty arrays when path has no detections; confirm `cargo test --test api_detections` FAILS

### Implementation

- [X] T034 [US4] Implement `GET /api/detections` handler in `tp-webapp/src/server/detections.rs` — read `PathResult.detection_provenance` from the persisted path result; partition records into applied punctual, applied linear, and discarded; serialize per `contracts/webapp-detections-api.md` as JSON response; include `netelement_id`, `timestamp(s)`, `status`, `reason` (when discarded), `id`, `source`, and `metadata` fields
- [X] T035 [US4] Register `GET /api/detections` route in `tp-webapp/src/server.rs`; wire handler from `server/detections.rs` (D11)
- [X] T036 [P] [US4] Implement Leaflet detections layer in `tp-webapp/static/js/detections.js` — `fetch("/api/detections")` on load; applied punctual → filled `L.circleMarker`; applied linear → semi-transparent `L.polyline` along netelement; discarded → muted/dashed/hollow style per FR-022; click on any marker/polyline → open details panel with id, source, timestamp(s), status, reason (FR-021..FR-023, D11)
- [X] T037 [US4] Add detections layer toggle checkbox and details panel DOM structure to `tp-webapp/static/index.html` — layer toggle in map controls; right-sidebar details panel with fields: id, source, timestamp(s), applied/discarded status, discard reason (when applicable), resolved `netelement_id` + `intrinsic`, raw `metadata` key/value table (FR-023, FR-024 read-only)

**Checkpoint**: `cargo test --test api_detections` passes. Manual quickstart.md scenario with detections renders correctly in the webapp.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Performance verification, coverage gate, and documentation completeness.

- [X] T038 [P] Implement Criterion benchmark for SC-005 in `tp-core/benches/detections_overhead.rs` — measure wall-clock time for path calculation with and without 1,000 detections over a 10,000-sample GNSS log; assert overhead ≤ 20%; run `cargo bench --bench detections_overhead`
- [X] T039 Run `cargo llvm-cov --package tp-lib-core` and verify 100% line coverage across all new modules: `tp-core/src/detections/`, `tp-core/src/models/detection.rs`, `tp-core/src/models/detection_record.rs`, `tp-core/src/io/csv/detections.rs`, `tp-core/src/io/geojson/detections.rs`
  - Result (81 tests in `tp-core/tests/detections_coverage.rs`, all passing):
    - `models/detection.rs`: 100% lines
    - `detections.rs`: 100% lines
    - `detections/load.rs`: 100% lines
    - `detections/filter.rs`: 100% lines
    - `detections/validate.rs`: 100% lines
    - `detections/anchor.rs`: 98.23% lines
    - `io/geojson/detections.rs`: 96.04% lines
    - `io/csv/detections.rs`: 94.78% lines
    - `detections/resolve.rs`: 93.28% lines
  - Remaining uncovered lines are defensive: `Result::map_err` on infallible operations (CRS already validated, geojson crate pre-validated coordinate-array shapes), `_ => {}` no-op match arms for tied-distance ties on identical netelements, and unreachable fallthrough returns after exhaustive matches. `models/detection_record.rs` does not exist as a separate file; `DetectionRecord` lives in `models/detection.rs` (100% covered).
- [X] T040 [P] Update `--punctual-detections`, `--linear-detections`, and `--cutoff-distance-detections` flag descriptions in `tp-cli/README.md` and CLI `--help` text; ensure summary line format documented
- [X] T041 Run quickstart.md validation scenarios end-to-end: create `test-data/sample_detections_punctual.csv` and `test-data/sample_detections_linear.geojson` per quickstart.md examples; run all three quickstart examples and confirm expected stderr summaries and path results
  - Result: All three scenarios executed against the existing fixtures (`test-data/sample_gnss.geojson`, `test-data/sample_network.geojson`, `test-data/sample_detections_punctual.csv`, `test-data/sample_detections_linear.geojson`).
    - Scenario 1 (punctual CSV on `NE001` at `2024-01-15T10:30:05+01:00`): exit 0, stderr `detections: 1 applied, 0 discarded`.
    - Scenario 2 (linear GeoJSON on `NE001`, `10:30:00..10:30:10+01:00`): exit 0, stderr `detections: 1 applied, 0 discarded`.
    - Scenario 3 (combined punctual + linear, `--cutoff-distance-detections 2.5`): exit 0, stderr `detections: 2 applied, 0 discarded`.
  - Fixtures: added `id` property mirroring `netelement_id` to the two LineString features in `test-data/sample_network.geojson` (the network loader requires a top-level `id` in feature properties; sample fixture predated this requirement).
  - quickstart.md updated to reflect the actual CLI surface: package name `tp-lib-cli` (not `tp-cli`), subcommand `calculate-path` (not `calculate`), output flag `-o` / `--output` (not `--json-output`), timestamps and netelement ids matching the shipped fixtures (`2024-01-15T10:30:00+01:00` and `NE001`).
  - Note: `PathResult.detection_provenance` is populated in memory but not yet serialized into the GeoJSON path output — flagged in quickstart.md for follow-up.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — start immediately
- **Phase 2 (Foundational)**: Depends on Phase 1 — **BLOCKS all user story phases**
- **Phase 3 (US1, P1)**: Depends on Phase 2 — independent of other stories
- **Phase 4 (US2, P1)**: Depends on Phase 2 — independent of US1; touches different Viterbi integration point
- **Phase 5 (US3, P2)**: Depends on Phase 3 — extends the punctual resolution path
- **Phase 6 (US4, P2)**: Depends on Phase 3 + Phase 4 — requires provenance populated by both punctual and linear anchors
- **Phase 7 (Polish)**: Depends on Phase 3 through Phase 6

### User Story Dependencies

| Story | Depends On | Can Parallel With |
|-------|-----------|-------------------|
| US1 (P1) | Phase 2 complete | US2 (different Viterbi changes) |
| US2 (P1) | Phase 2 complete | US1 (different Viterbi changes) |
| US3 (P2) | US1 complete (uses same punctual resolve path) | US2 (no shared files) |
| US4 (P2) | US1 + US2 complete (provenance needs both kinds) | US3 tail (JS/HTML vs Rust) |

### Within Each User Story

1. Write test tasks (must fail) before any implementation
2. Implement models / types first (if needed beyond Phase 2)
3. Implement core logic (resolve / viterbi changes)
4. Implement orchestration wiring (detections.rs)
5. Implement CLI integration (tp-cli/src/main.rs)
6. Confirm all tests pass

---

## Parallel Execution Examples

### Example: Phase 2 (Foundational) with two agents

```
Agent A: T004 (write load tests) → T006 (DetectionError) → T009 (CSV parser) →
         T011 (load.rs dispatch) → T012 (validate.rs) → T013 (filter.rs) → T014 (PathConfig)
Agent B: T005 (write filter tests) → T007 (detection.rs types) → T008 (detection_record.rs types)
         → T010 (GeoJSON parser) → T015 (PathResult)
```

### Example: US1 ∥ US2 after Phase 2 complete

```
Agent A: Phase 3 (US1) — T016→T017 (TDD) → T018→T019 (Viterbi forced-state) →
         T020→T021 (orchestration + CLI) → T022 (CLI tests)
Agent B: Phase 4 (US2) — T023→T024 (TDD) → T025 (candidate.rs) → T026 (viterbi.rs linear) →
         T027→T028 (orchestration + CLI)
```

### Example: Sequential TDD for User Story 1

```powershell
# Step 1: Write tests (TDD)
# T016: edit tp-core/tests/detections_anchor.rs
# T017: edit tp-core/tests/detections_provenance.rs
cargo test --test detections_anchor   # must FAIL
cargo test --test detections_provenance  # must FAIL

# Step 2: Implement
# T018: tp-core/src/detections/resolve.rs
# T019: tp-core/src/path/viterbi.rs
# T020: tp-core/src/detections.rs
# T021: tp-cli/src/main.rs

# Step 3: Verify
cargo test --test detections_anchor   # must PASS
cargo test --test detections_provenance  # must PASS
cargo test --test cli_detections       # must PASS (T022)
```
