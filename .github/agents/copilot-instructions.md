# tp-lib Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-09

## Active Technologies
- Rust 2021 edition, latest stable (1.80+) (003-path-review-webapp)
- File-based (GeoJSON network, CSV train path, CSV GNSS positions) — no database (003-path-review-webapp)
- Rust 1.91.1+ (workspace edition 2021) + `geo` 0.28, `rstar` 0.12, `geojson` 0.24, `csv` 1.x, `serde`/`serde_json`, `chrono` (DateTime<FixedOffset>), `petgraph`, `proj4rs` 0.1.9; webapp: `axum`, `tokio`, Leaflet (static) (004-train-detections)
- File-based I/O (CSV / GeoJSON); no DB. R-tree (`rstar`) in-memory spatial index reused for coordinate resolution. (004-train-detections)
- Rust 1.75 (tp-net crate) + C# 12 / .NET 8 (TpLib managed) + `csbindgen` (FFI stub generation), `serde_json` (FFI marshalling), `tp-lib-core` (core algorithms); C# side: `System.Text.Json` (deserialization), xUnit (testing) (005-dotnet-bindings)
- N/A — stateless function calls only (005-dotnet-bindings)

- Rust 1.75+ (edition 2021) (002-train-path-calculation)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test; cargo clippy

## Linting & Formatting Checks

After **any** change to a `.rs` file in this workspace, always run these checks in order and fix any reported issues before considering the task done:

1. `cargo fmt --check` — verify all Rust source is formatted with rustfmt.  
   Fix automatically with `cargo fmt` if there are diffs.
2. `cargo clippy --all-targets --all-features -- -D warnings` — zero-warning policy.
3. `cargo test --workspace` — full test suite must stay green.

For changes to `tp-py/` (Python bindings), also run:
- `cd tp-py && pytest python/tests/ -v` (requires the `.venv` to be active and the extension built with `maturin develop`).

For Python source files (`.py`) changed under `tp-py/`:
- The project follows PEP 8; run `ruff check tp-py/python` (if ruff is installed) or ensure no obvious style issues.

## Code Style

Rust 1.75+ (edition 2021): Follow standard conventions

## Recent Changes
- 005-dotnet-bindings: Added Rust 1.75 (tp-net crate) + C# 12 / .NET 8 (TpLib managed) + `csbindgen` (FFI stub generation), `serde_json` (FFI marshalling), `tp-lib-core` (core algorithms); C# side: `System.Text.Json` (deserialization), xUnit (testing)
- 004-train-detections: Added Rust 1.91.1+ (workspace edition 2021) + `geo` 0.28, `rstar` 0.12, `geojson` 0.24, `csv` 1.x, `serde`/`serde_json`, `chrono` (DateTime<FixedOffset>), `petgraph`, `proj4rs` 0.1.9; webapp: `axum`, `tokio`, Leaflet (static)
- 003-path-review-webapp: Added Rust 2021 edition, latest stable (1.80+)


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
