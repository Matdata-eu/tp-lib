# Tasks: Train Path Review Webapp

**Branch**: `003-path-review-webapp`
**Input**: Design documents from `specs/003-path-review-webapp/`
**Prerequisites**: [plan.md](plan.md) ✅ | [spec.md](spec.md) ✅ | [research.md](research.md) ✅ | [data-model.md](data-model.md) ✅ | [contracts/api.md](contracts/api.md) ✅ | [contracts/cli.md](contracts/cli.md) ✅

**Tests**: Included — TDD is **mandatory** per constitution principle IV. Tests MUST be written first and must FAIL before implementation begins.

---

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: User story label — [US1], [US2], [US3]
- Setup and Foundational phases have no story label

---

## Phase 1: Setup (Project Initialization)

**Purpose**: Register the new crate in the workspace and lay down the static asset scaffold.

- [X] T001 Add `tp-webapp` to workspace members list in the root Cargo.toml; add workspace-level shared dependencies for `axum`, `tokio`, `rust-embed`, `open`, `serde`, `serde_json`; do **not** add `reqwest` at workspace level — it belongs in tp-webapp's own `[dev-dependencies]` (B2 fix)
- [X] T002 Create tp-webapp/Cargo.toml with `[dependencies]`: `axum ^0.8`, `tokio ^1 (full)`, `rust-embed ^8 (debug-embed)`, `open ^5`, `serde`, `serde_json`, `tp-core` path dep; `[dev-dependencies]`: `reqwest` (with `json` feature, for integration tests), `tokio-util ^0.7` (with `rt` feature, for graceful shutdown in tests) — B2+B3 fix
- [X] T003 [P] Add Leaflet.js 1.9 distribution files to tp-webapp/static/leaflet/ (leaflet.js, leaflet.css, images/)

**Checkpoint**: `cargo build --package tp-webapp` compiles the empty crate; Leaflet assets are present in static/leaflet/

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types and infrastructure that MUST be complete before ANY user story implementation.

⚠️ **CRITICAL**: No user story work begins until this phase is complete.

- [X] T004 Create `PathOrigin` enum (`Algorithm | Manual`, `#[default = Algorithm]`, `serde rename_all = "lowercase"`) in tp-core/src/models/path_origin.rs
- [X] T005 Add `origin: PathOrigin` field with `#[serde(default)]` to `AssociatedNetElement` in tp-core/src/models/associated_net_element.rs; add `pub use path_origin::PathOrigin` to tp-core/src/models.rs; add `mod path_origin;` to tp-core/src/models.rs; add unit test: deserialize a JSON/CSV row **without** an `origin` field → assert `origin == PathOrigin::Algorithm` (backward-compat guard for existing path files produced before this change; validates `#[serde(default)]` is effective) — C4 fix
- [X] T006 [P] Create tp-webapp/src/embed.rs with `rust_embed::RustEmbed` `EmbeddedAssets` struct pointing to `../static/`; implement static-file axum handler `static_handler()`
- [X] T007 [P] Create tp-webapp/src/server/state.rs with `WebAppState` struct, `AppMode` enum (`Standalone | Integrated`), and `ConfirmResult` enum (`Confirmed | Aborted`) per data-model.md
- [X] T008 Create tp-webapp/src/server.rs: axum router builder wiring all routes + `Arc<RwLock<WebAppState>>`; port-selection loop (try 8765–8774, return bound `TcpListener` + actual port); expose `build_router()` and `bind_port()` functions
- [X] T009 Create tp-webapp/src/lib.rs: define `WebAppError`; implement **final** stub signatures — `run_webapp_standalone(network: &RailNetwork, path: TrainPath, output_path: Option<PathBuf>, gnss: Option<Vec<GnssPosition>>, port: u16, open_browser: bool) -> Result<(), WebAppError>` and `run_webapp_integrated(network: &RailNetwork, path: TrainPath, gnss: Option<Vec<GnssPosition>>, port: u16, open_browser: bool) -> Result<(ConfirmResult, TrainPath), WebAppError>` — both return `Err(WebAppError::NotImplemented)` for now; these signatures are **final** and MUST NOT change in later tasks (C1+C2 fix)

