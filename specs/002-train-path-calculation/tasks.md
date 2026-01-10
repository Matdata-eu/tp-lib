# Tasks: Continuous Train Path Calculation with Network Topology

**Feature**: 002-train-path-calculation  
**Input**: Design documents from `/specs/002-train-path-calculation/`  
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests are included per constitution requirement (TDD is NON-NEGOTIABLE).  
**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Add petgraph ~1.0 dependency to tp-core/Cargo.toml with MIT OR Apache-2.0 license
- [X] T002 [P] Create tp-core/src/path.rs as public API module with mod declarations
- [X] T003 [P] Create tp-core/src/path/ directory for implementation modules
- [X] T004 [P] Create tp-core/tests/contract/path_api_contract.rs test file structure
- [X] T005 [P] Create tp-core/tests/integration/path_calculation_test.rs test file structure
- [X] T006 [P] Create tp-core/tests/unit/path_candidate_test.rs test file structure
- [X] T007 [P] Create tp-core/tests/unit/path_probability_test.rs test file structure
- [X] T008 [P] Create tp-core/tests/unit/path_construction_test.rs test file structure
- [X] T009 [P] Create tp-core/benches/path_calculation_bench.rs benchmark file
- [X] T010 [P] Create test fixtures directory tp-core/tests/fixtures/train_path/

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Models (Data Structures)

- [X] T011 [P] Create tp-core/src/models/netrelation.rs with NetRelation struct (id, from_netelement_id, to_netelement_id, position_on_a, position_on_b, navigable_forward, navigable_backward)
- [X] T012 [P] Create tp-core/src/models/train_path.rs with TrainPath struct (id, segments, mode, overall_probability, metadata)
- [X] T013 [P] Create AssociatedNetElement struct in tp-core/src/models/train_path.rs (netelement_id, begin_intrinsic, end_intrinsic, length, probability)
- [X] T014 [P] Extend GnssPosition in tp-core/src/models/gnss.rs with optional heading and distance fields
- [X] T015 [P] Create GnssNetElementLink struct in tp-core/src/models/train_path.rs for candidate projections
- [X] T016 Add NetRelation module to tp-core/src/models.rs public exports
- [X] T017 Add TrainPath and AssociatedNetElement to tp-core/src/models.rs public exports

### Graph Infrastructure

- [X] T018 Create tp-core/src/path/graph.rs with NetelementSide struct (netelement_id, position: 0|1)
- [X] T019 Implement build_topology_graph() in tp-core/src/path/graph.rs to create petgraph DiGraph from netelements and netrelations
- [X] T020 Write unit test for NetelementSide node creation in tests/unit/path_construction_test.rs
- [X] T021 Write unit test for internal edge creation (start→end, end→start) in tests/unit/path_construction_test.rs
- [X] T022 Write unit test for netrelation connection edge creation in tests/unit/path_construction_test.rs

### Validation and Error Handling

- [X] T023 Extend ProjectionError enum in tp-core/src/errors.rs with PathCalculationFailed, NoNavigablePath, InvalidNetRelation variants
- [X] T024 Implement NetRelation::validate() method for position value validation (0 or 1 only)
- [X] T025 Implement NetRelation self-reference check (from != to) validation
- [X] T026 Write unit test for NetRelation validation rules in tests/unit/path_construction_test.rs
- [X] T026a Implement validate_netrelation_references() checking elementA/B IDs exist in netelements collection (FR-006a)
- [X] T026b Write unit test for invalid netelement reference handling and warning logs in tests/unit/path_construction_test.rs

### I/O Extensions

- [X] T027 [P] Extend GeoJSON parser in tp-core/src/io/geojson.rs to parse netrelations from features with type="netrelation"
- [X] T028 [P] Implement TrainPath GeoJSON serialization in tp-core/src/io/geojson.rs (FeatureCollection with segments)
- [X] T029 [P] Implement TrainPath CSV serialization in tp-core/src/io/csv.rs (one row per segment)
- [X] T030 [P] Implement TrainPath CSV deserialization for reading pre-calculated paths
- [X] T031 Write integration test for netrelation GeoJSON parsing in tests/integration/path_calculation_test.rs
- [X] T032 Write integration test for TrainPath serialization roundtrip in tests/integration/path_calculation_test.rs

