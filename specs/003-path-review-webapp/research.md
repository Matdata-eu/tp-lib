# Research: Train Path Review Webapp

**Phase**: 0 — Outline & Research  
**Feature**: `003-path-review-webapp`  
**Status**: Complete — all unknowns resolved

---

## Decision 1: Web Framework and Async Runtime

**Decision**: axum ^0.8 with tokio ^1 (multi-threaded runtime)

**Rationale**: axum is the canonical Rust web framework for async HTTP. It composes directly with Tower middleware, has first-class integration with the axum `TestClient` for testing, and works seamlessly with tokio's multi-threaded runtime. It is MIT-licensed and already adopted widely in the Rust ecosystem for exactly this use-case (short-lived local utility servers). The `tokio::main` macro in tp-cli's `main.rs` enables the entire CLI to remain async without restructuring existing synchronous code — non-async work just calls `spawn_blocking`.

**Alternatives considered**:
- `actix-web`: LGPL-licensed risk for some versions; more boilerplate for a single-session local server
- `warp`: Older wrapper model; less ergonomic for typed state sharing; largely superseded by axum
- `tiny-http` (synchronous): No async, would complicate the integrated-mode blocking pattern; poorer test story

---

## Decision 2: Static Asset Embedding Strategy

**Decision**: `rust-embed` ^8 — embed the entire `tp-webapp/static/` directory at compile time

**Rationale**: rust-embed generates asset-serving code at compile time, producing a single self-contained binary with no runtime file lookups. This is essential for the distribution model: a user runs `tp-cli webapp` without needing to know where the library's static files are installed. rust-embed supports both `debug` mode (read from disk for fast frontend iteration) and `release` mode (embed into binary), which is ideal for development. MIT/Apache-2.0 licensed — compatible.

**Alternatives considered**:
- Bundle assets with `include_str!` / `include_bytes!` manually: verbose, no directory support, no auto-reloading in dev
- Ship static files alongside the binary: requires installation step, breaks portable distribution
- WASM frontend (e.g., Yew, Leptos): Massive complexity increase — requires npm/wasm-pack, build pipeline, much larger binary. Rejected per spec constraint (no build step, no npm) and per constitution III (avoid unnecessary complexity)

---

## Decision 3: Frontend Stack

**Decision**: Leaflet.js 1.9 + vanilla HTML/CSS/JS (no framework, no build step)

**Rationale**: The feature spec explicitly requires no npm/build step. Leaflet is the standard open-source interactive map library for the browser; it is BSD-2-Clause licensed (compatible), well-documented, and trivially bundled as static files. Vanilla JS is sufficient for the interaction model: three layer groups (network segments, path segments, GNSS markers), a sidebar list with remove buttons, and click handlers. No reactive state management is needed for a single-session local tool.

**Frontend file layout**:
```
static/
├── index.html        # App shell: map div + sidebar div + script/style tags
├── app.js            # Map init, layer management, edit dispatch, sidebar sync
├── style.css         # Map container height, sidebar layout, confidence colour scale
└── leaflet/          # Leaflet.js 1.9 dist files (leaflet.js, leaflet.css, images/)
```

**Alternatives considered**:
- React/Vue/Svelte: Require npm and a build step — rejected per spec constraints
- OpenLayers: Heavier than Leaflet for this use-case; more complex API for basic vector overlays
- MapLibre GL: WebGL-based; dependency on a larger JS bundle; overkill for non-tile rendering

---

## Decision 4: AppState Design

**Decision**: `Arc<RwLock<WebAppState>>` shared across axum handlers via extension

```rust
pub struct WebAppState {
    pub network: RailwayNetwork,
    pub path: TrainPath,
    pub gnss: Option<Vec<GnssPosition>>,
    pub mode: AppMode,
    pub output_path: Option<PathBuf>,
    pub confirm_tx: Option<oneshot::Sender<ConfirmResult>>,
}

pub enum AppMode {
    Standalone,
    Integrated,
}

pub enum ConfirmResult {
    Confirmed,
    Aborted,
}
```

