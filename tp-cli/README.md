# tp-cli: Train Positioning CLI

> Command-line interface for calculating train paths and projecting GNSS positions onto railway track netelements

## Installation

### From Source

```bash
# Build release binary
cargo build --release --package tp-lib-cli --no-default-features

# Binary located at: target/release/tp-cli.exe (Windows) or target/release/tp-cli (Unix)
```

### Add to PATH

```bash
# Windows PowerShell
$env:PATH += ";$(pwd)\target\release"

# Unix/Linux/macOS
export PATH="$PATH:$(pwd)/target/release"
```

## Quick Start

### Default Mode (Path Calculation + Projection)

```bash
# Calculate train path and project GNSS coordinates onto track network
tp-cli --gnss positions.csv \
       --crs EPSG:4326 \
       --network network.geojson \
       --output projected.geojson
```

### Path Calculation Only

```bash
# Calculate the most likely train path without projecting coordinates
tp-cli calculate-path \
       --gnss positions.csv \
       --crs EPSG:4326 \
       --network network.geojson \
       --output path.csv
```

### With Pre-Calculated Path

```bash
# Use an existing train path file to skip path calculation
tp-cli --gnss positions.csv \
       --crs EPSG:4326 \
       --network network.geojson \
       --train-path path.csv \
       --output projected.geojson
```

### With Debug Output

```bash
# Enable debug mode to write intermediate GeoJSON files
tp-cli --gnss positions.csv \
       --crs EPSG:4326 \
       --network network.geojson \
       --output projected.geojson \
       --debug
```

## Commands

`tp-cli` has three modes of operation:

### Default (no subcommand)

Calculates the train path and projects GNSS coordinates onto the path. This is the recommended mode.

```bash
tp-cli --gnss <FILE> --network <FILE> --output <FILE> [OPTIONS]
```

### `calculate-path`

Calculates the most likely sequence of netelements (train path) without performing coordinate projection.

```bash
tp-cli calculate-path --gnss <FILE> --network <FILE> --output <FILE> [OPTIONS]
```

### `simple-projection`

Legacy nearest-netelement projection (feature 001 behavior). Projects each GNSS position to its nearest netelement independently, without path awareness.

```bash
tp-cli simple-projection --gnss <FILE> --network <FILE> --output <FILE> [OPTIONS]
```

## Arguments

### Required Arguments

#### `--gnss <FILE>` (or `-g`)

Path to GNSS input file (CSV or GeoJSON format).

**CSV Example:**

```csv
latitude,longitude,timestamp,altitude,hdop
50.8503,4.3517,2025-12-09T14:30:00+01:00,100.0,2.0
50.8504,4.3518,2025-12-09T14:30:01+01:00,100.5,2.1
```

**Requirements:**

- CSV must have latitude, longitude, and timestamp columns
- Timestamps must be RFC3339 format with timezone (e.g., `2025-12-09T14:30:00+01:00`)
- Additional columns preserved as metadata

#### `--network <FILE>` (or `-n`)

Path to railway network GeoJSON file.