### Configuration

- [X] T033 [P] Create PathConfig struct in tp-core/src/path.rs (distance_scale, heading_scale, cutoff_distance, heading_cutoff, probability_threshold, resampling_distance, max_candidates)
- [X] T034 [P] Implement PathConfig::default() with documented default values (distance_scale: 10.0, heading_scale: 2.0, cutoff_distance: 50.0, heading_cutoff: 5.0, probability_threshold: 0.25, max_candidates: 3)
- [X] T035 [P] Create PathConfigBuilder with fluent API and validation
- [X] T036 [P] Create PathResult struct (path: Option<TrainPath>, mode: PathCalculationMode enum, projected_positions, warnings)
- [X] T037 [P] Create PathCalculationMode enum (TopologyBased, FallbackIndependent) in tp-core/src/path.rs
- [X] T038 Write contract test for PathConfig defaults in tests/contract/path_api_contract.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Calculate Train Path from GNSS Data (Priority: P1) 🎯 MVP

**Goal**: Calculate the most probable continuous path through the rail network based on GNSS data and topology.

**Independent Test**: Provide GNSS coordinates and network with netrelations, verify output path is continuous and all connections are navigable.

### Tests for User Story 1 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T039 [P] [US1] Write integration test for successful path calculation with simple linear path in tests/integration/path_calculation_test.rs
- [X] T040 [P] [US1] Write integration test for path calculation with junction (3 candidate branches) in tests/integration/path_calculation_test.rs
- [X] T041 [P] [US1] Write integration test for heading filtering (exclude segments with >5° difference) in tests/integration/path_calculation_test.rs
- [X] T042 [P] [US1] Write integration test for selecting highest probability path from multiple candidates in tests/integration/path_calculation_test.rs
- [X] T043 [P] [US1] Write contract test verifying calculate_train_path() signature and error types in tests/contract/path_api_contract.rs

### Implementation for User Story 1

#### Phase 1: Candidate Selection Module

- [X] T044 [P] [US1] Create tp-core/src/path/candidate.rs module file
- [X] T045 [P] [US1] Implement find_candidate_netelements() using NetworkIndex (reuse from projection) with cutoff_distance filter
- [X] T046 [US1] Implement heading calculation at projection point on linestring (reuse from projection/geom.rs)
- [X] T047 [US1] Implement heading_difference() considering 180° equivalence (forward vs backward on same track)
- [X] T048 [US1] Write unit test for candidate selection within cutoff distance in tests/unit/path_candidate_test.rs
- [X] T049 [US1] Write unit test for heading difference calculation with 180° handling in tests/unit/path_candidate_test.rs

#### Phase 2: GNSS-Level Probability

- [X] T050 [P] [US1] Create tp-core/src/path/probability.rs module file
- [X] T051 [P] [US1] Implement calculate_distance_probability() with exponential decay formula exp(-distance/distance_scale)
- [X] T052 [P] [US1] Implement calculate_heading_probability() with exponential decay exp(-heading_diff/heading_scale) and heading_cutoff rejection
- [X] T053 [US1] Implement calculate_combined_probability() as product of distance and heading probabilities
- [X] T054 [US1] Implement assign_positions_to_netelements() mapping each GNSS position to candidate netelements with probabilities
- [X] T055 [P] [US1] Write unit test for distance probability formula validation (0m→1.0, scale→0.37) in tests/unit/path_probability_test.rs
- [X] T056 [P] [US1] Write unit test for heading probability with cutoff behavior in tests/unit/path_probability_test.rs
- [X] T057 [US1] Write unit test for combined probability calculation in tests/unit/path_probability_test.rs

#### Phase 3: Netelement-Level Probability

