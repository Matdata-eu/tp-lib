# Tasks: GNSS Track Axis Projection

**Feature**: 001-gnss-projection | **Date**: 2025-12-12  
**Input**: Design documents from `specs/001-gnss-projection/`  
**Prerequisites**: ✅ plan.md, ✅ spec.md, ✅ research.md, ✅ data-model.md, ✅ contracts/

**Organization**: Tasks organized by user story to enable independent implementation and testing.

**Tests**: Tests are NOT explicitly requested in the specification, so test tasks are OMITTED per template guidelines. Testing will be handled through TDD workflow (write test first, implement to pass).

---

## Task Format

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: User story label (e.g., [US1] for User Story 1)
- **Format**: `- [ ] [TaskID] [P?] [Story?] Description with file path`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and workspace structure. No story label (shared infrastructure).

- [X] T001 Create Rust workspace structure with Cargo.toml at tp-lib/ root defining workspace members [tp-core, tp-cli, tp-py]
- [X] T002 [P] Create tp-core/Cargo.toml with dependencies: geo, proj, rstar, arrow, polars, chrono, geojson, csv, thiserror, serde
- [X] T003 [P] Create tp-cli/Cargo.toml with dependencies: clap, tp-core (workspace dependency)
- [X] T004 [P] Create tp-py/Cargo.toml with dependencies: pyo3, tp-core (workspace dependency)
- [X] T005 [P] Create tp-py/pyproject.toml for Python packaging with maturin build system
- [X] T006 [P] Create .github/workflows/ci.yml for CI/CD pipeline (cargo test + cargo bench + pytest)
- [X] T007 Create tp-core/src/lib.rs with module declarations and public API exports
- [X] T008 [P] Create tp-core/src/models/mod.rs with submodule declarations (gnss, netelement, result)
- [X] T009 [P] Create tp-core/src/projection/mod.rs with submodule declarations (geom, spatial)
- [X] T010 [P] Create tp-core/src/io/mod.rs with submodule declarations (csv, geojson, arrow)
- [X] T011 [P] Create tp-core/src/crs/mod.rs with submodule declarations (transform)
- [X] T012 [P] Create tp-core/src/temporal/mod.rs as placeholder module
- [X] T013 [P] Create tp-core/tests/unit/ directory for unit tests
- [X] T014 [P] Create tp-core/tests/integration/ directory for integration tests
- [X] T015 [P] Create tp-core/benches/ directory for performance benchmarks

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before User Story 1 implementation. No story label (foundational/blocking).

**⚠️ CRITICAL**: This phase includes the FIRST TEST to validate environment setup per user requirement.

### Error Types (Foundation)

- [X] T016 Create tp-core/src/errors.rs with ProjectionError enum using thiserror: InvalidCrs, TransformFailed, InvalidCoordinate, MissingTimezone, EmptyNetwork, InvalidGeometry, CsvError, GeoJsonError, IoError variants

### Basic Environment Validation Test (FIRST TEST - CRITICAL) 🎯

- [X] T017 Create tp-core/tests/unit/projection_basic_test.rs with test_project_point_on_linestring() using hardcoded Point(50.0, 4.0) and LineString[(50.0, 4.0), (51.0, 4.0)] to verify geo crate works without file I/O
- [X] T018 Run `cargo test projection_basic_test` to validate environment setup, ensure test passes before proceeding to implementation

### Core Data Models (Foundation)

- [X] T019 [P] Create tp-core/src/models/gnss.rs with GnssPosition struct (latitude: f64, longitude: f64, timestamp: DateTime<FixedOffset>, crs: String, metadata: HashMap<String, String>)
- [X] T020 [P] Create tp-core/src/models/netelement.rs with Netelement struct (id: String, geometry: LineString<f64>, crs: String)
- [X] T021 [P] Create tp-core/src/models/result.rs with ProjectedPosition struct (original: GnssPosition, projected_coords: Point<f64>, netelement_id: String, measure_meters: f64, projection_distance_meters: f64, crs: String)
- [X] T022 Add validation methods to GnssPosition in tp-core/src/models/gnss.rs: validate_latitude(), validate_longitude(), validate_timezone()
- [X] T023 Add validation methods to Netelement in tp-core/src/models/netelement.rs: validate_id(), validate_geometry() (≥2 points)

### Temporal & CRS Utilities (Foundation)

