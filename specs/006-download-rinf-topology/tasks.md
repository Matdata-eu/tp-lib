# Tasks: ERA RINF Network Download

**Feature**: `006-download-rinf-topology`
**Input**: `specs/006-download-rinf-topology/`
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, contracts/api.md ✓, quickstart.md ✓

---

## Format: `[ID] [P?] [Story] Description with file path`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: User story label — US1, US2, US3 (setup/foundational/polish phases carry no label)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Prepare dependencies, fixtures, and workspace hooks required by the retrieval feature.

- [X] T001 Add RINF retrieval dependencies and shared feature flags in Cargo.toml and tp-core/Cargo.toml
- [X] T002 [P] Add covered, uncovered, and invalid GNSS fixtures in test-data/rinf_smoke_gnss.geojson, test-data/rinf_uncovered_gnss.geojson, and test-data/rinf_empty_gnss.geojson
- [X] T003 [P] Add reusable RINF endpoint test fixtures and sample responses in tp-core/tests/fixtures/rinf_smoke_netelements.json and tp-core/tests/fixtures/rinf_smoke_netrelations.json
- [X] T004 [P] Register feature validation commands for Rust, Python, and .NET in .github/workflows/ci.yml

**Checkpoint**: Dependencies, fixtures, and CI hooks exist so the retrieval implementation can be developed and validated consistently.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Build the shared retrieval and outcome infrastructure that every user story depends on.

**⚠️ CRITICAL**: No user story work should start until this phase is complete.

- [X] T005 Create retrieval domain models and status enums in tp-core/src/models/retrieval.rs and tp-core/src/models.rs
- [X] T006 [P] Add RINF-specific error variants and diagnostic payload support in tp-core/src/errors.rs
- [X] T007 [P] Implement retrieval-area construction and source-selection request types in tp-core/src/workflow.rs
- [X] T008 Implement the SPARQL query builder, blocking endpoint client, and row parsing in tp-core/src/io/rinf.rs and tp-core/src/io.rs
- [X] T009 Expose retrieval modules and shared configuration plumbing in tp-core/src/lib.rs
- [X] T010 [P] Add shared CLI and binding option models for endpoint override and buffer distance in tp-cli/src/main.rs, tp-py/src/lib.rs, and tp-net/csharp/Models.cs

**Checkpoint**: `tp-core` can build retrieval requests, parse endpoint responses, and represent typed outcomes before any workflow-specific integration begins.

---

## Phase 3: User Story 1 - Retrieve Network Topology Automatically (Priority: P1) 🎯 MVP

**Goal**: Users can run topology-dependent workflows without a supplied network file and the system downloads usable ERA RINF topology automatically.

**Independent Test**: Run a topology-dependent workflow with covered GNSS input and no network file, and verify that the workflow succeeds using downloaded netelements and netrelations.

### Tests for User Story 1 ⚠️

> **NOTE: Write these tests first, ensure they fail before implementation**

- [X] T011 [P] [US1] Add failing covered-area retrieval and assembly tests in tp-core/tests/rinf_topology.rs
- [X] T012 [P] [US1] Add failing CLI smoke tests for topology-dependent commands without `--network` in tp-cli/tests/cli_integration_test.rs
- [X] T013 [P] [US1] Add failing contract assertions for query shape and valid relation mapping in tp-core/tests/contract.rs

### Implementation for User Story 1

- [X] T014 [US1] Map RINF netelement and netrelation rows into existing topology structures in tp-core/src/io/rinf.rs
- [X] T015 [US1] Route topology-dependent core workflows through automatic source selection in tp-core/src/workflow.rs and tp-core/src/lib.rs
- [X] T016 [US1] Make manual topology optional and trigger auto-retrieval for CLI topology workflows in tp-cli/src/main.rs
- [X] T017 [US1] Add successful source-used diagnostics for auto-retrieval runs in tp-cli/src/main.rs
- [X] T018 [US1] Add covered-area smoke fixture usage to path-calculation and projection integration tests in tp-core/tests/integration/tests.rs and tp-cli/tests/cli_integration_test.rs

**Checkpoint**: User Story 1 is complete when covered-area workflows succeed from the core library and CLI without any supplied topology file.

