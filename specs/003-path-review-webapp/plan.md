# Implementation Plan: Train Path Review Webapp

**Branch**: `003-path-review-webapp` | **Date**: March 21, 2026 | **Spec**: [spec.md](spec.md)  
**Input**: Feature specification from `specs/003-path-review-webapp/spec.md`

## Summary

Implement a local-only web application that allows railway engineers to visually review and edit a calculated train path before using it for GNSS projection. A new `tp-webapp` library crate serves an embedded Leaflet.js map from an axum HTTP server, displaying the railway network, the calculated path with confidence scores, and optionally GNSS position markers. Users can add and remove segments, then save (standalone mode) or confirm/abort (integrated mode). The crate is integrated into `tp-cli` behind a `webapp` Cargo feature flag, exposed as a `webapp` subcommand and a `--review` flag on the default pipeline command.

## Technical Context

**Language/Version**: Rust 2021 edition, latest stable (1.80+)

**Primary Dependencies**:
- Core library: `tp-core` — existing `TrainPath`, `AssociatedNetElement`, `Netelement`, `NetRelation`, `RailwayNetwork`, and all I/O functions (`parse_network_geojson`, `parse_trainpath_csv`, `parse_gnss_csv`, `write_trainpath_csv`)
- Web server: `axum` ^0.8 with `tokio` ^1 (full async, multi-threaded runtime)
- Static assets: `rust-embed` ^8 (bundle `tp-webapp/static/` into binary at compile time; zero runtime file I/O)
- Browser launch: `open` ^5 (cross-platform default browser opener)
- Data serialization: `serde` + `serde_json` (existing workspace dependencies)
- Frontend: Leaflet.js 1.9 + vanilla HTML/CSS/JS (no build step, no npm, files live in `tp-webapp/static/`)

**Storage**: File-based (GeoJSON network, CSV train path, CSV GNSS positions) — no database  
**Testing**: `cargo test` — axum `TestClient` for unit endpoint tests; integration tests spinning up a live server on a random port; `cargo test --package tp-cli` for CLI integration tests for the `webapp` subcommand and `--review` flag  
**Target Platform**: Local desktop (Windows, macOS, Linux); localhost-only; modern desktop browser (Chrome ≥90, Firefox ≥90, Edge ≥90)  
**Project Type**: Web application with embedded frontend — new `tp-webapp` library crate in the workspace; Rust binary serves HTML/JS/CSS embedded at compile time via rust-embed  

**Performance Goals**:
- Map loads and becomes interactive in <10 s for a network of ≤5,000 netelements (SC-002)
- Path edits reflected on map within 1 s of user action (SC-003)
- Server startup → browser-open sequence completes within 5 s (SC-006)

**Constraints**:
- Server binds to `127.0.0.1` only; not accessible from other machines; no authentication needed
- No npm, no frontend build step; all assets embedded at compile time via rust-embed
- No mobile browser support (out of scope for MVP)
- OSM tile background optional and deactivatable; map remains fully usable without internet
- No undo/redo beyond session reset (refresh resets to original loaded path)
- Single concurrent review session at a time; multi-user is out of scope

**Scale/Scope**:
- ≤5,000 network netelements per session (all loaded in-memory in one shot)
- ≤200 path segments per session
- 1 new Cargo crate (`tp-webapp`) with ~1,500 lines of Rust + ~500 lines of HTML/JS/CSS
- 2 changes to `tp-core` models: new `PathOrigin` enum; new `origin` field on `AssociatedNetElement` (backward-compatible serde default)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ I. Library-First Architecture
- **Status**: PASS
- `tp-webapp` is a library crate (`lib.rs` exposes `run_webapp_standalone` and `run_webapp_integrated`); no business logic lives in `tp-cli`
- All data loading and path editing logic goes in `tp-webapp`; all data model logic stays in `tp-core`
- `tp-cli` only constructs arguments and calls into the library

### ✅ II. CLI Interface Mandatory
- **Status**: PASS
- `tp-cli webapp --network … --train-path …` subcommand (standalone mode)
- `tp-cli … --review` flag on the default pipeline command (integrated mode)
- All errors written to stderr; exit codes propagated correctly (non-zero on abort)
- `--help` and `--version` inherited from existing clap `Parser` setup