- [X] T024 Create tp-core/src/temporal/mod.rs with timezone validation utilities: parse_rfc3339_with_timezone(), validate_timezone_present()
- [X] T025 Create tp-core/src/crs/transform.rs with CrsTransformer struct wrapping proj::Proj: new(source_crs, target_crs), transform(Point) → Result<Point>

**Checkpoint**: Foundation ready - User Story 1 implementation can now begin

---

## Phase 3: User Story 1 - Basic GNSS Projection (Priority: P1) 🎯 MVP

**Goal**: Transform noisy GNSS CSV/GeoJSON data into accurate track-aligned positions with netelement IDs and measures.

**Independent Test**: Provide GNSS CSV (lat/lon/timestamp) + railway network GeoJSON (netelement LineStrings), verify output contains projected positions, netelement IDs, and measures for each input.

**Acceptance Scenarios**:
1. CSV + GeoJSON → output with original data + projected coords + netelement ID + measure
2. Positions on single netelement → monotonic measure increase
3. Junction positions → geometrically nearest netelement selected
4. Output record count = input record count (1:1 correspondence)

### Geometric Projection (US1 Core Logic)

- [X] T026 [P] [US1] Implement project_point_onto_linestring() in tp-core/src/projection/geom.rs using geo::algorithm::ClosestPoint to find nearest point on LineString
- [X] T027 [P] [US1] Implement calculate_measure_along_linestring() in tp-core/src/projection/geom.rs to compute distance from linestring start to projected point in meters
- [X] T028 [US1] Implement project_gnss_position() in tp-core/src/projection/geom.rs combining T026 + T027, returning ProjectedPosition with projection_distance_meters

### Spatial Indexing (US1 Performance)

- [X] T029 [P] [US1] Create NetworkIndex struct in tp-core/src/projection/spatial.rs wrapping rstar::RTree<NetelementIndexEntry>
- [X] T030 [US1] Implement build_spatial_index() in tp-core/src/projection/spatial.rs to populate RTree from Vec<Netelement> using bounding boxes
- [X] T031 [US1] Implement find_nearest_netelement() in tp-core/src/projection/spatial.rs using RTree::nearest_neighbor() for O(log n) queries

### Input Parsing (US1 Data Loading)

- [X] T032 [P] [US1] Implement parse_gnss_csv() in tp-core/src/io/csv.rs using csv crate with configurable column names (lat_col, lon_col, time_col), returning Vec<GnssPosition>
- [X] T033 [P] [US1] Implement parse_network_geojson() in tp-core/src/io/geojson.rs using geojson crate, extracting Features with LineString geometry into Vec<Netelement>
- [X] T034 [US1] Add CRS extraction logic to parse_network_geojson() in tp-core/src/io/geojson.rs validating WGS84 per RFC 7946
- [X] T035 [US1] Add validation to parse_gnss_csv() in tp-core/src/io/csv.rs: fail-fast for missing columns, invalid coordinates, missing timezone in timestamps

### Main Processing Pipeline (US1 Orchestration)

- [X] T036 [US1] Create RailwayNetwork struct in tp-core/src/lib.rs wrapping Vec<Netelement> and NetworkIndex with methods: new(), find_nearest(), get_by_id()
- [X] T037 [US1] Implement project_gnss() function in tp-core/src/lib.rs: accept &[GnssPosition], &RailwayNetwork, ProjectionConfig → Result<Vec<ProjectedPosition>>
- [X] T038 [US1] Add CRS transformation logic to project_gnss() in tp-core/src/lib.rs: if GNSS CRS ≠ Network CRS, use CrsTransformer to convert coordinates before projection
- [X] T039 [US1] Add temporal ordering preservation in project_gnss() in tp-core/src/lib.rs ensuring output Vec maintains input timestamp order
- [X] T040 [US1] Add diagnostic warning emission in project_gnss() in tp-core/src/lib.rs: if projection_distance_meters > threshold (default 50m), log to stderr

### Output Formatting (US1 Results)

- [ ] T041 [P] [US1] Implement write_csv() in tp-core/src/io/csv.rs using csv::Writer to serialize Vec<ProjectedPosition> with header: original_lat, original_lon, original_time, projected_lat, projected_lon, netelement_id, measure_meters, projection_distance_meters, crs
- [ ] T042 [P] [US1] Implement write_geojson() in tp-core/src/io/geojson.rs converting Vec<ProjectedPosition> to GeoJSON FeatureCollection with Point geometries and properties

### CLI Interface (US1 User Interaction)