**Example:**

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {
        "id": "NE001"
      },
      "geometry": {
        "type": "LineString",
        "coordinates": [
          [4.35, 50.85],
          [4.36, 50.86]
        ]
      }
    }
  ]
}
```

**Requirements:**

- LineString geometries (track centerlines)
- `id` property (unique identifier per netelement)

#### `--output <FILE>` (or `-o`)

Output file path. The format is determined by the file extension (`.csv` or `.geojson`/`.json`) or the `--format` flag.

### Optional Arguments

#### `--crs <CRS>`

Coordinate Reference System of GNSS data (e.g., `EPSG:4326`).

**Rules:**

- **Required** for CSV input
- **Not allowed** for GeoJSON input (CRS is `EPSG:4326` by default with GeoJSON)

**Common CRS codes:**

- `EPSG:4326` - WGS84 (standard GPS coordinates)
- `EPSG:31370` - Belgian Lambert 2008
- `EPSG:3857` - Web Mercator

#### `--format <FORMAT>`

Output format: `csv`, `geojson`, or `auto` (detect from file extension). Default: `auto`.

#### `--train-path <FILE>` (default mode only)

Pre-calculated train path file. When provided, path calculation is skipped and this path is used directly for projection.

#### `--save-path <FILE>` (default mode only)

Save the calculated train path to this file in addition to the projected output. Useful for inspecting or reusing the path.

#### `--warning-threshold <METERS>` (or `-w`)

Distance threshold in meters for emitting projection warnings. Default: `50.0`.

Warnings are printed to stderr when a GNSS position projects more than this distance from the track.

#### `--lat-col <COLUMN>`

Latitude column name in CSV input. Default: `latitude`.

#### `--lon-col <COLUMN>`

Longitude column name in CSV input. Default: `longitude`.

#### `--time-col <COLUMN>`

Timestamp column name in CSV input. Default: `timestamp`.

### Algorithm Parameters

These parameters control the path calculation algorithm. The defaults work well for typical railway scenarios.

| Parameter | Default | Description |
|-----------|---------|-------------|
| `--distance-scale` | `10.0` | Distance exponential decay scale (meters) |
| `--heading-scale` | `2.0` | Heading exponential decay scale (degrees) |
| `--cutoff-distance` | `50.0` | Maximum distance for candidate selection (meters) |
| `--heading-cutoff` | `10.0` | Maximum heading difference before rejection (degrees) |
| `--probability-threshold` | `0.02` | Minimum probability for path segment inclusion |
| `--max-candidates` | `3` | Maximum candidate netelements per GNSS position |
| `--resampling-distance` | _(none)_ | Resample GNSS data at this interval (meters) |

### Debug Options

#### `--debug`

Enable debug mode. Writes intermediate GeoJSON files for each phase of the path calculation process. The files are written to the output file's parent directory, or to a custom directory specified with `--debug-output-dir`.

**Debug files produced:**

| File | Description |
|------|-------------|
| `phase2_candidates.geojson` | GNSS points with candidate projections and per-position probabilities |
| `phase3_netelements.geojson` | Netelement-level aggregated probabilities |
| `phase4_netelement_map.geojson` | Final netelement map used for path construction |
| `candidates.json` | Raw candidate data (JSON) |
| `positions.json` | Position-level debug data (JSON) |
| `decisions.json` | Path construction decisions (JSON) |

#### `--debug-output-dir <DIR>`

Directory for debug output files. Only used when `--debug` is also specified; a warning is emitted if `--debug-output-dir` is given without `--debug`.

When `--debug` is given without `--debug-output-dir`, files are written to the output file's parent directory.

```bash
# Debug files go to the output file's directory
tp-cli --gnss data.csv --crs EPSG:4326 --network network.geojson \
       --output results/projected.geojson --debug

# Debug files go to a custom directory
tp-cli --gnss data.csv --crs EPSG:4326 --network network.geojson \
       --output results/projected.geojson --debug --debug-output-dir ./debug-out
```

### General Options

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Enable verbose logging output |
| `--quiet` | Suppress all non-error output |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

## Exit Codes

- **0**: Success - All positions processed successfully
- **1**: Validation error - Invalid arguments or configuration
- **2**: Processing error - Projection or computation failure
- **3**: I/O error - File not found, permission denied, or read/write failure

## Output

**CSV Output Example:**

```csv
original_lat,original_lon,original_time,projected_lat,projected_lon,netelement_id,measure_meters,projection_distance_meters,crs
50.8503,4.3517,2025-12-09T14:30:00+01:00,50.85074,4.35148,NE001,132.54,51.31,EPSG:4326
```

**GeoJSON Output Example:**

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {
        "type": "Point",
        "coordinates": [4.35148, 50.85074]
      },
      "properties": {
        "netelement_id": "NE001",
        "measure_meters": 132.54,
        "projection_distance_meters": 51.31,
        "original_lat": 50.8503,
        "original_lon": 4.3517,
        "original_time": "2025-12-09T14:30:00+01:00",
        "crs": "EPSG:4326"
      }
    }
  ]
}
```

## Examples

### Example 1: Full Pipeline with Path Saving

```bash
# Calculate path, save it, and project coordinates
tp-cli --gnss train_journey.csv \
       --crs EPSG:4326 \
       --network infrabel_network.geojson \
       --output projected.geojson \
       --save-path calculated_path.csv
```

### Example 2: Path Calculation Only

```bash
# Calculate the train path without projection
tp-cli calculate-path \
       --gnss train_journey.csv \
       --crs EPSG:4326 \
       --network infrabel_network.geojson \
       --output path.geojson
```

### Example 3: Reuse Existing Path

```bash
# Project using a previously calculated path
tp-cli --gnss train_journey.csv \
       --crs EPSG:4326 \
       --network infrabel_network.geojson \
       --train-path calculated_path.csv \
       --output projected.geojson
```

