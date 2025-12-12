# Implementation Plan: GNSS Track Axis Projection

**Branch**: `001-gnss-projection` | **Date**: 2025-12-09 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-gnss-projection/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Project GNSS positions from train journeys onto railway track axis centerlines (netelements) to improve positioning accuracy. Accept GNSS data (CSV/GeoJSON) and railway network (GeoJSON), perform geometric projection using pure spatial proximity, calculate measure along track, and output enriched positioning records. Technical approach leverages Rust core engine with Apache Arrow columnar memory, geo/proj crates for geospatial operations, and Python/NET integration layers for ecosystem compatibility.

## Technical Context

**Language/Version**: Rust 1.91.1+ (core engine), Python 3.12+ (integration layer), .NET 8.0+ (integration layer)  
**Primary Dependencies**: 
- **Rust**: Apache Arrow (columnar memory), Polars (DataFrames), geo/proj crates (geospatial), chrono (temporal), geodatafusion (PostGIS-compatible spatial functions)
- **Python**: NumPy/SciPy (mathematics), PyProj/GeoPandas (GNSS/GIS), Click (CLI), pytest (testing)
- **Shared**: GeoJSON parsing, CRS transformations (PROJ library), spatial indexing (R-tree)

**Storage**: In-memory processing only (Apache Arrow columnar format), no persistent storage  
**Testing**: cargo test (Rust unit/integration), pytest (Python bindings), TDD mandatory per Constitution Principle IV  
**Target Platform**: Cross-platform (Linux, Windows, macOS) via Rust compilation + Python/NET bindings  
**Project Type**: Single unified library with multi-language bindings  
**Performance Goals**: 
- Process 1000 GNSS positions with 50 netelements in <10 seconds (SC-001)
- Support datasets with 10,000+ positions without memory exhaustion (SC-006)
- Columnar Arrow format for cache-efficient batch operations

**Constraints**: 
- 95% of projections within 2m accuracy under normal conditions (SC-002)
- 98% correct netelement identification for clear paths (SC-003)
- 100% record correspondence (input count = output count) (SC-004)
- Configurable distance threshold for warnings (default 50m, FR-017)

**Scale/Scope**: Batch post-processing, single train journey (1000-10,000 GNSS points typical), 50-500 netelements per region

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: Library-First Architecture
**Status**: ✅ **PASS**  
**Evidence**: Single unified library (`tp-lib`) with GNSS projection as first module. Rust core + Python/.NET bindings maintain library-first approach. Uses quality external dependencies (Arrow, Polars, geo/proj) per constitution guidance.

### Principle II: CLI Interface Mandatory
**Status**: ✅ **PASS**  
**Evidence**: FR-020 to FR-023 mandate CLI with stdin/stdout/stderr separation, exit codes, --help flag. Python Click for CLI implementation.

### Principle III: High Performance
**Status**: ✅ **PASS**  
**Evidence**: Apache Arrow columnar memory format, Rust zero-copy operations, Polars for efficient DataFrames, spatial indexing (R-tree) for O(log n) netelement queries. Performance benchmarks required (SC-001, SC-006).

### Principle IV: Test-Driven Development (NON-NEGOTIABLE)
**Status**: ✅ **PASS WITH PLAN**  
**Evidence**: Implementation plan includes **Phase 1 Task 1: Create basic projection test** (point-on-linestring without real data) to validate environment setup before any implementation. TDD workflow enforced: cargo test + pytest, coverage tracking mandatory.

### Principle V: Full Test Coverage
**Status**: ✅ **PASS**  
**Evidence**: Testing framework specified (cargo test, pytest). Plan includes unit, integration, contract, and performance tests. Coverage reports required per constitution.

### Principle VI: Time with Timezone Awareness
**Status**: ✅ **PASS**  
**Evidence**: FR-001, FR-011 mandate timezone in GNSS timestamps. Rust `chrono` crate for timezone-aware temporal handling. Validation prevents naive datetime usage.

### Principle VII: Positions with Coordinate Reference System
**Status**: ✅ **PASS**  
**Evidence**: FR-003, FR-004 mandate explicit CRS specification. CLI parameter `--gnss-crs` for CSV, GeoJSON CRS extraction. Rust `proj` crate for CRS transformations (FR-008). Never assumes default CRS.

### Principle VIII: Thorough Error Handling
**Status**: ✅ **PASS**  
**Evidence**: FR-016 mandates fail-fast validation with actionable errors. Rust `Result<T, E>` types for typed errors, diagnostic info (FR-019), stderr for errors (FR-022), exit codes (FR-022).

