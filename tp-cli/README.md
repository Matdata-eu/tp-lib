# tp-cli: Train Positioning CLI

> Command-line interface for projecting GNSS positions onto railway track netelements

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

### Basic Usage

```bash
# Project CSV GNSS data onto railway network
tp-cli --gnss-file positions.csv \
       --crs EPSG:4326 \
       --network-file network.geojson

# Output defaults to CSV format on stdout
```

### With GeoJSON Output

```bash
tp-cli --gnss-file positions.csv \
       --crs EPSG:4326 \
       --network-file network.geojson \
       --output-format json > projected.geojson
```

### Custom Warning Threshold

```bash
# Warn only for projection distances > 100 meters
tp-cli --gnss-file positions.csv \
       --crs EPSG:4326 \
       --network-file network.geojson \
       --warning-threshold 100.0
```

### Custom CSV Column Names

```bash
tp-cli --gnss-file data.csv \
       --crs EPSG:4326 \
       --network-file network.geojson \
       --lat-col lat \
       --lon-col lon \
       --time-col ts
```

## Arguments

### Required Arguments

#### `--gnss-file <FILE>` (or `-g`)

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

#### `--network-file <FILE>` (or `-n`)

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

#### `--output-format <FORMAT>` (or `-o`)

Output format: `csv` or `json`. Default: `csv`.

**CSV Output Example:**

```csv
original_lat,original_lon,original_time,projected_lat,projected_lon,netelement_id,measure_meters,projection_distance_meters,crs
50.8503,4.3517,2025-12-09T14:30:00+01:00,50.85074,4.35148,NE001,132.54,51.31,EPSG:4326
```

**JSON Output Example:**

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

#### `--warning-threshold <METERS>` (or `-w`)

Distance threshold in meters for emitting projection warnings. Default: `50.0`.

Warnings are printed to stderr when a GNSS position projects more than this distance from the track.

**Example:**

```bash
# Warn only for distances > 100m
tp-cli --gnss-file data.csv --crs EPSG:4326 --network-file network.geojson -w 100.0
```

#### `--lat-col <COLUMN>`

Latitude column name in CSV input. Default: `latitude`.

#### `--lon-col <COLUMN>`

Longitude column name in CSV input. Default: `longitude`.

#### `--time-col <COLUMN>`

Timestamp column name in CSV input. Default: `timestamp`.

**Example with custom columns:**

```bash
tp-cli --gnss-file data.csv \
       --crs EPSG:4326 \
       --network-file network.geojson \
       --lat-col lat --lon-col lon --time-col ts
```

## Exit Codes

- **0**: Success - All positions processed successfully
- **1**: Validation error - Invalid arguments or configuration
- **2**: Processing error - Projection or computation failure
- **3**: I/O error - File not found, permission denied, or read/write failure

## Output Behavior

- **Stdout**: Projected results (CSV or GeoJSON)
- **Stderr**: Warnings and error messages

**Example:**

```bash
# Redirect output to file, view warnings in terminal
tp-cli --gnss-file data.csv --crs EPSG:4326 --network-file network.geojson > output.csv

# Redirect both output and errors
tp-cli --gnss-file data.csv --crs EPSG:4326 --network-file network.geojson > output.csv 2> errors.log
```

## Examples

### Example 1: Basic CSV Processing

```bash
tp-cli --gnss-file train_journey.csv \
       --crs EPSG:4326 \
       --network-file infrabel_network.geojson \
       > projected_positions.csv
```

**Input (train_journey.csv):**

```csv
latitude,longitude,timestamp,speed,heading
50.8503,4.3517,2025-12-09T14:30:00+01:00,80.5,45
50.8504,4.3518,2025-12-09T14:30:01+01:00,81.2,46
```

**Output (projected_positions.csv):**

```csv
original_lat,original_lon,original_time,projected_lat,projected_lon,netelement_id,measure_meters,projection_distance_meters,crs
50.8503,4.3517,2025-12-09T14:30:00+01:00,50.85074,4.35148,NE001,132.54,51.31,EPSG:4326
50.8504,4.3518,2025-12-09T14:30:01+01:00,50.8508,4.3516,NE001,143.28,46.64,EPSG:4326
```

### Example 2: GeoJSON Output with Custom Threshold

```bash
tp-cli --gnss-file positions.csv \
       --crs EPSG:31370 \
       --network-file network.geojson \
       --output-format json \
       --warning-threshold 75.0 \
       > projected.geojson
```

### Example 3: Custom Column Names

```bash
# Input CSV has columns: lat, lon, time
tp-cli --gnss-file gps_data.csv \
       --crs EPSG:4326 \
       --network-file tracks.geojson \
       --lat-col lat \
       --lon-col lon \
       --time-col time \
       > output.csv
```

### Example 4: Pipeline with Filtering

```bash
# Project positions and filter for low projection distances
tp-cli --gnss-file data.csv --crs EPSG:4326 --network-file network.geojson | \
  awk -F',' 'NR==1 || $8 < 30' > high_quality_positions.csv
```

## Troubleshooting

### Error: "CRS is required for CSV GNSS input"

**Problem:** Forgot to specify `--crs` for CSV input.

**Solution:**

```bash
# Add --crs flag
tp-cli --gnss-file data.csv --crs EPSG:4326 --network-file network.geojson
```

### Error: "CRS should not be specified for GeoJSON GNSS input"

**Problem:** Provided `--crs` with GeoJSON input (CRS read from file).

**Solution:**

```bash
# Remove --crs flag
tp-cli --gnss-file data.geojson --network-file network.geojson
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

### Performance Issues

**Symptom:** Processing takes too long (> 10 seconds for 1000 positions × 50 netelements).

**Possible causes:**

- Very large network (> 1000 netelements)
- Complex netelement geometries (> 1000 points per LineString)
- High I/O overhead (network file on slow disk)

**Solutions:**

- Simplify netelement geometries (reduce points with Douglas-Peucker algorithm)
- Split large networks into regional subsets
- Use SSD for input files
- Consider using library API for bulk processing (avoids CLI startup overhead)

## Advanced Usage

### Batch Processing Multiple Files

```bash
# Process all CSV files in directory
for file in data/*.csv; do
  echo "Processing $file..."
  tp-cli --gnss-file "$file" \
         --crs EPSG:4326 \
         --network-file network.geojson \
         > "output/$(basename $file .csv)_projected.csv"
done
```

### Integration with GIS Tools

```bash
# Convert output GeoJSON to Shapefile with ogr2ogr
tp-cli --gnss-file data.csv --crs EPSG:4326 --network-file network.geojson -o json | \
  ogr2ogr -f "ESRI Shapefile" output.shp /vsistdin/ -lco ENCODING=UTF-8
```

### Quality Filtering

```bash
# Extract only high-quality projections (< 20m distance)
tp-cli --gnss-file data.csv --crs EPSG:4326 --network-file network.geojson | \
  awk -F',' 'NR==1 || ($8+0) < 20' > high_quality.csv
```

## Help

```bash
# View all options
tp-cli --help

# View version
tp-cli --version
```

## See Also

- [TP-Lib README](../README.md) - Library usage and architecture
- [API Documentation](https://docs.rs/tp-core) - Rust API reference
- [Feature Specification](../specs/001-gnss-projection/spec.md) - Requirements and design

## Support

For issues or questions:

- File a GitHub issue: [infrabel/tp-lib/issues](https://github.com/infrabel/tp-lib/issues)
- Check documentation: [../specs/](../specs/)
- Review test cases: [../tp-core/tests/](../tp-core/tests/)