**Rationale**: `Arc<RwLock<…>>` is the idiomatic axum pattern for shared mutable state. `RwLock` allows concurrent reads (GET /api/network, GET /api/path) while exclusive writes are rare (PUT /api/path, POST /api/save). The `confirm_tx` is a one-shot tokio channel whose `Sender` lives in state; the `run_webapp_integrated` function awaits the corresponding `Receiver` on the blocking side, enabling the CLI to pause the projection pipeline cleanly without busy-waiting or thread parking.

**Alternatives considered**:
- `Arc<Mutex<…>>`: Simpler but blocks readers unnecessarily; GET /api/network and GET /api/path would serialize against each other
- `AtomicBool` signal: Works for a binary confirmed/not flag but cannot carry the `Aborted` result or be properly `await`-ed; a tokio `oneshot` is cleaner and more expressive
- `DashMap` or other concurrent collections: Over-engineered for a single-state model

---

## Decision 5: Integrated Mode Blocking Strategy

**Decision**: tokio `oneshot` channel — `run_webapp_integrated` `await`s the receiver after spawning the server task

```
CLI main thread (tokio runtime):
  1. calculate_train_path(…)   → TrainPath
  2. build WebAppState with oneshot::channel()
  3. spawn axum server task (with Sender in state)
  4. open browser
  5. AWAIT oneshot Receiver
     - POST /confirm → state.confirm_tx.take().send(Confirmed) → server shutdowns
     - POST /abort   → state.confirm_tx.take().send(Aborted) → server shutdowns
  6. match result { Confirmed → continue projection, Aborted → exit non-zero }
```

**Rationale**: A tokio oneshot channel is the exact tool for "wait for exactly one signal". It is cancellation-safe, places no spin-wait pressure on the CPU, and integrates naturally with tokio's async executor. The server task can be given a `CancellationToken` (from tokio-util) to cleanly shut down after the channel fires.

**Alternatives considered**:
- `std::sync::Condvar`: Would require blocking a thread rather than yielding the async executor; incompatible with tokio
- HTTP polling from CLI to its own server: Architecturally circular and wasteful
- Signal files on disk: Fragile, non-portable, adds I/O dependency

---

## Decision 6: Port Selection

**Decision**: Try port 8765 first; if in use, try successively incrementing ports up to 8774; print the actual bound URL to the terminal regardless

**Rationale**: A fixed default port (8765) is easy to remember and unlikely to conflict with common development tools (3000, 5173, 8080, 8443, etc.). If it is occupied the server should not fail silently — it tries 10 ports and then returns an error. The actual URL is always printed to the terminal (FR-019) regardless of browser auto-open success.

**Alternatives considered**:
- Port 0 (OS assigns): Guaranteed not to conflict but produces an unpredictable URL that the user cannot bookmark; worse UX
- Only port 8080: Too commonly occupied by other services on developer machines

---

## Decision 7: PathOrigin and AssociatedNetElement Extension

**Decision**: Add `PathOrigin` enum to `tp-core/src/models/path_origin.rs` and add `origin: PathOrigin` field to `AssociatedNetElement` with backward-compatible serde defaults

```rust
// tp-core/src/models/path_origin.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PathOrigin {
    #[default]
    Algorithm,
    Manual,
}
```

`AssociatedNetElement` gains:
```rust
#[serde(default)]
pub origin: PathOrigin,
```

For manually-added segments, `gnss_start_index` and `gnss_end_index` are both set to `0` (they are meaningless for segments with no GNSS association; downstream consumers that check `origin == Manual` should ignore them).

**Rationale**: Adding `origin` to the core model satisfies Constitution principle IX (Data Provenance). `#[serde(default)]` ensures all existing CSV files that lack this column deserialize correctly as `Algorithm` — fully backward-compatible. The field is also serialized to the output CSV, making provenance machine-readable.

**Alternatives considered**:
- Wrapper type `WebAssociatedNetElement` in `tp-webapp`: Keeps tp-core unchanged but breaks SC-004 (output must be accepted by `--train-path`) because the wrapper's CSV format would differ from the core type. Rejected.
- Separate sidecar metadata file: Over-complex; requires two output files where one suffices

---