### ✅ III. High Performance
- **Status**: PASS
- Entire network served as a single GeoJSON payload (no incremental streaming needed at ≤5,000 segments)
- rust-embed bundles static assets at compile time — zero filesystem I/O when serving HTML/JS/CSS
- axum uses Tokio async I/O; request handling is non-blocking
- Snap insertion uses petgraph traversal on the already-loaded network graph (reuses tp-core infrastructure)

### ✅ IV. Test-Driven Development (NON-NEGOTIABLE)
- **Status**: PASS — MANDATORY workflow enforced
- Tests written FIRST for every API endpoint handler, state machine transition, and snap insertion logic
- Stakeholder approval of tests required before any implementation code
- Red-Green-Refactor cycle strictly followed throughout

### ✅ V. Full Test Coverage
- **Status**: PASS — Target 100%
- Unit tests: every axum route handler, `WebAppState` mutation methods, `PathOrigin` dispatch, snap logic
- Integration tests: full server started on a random port, real HTTP requests via `reqwest` or axum `TestClient`
- CLI integration tests: `tp-cli webapp` and `--review` argument parsing and behaviour
- No coverage exclusions without written justification

### ✅ VI. Time with Timezone Awareness
- **Status**: PASS
- No new temporal data is introduced; the webapp reads and writes `TrainPath` via existing `tp-core` I/O which already enforces timezone-aware timestamps
- Session state is in-memory only; no new timestamp fields are stored

### ✅ VII. Positions with Coordinate Reference System
- **Status**: PASS
- All spatial loading goes through `tp-core` I/O functions (`parse_network_geojson`, `parse_trainpath_csv`, `parse_gnss_csv`) which already enforce CRS
- The webapp passes loaded data straight to the browser; no new coordinate operations are performed server-side
- The browser renders WGS-84 positions via Leaflet's default CRS with no transformation

### ✅ VIII. Thorough Error Handling
- **Status**: PASS
- Typed axum error responses for all endpoint failure modes (file not found, serialization error, already-confirmed, etc.)
- CLI prints actionable error messages to stderr with non-zero exit codes
- Port conflict → try next port and report actual URL; browser open failure → print URL and continue waiting
- No silent error swallowing; all `?` propagation handled at the call boundary

### ✅ IX. Data Provenance and Audit Trail
- **Status**: PASS
- New `PathOrigin` enum (`Algorithm` | `Manual`) added to `tp-core` models
- `AssociatedNetElement` extended with `origin: PathOrigin` field (backward-compatible `#[serde(default)]`)
- Saved/confirmed path CSV carries `origin` column, allowing downstream consumers to distinguish human edits from algorithm output
- Probability 1.0 for manually-added segments is an explicit data point, not a heuristic

### ✅ X. Integration Flexibility
- **Status**: PASS
- `tp-webapp` crate behind `webapp` Cargo feature in `tp-cli` (default-enabled; disable to build without axum/tokio)
- Library functions (`run_webapp_standalone`, `run_webapp_integrated`) accept `TrainPath` and friends, enabling future Python or other bindings
- Standard REST API with JSON bodies; any HTTP client (curl, browser, test harness) can interact with it

### ✅ XI. Modern Module Organization (Rust)
- **Status**: PASS
- `tp-webapp` uses `server.rs` + `server/` subdirectory pattern (not `server/mod.rs`)
- Module layout: `lib.rs`, `server.rs`, `server/routes.rs`, `server/state.rs`, `edit.rs`, `embed.rs`
- Consistent with existing tp-core convention

### ✅ Licensing and Legal Compliance
- **Status**: PASS
- `axum`: MIT license — compatible
- `tokio`: MIT license — compatible
- `rust-embed`: MIT OR Apache-2.0 — compatible
- `open`: MIT license — compatible
- No GPL/LGPL/proprietary dependencies introduced

**GATE STATUS**: ✅ **APPROVED** — All constitutional requirements satisfied

## Project Structure

### Documentation (this feature)