**Checkpoint**: `cargo build --package tp-webapp` clean; `cargo test --package tp-core` still passes (backward compat via `#[serde(default)]`)

---

## Phase 3: User Story 1 — Standalone Path Review and Export (Priority: P1) 🎯 MVP

**Goal**: Launch the webapp from the CLI, view all network segments on a Leaflet map, click to add/remove path segments, and save the edited path to a CSV file.

**Independent Test**: `tp-cli webapp --network test-data/sample_network.geojson --train-path <path.csv> --output /tmp/out.csv` opens the browser, map shows network + highlighted path, user can add/remove segments, Save writes a valid CSV accepted by `--train-path`.

### Tests for User Story 1 ⚠️ Write first — must FAIL before implementation

- [X] T010 [P] [US1] Write unit tests for `add_segment()` (snap insertion: topologically adjacent, ambiguous case → append-nearest-end, disconnected marker) and `remove_segment()` in tp-webapp/tests/unit/edit_test.rs
- [X] T011 [P] [US1] Write unit tests for GET `/`, GET `/api/network` (in_path/origin/confidence annotated), GET `/api/path` (ordered segments + path_index + mode), PUT `/api/path` (replace path, 422 on unknown netelement_id), POST `/api/save` (writes file, keeps server alive), POST `/api/save` with empty path segments (assert 200 + `{"ok": true}`; the empty-path guard is client-side per EC-3 — server saves as-is) using axum `TestClient` in tp-webapp/tests/unit/routes_test.rs — EC-3/C3 partial
- [X] T012 [US1] Write end-to-end integration test for US1 standalone flow: spin up server on random port, POST network + path via setup, add segment, save, read output CSV, assert result is valid via `parse_trainpath_csv` in tp-webapp/tests/integration/webapp_integration_test.rs

### Implementation for User Story 1

- [X] T013 [US1] Implement `add_segment(netelement_id, network, path) -> TrainPath` (petgraph netrelations snap insertion; append-nearest-end + disconnected marker when no unambiguous position) and `remove_segment(netelement_id, path) -> TrainPath` in tp-webapp/src/edit.rs
- [X] T014 [US1] Implement GET `/` (serve index.html from `EmbeddedAssets`) and generic static-asset handler for `/app.js`, `/style.css`, `/leaflet/*` in tp-webapp/src/server/routes.rs
- [X] T015 [US1] Implement GET `/api/network` handler: read `RwLock` state, build GeoJSON `FeatureCollection` annotating each netelement with `in_path`, `origin` (`null` when not in path), `confidence` (`null` when not in path) per contracts/api.md schema in tp-webapp/src/server/routes.rs
- [X] T016 [US1] Implement GET `/api/path` handler: return ordered `PathSegment` array with `path_index`, `overall_probability`, and `mode` field in tp-webapp/src/server/routes.rs
- [X] T017 [US1] Implement PUT `/api/path` handler: deserialise body, validate all `netelement_id` values exist in loaded network (422 if not), replace `state.path` under write lock in tp-webapp/src/server/routes.rs
- [X] T018 [US1] Implement POST `/api/save` handler: call `write_trainpath_csv` to `state.output_path` (derive default name `tp_reviewed_<timestamp>.csv` when `None`), return `{"ok": true, "path": "<written path>"}`, keep server alive in tp-webapp/src/server/routes.rs
- [X] T019 [P] [US1] Create tp-webapp/static/index.html: map container div, sidebar panel (segment list, confidence legend), Save button (standalone only), status bar; link Leaflet CSS + app.js + style.css
- [X] T020 [P] [US1] Create tp-webapp/static/style.css: full-page map layout, sidebar positioned right, confidence colour gradient (red 0.0 → yellow 0.5 → green 1.0), manual segment dashed-stroke style, disconnected segment cross-hatch style
- [X] T021 [US1] Create tp-webapp/static/app.js: fetch `/api/network` on load → render all netelements as Leaflet polylines coloured by confidence; click non-path segment → call `add_segment` logic (PUT /api/path with updated list); click in-path segment → remove (PUT /api/path); Save button → **if current rendered segment list is empty, show a browser confirmation dialog ('Save an empty path? This cannot be undone.') before proceeding (EC-3)** → POST /api/save → show status; handle standalone/integrated mode field from GET /api/path — C3 fix
- [X] T022 [US1] Extend tp-cli/src/main.rs with `webapp` subcommand (clap): `--network <FILE>` (required), `--train-path <FILE>` (required), `--output <FILE>` (optional), `--port <u16>` (optional, default 8765), `--no-browser` flag; load files via tp-core I/O; call `run_webapp_standalone(network, path, output, gnss: None, port, open_browser)` — pass `None` for `gnss` here; GNSS wiring is added in T034; print URL to stdout