- [ ] T043 [US1] Create tp-cli/src/main.rs with clap CLI definition: --gnss-file, --gnss-crs, --network-file, --output-format (csv|json), --warning-threshold, --lat-col, --lon-col, --time-col
- [ ] T044 [US1] Implement CLI argument validation in tp-cli/src/main.rs: reject --gnss-crs if GNSS input is GeoJSON, require --gnss-crs if CSV
- [ ] T045 [US1] Implement main() pipeline in tp-cli/src/main.rs: parse inputs → build network → project GNSS → write output to stdout, errors to stderr
- [ ] T046 [US1] Add exit code handling in tp-cli/src/main.rs: 0 for success, 1 for validation error, 2 for processing error, 3 for I/O error
- [ ] T047 [US1] Add --help flag documentation in tp-cli/src/main.rs with usage examples and parameter descriptions

### Integration Test (US1 End-to-End Validation)

- [ ] T048 [US1] Create tp-core/tests/integration/pipeline_test.rs with test_csv_to_projection_pipeline() using sample CSV (3 GNSS points) + GeoJSON (2 netelements), verifying output count = input count and all required fields present

### Configuration (US1 Tunables)

- [ ] T049 [US1] Create ProjectionConfig struct in tp-core/src/lib.rs with fields: warning_threshold: f64, transform_crs: bool, implementing Default trait (threshold=50.0, transform=true)

**Checkpoint**: User Story 1 complete - MVP functional with independent testability

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Finalize production-readiness with documentation, performance validation, and additional testing.

### Documentation

