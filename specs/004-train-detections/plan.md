# Implementation Plan: Absolute Train Position Detections (Punctual & Linear)

**Branch**: `004-train-detections` | **Date**: 2026-05-01 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-train-detections/spec.md`

## Summary

Extend the existing GNSS-driven path-calculation pipeline (`tp-core`) with **absolute position detections** that anchor the calculated path to known netelements at known timestamps. Two detection kinds are supported: **punctual** (single timestamp + topological or coordinate location) and **linear** (`[t_from, t_to]` window + netelement). Detections are loaded from optional GeoJSON or CSV files (auto-detected by extension), validated, time-range filtered against the GNSS log, coordinate-resolved when needed (via the existing R-tree), and injected into the HMM/Viterbi map-matcher as **hard anchors** (forced-emission states for punctual, window-restricted candidate sets for linear). Provenance of every detection (applied vs. discarded with reason) is recorded on the `PathResult` and surfaced through the CLI summary and the path-review webapp (markers + per-detection details panel; read-only).

Technical approach:

- New `Detection` enum (Punctual / Linear) in `tp-core/src/models/`, with parsers in `tp-core/src/io/{csv,geojson}/detections.rs`.
- New module `tp-core/src/detections.rs` (load, validate, time-filter, coordinate-resolve, conflict-detect).
- Extend `tp-core/src/path.rs::PathConfig` with optional anchors; `viterbi.rs` modifies emission/transition logic to force anchored states.
- Extend `PathResult` with `detection_provenance: Vec<DetectionRecord>`.
- Add `--punctual-detections`, `--linear-detections`, `--cutoff-distance-detections` CLI flags.
- Webapp: new `/api/detections` endpoint + Leaflet layer rendering markers/highlights + click-to-open details panel.

## Technical Context

**Language/Version**: Rust 1.91.1+ (workspace edition 2021)
**Primary Dependencies**: `geo` 0.28, `rstar` 0.12, `geojson` 0.24, `csv` 1.x, `serde`/`serde_json`, `chrono` (DateTime<FixedOffset>), `petgraph`, `proj4rs` 0.1.9; webapp: `axum`, `tokio`, Leaflet (static)
**Storage**: File-based I/O (CSV / GeoJSON); no DB. R-tree (`rstar`) in-memory spatial index reused for coordinate resolution.
**Testing**: `cargo test` (unit + integration in `tp-core/tests/` and `tp-cli/tests/`); `cargo bench` (Criterion) for SC-005 (≤20% overhead); `cargo llvm-cov` for coverage.
**Target Platform**: Linux/Windows/macOS (CLI library + webapp); `tp-py` PyO3 bindings unaffected by this feature in scope (detections not yet exposed to Python — out of scope).
**Project Type**: Single Rust workspace with sub-crates (`tp-core`, `tp-cli`, `tp-webapp`, `tp-py`). Webapp = backend (axum) + static frontend (Leaflet/JS); fits "web" sub-pattern within the unified library.
**Performance Goals**: SC-005 — path calculation with up to 1,000 detections completes within 20% of the wall-clock baseline of the same workload without detections (≤10,000 GNSS positions, typical workload).
**Constraints**: Detections MUST NOT influence GNSS projection (FR-016, SC-006 — bit-identical projected output when calculated path is unchanged). Memory: detections ≤ 1,000 → negligible vs. existing GNSS/network footprint.
**Scale/Scope**: ≤10k GNSS positions × ≤1k detections per run; webapp loads one path at a time.

## Constitution Check

| Principle | Compliance | Notes |
|---|---|---|
| I. Library-First | PASS | All logic in `tp-core`; CLI/webapp are thin frontends. |
| II. CLI Mandatory | PASS | New flags `--punctual-detections`, `--linear-detections`, `--cutoff-distance-detections`; summary line per FR-020. |
| III. High Performance | PASS | Anchors short-circuit Viterbi candidate sets (fewer states, not more). SC-005 budget set. Criterion benches added. |
| IV. TDD | PASS | Failing contract & integration tests written first (Phase 1 contracts/, then `tp-core/tests/detections_*.rs`). |
| V. Full Coverage | PASS | Each FR has at least one test; edge cases (FR-005..FR-007a, FR-009..FR-011, conflicts) all covered. |
| VI. Timezone Awareness | PASS | Detection timestamps parsed as `DateTime<FixedOffset>`; same validator as GNSS. |
| VII. CRS Explicit | PASS | Coordinate-only punctual detections REQUIRE `crs` field (FR-003); rejected otherwise. |
| VIII. Error Handling | PASS | Typed `DetectionError` variants; fatal vs. discard distinguished per FR (FR-005/006/007/007a fatal; FR-009/010/011 discard+warn). |
| IX. Provenance | PASS | `DetectionRecord` per detection on `PathResult` (id, source, status, reason). |
| X. Integration Flexibility | PASS | Both CSV and GeoJSON; auto-detected by extension; equivalent schemas (FR-002a/b). |
| XI. Modern Module Org | PASS | `detections.rs` + `detections/` submodule directory; no `mod.rs`. |
| Apache-2.0 Compatibility | PASS | No new dependencies; reuses already-vetted crates. |

**Result**: PASS — no violations. Complexity Tracking section unused.

### Post-Design Re-evaluation

After Phase 1 (data-model, contracts, quickstart) the constitution gates were re-checked against the concrete designs:

- **I/II Library + CLI**: All detection logic lives in `tp-core::detections`; `tp-cli` is a thin wrapper exposing `--punctual-detections`, `--linear-detections`, `--cutoff-distance-detections`. ✓
- **III Performance**: Anchor injection is O(N·K) on the existing Viterbi pass; resolution reuses the existing R-tree. SC-005 (≤20% overhead) enforced by `criterion` benchmark `detections_overhead.rs`. ✓
- **IV/V TDD + 100% coverage**: Contract tests `detections_load.rs`, `detections_filter.rs`, `detections_resolve.rs`, `detections_anchor.rs`, `detections_provenance.rs`, `cli_detections.rs`, `api_detections.rs` planned. ✓
- **VI Timezone**: All timestamps `DateTime<FixedOffset>`; rejection of naive timestamps documented in both contracts. ✓
- **VII CRS**: Punctual GeoJSON uses `EPSG:4326` default with optional `properties.crs` override; CSV requires explicit `crs` column for coordinate rows. ✓
- **VIII Errors**: `DetectionError` finalized with 8 variants in data-model.md; fatal vs. discard split matches spec FRs. ✓
- **IX Provenance**: `PathResult.detection_provenance` shape locked in `contracts/path-result-provenance.md`; preserves input order. ✓
- **X Integration**: CSV ⇄ GeoJSON round-trip equivalence asserted in both contracts. ✓
- **XI Modern Modules**: Layout (`detections.rs` + `detections/{load,validate,filter,resolve}.rs`) confirmed; no `mod.rs`. ✓

**Post-Design Result**: PASS — no new violations introduced by the design.

## Project Structure

### Documentation (this feature)

```text
specs/004-train-detections/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── detections-csv.md
│   ├── detections-geojson.md
│   ├── path-result-provenance.md
│   └── webapp-detections-api.md
└── tasks.md             # Phase 2 (NOT created here)
```

### Source Code (repository root)

```text
tp-core/
├── src/
│   ├── models.rs                        # add: pub mod detection; pub mod detection_record;
│   ├── models/
│   │   ├── detection.rs                 # NEW: Detection enum (Punctual / Linear), validation
│   │   └── detection_record.rs          # NEW: provenance record
│   ├── detections.rs                    # NEW: load/validate/filter/resolve orchestration
│   ├── detections/
│   │   ├── load.rs                      # NEW: format dispatch by extension
│   │   ├── validate.rs                  # NEW: FR-005..FR-007a validation
│   │   ├── filter.rs                    # NEW: FR-010, FR-011 time-range filter
│   │   └── resolve.rs                   # NEW: FR-008, FR-009 coord → netelement (R-tree)
│   ├── io/
│   │   ├── csv/detections.rs            # NEW: CSV parsers (punctual + linear)
│   │   └── geojson/detections.rs        # NEW: GeoJSON parsers (punctual + linear)
│   ├── path.rs                          # extend PathConfig with anchors
│   └── path/
│       ├── viterbi.rs                   # modify: forced-state for anchors
│       └── candidate.rs                 # modify: window-restricted candidates for linear
└── tests/
    ├── detections_load.rs               # NEW: format/extension/schema (FR-001..FR-007a)
    ├── detections_filter.rs             # NEW: time-range (FR-010, FR-011)
    ├── detections_resolve.rs            # NEW: coord-only (FR-008, FR-009)
    ├── detections_anchor.rs             # NEW: integration — punctual & linear anchoring (US1, US2, US3)
    └── detections_provenance.rs         # NEW: FR-017 record correctness

tp-cli/
├── src/main.rs                          # add CLI flags + summary line (FR-019, FR-020)
└── tests/
    └── cli_detections.rs                # NEW: invocation matrices

tp-webapp/
├── src/
│   ├── server.rs                        # add: GET /api/detections route
│   └── server/detections.rs             # NEW: handler returning provenance + geometry
├── static/
│   ├── index.html                       # add: detections layer toggle + details panel
│   └── js/detections.js                 # NEW: Leaflet layer + click-to-open panel
└── tests/
    └── api_detections.rs                # NEW: contract tests for /api/detections
```

**Structure Decision**: Single Rust workspace, multi-crate (existing). Detections live in `tp-core` (library-first); CLI and webapp are thin consumers. Module file naming follows Constitution XI (`detections.rs` + `detections/` directory; no `mod.rs`).

## Complexity Tracking

> No constitution violations — section intentionally empty.