### Principle IX: Data Provenance and Audit Trail
**Status**: ✅ **PASS**  
**Evidence**: FR-018 mandates logging of CRS conversions, projections, netelement assignments. FR-013 preserves original GNSS data in output, FR-019 includes projection distance diagnostics.

### Principle X: Integration Flexibility
**Status**: ✅ **PASS**  
**Evidence**: Rust core with Python/NET bindings (FFI), CLI interface, library API, standard formats (CSV/GeoJSON/JSON output per FR-001, FR-002, FR-015, FR-021).

---

### 🎯 Overall Gate Status: ✅ **ALL CHECKS PASS**

**Ready to proceed to Phase 0 (Research)**

No constitution violations. No complexity justifications required.

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
tp-lib/                  # Rust workspace root
├── Cargo.toml           # Workspace manifest
├── tp-core/             # Core Rust library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs       # Public API exports
│   │   ├── models/      # Data models
│   │   │   ├── mod.rs
│   │   │   ├── gnss.rs  # GnssPosition struct
│   │   │   ├── netelement.rs # Netelement struct
│   │   │   └── result.rs # ProjectedPosition struct
│   │   ├── projection/  # Projection engine
│   │   │   ├── mod.rs
│   │   │   ├── geom.rs  # Geometric projection algorithms
│   │   │   └── spatial.rs # R-tree spatial indexing
│   │   ├── io/          # Input/output module
│   │   │   ├── mod.rs
│   │   │   ├── csv.rs   # CSV parsing (arrow-csv)
│   │   │   ├── geojson.rs # GeoJSON parsing
│   │   │   └── arrow.rs # Arrow columnar conversion
│   │   ├── crs/         # Coordinate reference systems
│   │   │   ├── mod.rs
│   │   │   └── transform.rs # CRS transformations (proj crate)
│   │   ├── temporal/    # Timezone handling
│   │   │   └── mod.rs
│   │   └── errors.rs    # Error types (ProjectionError enum)
│   ├── tests/
│   │   ├── unit/        # Unit tests per module
│   │   │   ├── projection_basic_test.rs # FIRST TEST (point-on-linestring)
│   │   │   ├── gnss_model_test.rs
│   │   │   └── crs_transform_test.rs
│   │   ├── integration/ # End-to-end tests
│   │   │   └── pipeline_test.rs # CSV → GeoJSON → output
│   │   ├── contract/    # API contract tests
│   │   │   └── lib_api_stability_test.rs
│   │   └── property/    # Property-based tests (quickcheck)
│   │       └── projection_invariants_test.rs
│   └── benches/         # Performance benchmarks (criterion)
│       ├── projection_bench.rs
│       └── naive_baseline_bench.rs
├── tp-cli/              # CLI application
│   ├── Cargo.toml
│   ├── src/
│   │   └── main.rs      # CLI entry point (clap crate)
│   └── tests/
│       └── cli_integration_test.rs
├── tp-py/               # Python bindings (PyO3)
│   ├── Cargo.toml
│   ├── src/
│   │   └── lib.rs       # Python FFI interface
│   ├── python/
│   │   ├── tp_lib/      # Python package
│   │   │   └── __init__.py
│   │   └── tests/       # Python tests (pytest)
│   │       └── test_projection.py
│   └── pyproject.toml   # Python packaging
└── .github/
    └── workflows/
        └── ci.yml       # CI/CD pipeline (cargo test + pytest)
