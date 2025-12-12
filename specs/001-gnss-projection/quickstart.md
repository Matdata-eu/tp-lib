# Quick Start Guide: GNSS Track Axis Projection

**Feature**: 001-gnss-projection | **Date**: 2025-12-12

This guide helps you quickly start projecting GNSS positions onto railway track netelements using the `tp-lib` library.

---

## Installation

### Rust CLI

```bash
# Install from crates.io
cargo install tp-cli

# Verify installation
tp-cli --version
```

### Python Package

```bash
# Install from PyPI
pip install tp-lib

# Verify installation
python -c "import tp_lib; print(tp_lib.__version__)"
```

### Build from Source

```bash
# Clone repository
git clone https://github.com/infrabel/tp-lib.git
cd tp-lib

# Build Rust CLI
cargo build --release --bin tp-cli
./target/release/tp-cli --version

# Build Python bindings
cd tp-py
pip install maturin
maturin develop
```

---

## Basic Usage (CLI)

### Minimal Example

Project GNSS positions from a CSV file onto a GeoJSON railway network:

```bash
tp-cli project-gnss \
  --gnss-file train_journey.csv \
  --gnss-crs EPSG:31370 \
  --network-file belgium_network.geojson \
  > projected_output.csv
```

**Input Files**:

`train_journey.csv`:
```csv
latitude,longitude,timestamp
50.8503,4.3517,2025-12-09T14:30:00+01:00
50.8450,4.3610,2025-12-09T14:30:15+01:00
50.8400,4.3700,2025-12-09T14:30:30+01:00
```

`belgium_network.geojson`:
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
          [4.3610, 50.8450],
          [4.3700, 50.8400]
        ]
      },
      "properties": {
        "name": "Brussels-Midi to Central"
      }
    }
  ]
}
```

**Output** (`projected_output.csv`):
```csv
original_lat,original_lon,original_time,projected_lat,projected_lon,netelement_id,measure_meters,projection_distance_meters,crs
50.8503,4.3517,2025-12-09T14:30:00+01:00,50.8503,4.3517,NE-12345,0.0,0.5,EPSG:4326
50.8450,4.3610,2025-12-09T14:30:15+01:00,50.8450,4.3610,NE-12345,1234.5,1.2,EPSG:4326
50.8400,4.3700,2025-12-09T14:30:30+01:00,50.8400,4.3700,NE-12345,2468.9,0.8,EPSG:4326
```

---

## CLI Options

### Required Parameters

- `--gnss-file <path>`: Path to GNSS positions file (CSV or GeoJSON)
- `--gnss-crs <epsg>`: Coordinate Reference System for GNSS data (e.g., EPSG:31370)
- `--network-file <path>`: Path to railway network file (GeoJSON)

### Optional Parameters

- `--output-format <format>`: Output format (`csv` or `json`, default: `csv`)
- `--warning-threshold <meters>`: Distance threshold for diagnostic warnings (default: 50.0)
- `--lat-col <name>`: CSV column name for latitude (default: `latitude`)
- `--lon-col <name>`: CSV column name for longitude (default: `longitude`)
- `--time-col <name>`: CSV column name for timestamp (default: `timestamp`)

### Examples

**Custom CSV Column Names**:
```bash
tp-cli project-gnss \
  --gnss-file journey.csv \
  --gnss-crs EPSG:31370 \
  --lat-col "lat" \
  --lon-col "lon" \
  --time-col "time_utc" \
  --network-file network.geojson \
  > output.csv
```

**JSON Output Format**:
```bash
tp-cli project-gnss \
  --gnss-file journey.csv \
  --gnss-crs EPSG:4326 \
  --network-file network.geojson \
  --output-format json \
  > output.json
```

**Low Warning Threshold (10 meters)**:
```bash
tp-cli project-gnss \
  --gnss-file journey.csv \
  --gnss-crs EPSG:31370 \
  --network-file network.geojson \
  --warning-threshold 10.0 \
  > output.csv 2> warnings.log
```

---

## Python Library Usage

### Basic Example

```python
import tp_lib