---

## Phase 4: User Story 2 - Receive Clear Coverage Feedback (Priority: P2)

**Goal**: Users receive distinct, actionable outcomes for invalid input, missing coverage, incomplete topology, and endpoint failures.

**Independent Test**: Run the workflow with uncovered, partially covered, invalid, and endpoint-failure scenarios and verify that each produces a distinct failure outcome before downstream processing starts.

### Tests for User Story 2 ⚠️

> **NOTE: Write these tests first, ensure they fail before implementation**

- [X] T019 [P] [US2] Add failing missing-coverage, incomplete-topology, invalid-input, and endpoint-failure tests in tp-core/tests/rinf_topology.rs
- [X] T020 [P] [US2] Add failing CLI error-reporting tests for uncovered and invalid-input scenarios in tp-cli/tests/cli_integration_test.rs
- [X] T021 [P] [US2] Add failing coarse-geometry and zero-netrelation validation tests in tp-core/tests/unit.rs and tp-core/tests/contract.rs

### Implementation for User Story 2

- [X] T022 [US2] Implement uncovered-area assessment and affected-GNSS diagnostics in tp-core/src/workflow.rs and tp-core/src/models/retrieval.rs
- [X] T023 [US2] Implement coarse-geometry and zero-netrelation validation failures in tp-core/src/io/rinf.rs and tp-core/src/errors.rs
- [X] T024 [US2] Propagate distinct retrieval outcome messages and exit handling in tp-cli/src/main.rs
- [X] T025 [US2] Add retrieval provenance and failure-detail reporting for callers in tp-core/src/lib.rs and tp-core/src/models/retrieval.rs

**Checkpoint**: User Story 2 is complete when each failure mode is surfaced clearly and no downstream topology workflow runs on invalid or incomplete RINF data.

---

## Phase 5: User Story 3 - Use Retrieval Through Language Bindings (Priority: P3)

**Goal**: Python and .NET consumers can omit topology input and get the same retrieval behavior and failure semantics as the core library and CLI.

**Independent Test**: Invoke covered and uncovered workflows through Python and .NET bindings with `network` omitted and verify successful retrieval or equivalent failure semantics.

### Tests for User Story 3 ⚠️

> **NOTE: Write these tests first, ensure they fail before implementation**

- [X] T026 [P] [US3] Add failing Python auto-retrieval success and failure tests in tp-py/python/tests/test_path_calculation.py
- [X] T027 [P] [US3] Add failing Python manual-topology precedence tests in tp-py/python/tests/test_projection.py
- [X] T028 [P] [US3] Add failing .NET auto-retrieval success and failure tests in tp-net/csharp/Tests/PathCalculationTests.cs
- [X] T029 [P] [US3] Add failing .NET nullable-network and precedence tests in tp-net/csharp/Tests/ProjectionTests.cs

### Implementation for User Story 3

- [X] T030 [US3] Expose optional network and RINF retrieval arguments plus typed error mapping in tp-py/src/lib.rs
- [X] T031 [US3] Update Python package exports and developer-facing wrappers for auto-retrieval workflows in tp-py/python/tp_lib/__init__.py
- [X] T032 [US3] Add nullable-network retrieval support and RINF option plumbing to the Rust FFI surface in tp-net/src/lib.rs
- [X] T033 [US3] Expose `RinfRetrievalOptions` and nullable-network overloads in tp-net/csharp/Models.cs and tp-net/csharp/TpLib.cs
- [X] T034 [US3] Map core retrieval outcomes to distinct .NET exceptions and diagnostics in tp-net/csharp/Exceptions.cs and tp-net/csharp/TpLib.cs

**Checkpoint**: User Story 3 is complete when Python and .NET callers can use automatic retrieval without a network file and observe the same outcome categories as Rust callers.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Finish documentation, align examples, and run the full validation matrix requested for the feature.