```

**Structure Decision**: Single Rust workspace with three crates:
- **tp-core**: Core library containing all business logic as modules (models, projection, I/O, CRS, temporal). Exposes public API for library consumers.
- **tp-cli**: Thin CLI wrapper using `clap` crate, invokes tp-core functions.
- **tp-py**: Python bindings using `PyO3` for FFI, wraps tp-core with Python-friendly API.

This structure satisfies Constitution Principle I (single unified library) while enabling multi-language integration per Principle X. The modular organization supports incremental development and clear separation of concerns.

## Complexity Tracking

> No constitution violations identified. All 10 principles satisfied.

---

## Phase 0: Research 🔍

**Status**: ✅ **COMPLETE** (see [research.md](./research.md))

**Purpose**: Resolve technical unknowns before design begins.

### Research Questions Investigated

1. ✅ Spatial Indexing Strategy (`rstar` crate evaluation)
2. ✅ Arrow Interop (CSV → Arrow → Polars pipeline)
3. ✅ GeoJSON CRS Handling (RFC 7946 compliance)
4. ✅ Projection Algorithm (point-to-linestring with `geo` crate)
5. ✅ Timezone Parsing (chrono with ISO 8601)
6. ✅ PyO3 Error Handling (Rust Result → Python exceptions)
7. ✅ Performance Baseline (naive O(n*m) search benchmark)

**Key Findings**: [See research.md for detailed outcomes]

**Next**: Proceed to Phase 1 (Design)

---

## Phase 1: Design 📐

**Purpose**: Define data models, APIs, user workflows before implementation.

### 1.1 Data Model (`data-model.md`)

**Status**: 🔄 **IN PROGRESS**

**Core Entities**:
- `GnssPosition`: Raw GNSS input (lat, lon, timestamp, CRS, metadata)
- `Netelement`: Railway track segment (id, LineString geometry, CRS)
- `ProjectedPosition`: Output result (original + projected coords + netelement_id + measure)
- `NetworkIndex`: Spatial index (R-tree over netelements)

**Relationships**:
- 1 GnssPosition → 1 ProjectedPosition (1:1 per FR-012)
- 1 ProjectedPosition → 1 Netelement (nearest per FR-009)

See: [data-model.md](./data-model.md) (to be created)

### 1.2 API Contracts (`contracts/`)

**Status**: 📋 **PLANNED**

**Deliverables**:
- `contracts/cli.md`: Command-line interface specification
- `contracts/lib-api.md`: Rust public API
- `contracts/python-api.md`: Python binding interface

**CLI Example**:
```bash
tp-cli project-gnss \
  --gnss-file journey.csv \
  --gnss-crs EPSG:31370 \
  --network-file network.geojson \
  --output-format csv > output.csv
```

### 1.3 User Guide (`quickstart.md`)

**Status**: 📋 **PLANNED**

**Content**:
- Installation instructions (cargo install, pip install)
- Basic usage examples with sample data
- Output format documentation
- Troubleshooting guide

---

## Phase 2: Implementation Tasks 🛠️

**Status**: ⏳ **PENDING** (generated by `/speckit.tasks` command)

### Implementation Phasing Strategy

**Phase 1: Foundational** (BLOCKING)
- ✅ Project setup (Cargo workspace, folder structure)
- 🎯 **FIRST TASK**: Basic projection test (hardcoded point + linestring, no file I/O)
  - File: `tp-core/tests/unit/projection_basic_test.rs`
  - Purpose: Validate environment setup, verify geo crate integration
  - Test: Project (50.0, 4.0) onto LineString[(50.0, 4.0), (51.0, 4.0)]
- Error types and Result handling
- Core data models (structs)

**Phase 2: User Story 1 - Basic GNSS Projection** (P1 MVP)
- GeoJSON network parsing
- CSV GNSS parsing with column mapping
- Spatial indexing (R-tree)
- Projection algorithm + measure calculation
- CRS transformation
- Output generation (CSV + JSON)
- CLI implementation
- Diagnostic warnings

**Phase 3: Testing & Polish**
- Integration tests
- Contract tests
- Property-based tests
- Performance benchmarks
- Documentation

---

## Acceptance Criteria Mapping

| Acceptance Scenario | Implementation Component |
|---------------------|-------------------------|
| AS-1: CSV + GeoJSON → output | CSV parser + GeoJSON parser + projection engine + output formatter |
| AS-2: Monotonic measure | Measure calculation validates direction consistency |
| AS-3: Nearest netelement at junction | R-tree spatial index with distance comparison |
| AS-4: 1:1 record correspondence | Processing loop ensures output.len() == input.len() |

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| CRS transformation accuracy | High | Use proj crate; unit tests with known coordinate pairs |
| Performance at scale (10k) | Medium | R-tree spatial indexing; benchmark early with criterion |
| Timezone edge cases (DST) | Medium | chrono handles DST; test with DST boundaries |
| Parallel tracks ambiguity | Low | Pure proximity per clarification; defer map-matching |
| PyO3 build complexity | Low | Clear docs; pre-built wheels; CI for wheel generation |

---

## Next Steps

1. ✅ **Phase 0 Research**: Complete (see research.md)
2. 🔄 **Phase 1 Design**: 
   - Create `data-model.md` (core entities and relationships)
   - Create `contracts/cli.md`, `contracts/lib-api.md`, `contracts/python-api.md`
   - Create `quickstart.md` (user guide with examples)
3. ⏳ **Phase 2 Tasks**: Run `/speckit.tasks` to generate implementation tasks
4. 🎯 **Begin TDD**: Write basic projection test FIRST (per Constitution Principle IV)

**Estimated Timeline**:
- Phase 0 (Research): 1-2 days ✅
- Phase 1 (Design): 2-3 days 🔄
- Phase 2 (Implementation): 10-15 days ⏳
- Phase 3 (Polish): 3-5 days ⏳

**Total**: ~3-4 weeks for complete MVP with full test coverage