# Project GNSS positions
results = tp_lib.project_gnss(
    gnss_file="train_journey.csv",
    gnss_crs="EPSG:31370",
    network_file="belgium_network.geojson",
)

# Access results (list of ProjectedPosition objects)
for result in results:
    print(f"Netelement: {result.netelement_id}")
    print(f"Measure: {result.measure_meters:.2f} m")
    print(f"Distance: {result.projection_distance_meters:.2f} m")
```

### Advanced Example (Pandas Integration)

```python
import tp_lib
import pandas as pd

# Project GNSS positions
results = tp_lib.project_gnss(
    gnss_file="journey.csv",
    gnss_crs="EPSG:31370",
    network_file="network.geojson",
    config={
        "warning_threshold": 50.0,
        "output_format": "dataframe",
    }
)

# Convert to Pandas DataFrame
df = pd.DataFrame([
    {
        "original_lat": r.original.latitude,
        "original_lon": r.original.longitude,
        "original_time": r.original.timestamp,
        "projected_lat": r.projected_coords.y,
        "projected_lon": r.projected_coords.x,
        "netelement_id": r.netelement_id,
        "measure_meters": r.measure_meters,
        "projection_distance_meters": r.projection_distance_meters,
    }
    for r in results
])

# Analyze results
print(f"Mean projection distance: {df['projection_distance_meters'].mean():.2f} m")
print(f"Max projection distance: {df['projection_distance_meters'].max():.2f} m")
```

### Error Handling

```python
import tp_lib

try:
    results = tp_lib.project_gnss(
        gnss_file="journey.csv",
        gnss_crs="INVALID_CRS",
        network_file="network.geojson",
    )
except ValueError as e:
    print(f"Validation error: {e}")
except RuntimeError as e:
    print(f"Projection error: {e}")
```

---

## Understanding Output

### CSV Output Format

| Column | Type | Description |
|--------|------|-------------|
| `original_lat` | Float | Original GNSS latitude |
| `original_lon` | Float | Original GNSS longitude |
| `original_time` | Timestamp | Original GNSS timestamp with timezone |
| `projected_lat` | Float | Projected latitude on netelement |
| `projected_lon` | Float | Projected longitude on netelement |
| `netelement_id` | String | ID of nearest netelement |
| `measure_meters` | Float | Distance along netelement from start (meters) |
| `projection_distance_meters` | Float | Perpendicular distance from GNSS to projected point |
| `crs` | String | Coordinate Reference System (e.g., EPSG:4326) |

### JSON Output Format

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
        "measure_meters": 1234.56,
        "projection_distance_meters": 2.3,
        "crs": "EPSG:4326"
      }
    }
  ]
}
```

---

## Common Scenarios

### Scenario 1: Belgian Railway GNSS Data

Belgian railways use **Lambert 2008** (EPSG:3812) for local coordinates.

```bash
tp-cli project-gnss \
  --gnss-file belgian_train.csv \
  --gnss-crs EPSG:3812 \
  --network-file belgium_network.geojson \
  > output.csv
```

### Scenario 2: International GNSS Data (WGS84)

Modern GNSS devices output **WGS84** (EPSG:4326).

```bash
tp-cli project-gnss \
  --gnss-file gps_track.csv \
  --gnss-crs EPSG:4326 \
  --network-file network.geojson \
  > output.csv
```

### Scenario 3: Large Dataset (10,000+ positions)

For large datasets, monitor warnings and performance:

```bash
# Redirect warnings to log file
tp-cli project-gnss \
  --gnss-file large_journey.csv \
  --gnss-crs EPSG:31370 \
  --network-file network.geojson \
  --warning-threshold 50.0 \
  > output.csv 2> warnings.log

# Check warnings
grep "projection distance" warnings.log
```

---

## Diagnostic Warnings

### Warning: High Projection Distance

**Message**: `WARNING: Projection distance 85.3m exceeds threshold 50.0m for position at 2025-12-09T14:30:45+01:00`

**Meaning**: GNSS position is >50m away from nearest netelement (may indicate poor GNSS quality or train off-network).