```text
specs/003-path-review-webapp/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
│   ├── api.md           # REST API contract (6 endpoints)
│   └── cli.md           # CLI contract (webapp subcommand + --review flag)
└── tasks.md             # Phase 2 output (/speckit.tasks command — NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
tp-core/
└── src/
    ├── models.rs                  # Add PathOrigin to pub use exports
    └── models/
        ├── associated_net_element.rs  # EXTEND: add origin: PathOrigin field
        └── path_origin.rs             # NEW: PathOrigin enum (Algorithm | Manual)

tp-webapp/                         # NEW crate
├── Cargo.toml
├── src/
│   ├── lib.rs                     # Public API: run_webapp_standalone(), run_webapp_integrated()
│   ├── server.rs                  # Axum router construction + server startup + port selection
│   ├── server/
│   │   ├── routes.rs              # All axum route handlers (GET /api/network, GET /api/path, etc.)
│   │   └── state.rs               # WebAppState struct + AppMode enum
│   ├── edit.rs                    # Path editing logic: add segment (snap), remove segment
│   └── embed.rs                   # rust-embed static asset struct (embeds static/ at compile time)
├── static/
│   ├── index.html                 # Single-page application shell
│   ├── app.js                     # Leaflet map + edit logic + sidebar
│   ├── style.css                  # Map and sidebar styles
│   └── leaflet/                   # Leaflet.js 1.9 library files (JS + CSS + images)
└── tests/
    ├── integration/
    │   └── webapp_integration_test.rs   # Full server on random port, reqwest HTTP client
    └── unit/
        ├── edit_test.rs                 # Snap insertion + remove logic
        └── routes_test.rs              # Axum TestClient unit tests per endpoint

tp-cli/
└── src/
    └── main.rs                    # EXTEND: webapp subcommand; --review flag on default command; tokio runtime
```

**Structure Decision**: Web application option with a new `tp-webapp` library crate. The existing workspace crates (`tp-core`, `tp-cli`) are extended minimally — a new `PathOrigin` model in `tp-core` and a new `webapp` subcommand / `--review` flag in `tp-cli`. All web and embedding logic is isolated in the new crate, preserving the existing crate boundaries.

## Complexity Tracking

> **No violations detected** — All constitutional principles satisfied without exceptions.

---

## Phase Completion Status

### ✅ Phase 0: Research & Outline (COMPLETE)

**Deliverable**: [research.md](research.md)

**Contents**:
- Decision 1: `axum` ^0.8 + `tokio` ^1 as web framework and async runtime
- Decision 2: `rust-embed` ^8 for compile-time static asset bundling (`debug-embed` for fast frontend dev)
- Decision 3: Leaflet.js 1.9 + vanilla HTML/CSS/JS frontend (no build step, no npm)
- Decision 4: `Arc<RwLock<WebAppState>>` for shared axum server state
- Decision 5: `tokio::sync::oneshot` channel for integrated-mode blocking strategy (carry `ConfirmResult`)
- Decision 6: Port default 8765, retry 8765–8774 on conflict; always print actual URL
- Decision 7: `PathOrigin` enum in `tp-core` with backward-compatible `#[serde(default)]` on `origin` field
- Decision 8: Snap insertion via petgraph `DiGraph` from existing netrelations (O(|path| × degree))
- Decision 9: `webapp` Cargo feature flag in `tp-cli`, default-enabled; disabling excludes axum/tokio
- Decision 10: Three-tier test strategy — unit handlers (`axum::test`), unit edit logic, CLI integration tests

**All unknowns resolved** ✓

### ✅ Phase 1: Design & Contracts (COMPLETE)

**Deliverables**:
1. [data-model.md](data-model.md) — all Rust types (`PathOrigin`, extended `AssociatedNetElement`, `WebAppState`, `AppMode`, `ConfirmResult`) and REST JSON shapes
2. [contracts/api.md](contracts/api.md) — 6-endpoint REST API contract (`GET /`, `GET /api/network`, `GET /api/path`, `PUT /api/path`, `POST /api/save`, `POST /api/confirm`, `POST /api/abort`)
3. [contracts/cli.md](contracts/cli.md) — `webapp` subcommand + `--review` flag specification; Cargo feature flag design
4. [quickstart.md](quickstart.md) — developer setup, standalone and integrated usage examples, map interaction reference, test commands

**Agent context updated** ✓

**Constitution re-check post-design**: All 11 principles + licensing still satisfied after completing design.
- `PathOrigin` on `AssociatedNetElement` honours principle IX (Data Provenance) with zero breaking change
- `tp-webapp` as library crate behind feature flag honours principles I and X
- All external crates use MIT/Apache-2.0 licences

---

## Next Steps (Not Part of `/speckit.plan`)

