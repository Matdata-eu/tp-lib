# CLI Contract: GNSS Track Axis Projection

**Feature**: 001-gnss-projection | **Version**: 1.0.0 | **Date**: 2025-12-12

This document defines the command-line interface contract for the GNSS track axis projection feature. This contract is stable and breaking changes require a major version bump.

---

## Command Signature

```bash
tp-cli project-gnss [OPTIONS]
```

**Purpose**: Project GNSS positions onto railway track netelements (track axis centerlines).

---

## Required Options

### `--gnss-file <PATH>`

Path to GNSS positions file.

- **Type**: File path (absolute or relative)
- **Format**: CSV or GeoJSON
- **Required**: Yes
- **Example**: `--gnss-file train_journey.csv`

**Validation**:
- File must exist and be readable
- File must be valid CSV or GeoJSON

---

### `--gnss-crs <EPSG>`

Coordinate Reference System for GNSS data.

- **Type**: EPSG code (e.g., `EPSG:4326`)
- **Required**: Yes for CSV input, rejected for GeoJSON input
- **Example**: `--gnss-crs EPSG:31370`

**Validation**:
- Must be valid EPSG code
- Must be supported by PROJ library

**Clarification** (from spec.md Q5):
- **CSV**: MUST specify via `--gnss-crs` parameter
- **GeoJSON**: MUST reject `--gnss-crs` (GeoJSON includes CRS in file)

---

### `--network-file <PATH>`

Path to railway network file.

- **Type**: File path (absolute or relative)
- **Format**: GeoJSON FeatureCollection
- **Required**: Yes
- **Example**: `--network-file belgium_network.geojson`

**Validation**:
- File must exist and be readable
- Must be valid GeoJSON FeatureCollection
- Each Feature must have LineString geometry
- CRS must be WGS84 (EPSG:4326) per RFC 7946

---

## Optional Options

### `--output-format <FORMAT>`

Output format for projected positions.

- **Type**: Enum (`csv`, `json`)
- **Default**: `csv`
- **Example**: `--output-format json`

**Behavior**:
- `csv`: Output CSV with header row to stdout
- `json`: Output GeoJSON FeatureCollection to stdout

---

### `--warning-threshold <METERS>`

Distance threshold for diagnostic warnings.

- **Type**: Float (meters)
- **Default**: `50.0`
- **Example**: `--warning-threshold 100.0`

**Behavior**:
- If projection distance > threshold, emit warning to stderr
- Warning format: `WARNING: Projection distance {distance}m exceeds threshold {threshold}m for position at {timestamp}`

---

### `--lat-col <NAME>`

CSV column name for latitude (CSV input only).

- **Type**: String
- **Default**: `latitude`
- **Example**: `--lat-col "lat"`

---

### `--lon-col <NAME>`

CSV column name for longitude (CSV input only).

- **Type**: String
- **Default**: `longitude`
- **Example**: `--lon-col "lon"`

---

### `--time-col <NAME>`

CSV column name for timestamp (CSV input only).

- **Type**: String
- **Default**: `timestamp`
- **Example**: `--time-col "time_utc"`

---

### `--help`

Display help message and exit.

- **Alias**: `-h`
- **Example**: `tp-cli project-gnss --help`

---

### `--version`

Display version and exit.

- **Alias**: `-V`
- **Example**: `tp-cli project-gnss --version`

---

## Input/Output Protocol

### Standard Input (stdin)

**Not used** (all input via files).

---

### Standard Output (stdout)

**Format**: CSV or JSON depending on `--output-format`

**CSV Example**:
```csv
original_lat,original_lon,original_time,projected_lat,projected_lon,netelement_id,measure_meters,projection_distance_meters,crs
50.8503,4.3517,2025-12-09T14:30:00+01:00,50.8503,4.3517,NE-12345,0.0,0.5,EPSG:4326
```