**Actions**:
1. Inspect GNSS data quality (satellite count, HDOP)
2. Verify railway network completeness (missing tracks?)
3. Adjust warning threshold: `--warning-threshold 100.0`

### Warning: CRS Transformation

**Message**: `INFO: Transforming GNSS CRS EPSG:3812 → Network CRS EPSG:4326`

**Meaning**: GNSS and network use different coordinate systems; transformation applied.

**Actions**:
- Verify transformation accuracy for region
- Ensure EPSG codes are correct

---

## Troubleshooting

### Error: Invalid CRS

**Message**: `Error: Invalid CRS: EPSG:9999`

**Cause**: Unknown or unsupported EPSG code.

**Fix**: Use standard EPSG codes:
- `EPSG:4326`: WGS84 (global)
- `EPSG:3812`: Belgian Lambert 2008
- `EPSG:31370`: Belgian Lambert 72 (legacy)

### Error: Missing Timezone

**Message**: `Error: Missing timezone in timestamp: 2025-12-09T14:30:00`

**Cause**: Timestamp lacks timezone offset.

**Fix**: Add timezone to CSV:
```csv
timestamp
2025-12-09T14:30:00+01:00
```

### Error: No Netelements in Network

**Message**: `Error: No netelements in network`

**Cause**: GeoJSON file is empty or malformed.

**Fix**: Validate GeoJSON structure:
```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": { "type": "LineString", "coordinates": [...] }
    }
  ]
}
```

### Error: Invalid Coordinate

**Message**: `Error: Invalid coordinate: lat=91.0, lon=4.0`

**Cause**: Latitude/longitude out of valid range.

**Fix**: Validate CSV data:
- Latitude: -90.0 to 90.0
- Longitude: -180.0 to 180.0

---

## Best Practices

### 1. Always Specify GNSS CRS

GeoJSON mandates WGS84, but CSV files often use local CRS. **Always** use `--gnss-crs`:

```bash
# ✅ Good: Explicit CRS
tp-cli project-gnss --gnss-file data.csv --gnss-crs EPSG:31370 --network-file network.geojson

# ❌ Bad: CLI will reject missing --gnss-crs for CSV input
tp-cli project-gnss --gnss-file data.csv --network-file network.geojson
```

### 2. Include Timezone in Timestamps

Use ISO 8601 format with timezone:

```csv
# ✅ Good
timestamp
2025-12-09T14:30:00+01:00

# ❌ Bad (rejected)
timestamp
2025-12-09 14:30:00
```

### 3. Validate Input Data First

```bash
# Check CSV structure
head -n 5 journey.csv

# Validate GeoJSON
cat network.geojson | jq '.features | length'
```

### 4. Monitor Projection Quality

```bash
# Extract projection distances from output
cut -d',' -f8 output.csv | sort -n | tail -n 10
```

### 5. Use Version Control for Network Data

Railway networks change over time. Version control ensures reproducibility:

```bash
# Tag network version
git tag -a network-v1.0 -m "Belgium network as of 2025-12-09"
```

---

## Performance Tips

### Large Datasets

For datasets with >10,000 positions:

1. **Use Arrow format** (future enhancement):
   ```bash
   tp-cli project-gnss --input-format arrow --gnss-file journey.arrow ...
   ```

2. **Split into batches**:
   ```bash
   split -l 5000 large_journey.csv batch_
   for file in batch_*; do
     tp-cli project-gnss --gnss-file $file ... >> all_output.csv
   done
   ```

3. **Monitor memory usage**:
   ```bash
   /usr/bin/time -v tp-cli project-gnss ...
   ```

---

## Next Steps

- **API Reference**: See [contracts/cli.md](./contracts/cli.md) for full CLI specification
- **Library API**: See [contracts/lib-api.md](./contracts/lib-api.md) for Rust API
- **Python API**: See [contracts/python-api.md](./contracts/python-api.md) for Python bindings
- **Data Model**: See [data-model.md](./data-model.md) for entity definitions

---

## Support

- **GitHub Issues**: https://github.com/infrabel/tp-lib/issues
- **Documentation**: https://docs.infrabel.be/tp-lib
- **Email**: tp-lib-support@infrabel.be