- [X] T058 [US1] Implement calculate_netelement_probability() averaging GNSS position probabilities for a netelement
- [X] T059 [US1] Implement identify_consecutive_positions() to find sequential GNSS positions assigned to same netelement
- [X] T060 [US1] Implement calculate_coverage_factor() as (consecutive_distance_sum / total_distance_first_to_last)
- [X] T061 [US1] Apply coverage correction factor to netelement probability
- [X] T062 [P] [US1] Write unit test for netelement probability averaging in tests/unit/path_probability_test.rs
- [X] T063 [US1] Write unit test for consecutive position identification in tests/unit/path_probability_test.rs
- [X] T064 [US1] Write unit test for coverage factor calculation in tests/unit/path_probability_test.rs

#### Phase 4: Path Construction (Bidirectional)

- [X] T065 [P] [US1] Create tp-core/src/path/construction.rs module file
- [X] T066 [P] [US1] Implement construct_forward_path() starting from highest probability netelement at first position
- [X] T067 [P] [US1] Implement construct_backward_path() starting from highest probability netelement at last position
- [X] T068 [US1] Implement graph traversal with navigability constraints using petgraph neighbors()
- [X] T069 [US1] Implement probability threshold filtering (default 25%, except when only navigable option)
- [X] T070 [US1] Implement path reversal for backward path (reverse segment order + swap intrinsic coordinates)
- [X] T071 [US1] Implement bidirectional validation comparing forward and reversed backward paths
- [X] T072 [US1] Write unit test for forward path construction in tests/unit/path_construction_test.rs
- [X] T073 [US1] Write unit test for backward path construction and reversal in tests/unit/path_construction_test.rs
- [X] T074 [US1] Write unit test for bidirectional agreement detection in tests/unit/path_construction_test.rs

#### Phase 5: Path Selection

- [X] T075 [P] [US1] Create tp-core/src/path/selection.rs module file
- [X] T076 [US1] Implement calculate_path_probability() as length-weighted average of netelement probabilities
- [X] T077 [US1] Implement bidirectional probability averaging: (P_forward + P_backward) / 2
- [X] T078 [US1] Handle unidirectional paths (only forward or only backward) with 0 for missing direction
- [X] T079 [US1] Implement select_best_path() choosing path with highest probability (first if tied)
- [X] T080 [US1] Assign probability 0 to paths that terminate before reaching end position
- [X] T081 [P] [US1] Write unit test for path probability calculation in tests/unit/path_probability_test.rs
- [X] T082 [US1] Write unit test for bidirectional averaging in tests/unit/path_probability_test.rs
- [X] T083 [US1] Write unit test for early termination detection in tests/unit/path_construction_test.rs

#### Integration: Main API Function

- [X] T084 [US1] Implement calculate_train_path() main public function in tp-core/src/path.rs
- [X] T085 [US1] Wire candidate selection → probability calculation → path construction → path selection
- [X] T086 [US1] Add input validation (empty network, no netrelations, invalid geometry checks)
- [X] T087 [US1] Add logging for audit trail (CRS conversions, netelement selections, path probability scores)
- [X] T088 [US1] Verify all User Story 1 integration tests pass

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Project Coordinates on Calculated Path (Priority: P2)

**Goal**: Project GNSS coordinates onto the calculated train path with intrinsic coordinates.

**Independent Test**: Provide pre-calculated train path and GNSS coordinates, verify each coordinate projected onto correct segment with accurate intrinsics.

### Tests for User Story 2 ⚠️

- [X] T089 [P] [US2] Write integration test for projecting coordinates onto calculated path in tests/integration/path_calculation_test.rs
- [X] T090 [P] [US2] Write integration test for coordinates between segments assigned to nearest segment in path in tests/integration/path_calculation_test.rs
- [X] T091 [P] [US2] Write integration test for pre-supplied path skipping calculation in tests/integration/path_calculation_test.rs
- [X] T092 [P] [US2] Write contract test for project_onto_path() function signature in tests/contract/path_api_contract.rs

### Implementation for User Story 2