**Checkpoint**: US1 is fully functional end-to-end. Standalone launch, map loads, segments add/remove, Save writes valid CSV.

---

## Phase 4: User Story 2 — Integrated Review During GNSS Projection Pipeline (Priority: P2)

**Goal**: Add `--review` to the standard pipeline command; after path calculation the webapp opens, the user reviews/edits, Confirm unblocks the pipeline to continue projection, Abort exits non-zero.

**Independent Test**: Run full pipeline with `--review`; process pauses after path calculation, browser opens; clicking Confirm returns exit code 0 and projection output matches confirmed path; clicking Abort returns non-zero exit code with cancellation message on stderr.

### Tests for User Story 2 ⚠️ Write first — must FAIL before implementation

- [X] T023 [P] [US2] Write unit tests for POST `/api/confirm` (sends `ConfirmResult::Confirmed` on oneshot channel, returns 200 `{"ok": true}`) and POST `/api/abort` (sends `ConfirmResult::Aborted`, returns 200 `{"ok": true}`) using axum `TestClient` in tp-webapp/tests/unit/routes_test.rs
- [X] T024 [US2] Write integration test for integrated mode: start server in `Integrated` mode with oneshot channel, POST /confirm → assert `ConfirmResult::Confirmed` received on channel; start second server, POST /abort → assert `ConfirmResult::Aborted`; assert server shuts down after confirm/abort in tp-webapp/tests/integration/webapp_integration_test.rs

### Implementation for User Story 2

- [X] T025 [US2] Implement POST `/api/confirm` handler: take `confirm_tx` from `WebAppState` under write lock (error 409 if already consumed), send `ConfirmResult::Confirmed`, initiate server shutdown in tp-webapp/src/server/routes.rs
- [X] T026 [US2] Implement POST `/api/abort` handler: take `confirm_tx` from `WebAppState` under write lock (error 409 if already consumed), send `ConfirmResult::Aborted`, initiate server shutdown in tp-webapp/src/server/routes.rs
- [X] T027 [US2] Implement `run_webapp_integrated(network, path, gnss: Option<Vec<GnssPosition>>, port, open_browser)` in tp-webapp/src/lib.rs: build `WebAppState` with `AppMode::Integrated`, `oneshot::channel()`, and the provided `gnss` data (enables GNSS markers in integrated review per US2 AS-2); spawn server on tokio runtime; block `await` on the oneshot receiver; return `Ok((ConfirmResult, TrainPath))` or `Err(WebAppError::ChannelClosed)` — C1 fix
- [X] T028 [P] [US2] Extend tp-webapp/static/app.js: when GET `/api/path` returns `mode: "integrated"`, show Confirm + Abort buttons (hide Save button); wire Confirm button → POST `/api/confirm`; wire Abort button → POST `/api/abort`; show "Confirmed — projection continuing…" or "Aborted — pipeline cancelled" feedback
- [X] T029 [US2] Extend tp-cli/src/main.rs: add `--review` flag to the existing pipeline command; after `calculate_path()` succeeds, call `run_webapp_integrated(network, path, gnss, port, open_browser)` inside a tokio runtime, passing the already-parsed `gnss` data from the pipeline's `--gnss` argument (or `None`) so GNSS markers appear per US2 AS-2 — C1 fix; if `ConfirmResult::Aborted`, print cancellation message to stderr and `std::process::exit(1)`; if `ConfirmResult::Confirmed`, continue pipeline with the (possibly edited) path