### Phase 2: Task Breakdown
**Command**: `/speckit.tasks`
**Purpose**: Decompose the design into ordered, atomic implementation tasks with per-task acceptance criteria
**Expected Output**: `specs/003-path-review-webapp/tasks.md` — implementation task list ready for execution

---

## Summary

**Branch**: `003-path-review-webapp`  
**Plan**: `specs/003-path-review-webapp/plan.md`  
**Status**: Planning Complete — Ready for Task Breakdown

**Artifacts produced**:

| File | Purpose |
|------|---------|
| [research.md](research.md) | Phase 0 — 10 design decisions, all unknowns resolved |
| [data-model.md](data-model.md) | Phase 1 — all Rust types and REST JSON shapes |
| [contracts/api.md](contracts/api.md) | Phase 1 — 6-endpoint REST API contract |
| [contracts/cli.md](contracts/cli.md) | Phase 1 — CLI contract (`webapp` subcommand + `--review`) |
| [quickstart.md](quickstart.md) | Phase 1 — developer guide and usage examples |

**Net new code** (estimates):
- `tp-webapp/`: ~1,500 lines Rust + ~500 lines HTML/JS/CSS
- `tp-core/` changes: ~30 lines (`PathOrigin` enum + `origin` field)
- `tp-cli/` changes: ~60 lines (`webapp` subcommand + `--review` flag + feature gate)
- Tests: ~400 lines (unit handlers + unit edit logic + CLI integration)

---

## Post-Implementation Changes

The following deviations from the original plan occurred during and after core implementation. These are recorded here for traceability.

### API Endpoint Redesign: `PUT /api/path` → `POST /api/path/add` + `POST /api/path/remove`

**Original design**: The browser maintained the full ordered segment list and sent it via `PUT /api/path` on every edit.

**Actual implementation**: Two granular endpoints were introduced. The browser sends a single `{ netelement_id }` body; the server handles all ordering via the existing `edit::add_segment()` / `edit::remove_segment()` logic.

- `POST /api/path/add` — calls `edit::add_segment()` (snap insertion via netrelations)
- `POST /api/path/remove` — calls `edit::remove_segment()`
- Browser refreshes `GET /api/path` + `GET /api/network` after each call

**Rationale**: The client-managed list approach required the browser to know the correct insertion position, which duplicates the snap insertion logic from `edit.rs`. The server-side approach keeps ordering canonical and eliminates a class of client–server desync bugs.

### UI Additions: Dark Mode, Basemap Toggle, Close Tab Button

Three UI controls were added after core user story implementation, covering practical usability gaps not specified in the original requirements:

1. **Dark mode** (`FR-022`): Full CSS custom property system (`--bg`, `--surface`, `--text`, etc.) with `body.dark` class toggle. `prefers-color-scheme: dark` is checked at startup to auto-apply. Leaflet tooltip/popup/bar elements also receive dark overrides.

2. **Basemap toggle** (`FR-023`): Checkbox in the map controls area of the sidebar. Toggles the OpenStreetMap tile layer on/off. Useful in offline/air-gapped environments.

3. **Close Tab button** (`FR-024`): Always-visible button in the action buttons area. Calls `window.close()`. Useful after Confirm/Abort when the server has shut down but the tab remains open.

### Non-Path Element Visibility Improvement

Default style for non-path netelements was adjusted for better visibility:

| Property | Original | Actual |
|----------|----------|--------|
| Color | `#9ca3af` | `#6b7280` |
| Weight | `2` | `3` |
| Opacity | `0.6` | `0.9` |

### `--review` Path Artifact (no `--save-path` flag)

**Original design**: An optional `--save-path <FILE>` flag was planned to allow saving the confirmed path.

**Actual implementation**: No `--save-path` flag. Instead, the confirmed path is always saved automatically on Confirm as `<output-stem>-path.<ext>`, derived from the existing `--output` flag by the `derive_path_output()` function in `tp-cli/src/main.rs`. The file is written before projection proceeds; the path is printed to stderr.

### Bug Fix: `pathData` Module Scope

The JavaScript `pathData` variable was originally declared as `const pathData = null` inside `init()`. The `onNetElementClick` handler's assignment `pathData = await apiFetch(...)` threw a `ReferenceError` in strict mode, silently aborting the post-edit network refresh when removing a middle segment. Fixed by declaring `let pathData = null` at module scope.