**JSON Example**:
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
        "original_lat": 50.8503,
        "original_lon": 4.3517,
        "original_time": "2025-12-09T14:30:00+01:00",
        "netelement_id": "NE-12345",
        "measure_meters": 0.0,
        "projection_distance_meters": 0.5,
        "crs": "EPSG:4326"
      }
    }
  ]
}
```

---

### Standard Error (stderr)

**Format**: Plain text log messages

**Message Types**:
1. **Warnings**: Projection distance exceeds threshold
2. **Errors**: Validation failures, processing errors
3. **Info**: CRS transformations, processing stats

**Example**:
```text
INFO: Loaded 1000 GNSS positions from train_journey.csv
INFO: Loaded 50 netelements from belgium_network.geojson
INFO: Transforming GNSS CRS EPSG:3812 → Network CRS EPSG:4326
WARNING: Projection distance 85.3m exceeds threshold 50.0m for position at 2025-12-09T14:30:45+01:00
INFO: Projected 1000 positions in 2.3 seconds
```

---

### Exit Codes

| Code | Meaning | Scenarios |
|------|---------|-----------|
| `0` | Success | All positions projected successfully |
| `1` | Validation error | Invalid input files, missing CRS, malformed data |
| `2` | Processing error | CRS transformation failed, projection failed |
| `3` | I/O error | File not found, permission denied |

---

## Examples

### Example 1: Basic Usage

```bash
tp-cli project-gnss \
  --gnss-file journey.csv \
  --gnss-crs EPSG:31370 \
  --network-file network.geojson \
  > output.csv
```

**Input**: `journey.csv` (Belgian Lambert 72), `network.geojson` (WGS84)  
**Output**: `output.csv` with projected positions  
**Exit Code**: `0`

---

### Example 2: Custom Column Names

```bash
tp-cli project-gnss \
  --gnss-file journey.csv \
  --gnss-crs EPSG:4326 \
  --lat-col "lat" \
  --lon-col "lon" \
  --time-col "time_utc" \
  --network-file network.geojson \
  > output.csv
```

**Input**: `journey.csv` with columns `lat`, `lon`, `time_utc`  
**Output**: `output.csv`

---

### Example 3: JSON Output with Warnings

```bash
tp-cli project-gnss \
  --gnss-file journey.csv \
  --gnss-crs EPSG:31370 \
  --network-file network.geojson \
  --output-format json \
  --warning-threshold 30.0 \
  > output.json 2> warnings.log
```

**Input**: `journey.csv`, `network.geojson`  
**Output**: `output.json` (GeoJSON)  
**Warnings**: `warnings.log` (positions >30m from track)

---

### Example 4: Error Handling

```bash
tp-cli project-gnss \
  --gnss-file missing.csv \
  --gnss-crs EPSG:31370 \
  --network-file network.geojson

# Output (stderr):
# Error: File not found: missing.csv
# Exit code: 3
```

---

## CSV Input Format

### Required Columns

- **Latitude**: Decimal degrees (default column: `latitude`)
- **Longitude**: Decimal degrees (default column: `longitude`)
- **Timestamp**: ISO 8601 with timezone (default column: `timestamp`)

### Optional Columns

All other columns are preserved in output as metadata.

### Example CSV

```csv
latitude,longitude,timestamp,train_id,speed_kmh
50.8503,4.3517,2025-12-09T14:30:00+01:00,TR-123,80
50.8450,4.3610,2025-12-09T14:30:15+01:00,TR-123,85
```

**Validation Rules**:
- Header row required
- Required columns must exist
- Latitude: `-90.0 ≤ lat ≤ 90.0`
- Longitude: `-180.0 ≤ lon ≤ 180.0`
- Timestamp: Must include timezone (e.g., `+01:00`, `Z`)

---

## GeoJSON Input Format (GNSS)

### Structure

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
        "timestamp": "2025-12-09T14:30:00+01:00",
        "train_id": "TR-123"
      }
    }
  ]
}
```

**Validation Rules**:
- CRS must be WGS84 (EPSG:4326) per RFC 7946
- Each Feature must have Point geometry
- `properties.timestamp` required

---

## GeoJSON Network Format

### Structure

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "id": "NE-12345",
      "geometry": {
        "type": "LineString",
        "coordinates": [
          [4.3517, 50.8503],
          [4.3610, 50.8450]
        ]
      },
      "properties": {
        "name": "Brussels-Midi to Central"
      }
    }
  ]
}
```

**Validation Rules**:
- CRS must be WGS84 (EPSG:4326) per RFC 7946
- Each Feature must have LineString geometry
- Feature `id` is used as netelement ID
- LineString must have ≥2 coordinates

---

## CSV Output Format

### Columns

| Column | Type | Description |
|--------|------|-------------|
| `original_lat` | Float | Original GNSS latitude |
| `original_lon` | Float | Original GNSS longitude |
| `original_time` | Timestamp | Original GNSS timestamp (ISO 8601 with timezone) |
| `projected_lat` | Float | Projected latitude on netelement |
| `projected_lon` | Float | Projected longitude on netelement |
| `netelement_id` | String | Netelement ID from GeoJSON |
| `measure_meters` | Float | Distance along netelement from start (meters) |
| `projection_distance_meters` | Float | Perpendicular distance from GNSS to projected point |
| `crs` | String | CRS of projected coordinates (e.g., EPSG:4326) |
| `[metadata columns]` | Various | Additional columns from input CSV |

### Example

```csv
original_lat,original_lon,original_time,projected_lat,projected_lon,netelement_id,measure_meters,projection_distance_meters,crs,train_id,speed_kmh
50.8503,4.3517,2025-12-09T14:30:00+01:00,50.8503,4.3517,NE-12345,0.0,0.5,EPSG:4326,TR-123,80
```

---

## JSON Output Format

### Structure

GeoJSON FeatureCollection with projected positions as Point features.

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
        "original_lat": 50.8503,
        "original_lon": 4.3517,
        "original_time": "2025-12-09T14:30:00+01:00",
        "netelement_id": "NE-12345",
        "measure_meters": 0.0,
        "projection_distance_meters": 0.5,
        "crs": "EPSG:4326",
        "train_id": "TR-123",
        "speed_kmh": 80
      }
    }
  ]
}
```