**Checkpoint**: US1 and US2 both work independently. `--review` pipeline pauses, review is applied, projection continues.

---

## Phase 5: User Story 3 — GNSS Position Overlay in Standalone Mode (Priority: P3)

**Goal**: When `--gnss` is provided to the standalone webapp command, GNSS position markers appear on the map for diagnostic context alongside the network and path.

**Independent Test**: `tp-cli webapp --network … --train-path … --gnss positions.csv` shows GNSS circle markers on the map; omitting `--gnss` shows no markers; map interaction is not affected by marker presence.

### Tests for User Story 3 ⚠️ Write first — must FAIL before implementation

- [X] T030 [P] [US3] Write unit tests for GET `/api/gnss`: assert returns GeoJSON `FeatureCollection` of `Point` features when GNSS loaded; assert returns empty `FeatureCollection` (`{"type":"FeatureCollection","features":[]}`) when `state.gnss` is `None` in tp-webapp/tests/unit/routes_test.rs
- [X] T031 [US3] Write integration test for GNSS overlay: launch server with GNSS data → GET `/api/gnss` returns expected count of Point features at correct coordinates; launch without GNSS → GET `/api/gnss` returns empty FeatureCollection in tp-webapp/tests/integration/webapp_integration_test.rs

### Implementation for User Story 3

- [X] T032 [US3] Implement GET `/api/gnss` handler in tp-webapp/src/server/routes.rs: returns GeoJSON `FeatureCollection` of `Point` features (`[lon, lat]`) one per `GnssPosition`; returns empty FeatureCollection when `state.gnss` is `None`
- [X] T033 [P] [US3] Extend tp-webapp/static/app.js: fetch GET `/api/gnss` on load; if `features.length > 0`, render each point as a small Leaflet `circleMarker` (low z-index, non-interactive, distinct colour); skip rendering silently when array is empty
- [X] T034 [US3] Add `--gnss <FILE>` (optional) to the `webapp` subcommand in tp-cli/src/main.rs; when present, parse via `parse_gnss_csv()` and pass positions as `Some(gnss)` to `run_webapp_standalone()`; when absent, pass `None` — the signature already accepts `Option<Vec<GnssPosition>>` as finalized in T009; **no signature changes needed** (C2 fix)

**Checkpoint**: All three user stories work independently. GNSS markers appear/disappear based on CLI flag.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T035 [P] Add `tp-webapp` to deny.toml `[advisories]` and `[licenses]` sections; run `cargo deny check` and fix any issues
- [X] T036 [P] Run `cargo clippy --package tp-webapp --all-targets -- -D warnings` and fix all warnings
- [X] T037 [P] Run `cargo fmt --all` and verify no unformatted files remain
- [X] T038 Verify `webapp` feature flag: `cargo build --package tp-cli --no-default-features` (must compile without axum/tokio); `cargo build --package tp-cli` (must compile with webapp); `cargo test --package tp-webapp` all green

---

## Dependencies (User Story Completion Order)

```
Phase 1 (Setup)
    │
    ▼
Phase 2 (Foundational)
    │
    ▼
Phase 3 (US1 — MVP)  ◄── Can deliver standalone review independently
    │
    ▼
Phase 4 (US2)  ◄── Requires US1 server + edit logic + CLI subcommand
    │
Phase 3 (US1) ──► Phase 5 (US3)  ◄── Requires US1 server + webapp subcommand; independent of US2
    │
    ▼
Phase 6 (Polish)  ◄── Requires all story phases complete
```

**Key dependency constraints**:
- T005 depends on T004 (`PathOrigin` must exist before `AssociatedNetElement` is extended)
- T008 depends on T006 (embed.rs) and T007 (state.rs)
- T009 depends on T008 (router must be built before lib entry points)
- T010, T011 (write tests) can be written before T013–T018 (implementations); TDD order is intentional
- T013 (edit.rs) depends on T004/T005 (PathOrigin type) and T007 (WebAppState for context)
- T021 (app.js) depends on T019 (index.html structure) and T020 (style.css class names)
- T022 (tp-cli webapp subcommand) depends on T009 (lib.rs entry point signature)
- T025/T026 depend on T007 (ConfirmResult type) from Foundational phase
- T027 depends on T025/T026 and T009
- T029 depends on T027
- T034 wires the `--gnss` CLI argument to `run_webapp_standalone()`; the function signature is already finalized in T009 (both entry-point functions accept `gnss: Option<Vec<GnssPosition>>`)