- [X] T035 [P] Update feature overview and validation commands in README.md
- [X] T036 [P] Update CLI auto-retrieval usage in tp-cli/README.md
- [X] T037 [P] Update Python binding auto-retrieval examples in tp-py/README.md
- [X] T038 [P] Update .NET binding auto-retrieval examples in tp-net/README.md
- [X] T039 [P] Update RINF fixture guidance and smoke-test data notes in test-data/README.md
- [X] T040 Update developer quickstart and acceptance scenarios in specs/006-download-rinf-topology/quickstart.md and specs/006-download-rinf-topology/contracts/api.md
- [X] T041 Run `cargo fmt --all -- --check` for Cargo.toml, tp-core/src/, tp-cli/src/, tp-py/src/, tp-net/src/, and tp-webapp/src/
- [X] T042 Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` for Cargo.toml, tp-core/src/, tp-cli/src/, tp-py/src/, tp-net/src/, and tp-webapp/src/
- [X] T043 Run `cargo test --workspace` covering tp-core/tests/, tp-cli/tests/, tp-net/src/, and tp-webapp/tests/
- [X] T044 Run `maturin develop` and `pytest python/tests/ -v` in tp-py/ for tp-py/src/lib.rs and tp-py/python/tests/
- [X] T045 Run `ruff check tp-py/python` for tp-py/python/
- [X] T046 Run `dotnet test tp-net/csharp/Tests/TpLib.Tests.csproj -c Debug --verbosity minimal` for tp-net/csharp/Tests/ and tp-net/src/lib.rs

**Checkpoint**: Documentation is updated across every README surface and the full Rust, Python, and .NET validation matrix passes, including linting.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion - MVP increment
- **User Story 2 (Phase 4)**: Depends on User Story 1 core retrieval path and can start once Phase 3 infrastructure is in place
- **User Story 3 (Phase 5)**: Depends on Foundational completion and should start after User Story 2 outcome shapes are stable
- **Polish (Phase 6)**: Depends on all desired user stories completing

### User Story Dependencies

- **User Story 1 (P1)**: First deliverable after Phase 2; no dependency on other stories
- **User Story 2 (P2)**: Builds on User Story 1 retrieval flow to harden validation and messaging
- **User Story 3 (P3)**: Reuses the completed core retrieval and outcome contracts from User Stories 1 and 2

### Within Each User Story

- Tests MUST be written and observed failing before implementation begins
- Core retrieval and validation code in tp-core precedes CLI and binding integration
- CLI and binding adapters should consume typed outcomes rather than reimplement validation logic
- Story checkpoints must pass before starting polish work

### Parallel Opportunities

- Setup tasks marked `[P]` can run in parallel because they touch different fixtures and workflow files
- Foundational tasks marked `[P]` can run in parallel once the retrieval module boundaries are agreed
- User Story 1 test tasks can run in parallel across tp-core and tp-cli
- User Story 2 validation tests can run in parallel across tp-core and tp-cli
- User Story 3 binding tests can run in parallel across tp-py and tp-net
- README updates in Phase 6 can run in parallel because each task touches a separate documentation file

---

## Summary

| Phase | Tasks | User Story | Priority |
|---|---|---|---|
| Phase 1 — Setup | T001-T004 | — | — |
| Phase 2 — Foundational | T005-T010 | — | — |
| Phase 3 — Auto Retrieval | T011-T018 | US1 | P1 🎯 MVP |
| Phase 4 — Coverage Feedback | T019-T025 | US2 | P2 |
| Phase 5 — Language Bindings | T026-T034 | US3 | P3 |
| Phase 6 — Polish | T035-T046 | — | — |

**Total**: 46 tasks.

**Task count by user story**:
- **US1**: 8 tasks
- **US2**: 7 tasks
- **US3**: 9 tasks

**Parallel opportunities identified**: 21 tasks marked `[P]`, plus story-level parallelism between tp-py and tp-net once the tp-core outcome contract is stable.

**Independent test criteria**:
- **US1**: Covered-area workflow succeeds without `--network` and uses ERA RINF topology
- **US2**: Uncovered, incomplete, invalid, and endpoint-failure scenarios each return a distinct failure outcome
- **US3**: Python and .NET workflows succeed or fail with the same semantics as the core library when topology is omitted

**Format validation**: Every task uses the required checklist structure with checkbox, task ID, optional `[P]`, required story labels for user-story phases, and explicit file paths.