## Decision 8: Snap Insertion Algorithm

**Decision**: Use the existing petgraph `DiGraph` of netrelations (already constructed in tp-core's path module) to determine where a manually-added netelement fits in the current path order

**Algorithm sketch**:
1. Load netrelations from the network GeoJSON (already done during server startup)
2. When the user adds netelement `N` to the path:
   a. For each position `i` in the current path, check if there is a navigable edge from `path[i]` → `N` (N follows path[i]) or from `N` → `path[i+1]` (N precedes path[i+1])
   b. If exactly one insertion gap satisfies both constraints, insert N there
   c. If multiple positions satisfy, pick the one where the combined edge weights (haversine distances) are smallest
   d. If none satisfy, append at the nearest end and mark `origin = Manual` with a disconnected flag in the response body (the client renders the disconnected-marker style per FR-009)
3. Insertion is O(|path| × degree_of_N) — negligible at ≤200 path segments

**Rationale**: Reuses the network graph already constructed by the path calculation feature; no new graph-building code required. Respects FR-009 (no geometry guessing; netrelations required).

**Alternatives considered**:
- Spatial proximity snap (nearest geometry endpoint): Explicitly forbidden by spec (FR-009, Clarification 5)
- Full BFS/DFS through the graph to find shortest route: Overkill for ≤200 segments; the simpler scan is O(|path|) and sufficient

---

## Decision 9: Feature Flag Design

**Decision**: `webapp` Cargo feature in `tp-cli/Cargo.toml`, default-enabled; `tp-webapp` crate is only a dependency when that feature is active

```toml
# tp-cli/Cargo.toml
[features]
default = ["webapp"]
webapp = ["dep:tp-webapp"]

[dependencies]
tp-webapp = { path = "../tp-webapp", optional = true }
```

**Rationale**: Users who want a minimal CLI binary without the web server can compile with `--no-default-features`. This also keeps the dependency tree clean for tp-py (Python bindings) which does not need axum at all.

**Alternatives considered**:
- Always-on dependency: Would force axum/tokio into every consumer of tp-cli — unnecessary for automated pipelines
- Separate binary crate `tp-webapp-cli`: Would fragment the CLI; users would need to know about two binaries

---

## Decision 10: Testing Strategy

**Decision**: Three-tier test approach

| Tier | Location | Tool | Covers |
|------|----------|------|--------|
| Unit — endpoint handlers | `tp-webapp/tests/unit/routes_test.rs` | `axum::extract::testing::TestClient` (no network) | Each handler in isolation with a pre-built `WebAppState` |
| Unit — edit logic | `tp-webapp/tests/unit/edit_test.rs` | `#[test]` (sync) | `add_segment` snap insertion + `remove_segment` with various netrelation graphs |
| Integration | `tp-webapp/tests/integration/webapp_integration_test.rs` | `tokio::test` + `reqwest` | Full server on a random port; tests all 6 endpoints end-to-end; verifies confirm/abort lifecycle |

CLI integration tests (`tp-cli/tests/`) cover argument parsing and verify that `--review` invokes the library correctly (mock the `run_webapp_integrated` fn behind a feature).

**Property-based testing**: not required for this feature — the edit logic is deterministic graph traversal, not a probability formula. If snap insertion logic grows complex, `quickcheck` is already available in the workspace.

---

## Summary

All unknowns from Technical Context are resolved:

| Unknown | Resolution |
|---------|-----------|
| Web framework | axum ^0.8 + tokio ^1 |
| Static asset strategy | rust-embed ^8 (compile-time embedding) |
| Frontend stack | Leaflet.js 1.9 + vanilla HTML/CSS/JS |
| AppState concurrency | `Arc<RwLock<WebAppState>>` |
| Integrated-mode blocking | tokio oneshot channel |
| Port selection | Start at 8765, try 10 consecutively |
| PathOrigin model | New enum in tp-core, serde default backward-compatible |
| Snap insertion | netrelations graph scan, O(|path|) |
| Feature flag | `webapp` feature, default-enabled in tp-cli |
| Testing approach | Three-tier: unit handlers, unit edit, integration |