---

## Parallel Execution Examples

### Phase 2 (after T004 → T005 complete)
| Agent A | Agent B |
|---------|---------|
| T006 — embed.rs | T007 — server/state.rs |
| T008 — server.rs (depends on T006, T007) | — |
| T009 — lib.rs (depends on T008) | — |

### Phase 3 Tests (before implementation)
| Agent A | Agent B |
|---------|---------|
| T010 — edit_test.rs tests | T011 — routes_test.rs tests |
| T012 — integration test (depends on T010, T011) | — |

### Phase 3 Frontend (parallel with route implementation)
| Agent A (backend) | Agent B (frontend) |
|-------------------|--------------------|
| T013 — edit.rs | T019 — index.html |
| T014–T018 — routes.rs | T020 — style.css |
| — | T021 — app.js (depends on T019, T020) |

### Phase 4 (after Phase 3 complete)
| Agent A | Agent B |
|---------|---------|
| T023 — confirm/abort unit tests | T024 — integration test |
| T025 — POST /confirm | T028 — app.js integrated mode UI |
| T026 — POST /abort | — |
| T027 — lib.rs run_webapp_integrated | — |
| T029 — tp-cli --review flag | — |

### Phase 6
| Agent A | Agent B | Agent C |
|---------|---------|---------|
| T035 — deny.toml | T036 — clippy | T037 — fmt |
| T038 — feature flag smoke test (depends on T035–T037) | — | — |

---

## Implementation Strategy

### MVP Scope: Phase 3 (US1) Only

Deliver Phase 1 → Phase 2 → Phase 3 to get a working standalone path review tool. This is independently valuable and can be shipped before US2 and US3 are complete.

**MVP acceptance**: `tp-cli webapp --network sample_network.geojson --train-path path.csv --output reviewed.csv` works end-to-end; output CSV is accepted by `--train-path`.

### Incremental Delivery

1. **T001–T009** (Setup + Foundational) — scaffolding only, no user-visible features
2. **T010–T022** (US1 MVP) — first working product increment; all three test layers green ✅
3. **T023–T029** (US2) — integrated pipeline review; depends on US1 ✅
4. **T030–T034** (US3) — GNSS overlay; depends on US1; independent of US2 ✅
5. **T035–T038** (Polish) — quality gate before merge ✅

### TDD Reminder

Per constitution principle IV (non-negotiable):

> Tests are written **FIRST**. Implementation is **not started** until the failing test exists.
> Red → Green → Refactor, per phase, per story.

In each user story phase: run test tasks (T010–T012, T023–T024, T030–T031) first, **confirm they fail**, then write implementation.

---

## Summary

| Phase | Tasks | User Story | Parallel Opportunities |
|-------|-------|------------|------------------------|
| Phase 1: Setup | T001–T003 | — | T003 independent of T001 |
| Phase 2: Foundational | T004–T009 | — | T006 ‖ T007 |
| Phase 3: US1 MVP | T010–T022 | US1 (P1) | T010 ‖ T011; T013 ‖ T019 ‖ T020 |
| Phase 4: US2 | T023–T029 | US2 (P2) | T023 ‖ T024; T025 ‖ T028; T026 ‖ T028 |
| Phase 5: US3 | T030–T034 | US3 (P3) | T030 ‖ T031; T033 ‖ T032 |
| Phase 6: Polish | T035–T038 | — | T035 ‖ T036 ‖ T037 |
| **Total** | **38 tasks** | | |

**Independent test criteria per story**:
- **US1**: `tp-cli webapp --network … --train-path … --output …` full flow without any other feature
- **US2**: Full pipeline with `--review` — confirm and abort paths; independent of US3
- **US3**: `tp-cli webapp … --gnss …` GNSS overlay; independent of US2