### Example 4: Debug Mode with Custom Directory

```bash
# Write debug GeoJSON files for inspection in QGIS
tp-cli --gnss train_journey.csv \
       --crs EPSG:4326 \
       --network infrabel_network.geojson \
       --output projected.geojson \
       --debug --debug-output-dir ./debug
```

### Example 5: Custom Algorithm Parameters

```bash
# Adjust for a dense urban network with tight track spacing
tp-cli --gnss data.csv \
       --crs EPSG:4326 \
       --network network.geojson \
       --output projected.csv \
       --cutoff-distance 30.0 \
       --max-candidates 5 \
       --distance-scale 5.0
```

### Example 6: Custom CSV Column Names

```bash
tp-cli --gnss gps_data.csv \
       --crs EPSG:4326 \
       --network tracks.geojson \
       --output projected.csv \
       --lat-col lat --lon-col lon --time-col ts
```

## Troubleshooting

### Error: "CRS is required for CSV GNSS input"

**Problem:** Forgot to specify `--crs` for CSV input.

**Solution:**

```bash
tp-cli --gnss data.csv --crs EPSG:4326 --network network.geojson --output out.csv
```

### Error: "CRS should not be specified for GeoJSON GNSS input"

**Problem:** Provided `--crs` with GeoJSON input (CRS read from file).

**Solution:**

```bash
# Remove --crs flag
tp-cli --gnss data.geojson --network network.geojson --output out.csv
```

### Warning: "Large projection distance (X.XXm > threshold)"

**Meaning:** GNSS position is far from nearest track.

**Possible causes:**

1. **GPS Inaccuracy**: Poor satellite signal or multipath interference
2. **Parallel Track**: Train on adjacent track not in network
3. **Missing Netelement**: Track segment not included in network GeoJSON
4. **CRS Mismatch**: GNSS and network using different coordinate systems
5. **Outdated Geometry**: Track centerline geometry outdated or incorrect

**Solutions:**

- Increase `--warning-threshold` if distances are expected
- Check network completeness (all relevant tracks included)
- Verify CRS consistency between GNSS and network
- Validate GNSS data quality (check HDOP values)
- Update network geometry if tracks have changed

### Error: "Failed to load GNSS data: Invalid timestamp"

**Problem:** Timestamps not in RFC3339 format or missing timezone.

**Required format:** `YYYY-MM-DDTHH:MM:SS±HH:MM`

**Examples:**

- ✅ `2025-12-09T14:30:00+01:00` (Brussels time)
- ✅ `2025-12-09T13:30:00Z` (UTC)
- ❌ `2025-12-09 14:30:00` (missing T and timezone)
- ❌ `2025-12-09T14:30:00` (missing timezone)

**Solution:** Fix timestamps in input CSV to include timezone.

### Error: "Failed to load network: Invalid geometry"

**Problem:** Netelement LineString has < 2 points or invalid coordinates.

**Solution:** Validate network GeoJSON:

- Each LineString must have at least 2 coordinate pairs
- Coordinates must be valid numbers
- Order: `[longitude, latitude]` (not lat/lon)

## Advanced Usage

### Batch Processing Multiple Files

```bash
# Process all CSV files in directory
for file in data/*.csv; do
  echo "Processing $file..."
  tp-cli --gnss "$file" \
         --crs EPSG:4326 \
         --network network.geojson \
         --output "output/$(basename $file .csv)_projected.csv"
done
```

### Integration with GIS Tools

```bash
# Convert output GeoJSON to Shapefile with ogr2ogr
ogr2ogr -f "ESRI Shapefile" output.shp projected.geojson -lco ENCODING=UTF-8
```

## Help

```bash
# View all options
tp-cli --help

# View subcommand options
tp-cli calculate-path --help

# View version
tp-cli --version
```

## See Also

- [TP-Lib README](../README.md) - Library usage and architecture
- [GNSS Projection Specification](../specs/001-gnss-projection/spec.md) - Projection requirements and design
- [Train Path Calculation Specification](../specs/002-train-path-calculation/spec.md) - Path algorithm specification

## Support

For issues or questions:

- File a GitHub issue: [Matdata-eu/tp-lib/issues](https://github.com/Matdata-eu/tp-lib/issues)
- Check documentation: [../specs/](../specs/)
- Review test cases: [../tp-core/tests/](../tp-core/tests/)