- [ ] T050 [P] Add rustdoc comments to all public API functions in tp-core/src/lib.rs with examples
- [ ] T051 [P] Add rustdoc comments to all public structs in tp-core/src/models/*.rs
- [ ] T052 [P] Create README.md at tp-lib/ root with installation, quick start, and examples
- [ ] T053 [P] Create tp-cli/README.md with CLI usage examples and troubleshooting

### Performance Benchmarks

- [ ] T054 [P] Create tp-core/benches/naive_baseline_bench.rs using criterion to benchmark O(n*m) brute-force nearest-netelement search (1000 points × 50 netelements)
- [ ] T055 [P] Create tp-core/benches/projection_bench.rs using criterion to benchmark complete pipeline with RTree spatial indexing (1000 points × 50 netelements), targeting <10s per SC-001
- [ ] T056 Run `cargo bench` and validate SC-001 (<10s for 1000 positions/50 netelements) and SC-006 (10,000+ positions without memory exhaustion)

### Python Bindings (Multi-Language Integration)

- [ ] T057 [P] Create tp-py/src/lib.rs with PyO3 #[pyfunction] wrapper: project_gnss(gnss_file, gnss_crs, network_file, config) → PyResult<Vec<ProjectedPosition>>
- [ ] T058 [P] Implement Python error conversion in tp-py/src/lib.rs: ProjectionError → PyValueError/PyRuntimeError using From<ProjectionError> for PyErr
- [ ] T059 [P] Create tp-py/python/tp_lib/__init__.py exposing project_gnss function with type hints
- [ ] T060 [P] Create tp-py/python/tests/test_projection.py with pytest test cases: test_basic_projection(), test_invalid_crs()

### Additional Testing (Optional Quality Gates)

- [ ] T061 [P] Create tp-core/tests/contract/lib_api_stability_test.rs verifying public API signatures haven't changed (snapshot testing)
- [ ] T062 [P] Create tp-core/tests/unit/gnss_model_test.rs testing GnssPosition validation: invalid lat/lon, missing timezone
- [ ] T063 [P] Create tp-core/tests/unit/crs_transform_test.rs testing CrsTransformer with known coordinate pairs (Belgian Lambert 2008 → WGS84)
- [ ] T064 Create tp-cli/tests/cli_integration_test.rs testing CLI end-to-end: valid input → stdout CSV, invalid CRS → stderr error, missing file → exit code 3

### Logging & Audit Trail

- [ ] T065 Add logging to project_gnss() in tp-core/src/lib.rs using tracing crate: log CRS conversions, netelement assignments, projection calculations per FR-018
- [ ] T066 Configure tracing subscriber in tp-cli/src/main.rs to emit structured logs to stderr

---

## Dependencies & Execution Strategy

### User Story Completion Order

**Only 1 User Story (MVP)**: User Story 1 (P1) is the complete MVP. No additional user stories defined in specification.

```
Phase 1 (Setup) → Phase 2 (Foundational) → Phase 3 (User Story 1) → Phase 4 (Polish)
     ↓                   ↓                           ↓                      ↓
  T001-T015          T016-T025                   T026-T049              T050-T066
```

### Parallel Execution Opportunities

**Phase 1 (Setup)**: Tasks T002-T015 can run in parallel after T001 (workspace structure)

**Phase 2 (Foundational)**:
- T019-T021 (data models) can run in parallel with T024-T025 (utilities)
- T016 (errors) must complete before any task using ProjectionError
- **T017-T018 (FIRST TEST) must complete and pass before any implementation begins**

**Phase 3 (User Story 1)** - Maximum parallelization within subsystems:
- **Geometric Projection** (T026-T028): Can start immediately after T016-T023
- **Spatial Indexing** (T029-T031): Can start immediately after T016-T023
- **Input Parsing** (T032-T035): Can start immediately after T016-T023
- **Output Formatting** (T041-T042): Can start immediately after T021 (ProjectedPosition struct)
- **Configuration** (T049): Can start immediately after T016

Sequential dependencies within US1:
- T036 (RailwayNetwork) requires T029-T031 (spatial indexing)
- T037-T040 (project_gnss pipeline) requires T026-T036
- T043-T047 (CLI) requires T037-T040 (pipeline)
- T048 (integration test) requires T037-T047 (complete pipeline)

**Phase 4 (Polish)**: Most tasks can run in parallel except T065-T066 (logging) which may require tracing crate addition to Cargo.toml

### Critical Path (Longest Sequential Chain)

```
T001 → T002 → T016 → T017-T018 (FIRST TEST GATE) → T019-T023 → T026-T028 → T036 → T037-T040 → T043-T047 → T048
```

**Estimated Timeline**: 12-15 days for complete MVP (Phase 1-4)

---

## Implementation Strategy

### MVP-First Approach

**Phase 3 (User Story 1) is the complete MVP.** Deliver this incrementally:

1. **Milestone 1** (Days 1-2): Setup + Foundational (T001-T025) → FIRST TEST PASSING
2. **Milestone 2** (Days 3-5): Core projection logic (T026-T031) → Point-on-linestring works
3. **Milestone 3** (Days 6-8): Input/Output + Pipeline (T032-T042, T036-T040) → End-to-end CSV → CSV
4. **Milestone 4** (Days 9-10): CLI (T043-T048) → Usable command-line tool
5. **Milestone 5** (Days 11-12): Polish (T050-T066) → Production-ready

### Testing Strategy

**TDD Workflow** (Per Constitution Principle IV):
1. Write test for task (e.g., T026 projection test)
2. Run test → RED (failing)
3. Implement feature (T026 code)
4. Run test → GREEN (passing)
5. Refactor if needed
6. Commit

**Critical First Test** (T017-T018):
- MUST pass before ANY implementation begins
- Validates: geo crate installed, projection math works, test framework operational
- Hardcoded data (no file I/O) ensures minimal dependencies

### Validation Checkpoints

After completing each phase, verify:

**Phase 1**: `cargo build --workspace` succeeds  
**Phase 2**: `cargo test projection_basic_test` passes (FIRST TEST)  
**Phase 3**: Integration test passes, CLI runs successfully with sample data  
**Phase 4**: Benchmarks meet SC-001 (<10s), documentation complete

---

## Task Summary

**Total Tasks**: 66  
**Setup Phase**: 15 tasks (T001-T015)  
**Foundational Phase**: 10 tasks (T016-T025) - **includes FIRST TEST**  
**User Story 1 Phase**: 24 tasks (T026-T049)  
**Polish Phase**: 17 tasks (T050-T066)

**Parallel Opportunities**: ~40 tasks marked with [P] can run in parallel (60% of tasks)

**Independent Test Criteria for US1**:
- ✅ Input: GNSS CSV with 3 positions + Network GeoJSON with 2 netelements
- ✅ Output: 3 projected positions with all required fields (projected coords, netelement IDs, measures)
- ✅ Validation: Output count = input count, measures ≥ 0, projection distances ≥ 0

**MVP Scope**: User Story 1 (Phase 3) delivers complete MVP meeting all acceptance scenarios.

---

## Format Validation

✅ All tasks follow checklist format: `- [ ] [TaskID] [P?] [Story?] Description with file path`  
✅ Sequential Task IDs: T001-T066  
✅ [P] markers present for parallelizable tasks  
✅ [US1] labels for User Story 1 tasks only  
✅ File paths included in all implementation tasks  
✅ Setup/Foundational phases have NO story labels (shared infrastructure)  
✅ FIRST TEST (T017-T018) clearly marked as CRITICAL gate