- [X] T093 [P] [US2] Implement project_onto_path() function in tp-core/src/path.rs
- [X] T094 [US2] Implement segment selection logic: choose segment in path closest to GNSS coordinate
- [X] T095 [US2] Reuse project_point_onto_linestring() from projection/geom.rs for intrinsic calculation
- [X] T096 [US2] Calculate intrinsic coordinate (0-1 range) relative to segment start
- [X] T097 [US2] Handle pre-supplied train path input (skip path calculation, directly project)
- [X] T098 [US2] Extend calculate_train_path() with path_only parameter
- [X] T099 [US2] Update PathResult to include projected_positions vector
- [X] T100 [US2] Add validation: intrinsic coordinates must be between 0 and 1
- [X] T101 [US2] Write unit test for intrinsic coordinate calculation in tests/unit/path_construction_test.rs
- [X] T102 [US2] Verify all User Story 2 integration tests pass

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Export Train Path Only (Priority: P3)

**Goal**: Export calculated train path without processing coordinate projections for debugging and validation.

**Independent Test**: Request path-only export mode and verify output contains ordered track segment sequence without projection data.

### Tests for User Story 3 ⚠️

- [X] T103 [P] [US3] Write integration test for path-only export in CSV format in tests/integration/path_calculation_test.rs
- [X] T104 [P] [US3] Write integration test for path-only export in GeoJSON format in tests/integration/path_calculation_test.rs
- [X] T105 [P] [US3] Write integration test verifying path shows segment sequence and connection info in tests/integration/path_calculation_test.rs
- [X] T106 [P] [US3] Write integration test for failed path calculation with diagnostic output in tests/integration/path_calculation_test.rs

### Implementation for User Story 3

- [X] T107 [P] [US3] Extend calculate_train_path() to handle path_only=true parameter
- [X] T108 [US3] Skip projection phase when path_only=true, return PathResult with empty projected_positions
- [X] T109 [US3] Ensure TrainPath includes all diagnostic information (segment order, intrinsic ranges, probabilities)
- [X] T110 [US3] Add failure reporting: when path calculation fails, populate warnings with diagnostic information
- [X] T111 [US3] Verify all User Story 3 integration tests pass

**Checkpoint**: User Stories 1, 2, AND 3 should all work independently

---

## Phase 6: User Story 4 - Enhanced GNSS Data with Heading and Distance (Priority: P2)

**Goal**: Use optional heading and distance data from GNSS to improve path calculation accuracy.

**Independent Test**: Provide GNSS data with heading/distance columns, compare path calculation results against coordinate-only data to verify improved accuracy.

### Tests for User Story 4 ⚠️

- [X] T112 [P] [US4] Write integration test for heading-enhanced path calculation in tests/integration/path_calculation_test.rs
- [X] T113 [P] [US4] Write integration test for distance-based spacing calculation in tests/integration/path_calculation_test.rs
- [X] T114 [P] [US4] Write integration test for GNSS data without heading/distance still works in tests/integration/path_calculation_test.rs

### Implementation for User Story 4

- [X] T115 [P] [US4] Extend CSV parser in tp-core/src/io/csv.rs to read optional "heading" column
- [X] T116 [P] [US4] Extend CSV parser to read optional "distance" column
- [X] T117 [P] [US4] Extend GeoJSON parser in tp-core/src/io/geojson.rs to read optional heading and distance properties
- [X] T118 [US4] Update calculate_heading_probability() to use GNSS heading when available (already in probability.rs)
- [X] T119 [US4] Implement use_distance_values_for_spacing() to calculate mean spacing using distance column
- [X] T120 [US4] Add backward compatibility check: heading/distance fields are optional, default to None
- [X] T121 [US4] Write unit test for heading-based filtering improvement in tests/unit/path_probability_test.rs
- [X] T122 [US4] Write unit test for distance-based spacing calculation in tests/unit/path_probability_test.rs
- [X] T123 [US4] Verify all User Story 4 integration tests pass

**Checkpoint**: User Stories 1-4 should all work independently

---

## Phase 7: User Story 5 - Performance-Optimized Processing (Priority: P3)

**Goal**: Enable resampling for dense GNSS data while maintaining full output (all positions projected).

**Independent Test**: Process same dataset with different resampling values, measure execution time and verify all original positions in output.

