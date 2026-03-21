# CLI Contract: Train Path Review Webapp

**Crate**: `tp-cli`  
**Feature**: `003-path-review-webapp`  
**Cargo feature gate**: `webapp` (default-enabled in `tp-cli/Cargo.toml`)

---

## New: `webapp` Subcommand (Standalone Mode)

### Synopsis

```
tp-cli webapp --network <FILE> --train-path <FILE> [OPTIONS]
```

### Description

Launches a local web server and opens the browser to a map where the provided train path can be visually reviewed and edited. The server stays alive until the user terminates the process (Ctrl+C). Edited paths can be saved repeatedly. The saved CSV is compatible with the existing `--train-path` flag.

### Required Arguments

| Flag | Short | Value | Description |
|------|-------|-------|-------------|
| `--network` | `-n` | `<FILE>` | Path to railway network GeoJSON file (must include netelements + netrelations) |
| `--train-path` | | `<FILE>` | Path to pre-calculated train path CSV file |

### Optional Arguments

| Flag | Short | Value | Default | Description |
|------|-------|-------|---------|-------------|
| `--gnss` | `-g` | `<FILE>` | (none) | Path to GNSS positions file (CSV or GeoJSON); rendered as markers for visual reference only |
| `--output` | `-o` | `<FILE>` | `<train-path-stem>_reviewed.csv` | Output file path for saved path |
| `--port` | | `<PORT>` | `8765` | Starting port to try; increments on conflict up to `<PORT>+9` |
| `--no-browser` | | | (false) | Suppress automatic browser opening; print URL only |

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Process terminated normally (Ctrl+C) |
| `1` | Invalid arguments or file I/O error |

### Standard Streams

- **stdout**: URL of the local server (e.g. `http://127.0.0.1:8765`) plus startup message
- **stderr**: Error messages and diagnostics

### Examples

```sh
# Minimal: review a calculated path
tp-cli webapp --network network.geojson --train-path path.csv

# With GNSS overlay and explicit output file
tp-cli webapp \
  --network network.geojson \
  --train-path path.csv \
  --gnss positions.csv \
  --output reviewed_path.csv

# Suppress browser opening (headless / CI environment)
tp-cli webapp --network network.geojson --train-path path.csv --no-browser
```

### Terminal Output

```
Train Path Review Webapp
  Network    : network.geojson (1234 netelements)
  Train path : path.csv (42 segments)
  GNSS       : (none)
  Output     : path_reviewed.csv
  
  Server started: http://127.0.0.1:8765
  Press Ctrl+C to stop.
```

If browser opening fails:
```
  Browser could not be opened automatically.
  Open manually: http://127.0.0.1:8765
```

---

## Modified: Default Command — `--review` Flag (Integrated Mode)

### Synopsis

```
tp-cli --gnss <FILE> --network <FILE> --output <FILE> --review [OPTIONS]
```

### Description

Runs the full GNSS projection pipeline but pauses after path calculation to open the review webapp. The pipeline does not continue until the user clicks **Confirm** (or **Abort**) in the browser. The projection uses the path as it exists in the webapp at the moment of confirmation.

### New Flag

| Flag | Short | Value | Default | Description |
|------|-------|-------|---------|-------------|
| `--review` | | | (false) | Pause after path calculation and launch the review webapp before projecting GNSS positions |

### Path Artifact (auto-saved on Confirm)

When the user clicks **Confirm**, the reviewed path is automatically saved to a file derived from `--output` **before** projection proceeds. The filename is formed by inserting `-path` before the file extension:

| `--output` value | Path artifact file |
|------------------|--------------------|
| `result.csv` | `result-path.csv` |
| `result.geojson` | `result-path.geojson` |
| `result` (no extension) | `result-path` |

The CLI prints `Path saved to: <file>` to stderr after writing the artifact. No explicit flag is required — the artifact is always produced when `--review` is used and the user confirms. The `--output` flag must be provided (this is already required for projection output).

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Pipeline completed successfully (including projection after review) |
| `1` | User clicked Abort in the review webapp |
| `1` | Invalid arguments or file I/O error |

### Standard Streams

- **stdout**: Same as default pipeline (final output file path)
- **stderr**: Error messages; "Path review aborted by user" on abort exit

### Terminal Output During Review

After path calculation completes:
```
Path calculation complete (42 segments, overall confidence: 0.89)
Launching review webapp...

  Waiting for user confirmation at: http://127.0.0.1:8765
  Confirm in the browser when ready, or press Ctrl+C to abort.
```

After user clicks Confirm:
```
Path review confirmed. Proceeding with GNSS projection...
```

After user clicks Abort (stderr):
```
Path review aborted by user.
```

### Example

```sh
tp-cli \
  --gnss positions.csv \
  --network network.geojson \
  --output result.csv \
  --review
```

This will: calculate the path, open the review webapp, wait for the user to confirm, then save `result-path.csv` (the reviewed path) and proceed with GNSS projection to produce `result.csv`.

---

## Cargo Feature Flag

The `webapp` feature controls whether `tp-webapp` is compiled in. It is default-enabled.

```toml
# tp-cli/Cargo.toml

[features]
default = ["webapp"]
webapp = ["dep:tp-webapp"]
```

To build a minimal CLI without the web server:

```sh
cargo build --package tp-cli --no-default-features
```

When the `webapp` feature is disabled, the `webapp` subcommand and `--review` flag are not compiled in. Attempting to use them in that build is a compile-time error (conditional compilation with `#[cfg(feature = "webapp")]`).

---

## `--help` Output

### `tp-cli webapp --help`

```
Launch the train path review webapp in standalone mode

Usage: tp-cli webapp [OPTIONS] --network <FILE> --train-path <FILE>

Options:
  -n, --network <FILE>      Railway network GeoJSON file (netelements + netrelations required)
      --train-path <FILE>   Pre-calculated train path CSV file
  -g, --gnss <FILE>         GNSS positions file for overlay display (optional)
  -o, --output <FILE>       Output file path for saved path [default: <train-path-stem>_reviewed.csv]
      --port <PORT>         Starting port [default: 8765]
      --no-browser          Do not open browser automatically; only print URL
  -h, --help                Print help
  -V, --version             Print version
```

### `tp-cli --help` (modified excerpt showing `--review`)

```
  ...
      --train-path <FILE>   Pre-calculated train path CSV (skip path calculation)
      --review              Pause after path calculation to review the path in the browser
  ...
```
