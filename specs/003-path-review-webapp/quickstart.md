# Quickstart: Train Path Review Webapp

**Feature**: `003-path-review-webapp`  
**Crate**: `tp-webapp` (new) + `tp-cli webapp` subcommand + `--review` flag

---

## Prerequisites

- Rust toolchain (stable, 1.80+): `rustup update stable`
- A modern desktop browser (Chrome ≥90, Firefox ≥90, Edge ≥90)
- A network GeoJSON file with netelements **and** netrelations (same file format as feature 002)
- A train path CSV file (output of `tp-cli calculate-path` or `tp-cli default`)

No npm, no Node.js, no frontend build step required.

---

## Build

```sh
# Build the CLI with the webapp feature (default-on)
cargo build --package tp-cli

# Build in release mode (recommended for production use)
cargo build --release --package tp-cli
```

To build *without* the web server (minimal CLI binary):

```sh
cargo build --package tp-cli --no-default-features
```

---

## Standalone Mode — Review a Pre-Calculated Path

Use this mode when you have a path CSV and want to inspect or edit it before using it for GNSS projection.

```sh
# Minimal: opens browser to http://127.0.0.1:8765
tp-cli webapp \
  --network test-data/sample_network.geojson \
  --train-path path.csv

# With GNSS overlay and explicit output file
tp-cli webapp \
  --network test-data/sample_network.geojson \
  --train-path path.csv \
  --gnss test-data/sample_gnss.geojson \
  --output reviewed_path.csv
```

**What happens**:
1. CLI loads the network and path files
2. A local server starts at `http://127.0.0.1:8765` (or the next available port)
3. The browser opens automatically to the map
4. Edit the path in the browser (add/remove segments)
5. Click **Save** to write `reviewed_path.csv` — the server stays alive
6. Press Ctrl+C in the terminal when done

**The saved file** can be fed directly to the projection pipeline:
```sh
tp-cli --gnss positions.csv --network network.geojson --train-path reviewed_path.csv --output result.csv
```

---

## Integrated Mode — Review During GNSS Projection Pipeline

Use this mode to pause the pipeline after automatic path calculation and review before projection.

```sh
tp-cli \
  --gnss test-data/sample_gnss.geojson \
  --network test-data/sample_network.geojson \
  --output result.csv \
  --review
```

**What happens**:
1. CLI calculates the train path from GNSS + network
2. A local server starts and the browser opens to the review map
3. CLI prints a waiting message with the URL
4. Review and optionally edit the path in the browser
5. Click **Confirm** → projection runs with the confirmed path; CLI exits 0
   or Click **Abort** → CLI exits with code 1 and prints a cancellation message

To also save the confirmed path to a file:
```sh
tp-cli \
  --gnss positions.csv \
  --network network.geojson \
  --output result.csv \
  --review \
  --save-path confirmed_path.csv
```

---

## Map Interaction Reference

| Action | How |
|--------|-----|
| Add a segment to the path | Click a non-highlighted netelement on the map |
| Remove a segment from the path | Click a highlighted (path) segment on the map |
| Identify a segment | Hover over any segment to see its netelement ID |
| Pan the map | Click and drag |
| Zoom | Scroll wheel / pinch / zoom buttons |
| Toggle OSM background tiles | Use the layers control button (top-right) |
| Save (standalone) | Click the **Save** button in the sidebar |
| Confirm (integrated) | Click the **Confirm** button in the sidebar |
| Abort (integrated) | Click the **Abort** button in the sidebar |

**Disconnected segments**: If a manually-added segment cannot be inserted at an unambiguous position in the path (based on netrelations), it is appended at the nearest end of the path and shown with a disconnected marker (dashed outline). The sidebar also flags it.

**Confidence colour scale**: Path segments are coloured on a red → yellow → green scale based on their probability score (0.0 → 1.0). Manually-added segments always show green (probability 1.0) with a distinct border style.

---

## Running Tests

```sh
# All tests for the webapp crate (unit + integration)
cargo test --package tp-webapp

# Unit tests only (fast, no network)
cargo test --package tp-webapp --lib

# Integration tests only (spins up a live server)
cargo test --package tp-webapp --test webapp_integration_test

# CLI integration tests
cargo test --package tp-cli

# Full workspace tests
cargo test --workspace
```

---

## Development Workflow (Frontend Iteration)

rust-embed reads assets from the filesystem in `debug` builds (not embedded), which means you can edit `tp-webapp/static/app.js` or `style.css` and reload the browser without rebuilding Rust.

```sh
# Start the server in dev mode
cargo run --package tp-cli -- webapp \
  --network test-data/sample_network.geojson \
  --train-path path.csv

# Edit static/app.js or static/style.css
# Reload the browser — changes are picked up immediately (debug build)
```

For release builds, change to `static/` require a `cargo build --release`.

---

## Workspace Cargo.toml Changes

The following new workspace dependencies must be added to `Cargo.toml`:

```toml
[workspace.dependencies]
axum        = { version = "0.8", features = ["json"] }
tokio       = { version = "1",   features = ["full"] }
rust-embed  = { version = "8",   features = ["debug-embed"] }
open        = "5"
tokio-util  = { version = "0.7", features = ["rt"] }
reqwest     = { version = "0.12", features = ["json"], default-features = false }
```

Note: `reqwest` is a dev/test dependency (used in integration tests). It can be scoped to `[dev-dependencies]` in `tp-webapp/Cargo.toml` to avoid inflating the production binary.

---

## Troubleshooting

**Port already in use**: The CLI automatically tries ports 8765–8774. If all are occupied, it prints an error. Free a port or re-run; the actual bound URL is always printed.

**Browser does not open automatically**: The URL is always printed to stdout. Open it manually. Pass `--no-browser` to suppress the open attempt.

**Network file has no netrelations**: The `webapp` subcommand requires netrelations for snap insertion. If the network file lacks them, the server starts but newly-added segments will always be marked as disconnected. Use the same network file format as the path calculation pipeline.

**Confidence colours look wrong**: Check that the path CSV has valid probability values (0.0–1.0). Values outside this range are clamped at the client.

**Empty path on save**: The browser will prompt for confirmation before saving an empty path (all segments removed). This cannot be suppressed.