### Tests for User Story 5 ⚠️

- [X] T124 [P] [US5] Write integration test for resampling with 10m interval on 1m-spaced data in tests/integration/path_calculation_test.rs
- [X] T125 [P] [US5] Write integration test verifying all original positions in output despite resampling in tests/integration/path_calculation_test.rs
- [X] T126 [P] [US5] Write performance benchmark comparing resampled vs full processing in benches/path_calculation_bench.rs

### Implementation for User Story 5

- [X] T127 [P] [US5] Implement calculate_mean_spacing() to determine average distance between GNSS positions
- [X] T128 [P] [US5] Use distance column if available, else calculate geometric distance for spacing
- [X] T129 [US5] Implement select_resampled_subset() choosing positions at resampling interval for path calculation
- [X] T130 [US5] Apply path calculation on resampled subset only (smaller dataset)
- [X] T131 [US5] After path calculated, project ALL original positions onto path (full dataset)
- [X] T132 [US5] Update PathConfig to include resampling_distance: Option<f64> field
- [X] T133 [US5] Add validation: resampling_distance must be positive if Some
- [X] T134 [US5] Write unit test for mean spacing calculation in tests/unit/path_probability_test.rs
- [X] T135 [US5] Write unit test for resampled subset selection in tests/unit/path_construction_test.rs
- [X] T136 [US5] Verify all User Story 5 integration tests pass and performance benchmark runs

**Checkpoint**: User Stories 1-5 should all work independently

---

## Phase 8: User Story 6 - Fallback to Simple Projection (Priority: P2)

**Goal**: Graceful degradation when path calculation fails - fall back to simple nearest-segment projection.

**Independent Test**: Provide data that cannot form continuous path (disconnected segments), verify system produces simple projection with warnings.

### Tests for User Story 6 ⚠️

- [X] T137 [P] [US6] Write integration test for fallback with disconnected network in tests/integration/path_calculation_test.rs
- [X] T138 [P] [US6] Write integration test verifying fallback notification to user in tests/integration/path_calculation_test.rs
- [X] T139 [P] [US6] Write integration test for fallback ignoring navigability constraints in tests/integration/path_calculation_test.rs

### Implementation for User Story 6

- [X] T140 [P] [US6] Implement detect_path_calculation_failure() checking if no valid path found
- [X] T141 [US6] Implement fallback_to_simple_projection() projecting each coordinate to nearest netelement independently
- [X] T142 [US6] Reuse existing simple projection logic from feature 001 (projection module)
- [X] T143 [US6] Set PathResult.mode = PathCalculationMode::FallbackIndependent when fallback used
- [X] T144 [US6] Populate PathResult.warnings with "No continuous path found, using fallback projection"
- [X] T145 [US6] Ensure fallback ignores navigability (project to geometrically nearest regardless)
- [X] T146 [US6] Add logging for fallback trigger event (optional enhancement)
- [X] T147 [US6] Write unit test for fallback detection logic in tests/unit/path_construction_test.rs (covered by integration tests)
- [X] T148 [US6] Verify all User Story 6 integration tests pass

**Checkpoint**: User Stories 1-6 should all work independently

---

## Phase 9: User Story 7 - Debug Path Calculation (Priority: P4)

**Goal**: Export intermediate results for troubleshooting and parameter tuning (developer/support tool).

**Independent Test**: Enable debug export mode, verify intermediate files contain probability calculations and decision criteria.

### Tests for User Story 7 ⚠️

- [X] T149 [P] [US7] Write integration test for debug export of candidate paths in tests/integration/path_calculation_test.rs
- [X] T150 [P] [US7] Write integration test for debug export showing track segment candidates per coordinate in tests/integration/path_calculation_test.rs
- [X] T151 [P] [US7] Write integration test for debug export showing forward/backward probability averaging in tests/integration/path_calculation_test.rs

### Implementation for User Story 7

