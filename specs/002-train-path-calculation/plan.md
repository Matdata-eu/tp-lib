# Implementation Plan: Continuous Train Path Calculation

- [Implementation Plan: Continuous Train Path Calculation](#implementation-plan-continuous-train-path-calculation)
  - [Summary](#summary)
  - [Technical Context](#technical-context)
  - [Constitution Check](#constitution-check)
    - [✅ I. Library-First Architecture](#-i-library-first-architecture)
    - [✅ II. CLI Interface Mandatory](#-ii-cli-interface-mandatory)
    - [✅ III. High Performance](#-iii-high-performance)
    - [✅ IV. Test-Driven Development (NON-NEGOTIABLE)](#-iv-test-driven-development-non-negotiable)
    - [✅ V. Full Test Coverage](#-v-full-test-coverage)
    - [✅ VI. Time with Timezone Awareness](#-vi-time-with-timezone-awareness)
    - [✅ VII. Positions with CRS](#-vii-positions-with-crs)
    - [✅ VIII. Thorough Error Handling](#-viii-thorough-error-handling)
    - [✅ IX. Data Provenance and Audit Trail](#-ix-data-provenance-and-audit-trail)
    - [✅ X. Integration Flexibility](#-x-integration-flexibility)
    - [✅ XI. Modern Module Organization (Rust)](#-xi-modern-module-organization-rust)
    - [✅ Licensing and Legal Compliance](#-licensing-and-legal-compliance)
  - [Project Structure](#project-structure)
    - [Documentation (this feature)](#documentation-this-feature)
    - [Source Code (repository root)](#source-code-repository-root)
  - [Complexity Tracking](#complexity-tracking)
  - [Phase Completion Status](#phase-completion-status)
    - [✅ Phase 0: Research \& Outline (COMPLETE)](#-phase-0-research--outline-complete)
    - [✅ Phase 1: Design \& Contracts (COMPLETE)](#-phase-1-design--contracts-complete)
  - [Next Steps (Not Part of /speckit.plan)](#next-steps-not-part-of-speckitplan)
    - [Phase 2: Task Breakdown](#phase-2-task-breakdown)
    - [Phase 3: Implementation](#phase-3-implementation)
    - [Phase 4: Validation](#phase-4-validation)
  - [Summary](#summary-1)


**Branch**: `002-train-path-calculation` | **Date**: January 9, 2026 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-train-path-calculation/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

This feature implements probabilistic train path calculation through a rail network using GNSS data and network topology. The algorithm identifies the most likely continuous sequence of track segments (netelements) that a train traversed by combining spatial proximity analysis, directional alignment, and navigability constraints (netrelations). The implementation extends the existing tp-lib projection engine with topology-aware path calculation, delivering results via library API and CLI interface in CSV/GeoJSON formats.

## Technical Context

**Language/Version**: Rust 1.75+ (edition 2021)  
**Primary Dependencies**: 
- Spatial: geo 0.28, proj4rs 0.1.9, rstar 0.12 (existing), geojson 0.24 (existing)
- Data: polars 0.44, arrow 53.0, csv 1.3, serde 1.0 (existing)
- Temporal: chrono 0.4 with serde (existing)
- Error: thiserror 1.0 (existing)
- CLI: clap 4.5 (existing)
- New: petgraph ~1.0 (for network topology graph algorithms)

**Storage**: File-based (CSV, GeoJSON inputs/outputs)  
**Testing**: cargo test with unit tests, integration tests, contract tests (existing structure in tp-core/tests/)  
**Target Platform**: Linux/Windows/macOS command-line, library embeddings (Python via pyo3)  
**Project Type**: Single library with CLI (workspace: tp-core lib, tp-cli binary, tp-py bindings)  
**Performance Goals**: 
- Process 10,000 GNSS coordinates in <2 minutes
- Support networks with 50,000+ track segments
- Memory efficient: <500MB for typical datasets

**Constraints**: 
- Must maintain backward compatibility with existing projection API
- All temporal data timezone-aware (constitution requirement)
- All spatial data CRS-explicit (constitution requirement)
- 100% test coverage target (constitution requirement)
- Apache 2.0 compatible dependencies only

**Scale/Scope**: 
- 5 new data models (NetRelation, AssociatedNetElement, TrainPath, extended GnssPosition)
- 6 core algorithm modules (candidate selection, probability calculation, path construction, validation)
- ~2000 lines of implementation code + ~1500 lines of tests
- Extends existing tp-core with new path module, preserves existing projection module

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ I. Library-First Architecture
- **Status**: PASS
- Path calculation implemented as modules within tp-core library
- Reuses existing dependencies (geo, rstar, serde) and spatial indexing infrastructure
- CLI interface will expose all functionality via tp-cli

### ✅ II. CLI Interface Mandatory
- **Status**: PASS
- Three-command architecture: default (calculate+project), calculate-path (path only), simple-projection (legacy)
- Input via files, output to files/stdout (JSON/CSV), errors to stderr
- Maintains existing CLI patterns from GNSS projection feature
- Composable workflows enable debugging, caching, and flexible analysis

### ✅ III. High Performance
- **Status**: PASS
- Reuses existing NetworkIndex (R-tree) for spatial queries O(log N)
- Graph traversal with pruning to avoid exponential path explosion
- Resampling optimization for high-frequency GNSS data
- Performance benchmarks in benches/path_calculation_bench.rs

### ✅ IV. Test-Driven Development (NON-NEGOTIABLE)
- **Status**: PASS - MANDATORY workflow enforced
- Tests written FIRST for all path calculation logic
- User/stakeholder approval of tests before implementation
- Red-Green-Refactor cycle strictly followed

### ✅ V. Full Test Coverage
- **Status**: PASS - Target 100%
- Unit tests: probability calculations, candidate selection, path validation
- Integration tests: end-to-end path calculation scenarios
- Contract tests: API stability verification
- Property-based tests: probability formula properties

### ✅ VI. Time with Timezone Awareness
- **Status**: PASS
- Reuses existing GnssPosition with DateTime<FixedOffset>
- No new temporal logic required; maintains constitution compliance

### ✅ VII. Positions with CRS
- **Status**: PASS
- All spatial data (Netelement, GnssPosition) has explicit CRS field
- CRS validation enforced in existing models
- No changes to CRS handling required

### ✅ VIII. Thorough Error Handling
- **Status**: PASS
- Extend existing ProjectionError enum with path-specific variants
- Typed errors for invalid topology, no navigable path, probability threshold failures
- Fallback behavior clearly documented

### ✅ IX. Data Provenance and Audit Trail
- **Status**: PASS
- TrainPath includes probability scores and algorithm metadata
- AssociatedNetElement includes projection details and likelihood scores
- Output formats capture data lineage for audit purposes

### ✅ X. Integration Flexibility
- **Status**: PASS
- Library API as primary interface
- CLI for batch processing
- Python bindings via existing tp-py infrastructure
- Standard data formats (CSV, GeoJSON)

### ✅ XI. Modern Module Organization (Rust)
- **Status**: PASS
- Uses path.rs + path/ directory structure (not path/mod.rs)
- Follows modern Rust 1.30+ conventions
- Consistent with existing tp-core module organization

### ✅ Licensing and Legal Compliance
- **Status**: PASS
- New dependency petgraph: MIT OR Apache-2.0 (compatible)
- All existing dependencies Apache 2.0 compatible
- No prohibited licenses introduced

**GATE STATUS**: ✅ **APPROVED** - All constitutional requirements satisfied

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
tp-core/
├── src/
│   ├── lib.rs
│   ├── models.rs              # Add netrelation, train_path modules
│   ├── models/
│   │   ├── gnss.rs           # Extend with heading, distance fields
│   │   ├── netelement.rs     # Existing
│   │   ├── netrelation.rs    # NEW - topology connections
│   │   ├── train_path.rs     # NEW - path + associated netelements
│   │   └── result.rs         # Existing
│   ├── path.rs               # NEW - path calculation public API
│   ├── path/
│   │   ├── candidate.rs      # NEW - Phase 1: candidate selection
│   │   ├── probability.rs    # NEW - Phase 2-3: probability calculations
│   │   ├── construction.rs   # NEW - Phase 4: path construction
│   │   ├── selection.rs      # NEW - Phase 5: path selection
│   │   └── graph.rs          # NEW - network topology graph representation
│   ├── projection.rs         # Existing (unchanged)
│   ├── projection/
│   │   ├── geom.rs           # Existing - REUSE for projection onto path
│   │   └── spatial.rs        # Existing - REUSE NetworkIndex for candidates
│   ├── io.rs                 # Extend with train_path I/O
│   ├── io/
│   │   ├── csv.rs            # Extend for train path CSV
│   │   ├── geojson.rs        # Extend for netrelation + train path GeoJSON
│   │   └── arrow.rs          # Existing
│   └── errors.rs             # Extend with path-specific errors
│
└── tests/
    ├── contract/
    │   └── path_api_contract.rs  # NEW - path API stability tests
    ├── integration/
    │   └── path_calculation_test.rs  # NEW - end-to-end scenarios
    └── unit/
        ├── path_candidate_test.rs    # NEW
        ├── path_probability_test.rs  # NEW
        └── path_construction_test.rs # NEW

tp-cli/
└── src/
    └── main.rs               # Add three command modes: default, calculate-path, simple-projection

benches/
└── path_calculation_bench.rs # NEW - performance benchmarks
```

**Structure Decision**: Single library project (Option 1). The path calculation functionality is implemented as a new module within the existing tp-core library, reusing spatial indexing (NetworkIndex), geometry operations (project_point_onto_linestring), and I/O infrastructure. The CLI is extended with three command modes in tp-cli: default (calculate+project), calculate-path (path only), and simple-projection (legacy). This maintains consistency with the existing GNSS projection feature (001) and follows the library-first architecture principle.

## Complexity Tracking

> **No violations detected** - All constitutional principles satisfied without exceptions.

---

## Phase Completion Status

### ✅ Phase 0: Research & Outline (COMPLETE)

**Deliverable**: [research.md](research.md)

**Contents**:
- Network topology graph representation (petgraph directed graph)
- Exponential decay probability formulas
- Bidirectional path construction algorithm
- Distance coverage correction factor calculation
- Code reuse strategy (NetworkIndex, projection functions, I/O)
- Performance optimization (resampling)
- Error handling and fallback behavior
- Testing strategy
- CLI interface design
- Dependencies and licenses

**All unknowns resolved** ✓

### ✅ Phase 1: Design & Contracts (COMPLETE)

**Deliverables**:
1. [data-model.md](data-model.md) - Data structures and entity relationships
2. [contracts/lib-api.md](contracts/lib-api.md) - Rust library API contract
3. [contracts/cli.md](contracts/cli.md) - Command-line interface contract
4. [quickstart.md](quickstart.md) - Usage examples and workflows

**Agent context updated** ✓

**Constitution re-check**: All principles still satisfied after design phase.

---

## Next Steps (Not Part of /speckit.plan)

The `/speckit.plan` command ends here. Phase 2 and beyond are handled by separate commands:

### Phase 2: Task Breakdown
**Command**: `/speckit.tasks`
- Generate [tasks.md](tasks.md) with implementation task list
- Break down into TDD-ready work items
- Estimate complexity and dependencies

### Phase 3: Implementation
**Command**: `/speckit.implement`
- Write tests first (Red phase)
- Get stakeholder approval of tests
- Implement to pass tests (Green phase)
- Refactor while maintaining green tests

### Phase 4: Validation
- Run all tests (target 100% coverage)
- Execute performance benchmarks
- Validate against contract tests
- Generate coverage reports

---

## Summary

**Branch**: `002-train-path-calculation`  
**Status**: Planning Complete, Ready for Task Breakdown  

**Implementation Plan Location**: `specs/002-train-path-calculation/plan.md`

**Key Artifacts**:
- ✅ Technical context defined (Rust 1.75+, petgraph, performance goals)
- ✅ Constitution compliance verified (all 11 principles + licensing)
- ✅ Research complete (10 technical decisions documented)
- ✅ Data models designed (4 new models, 1 extended model)
- ✅ API contracts specified (library + CLI)
- ✅ Quickstart guide created (Rust, CLI, Python examples)
- ✅ Agent context updated (GitHub Copilot)

**Next Command**: `/speckit.tasks` to generate implementation task list

---

**Plan Version**: 1.0  
**Completed**: January 9, 2026  
**Planning Duration**: Phase 0-1 complete
