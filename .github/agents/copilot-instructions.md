# tp-lib Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-09

## Active Technologies
- Rust 2021 edition, latest stable (1.80+) (003-path-review-webapp)
- File-based (GeoJSON network, CSV train path, CSV GNSS positions) — no database (003-path-review-webapp)
- Rust 1.91.1+ (workspace edition 2021) + `geo` 0.28, `rstar` 0.12, `geojson` 0.24, `csv` 1.x, `serde`/`serde_json`, `chrono` (DateTime<FixedOffset>), `petgraph`, `proj4rs` 0.1.9; webapp: `axum`, `tokio`, Leaflet (static) (004-train-detections)
- File-based I/O (CSV / GeoJSON); no DB. R-tree (`rstar`) in-memory spatial index reused for coordinate resolution. (004-train-detections)

- Rust 1.75+ (edition 2021) (002-train-path-calculation)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test; cargo clippy

## Code Style

Rust 1.75+ (edition 2021): Follow standard conventions

## Recent Changes
- 004-train-detections: Added Rust 1.91.1+ (workspace edition 2021) + `geo` 0.28, `rstar` 0.12, `geojson` 0.24, `csv` 1.x, `serde`/`serde_json`, `chrono` (DateTime<FixedOffset>), `petgraph`, `proj4rs` 0.1.9; webapp: `axum`, `tokio`, Leaflet (static)
- 003-path-review-webapp: Added Rust 2021 edition, latest stable (1.80+)

- 002-train-path-calculation: Added Rust 1.75+ (edition 2021)

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