- [X] T152 [P] [US7] Add debug_mode: bool field to PathConfig
- [X] T153 [P] [US7] Create DebugInfo struct to collect intermediate results (candidate_paths, position_candidates, decision_tree)
- [X] T154 [US7] Implement export_candidate_paths() writing all candidate paths with probability scores
- [X] T155 [US7] Implement export_position_candidates() writing netelement candidates and probabilities per GNSS coordinate
- [X] T156 [US7] Implement export_decision_tree() showing bidirectional averaging and final path selection
- [X] T157 [US7] Integrate debug info collection throughout path calculation pipeline
- [X] T158 [US7] Write debug output to separate files (candidates.json, decisions.json) when debug_mode=true
- [X] T159 [US7] Verify all User Story 7 integration tests pass

**Checkpoint**: All user stories (1-7) should now be independently functional

---

## Phase 10: CLI Integration

**Purpose**: Expose path calculation functionality via command-line interface

### Tests for CLI

- [X] T160 [P] Write CLI integration test for default command (calculate + project) in tp-cli/tests/cli_integration_test.rs
- [X] T161 [P] Write CLI integration test for calculate-path command (path only) in tp-cli/tests/cli_integration_test.rs
- [X] T162 [P] Write CLI integration test for simple-projection command (feature 001 legacy) in tp-cli/tests/cli_integration_test.rs
- [X] T163 [P] Write CLI integration test for --train-path parameter (use existing path) in tp-cli/tests/cli_integration_test.rs

### Implementation for CLI

- [X] T164 Extend tp-cli/src/main.rs with three command structure (default, calculate-path, simple-projection)
- [X] T165 [P] Implement default command: parse args, call calculate_train_path(path_only=false), write output
- [X] T166 [P] Implement calculate-path command: parse args, call calculate_train_path(path_only=true), write path output
- [X] T167 [P] Implement simple-projection command: maintain existing feature 001 behavior
- [X] T168 Add --train-path parameter to default command for pre-calculated path input
- [X] T169 [P] Add algorithm parameters as CLI options (--distance-scale, --heading-scale, --cutoff-distance, --heading-cutoff, --probability-threshold, --max-candidates)
- [X] T170 [P] Add --resampling-distance parameter to default and calculate-path commands
- [X] T171 [P] Add --save-path parameter to default command for saving calculated path
- [X] T172 [P] Add --format parameter for output format selection (csv, geojson, auto)
- [X] T173 Add --verbose and --quiet flags for logging control
- [X] T174 Implement proper exit codes (0 for success, non-zero for errors)
- [X] T175 Implement stderr for errors/warnings, stdout for results
- [X] T176 Add --help documentation for all commands and parameters
- [X] T177 Verify all CLI integration tests pass

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T178 [P] Add documentation comments to all public API functions in tp-core/src/path.rs
- [X] T179 [P] Add usage examples to documentation for calculate_train_path() and project_onto_path()
- [X] T180 [P] Update README.md with train path calculation feature overview
- [X] T181 [P] Update quickstart.md validation: verify all examples execute successfully
- [X] T182 Run full test suite with cargo test and verify 100% pass rate
- [X] T183 Run cargo clippy and fix all warnings
- [X] T184 Run cargo fmt to ensure consistent code formatting
- [X] T185 Generate code coverage report and verify target coverage (aim for 100%)
- [X] T186 Run performance benchmarks (benches/path_calculation_bench.rs) and document results
- [X] T187 Validate performance goals: 10k positions in <2min, support 50k+ segments, <500MB memory
- [X] T188 Review and optimize hot paths identified by benchmarks
- [X] T189 [P] Add error message improvements for common failure cases
- [X] T190 [P] Add diagnostic logging for algorithm decision points
- [X] T191 Perform security review of input validation and error handling
- [X] T192 Verify constitution compliance: all 11 principles + licensing
- [X] T193 Review and update CONTRIBUTING.md with path calculation development guidelines

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational phase completion - Core MVP functionality
- **User Story 2 (Phase 4)**: Depends on US1 (needs path calculation) - Projects coordinates onto path
- **User Story 3 (Phase 5)**: Depends on US1 (needs path calculation) - Path-only export
- **User Story 4 (Phase 6)**: Depends on US1 (enhances path calculation) - Optional enhancement
- **User Story 5 (Phase 7)**: Depends on US1 (optimizes path calculation) - Performance optimization
- **User Story 6 (Phase 8)**: Depends on US1+US2 (fallback from path projection) - Graceful degradation
- **User Story 7 (Phase 9)**: Depends on US1 (debugs path calculation) - Developer tool
- **CLI Integration (Phase 10)**: Depends on US1+US2+US3 complete - Exposes functionality
- **Polish (Phase 11)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories ✅ MVP START HERE
- **User Story 2 (P2)**: Requires US1 complete (needs calculated path) - Can integrate with US1
- **User Story 3 (P3)**: Requires US1 complete (exports path) - Independent from US2
- **User Story 4 (P2)**: Requires US1 complete (enhances probability) - Independent from US2/US3
- **User Story 5 (P3)**: Requires US1 complete (optimizes path calc) - Independent from US2/US3/US4
- **User Story 6 (P2)**: Requires US1+US2 complete (fallback behavior) - Needs both path and projection
- **User Story 7 (P4)**: Requires US1 complete (debugs path calc) - Independent from others