---

## Error Messages

### Validation Errors (Exit Code 1)

| Error | Cause | Example |
|-------|-------|---------|
| `File not found: {path}` | Input file doesn't exist | `--gnss-file missing.csv` |
| `Invalid CSV: missing required column '{col}'` | CSV lacks lat/lon/time column | Missing `latitude` column |
| `Invalid coordinate: lat={lat}, lon={lon}` | Coordinate out of range | `lat=91.0` |
| `Missing timezone in timestamp: {ts}` | Timestamp lacks timezone | `2025-12-09T14:30:00` |
| `Invalid CRS: {crs}` | Unknown EPSG code | `EPSG:9999` |
| `Invalid GeoJSON: {reason}` | Malformed GeoJSON | Missing `features` array |

### Processing Errors (Exit Code 2)

| Error | Cause | Example |
|-------|-------|---------|
| `CRS transformation failed: {reason}` | PROJ library error | Invalid coordinate for CRS |
| `No netelements in network` | Empty GeoJSON | GeoJSON with 0 features |
| `Projection failed: {reason}` | Geometric error | Invalid LineString |

### I/O Errors (Exit Code 3)

| Error | Cause | Example |
|-------|-------|---------|
| `Permission denied: {path}` | File not readable | Protected file |
| `Failed to write output` | stdout write error | Disk full |

---

## Performance Guarantees

| Metric | Target | Notes |
|--------|--------|-------|
| Throughput | 1000 positions in <10s | With 50 netelements (SC-001) |
| Memory | <500 MB | For typical datasets (1000 positions) |
| Scalability | 10,000+ positions | Without memory exhaustion (SC-006) |

---

## Contract Stability

**Version**: 1.0.0  
**Stability**: Stable

**Breaking Changes** (require major version bump):
- Rename CLI options
- Change default values
- Remove options
- Change output format structure

**Non-Breaking Changes** (minor version bump):
- Add new optional options
- Add new output columns (CSV)
- Add new properties (JSON)
- Improve error messages

**Backward Compatibility**:
- All 1.x.x versions must accept 1.0.0 input
- Output format may gain new fields, but existing fields remain stable

---

## Testing Contract

### Contract Test Cases

**Test 1: Basic Projection**
```bash
tp-cli project-gnss \
  --gnss-file test_journey.csv \
  --gnss-crs EPSG:4326 \
  --network-file test_network.geojson \
  > output.csv

# Verify:
# - Exit code 0
# - Output CSV has header row
# - Output has same number of records as input
```

**Test 2: Invalid CRS**
```bash
tp-cli project-gnss \
  --gnss-file journey.csv \
  --gnss-crs EPSG:9999 \
  --network-file network.geojson

# Verify:
# - Exit code 1
# - stderr contains "Invalid CRS: EPSG:9999"
```

**Test 3: Missing File**
```bash
tp-cli project-gnss \
  --gnss-file missing.csv \
  --gnss-crs EPSG:4326 \
  --network-file network.geojson

# Verify:
# - Exit code 3
# - stderr contains "File not found: missing.csv"
```

**Test 4: Warning Threshold**
```bash
tp-cli project-gnss \
  --gnss-file far_positions.csv \
  --gnss-crs EPSG:4326 \
  --network-file network.geojson \
  --warning-threshold 10.0 \
  2> warnings.log

# Verify:
# - warnings.log contains "WARNING: Projection distance"
```

---

## Next Steps

- **Library API**: See [lib-api.md](./lib-api.md) for Rust public API
- **Python API**: See [python-api.md](./python-api.md) for Python bindings
- **Data Model**: See [data-model.md](../data-model.md) for entity definitions
- **User Guide**: See [quickstart.md](../quickstart.md) for usage examples
