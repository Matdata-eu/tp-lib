# Implementation Plan: ERA RINF Network Download

**Branch**: `006-download-rinf-topology` | **Date**: 2026-05-13 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/006-download-rinf-topology/spec.md`

## Summary

Add an automatic topology source for all existing topology-dependent workflows when the caller does not supply a network file or network object. The implementation keeps the current pure topology algorithms unchanged and adds a new retrieval-and-validation layer in `tp-core` that: derives a GNSS search polygon from the dataset bounds, expands it by 1 km, issues two SPARQL `SELECT` queries against the ERA RINF endpoint, converts tabular rows into existing `Netelement` and `NetRelation` structures, validates microtopology quality, and returns explicit outcomes for invalid input, missing coverage, incomplete topology, or endpoint failure. The same behavior is then surfaced through the CLI, Python bindings, and .NET bindings.

## Technical Context

**Language/Version**: Rust 2021 workspace (`tp-core`, `tp-cli`, `tp-py`, `tp-net`) + Python bindings via `pyo3` + C# 12 / .NET 8 bindings  
**Primary Dependencies**: Existing workspace crates (`geo`, `geojson`, `chrono`, `serde`, `serde_json`, `clap`, `pyo3`, `csbindgen`) plus an HTTPS client for SPARQL access (`reqwest` blocking client with JSON response handling)  
**Storage**: N/A for persistent storage; per-run in-memory retrieval/validation only  
**Testing**: `cargo test --workspace`, targeted `tp-core` integration tests for SPARQL query generation and topology validation, `pytest` for `tp-py`, `dotnet test` for `tp-net`, CLI smoke tests against a fixed polygon fixture  
**Target Platform**: Cross-platform Rust library/CLI plus Python and .NET consumers with outbound HTTPS access to `https://graph.data.era.europa.eu/repositories/rinf-plus`  
**Project Type**: Multi-crate Rust workspace with CLI and language bindings  
**Performance Goals**: Bounding-box derivation and query generation under 10 ms for 10k GNSS points; response parsing and validation under 500 ms for a route-sized payload; topology-dependent workflows should add no more than one retrieval step per run when topology is absent  
**Constraints**: Manual topology remains authoritative when provided; no partial workflow results on incomplete coverage; fail validation when any returned netelement longer than 250 m has WKT with 2 or fewer points; fail validation when netelements are returned but no netrelations are returned; preserve explicit CRS and timezone handling; default retrieval region is a single GNSS-derived polygon expanded by 1 km  
**Scale/Scope**: One external SPARQL endpoint, one retrieval region per workflow invocation, all existing topology-dependent workflows in core/CLI/Python/.NET, route-sized datasets rather than nationwide bulk sync

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Phase 0 Gate

| Principle | Check | Notes |
|---|---|---|
| I — Library-First | ✅ PASS | Retrieval lives in `tp-core` as a reusable library capability rather than CLI-only logic |
| II — CLI Interface Mandatory | ✅ PASS | Existing `tp-cli` commands gain the automatic retrieval path when `--network` is omitted |
| III — High Performance | ✅ PASS | One retrieval per run, route-sized search region, no persistent graph materialization |
| IV — TDD | ✅ GATE MUST PASS | Query builder, parser, validator, CLI behavior, Python/.NET wrappers all require failing tests first |
| V — Full Test Coverage | ✅ GATE MUST PASS | Unit, integration, contract, and binding tests required for retrieval outcomes and validation failures |
| VI — Timezone Awareness | ✅ PASS | GNSS timestamps remain timezone-aware; RINF validity filtering is date-based and explicit |
| VII — CRS Awareness | ✅ PASS | Retrieval area is derived from explicit GNSS coordinates and emitted as WGS84 polygon WKT for GeoSPARQL |
| VIII — Thorough Error Handling | ✅ PASS | Invalid input, missing coverage, incomplete topology, and endpoint failures are distinct typed outcomes |
| IX — Data Provenance and Audit Trail | ✅ PASS | Retrieval diagnostics include endpoint, polygon, counts, and validation result for downstream reporting |
| X — Integration Flexibility | ✅ PASS | Same source-selection behavior is exposed through core, CLI, Python, and .NET |
| XI — Modern Module Organization | ✅ PASS | New Rust modules use `foo.rs` + `foo/` layout, not `mod.rs` |

### Post-Phase 1 Re-check

| Principle | Check | Notes |
|---|---|---|
| I — Library-First | ✅ PASS | Design centers on `tp-core` retrieval services reused by all callers |
| II — CLI Interface Mandatory | ✅ PASS | Quickstart and contracts define CLI usage without a topology file |
| III — High Performance | ✅ PASS | Two tabular `SELECT` queries avoid RDF graph parsing overhead |
| IV — TDD | ✅ PASS (planned) | Contracts and quickstart define the failing test surfaces before implementation |
| V — Full Test Coverage | ✅ PASS (planned) | Validation matrix covers covered, uncovered, coarse-geometry, and zero-netrelation cases |
| VI — Timezone Awareness | ✅ PASS | No naive timestamps introduced |
| VII — CRS Awareness | ✅ PASS | Retrieval polygon and parsed geometry remain explicitly WGS84 |
| VIII — Thorough Error Handling | ✅ PASS | Data model includes outcome categories and validation reports |
| IX — Data Provenance and Audit Trail | ✅ PASS | Retrieval outcome model includes source and validation metadata |
| X — Integration Flexibility | ✅ PASS | API contract includes CLI, Python, and .NET surfaces |
| XI — Modern Module Organization | ✅ PASS | Proposed file layout conforms to constitution |

## Project Structure

### Documentation (this feature)

```text
specs/006-download-rinf-topology/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── api.md
└── tasks.md
```

### Source Code (repository root)

```text
tp-core/
├── src/
│   ├── lib.rs
│   ├── errors.rs
│   ├── io.rs
│   ├── io/
│   │   ├── geojson.rs
│   │   └── rinf.rs                  # SPARQL query builder, endpoint client, row parsing
│   ├── workflow.rs                  # High-level topology source selection for topology-dependent flows
│   ├── models.rs
│   └── models/
│       └── retrieval.rs             # Retrieval outcome, validation report, options
└── tests/
    └── rinf_topology.rs             # Integration/contract tests for query generation and validation

tp-cli/
└── src/
    └── main.rs                      # Optional network argument, RINF flags, diagnostics mapping

tp-py/
├── src/
│   └── lib.rs                       # Optional network input / RINF-enabled overloads
└── python/
    └── tp_lib/
        └── __init__.py              # Wrapper docs for auto-retrieval entry points

tp-net/
├── src/
│   └── lib.rs                       # FFI entry points for auto-topology workflows
└── csharp/
    ├── TpLib.cs                     # Public API overloads / nullable network input
    ├── Models.cs                    # Retrieval options and result enums
    └── Tests/
        └── RinfTopologyTests.cs

test-data/
└── ...                              # Smoke-test GNSS fixture inside the known-good polygon added during implementation
```

**Structure Decision**: Extend the existing multi-crate workspace. Keep the low-level map-matching and parsing algorithms in `tp-core` unchanged, and add a reusable retrieval orchestration layer plus thin CLI/Python/.NET adapters on top.

## Complexity Tracking

No constitution violations currently require justification.
