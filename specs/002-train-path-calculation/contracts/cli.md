# CLI Contract: Path Calculation and Projection Commands

**Feature**: 002-train-path-calculation  
**Date**: January 9, 2026  
**Phase**: 1 - Design & Contracts

- [CLI Contract: Path Calculation and Projection Commands](#cli-contract-path-calculation-and-projection-commands)
  - [Overview](#overview)
  - [Command Architecture](#command-architecture)
  - [Command: `tp-cli` (Default - Path-Based Projection)](#command-tp-cli-default---path-based-projection)
    - [Synopsis](#synopsis)
    - [Description](#description)
    - [Required Arguments](#required-arguments)
    - [Optional Arguments](#optional-arguments)
    - [Behavior](#behavior)
    - [Examples](#examples)
  - [Command: `calculate-path`](#command-calculate-path)
    - [Synopsis](#synopsis-1)
    - [Description](#description-1)
    - [Required Arguments](#required-arguments-1)
    - [Optional Arguments](#optional-arguments-1)
    - [Output](#output)
    - [Examples](#examples-1)
  - [Command: `simple-projection`](#command-simple-projection)
    - [Synopsis](#synopsis-2)
    - [Description](#description-2)
    - [Required Arguments](#required-arguments-2)
    - [Optional Arguments](#optional-arguments-2)
    - [Output](#output-1)
    - [Examples](#examples-2)
  - [Input File Formats](#input-file-formats)
    - [GNSS Data (CSV)](#gnss-data-csv)
    - [GNSS Data (GeoJSON)](#gnss-data-geojson)
    - [Network Topology (GeoJSON)](#network-topology-geojson)
  - [Output Formats](#output-formats)
    - [Projected Coordinates (CSV)](#projected-coordinates-csv)
    - [Projected Coordinates (GeoJSON)](#projected-coordinates-geojson)
    - [Train Path (CSV)](#train-path-csv)
    - [Train Path (GeoJSON)](#train-path-geojson)
  - [Exit Codes](#exit-codes)
  - [Standard Streams](#standard-streams)
  - [Warnings and Diagnostics](#warnings-and-diagnostics)
    - [Standard Warnings (stderr)](#standard-warnings-stderr)
    - [Verbose Progress (stderr with `--verbose`)](#verbose-progress-stderr-with---verbose)
    - [Error Messages (stderr)](#error-messages-stderr)
  - [Environment Variables](#environment-variables)
    - [Example](#example)
  - [Stability Guarantees](#stability-guarantees)
  - [Backward Compatibility](#backward-compatibility)
    - [Feature 001 Commands (Unchanged)](#feature-001-commands-unchanged)
  - [Testing Contract](#testing-contract)


## Overview

This document defines the command-line interface contract for train path calculation and projection in `tp-cli`. The CLI provides three distinct workflows:

1. **Default command** (`tp-cli`): Calculate path and project coordinates in one step (or use existing path)
2. **Path-only command** (`calculate-path`): Calculate path only, no projection
3. **Legacy command** (`simple-projection`): Project to nearest netelement (feature 001 behavior)

All commands, options, and behaviors are guaranteed stable across minor version releases.

---

## Command Architecture

```
tp-cli                    # Calculate path + project (or use existing path)
├── --train-path <FILE>   # Optional: skip calculation, use existing path
├── --gnss <FILE>
├── --network <FILE>
└── --output <FILE>       # Outputs projected coordinates

tp-cli calculate-path     # Path calculation only
├── --gnss <FILE>
├── --network <FILE>
└── --output <FILE>       # Outputs train path

tp-cli simple-projection  # Legacy: nearest netelement projection
├── --gnss <FILE>
├── --network <FILE>
└── --output <FILE>       # Outputs projected coordinates
```

---

## Command: `tp-cli` (Default - Path-Based Projection)

Calculate train path through the network and project all GNSS coordinates onto that path. Optionally, use a pre-calculated path instead of calculating a new one.

### Synopsis

```bash
# Calculate path automatically and project
tp-cli [OPTIONS] --gnss <FILE> --network <FILE> --output <FILE>

# Use existing path and project
tp-cli [OPTIONS] --gnss <FILE> --network <FILE> --train-path <FILE> --output <FILE>
```

### Description

This is the **primary workflow** for path-based GNSS projection. It combines path calculation and coordinate projection in a single command. The output is always projected coordinates (not the path itself).

**Two modes**:
1. **Automatic path calculation** (no `--train-path`): Calculates optimal path, then projects coordinates
2. **Pre-calculated path** (with `--train-path`): Loads existing path, skips calculation, projects coordinates

### Required Arguments

| Option | Value | Description |
|--------|-------|-------------|
| `--gnss <FILE>` | Path | GNSS coordinate data (CSV or GeoJSON) |
| `--network <FILE>` | Path | Network topology file (GeoJSON with netelements and netrelations) |
| `--output <FILE>` | Path | Output file for projected coordinates (format determined by extension) |

### Optional Arguments

**Path Input:**

| Option | Description |
|--------|-------------|
| `--train-path <FILE>` | Use pre-calculated train path (CSV or GeoJSON) instead of calculating new path |

**Algorithm Parameters** (ignored when `--train-path` is provided):

| Option | Default | Range | Description |
|--------|---------|-------|-------------|
| `--distance-scale <VALUE>` | 10.0 | > 0.0 | Distance exponential decay scale parameter (meters) |
| `--heading-scale <VALUE>` | 2.0 | > 0.0 | Heading exponential decay scale parameter (degrees) |
| `--cutoff-distance <VALUE>` | 50.0 | > 0.0 | Maximum distance for candidate selection (meters) |
| `--heading-cutoff <VALUE>` | 5.0 | 0.0-180.0 | Maximum heading difference before rejection (degrees) |
| `--probability-threshold <VALUE>` | 0.25 | 0.0-1.0 | Minimum probability for path segment inclusion |
| `--max-candidates <N>` | 3 | ≥ 1 | Maximum candidate netelements per GNSS position |

**Performance Optimization:**

| Option | Default | Description |
|--------|---------|-------------|
| `--resampling-distance <VALUE>` | None | Resample GNSS data at specified interval (meters) for path calculation only. All original positions still projected in output. |

**Output Control:**

| Option | Default | Description |
|--------|---------|-------------|
| `--format <FORMAT>` | auto | Output format: `csv`, `geojson`, or `auto` (detect from extension) |
| `--save-path <FILE>` | None | Optionally save calculated path to file (in addition to projected coordinates) |

**General Options:**

| Option | Description |
|--------|-------------|
| `--help` | Display help information |
| `--version` | Display version information |
| `-v, --verbose` | Enable verbose logging output |
| `--quiet` | Suppress all non-error output |

### Behavior

**Without `--train-path`** (automatic mode):
1. Load GNSS data and network topology
2. Calculate optimal train path through network
3. Project all GNSS coordinates onto calculated path
4. Output projected coordinates
5. Optionally save path if `--save-path` specified

**With `--train-path`** (pre-calculated mode):
1. Load GNSS data, network topology, and pre-calculated path
2. Skip path calculation
3. Project all GNSS coordinates onto provided path
4. Output projected coordinates

### Examples

**Complete workflow (calculate + project):**

```bash
# Calculate path and project coordinates in one step
tp-cli \
  --gnss train_gnss.csv \
  --network rail_network.geojson \
  --output projected.csv
```

**Two-step workflow with path editing:**

```bash
# Step 1: Calculate path only (using calculate-path command)
tp-cli calculate-path \
  --gnss train_gnss.csv \
  --network rail_network.geojson \
  --output path.json

# Step 2: Manually edit path.json if needed

# Step 3: Project coordinates using edited path
tp-cli \
  --gnss train_gnss.csv \
  --network rail_network.geojson \
  --train-path path.json \
  --output projected.csv
```

**Using pre-calculated path:**

```bash
# Use existing path, save calculation time
tp-cli \
  --gnss train_gnss.csv \
  --network rail_network.geojson \
  --train-path existing_path.geojson \
  --output projected.csv
```

**Save path for later reuse:**

```bash
# Calculate and save path alongside projected coordinates
tp-cli \
  --gnss train_gnss.csv \
  --network rail_network.geojson \
  --output projected.csv \
  --save-path path.geojson
```

---

## Command: `calculate-path`

Calculate train path through the network **without** projecting coordinates. Output is the path only.

### Synopsis

```bash
tp-cli calculate-path [OPTIONS] --gnss <FILE> --network <FILE> --output <FILE>
```

### Description

Analyzes GNSS coordinate data and rail network topology to determine the most probable continuous path the train took through the network. **Outputs only the calculated path** (no projected coordinates).

Use this command when you want to:
- Inspect the path before projection
- Edit the path manually
- Store paths for later reuse
- Debug path calculation behavior

### Required Arguments

| Option | Value | Description |
|--------|-------|-------------|
| `--gnss <FILE>` | Path | GNSS coordinate data (CSV or GeoJSON) |
| `--network <FILE>` | Path | Network topology file (GeoJSON with netelements and netrelations) |
| `--output <FILE>` | Path | Output file path for train path (format determined by extension) |

### Optional Arguments

**Algorithm Parameters:**

| Option | Default | Range | Description |
|--------|---------|-------|-------------|
| `--distance-scale <VALUE>` | 10.0 | > 0.0 | Distance exponential decay scale parameter (meters) |
| `--heading-scale <VALUE>` | 2.0 | > 0.0 | Heading exponential decay scale parameter (degrees) |
| `--cutoff-distance <VALUE>` | 50.0 | > 0.0 | Maximum distance for candidate selection (meters) |
| `--heading-cutoff <VALUE>` | 5.0 | 0.0-180.0 | Maximum heading difference before rejection (degrees) |
| `--probability-threshold <VALUE>` | 0.25 | 0.0-1.0 | Minimum probability for path segment inclusion |
| `--max-candidates <N>` | 3 | ≥ 1 | Maximum candidate netelements per GNSS position |

**Performance Optimization:**

| Option | Default | Description |
|--------|---------|-------------|
| `--resampling-distance <VALUE>` | None | Resample GNSS data at specified interval (meters) for path calculation only |

**Output Control:**

| Option | Default | Description |
|--------|---------|-------------|
| `--format <FORMAT>` | auto | Output format: `csv`, `geojson`, or `auto` (detect from extension) |

**General Options:**

| Option | Description |
|--------|-------------|
| `--help` | Display help information |
| `--version` | Display version information |
| `-v, --verbose` | Enable verbose logging output |
| `--quiet` | Suppress all non-error output |

### Output

Train path only (CSV or GeoJSON format). See [Output Formats](#output-formats) section for details.

### Examples

**Basic path calculation:**

```bash
# Calculate path only, output as GeoJSON
tp-cli calculate-path \
  --gnss train_gnss.csv \
  --network rail_network.geojson \
  --output path.geojson
```

**Custom parameters:**

```bash
# Tune parameters for urban rail network
tp-cli calculate-path \
  --gnss train_gnss.csv \
  --network rail_network.geojson \
  --output path.csv \
  --distance-scale 15.0 \
  --heading-cutoff 10.0 \
  --probability-threshold 0.20 \
  --verbose
```

---

## Command: `simple-projection`

Project GNSS coordinates to the nearest netelement **without** using path calculation or topology constraints. This is the **legacy behavior** from feature 001.

### Synopsis

```bash
tp-cli simple-projection [OPTIONS] --gnss <FILE> --network <FILE> --output <FILE>
```

### Description

Projects each GNSS position independently to the nearest netelement in the network. No path calculation is performed, and topology (netrelations) is ignored. Each position is projected to its closest track segment regardless of connectivity.

Use this command for:
- Backwards compatibility with feature 001
- Quick projection without path analysis
- Situations where path continuity is not required
- Debugging or comparison with path-based projection

### Required Arguments

| Option | Value | Description |
|--------|-------|-------------|
| `--gnss <FILE>` | Path | GNSS coordinate data (CSV or GeoJSON) |
| `--network <FILE>` | Path | Network topology file (GeoJSON, only netelements used) |
| `--output <FILE>` | Path | Output file for projected coordinates (format determined by extension) |

### Optional Arguments

| Option | Default | Description |
|--------|---------|-------------|
| `--format <FORMAT>` | auto | Output format: `csv`, `geojson`, or `auto` (detect from extension) |
| `--help` | - | Display help information |
| `--version` | - | Display version information |
| `-v, --verbose` | - | Enable verbose logging output |
| `--quiet` | - | Suppress all non-error output |

**Note**: Algorithm parameters (distance-scale, heading-scale, etc.) are **not applicable** to simple projection.

### Output

Projected coordinates only (CSV or GeoJSON format). Each position is projected independently.

### Examples

**Basic simple projection:**

```bash
# Legacy independent projection (feature 001 behavior)
tp-cli simple-projection \
  --gnss train_gnss.csv \
  --network rail_network.geojson \
  --output projected.csv
```

---

## Input File Formats

### GNSS Data (CSV)

```csv
timestamp,latitude,longitude,crs,heading,distance
2026-01-09T10:00:00+01:00,50.8503,4.3517,EPSG:4326,45.3,
2026-01-09T10:00:01+01:00,50.8504,4.3518,EPSG:4326,47.1,12.5
```

**Required columns:** `timestamp`, `latitude`, `longitude`, `crs`  
**Optional columns:** `heading` (degrees 0-360), `distance` (meters)

### GNSS Data (GeoJSON)

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {
        "type": "Point",
        "coordinates": [4.3517, 50.8503]
      },
      "properties": {
        "timestamp": "2026-01-09T10:00:00+01:00",
        "crs": "EPSG:4326",
        "heading": 45.3,
        "distance": 12.5
      }
    }
  ]
}
```

### Network Topology (GeoJSON)

Single feature collection containing both netelements and netrelations, distinguished by `type` property:

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {
        "type": "netelement",
        "id": "NE_A"
      },
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.35, 50.85], [4.36, 50.86]]
      }
    },
    {
      "type": "Feature",
      "properties": {
        "type": "netrelation",
        "id": "NR001",
        "netelementA": "NE_A",
        "netelementB": "NE_B",
        "positionOnA": 1,
        "positionOnB": 0,
        "navigability": "both"
      },
      "geometry": {
        "type": "Point",
        "coordinates": [4.355, 50.855]
      }
    }
  ]
}
```

**Navigability values**: `"both"` (bidirectional), `"AB"` (A→B only), `"BA"` (B→A only), `"none"` (not navigable)

**Geometry**: NetRelation geometry can be `null` or a `Point` representing the connection point between netelements (useful for GIS visualization)

---

## Output Formats

### Projected Coordinates (CSV)

Output from `tp-cli` (default command) or `tp-cli simple-projection`:

```csv
timestamp,latitude,longitude,crs,netelement_id,projected_lat,projected_lon,distance_meters,intrinsic_coordinate
2026-01-09T10:00:00+01:00,50.8503,4.3517,EPSG:4326,NE_A,50.8503,4.3517,2.3,0.45
2026-01-09T10:00:01+01:00,50.8504,4.3518,EPSG:4326,NE_A,50.8504,4.3518,1.8,0.47
```

### Projected Coordinates (GeoJSON)

Output from `tp-cli` (default command) or `tp-cli simple-projection`:

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {
        "type": "Point",
        "coordinates": [4.3517, 50.8503]
      },
      "properties": {
        "timestamp": "2026-01-09T10:00:00+01:00",
        "crs": "EPSG:4326",
        "netelement_id": "NE_A",
        "original_lat": 50.8503,
        "original_lon": 4.3517,
        "distance_meters": 2.3,
        "intrinsic_coordinate": 0.45
      }
    }
  ]
}
```

### Train Path (CSV)

Output from `tp-cli calculate-path`:

```csv
sequence,netelement_id,probability,start_intrinsic,end_intrinsic,gnss_start_index,gnss_end_index
1,NE_A,0.87,0.25,0.78,0,10
2,NE_B,0.92,0.0,0.65,11,18
```

### Train Path (GeoJSON)

Output from `tp-cli calculate-path`:

```json
{
  "type": "FeatureCollection",
  "properties": {
    "overall_probability": 0.89,
    "calculated_at": "2026-01-09T10:15:30Z",
    "metadata": {
      "distance_scale": 10.0,
      "heading_scale": 2.0,
      "cutoff_distance": 50.0,
      "heading_cutoff": 5.0,
      "probability_threshold": 0.25,
      "resampling_distance": null,
      "fallback_mode": false,
      "candidate_paths_evaluated": 3,
      "bidirectional_path": true
    }
  },
  "features": [
    {
      "type": "Feature",
      "properties": {
        "type": "associated_netelement",
        "sequence": 1,
        "netelement_id": "NE_A",
        "probability": 0.87,
        "start_intrinsic": 0.25,
        "end_intrinsic": 0.78,
        "gnss_start_index": 0,
        "gnss_end_index": 10
      },
      "geometry": null
    }
  ]
}
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (includes fallback mode for path calculation) |
| 1 | Invalid arguments or options |
| 2 | Input file not found or unreadable |
| 3 | Invalid input data format |
| 4 | Output file write error |
| 5 | Empty network or GNSS data |
| 6 | CRS transformation error |
| 127 | Unexpected internal error |

---

## Standard Streams

**stdout:**
- Contains output data (CSV or GeoJSON as specified)
- Machine-readable format only
- Silent when `--quiet` specified

**stderr:**
- Progress messages (when `--verbose`)
- Warning messages (e.g., "Fallback mode used: no navigable path found")
- Error messages with diagnostic context
- Always written unless `--quiet`

**stdin:**
- Not used (all input from files)
- Reserved for future streaming support

---

## Warnings and Diagnostics

### Standard Warnings (stderr)

```
Warning: No navigable path found, falling back to independent projection
Warning: 15 GNSS positions beyond cutoff distance (50.0m), excluded from output
Warning: NetRelation NR042 references unknown netelement NE_999, skipped
Warning: Path calculation used fallback mode due to topology gap
```

### Verbose Progress (stderr with `--verbose`)

**Default command (`tp-cli`)**:
```
Loading GNSS data from train_gnss.csv... 1250 positions
Loading network from rail_network.geojson... 487 netelements, 1024 netrelations
Building spatial index... done (125ms)
Building topology graph... done (42ms)
Phase 1: Candidate selection... done (78ms)
Phase 2-3: Probability calculation... done (134ms)
Phase 4: Path construction... done (108ms)
Phase 5: Path selection... done (8ms)
Selected path with 12 segments, probability 0.87
Projecting 1250 GNSS positions onto path... done (215ms)
Writing output to projected.csv... done
Total time: 710ms
```

**Calculate-path command**:
```
Loading GNSS data from train_gnss.csv... 1250 positions
Loading network from rail_network.geojson... 487 netelements, 1024 netrelations
Building spatial index... done (125ms)
Building topology graph... done (42ms)
Phase 1: Candidate selection... done (78ms)
Phase 2-3: Probability calculation... done (134ms)
Phase 4: Path construction... done (108ms)
Phase 5: Path selection... done (8ms)
Selected path with 12 segments, probability 0.87
Writing path to path.geojson... done
Total time: 495ms
```

**Simple-projection command**:
```
Loading GNSS data from train_gnss.csv... 1250 positions
Loading network from rail_network.geojson... 487 netelements
Building spatial index... done (125ms)
Projecting 1250 positions independently... done (198ms)
Writing output to projected.csv... done
Total time: 323ms
```

### Error Messages (stderr)

```
Error: Input file not found: train_gnss.csv
Error: Invalid GeoJSON format in rail_network.geojson: missing 'features' field
Error: Empty network data: no netelements found
Error: Empty network data: no netrelations found
Error: Invalid heading value 450.0 in GNSS data (must be 0-360)
Error: Output file is not writable: /readonly/result.csv
Error: Cannot use --train-path with calculate-path command
Error: Cannot use algorithm parameters with simple-projection command
```

---

## Environment Variables

| Variable | Effect |
|----------|--------|
| `TP_LOG` | Set log level: `error`, `warn`, `info`, `debug`, `trace` |
| `TP_LOG_STYLE` | Log output style: `auto`, `always`, `never` |

### Example

```bash
# Enable debug logging
TP_LOG=debug tp-cli calculate-path --gnss train.csv --network net.geojson --output path.json
```

---

## Stability Guarantees

| CLI Element | Stability | Notes |
|-------------|-----------|-------|
| Command names | **Stable** | `tp-cli`, `calculate-path`, `simple-projection` will not change |
| Required arguments | **Stable** | `--gnss`, `--network`, `--output` signatures fixed |
| Optional argument names | **Stable** | Flag names will not change in minor versions |
| Default parameter values | **Subject to tuning** | May change in minor versions based on research |
| Output CSV schema | **Versioned** | New columns may be added; existing columns stable |
| Output GeoJSON schema | **Versioned** | Version field ensures forward/backward compatibility |
| Exit codes | **Stable** | Meanings will not change |
| Error message format | **Best effort** | Error text may improve; parsers should use exit codes |

---

## Backward Compatibility

### Feature 001 Commands (Unchanged)

```bash
# Feature 001 behavior is now available via simple-projection command
tp-cli simple-projection --gnss train.csv --network network.geojson --output result.csv
```

**Migration path**: The original `project` command behavior is preserved in `simple-projection`. Users of feature 001 can either:
- Use `simple-projection` for identical behavior
- Migrate to `tp-cli` (default command) for path-based projection

**No breaking changes** to data formats or APIs. All feature 002 functionality is additive.

---

## Testing Contract

All CLI behaviors have corresponding tests:

- **Integration tests**: Real command execution with file I/O
- **Option parsing tests**: Verify all flags and arguments
- **Error handling tests**: Verify exit codes and error messages
- **Command interaction tests**: Verify `--train-path` with default command
- **Backward compatibility tests**: Ensure `simple-projection` matches feature 001 `project` behavior

---

**CLI Contract Version**: 2.0  
**Feature Version**: 002-train-path-calculation  
**Last Updated**: January 9, 2026