### Within Each User Story

- Tests MUST be written and FAIL before implementation (TDD non-negotiable)
- Models before services
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes:
  - **US1 (P1)** must complete first (MVP foundation)
  - After US1: **US2, US3, US4** can be developed in parallel (all depend only on US1)
  - After US1+US2: **US6** can start (needs both)
  - **US5** and **US7** can be developed anytime after US1
- Within each user story:
  - All tests marked [P] can run in parallel
  - Module creation tasks marked [P] can run in parallel
  - Different implementation modules marked [P] can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together:
Task T039: Integration test for successful path calculation
Task T040: Integration test for junction handling
Task T041: Integration test for heading filtering
Task T042: Integration test for highest probability selection
Task T043: Contract test for API signature

# Launch all module creation tasks for US1 together:
Task T044: Create candidate.rs module
Task T050: Create probability.rs module
Task T065: Create construction.rs module
Task T075: Create selection.rs module

# Within Probability module, these can run in parallel:
Task T051: Implement distance probability formula
Task T052: Implement heading probability formula
# Then Task T053 depends on T051+T052 (product calculation)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (core path calculation)
4. **STOP and VALIDATE**: Test User Story 1 independently
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo (path + projection)
4. Add User Story 3 → Test independently → Deploy/Demo (debugging support)
5. Add User Stories 4-7 as needed for production features

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - **One developer: User Story 1** (MUST complete first - MVP blocker)
3. After US1 complete:
   - Developer A: User Story 2 (projection)
   - Developer B: User Story 3 (path export)
   - Developer C: User Story 4 (heading/distance)
4. After US1+US2 complete:
   - Any developer: User Story 6 (fallback)
5. Anytime after US1:
   - Any developer: User Story 5 (resampling)
   - Any developer: User Story 7 (debug)

---

## Total Task Summary

- **Setup**: 10 tasks
- **Foundational**: 30 tasks (BLOCKING) - includes T026a, T026b for netrelation reference validation
- **User Story 1 (P1)**: 50 tasks - MVP Core
- **User Story 2 (P2)**: 14 tasks - Projection
- **User Story 3 (P3)**: 9 tasks - Export
- **User Story 4 (P2)**: 13 tasks - Enhancement
- **User Story 5 (P3)**: 13 tasks - Optimization
- **User Story 6 (P2)**: 12 tasks - Fallback
- **User Story 7 (P4)**: 11 tasks - Debug
- **CLI Integration**: 18 tasks
- **Polish**: 16 tasks

**Total: 195 tasks** (updated from 193 after adding reference validation tasks)

**Estimated MVP Scope** (Setup + Foundational + US1 + US2 + CLI basics):  
~100 tasks for functional path-based projection

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing (TDD non-negotiable per constitution)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Constitution Principle IV (TDD) is NON-NEGOTIABLE: tests MUST be written first

---

**Feature Version**: 1.0  
**Generated**: January 2026  
**Implementation Start**: After plan.md approval
