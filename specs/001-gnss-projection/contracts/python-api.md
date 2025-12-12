# Python API Contract: GNSS Track Axis Projection

**Feature**: 001-gnss-projection | **Version**: 1.0.0 | **Date**: 2025-12-12

This document defines the stable Python API for the GNSS track axis projection library. The Python binding wraps the Rust core library using PyO3.

---

## Installation

```bash
pip install tp-lib
```

**Requirements**: Python 3.12+

---

## Public API Surface

### Core Function: `project_gnss`

```python
def project_gnss(
    gnss_file: str,
    gnss_crs: str,
    network_file: str,
    *,
    output_format: str = "dataframe",
    warning_threshold: float = 50.0,
    lat_col: str = "latitude",
    lon_col: str = "longitude",
    time_col: str = "timestamp",
) -> list[ProjectedPosition] | pd.DataFrame:
    """
    Project GNSS positions onto railway track netelements.
    
    Args:
        gnss_file: Path to GNSS positions file (CSV or GeoJSON)
        gnss_crs: Coordinate Reference System for GNSS data (e.g., "EPSG:4326")
        network_file: Path to railway network file (GeoJSON)
        output_format: Output format ("list", "dataframe", "geodataframe")
        warning_threshold: Distance threshold for warnings in meters (default: 50.0)
        lat_col: CSV column name for latitude (default: "latitude")
        lon_col: CSV column name for longitude (default: "longitude")
        time_col: CSV column name for timestamp (default: "timestamp")
    
    Returns:
        list[ProjectedPosition] if output_format="list"
        pd.DataFrame if output_format="dataframe"
        gpd.GeoDataFrame if output_format="geodataframe"
    
    Raises:
        ValueError: Invalid input (bad CRS, missing columns, invalid coordinates)
        RuntimeError: Processing error (CRS transformation failed, projection failed)
        FileNotFoundError: Input file not found
    
    Example:
        >>> import tp_lib
        >>> results = tp_lib.project_gnss(
        ...     gnss_file="journey.csv",
        ...     gnss_crs="EPSG:31370",
        ...     network_file="network.geojson"
        ... )
        >>> print(results.head())
    """
```

---

## Data Classes

### `GnssPosition`

```python
from dataclasses import dataclass
from datetime import datetime

@dataclass
class GnssPosition:
    """Raw GNSS position measurement."""
    
    latitude: float
    """Latitude in decimal degrees (-90.0 to 90.0)"""
    
    longitude: float
    """Longitude in decimal degrees (-180.0 to 180.0)"""
    
    timestamp: datetime
    """Timestamp with timezone information"""
    
    crs: str
    """Coordinate Reference System (e.g., 'EPSG:4326')"""
    
    metadata: dict[str, str]
    """Additional metadata from CSV"""
```

---

### `Netelement`

```python
from dataclasses import dataclass

@dataclass
class Netelement:
    """Railway track segment."""
    
    id: str
    """Unique identifier"""
    
    geometry: list[tuple[float, float]]
    """Track centerline as list of (lon, lat) coordinates"""
    
    crs: str
    """Coordinate Reference System"""
```

---

### `ProjectedPosition`

```python
from dataclasses import dataclass

@dataclass
class ProjectedPosition:
    """Result of projecting a GNSS position onto a netelement."""
    
    original: GnssPosition
    """Original GNSS position (preserved)"""
    
    projected_coords: tuple[float, float]
    """Projected coordinates (lon, lat) on netelement"""
    
    netelement_id: str
    """ID of netelement where position was projected"""
    
    measure_meters: float
    """Distance along netelement from start in meters"""
    
    projection_distance_meters: float
    """Perpendicular distance from GNSS to projected point in meters"""
    
    crs: str
    """Coordinate Reference System of projected coordinates"""
```

---

## Usage Examples

### Example 1: Basic Usage

```python
import tp_lib

# Project GNSS positions
results = tp_lib.project_gnss(
    gnss_file="train_journey.csv",
    gnss_crs="EPSG:31370",
    network_file="belgium_network.geojson",
)

# Access results as Pandas DataFrame
print(results.head())
print(f"Processed {len(results)} positions")
```

**Output**:
```
   original_lat  original_lon           original_time  projected_lat  projected_lon netelement_id  measure_meters  projection_distance_meters      crs
0      50.8503       4.3517  2025-12-09 14:30:00+01:00       50.8503        4.3517     NE-12345            0.00                        0.50  EPSG:4326
1      50.8450       4.3610  2025-12-09 14:30:15+01:00       50.8450        4.3610     NE-12345         1234.50                        1.20  EPSG:4326
```

---

### Example 2: List Output Format

```python
import tp_lib

# Get results as list of ProjectedPosition objects
results = tp_lib.project_gnss(
    gnss_file="journey.csv",
    gnss_crs="EPSG:4326",
    network_file="network.geojson",
    output_format="list",
)

# Access individual results
for result in results:
    print(f"Netelement: {result.netelement_id}")
    print(f"Measure: {result.measure_meters:.2f} m")
    print(f"Distance: {result.projection_distance_meters:.2f} m")
```

---

### Example 3: GeoPandas Integration

```python
import tp_lib
import geopandas as gpd

# Get results as GeoDataFrame
results = tp_lib.project_gnss(
    gnss_file="journey.csv",
    gnss_crs="EPSG:31370",
    network_file="network.geojson",
    output_format="geodataframe",
)

# Perform spatial analysis
results.plot(column="projection_distance_meters", legend=True)

# Save to GeoJSON
results.to_file("projected_positions.geojson", driver="GeoJSON")
```

---

### Example 4: Custom Column Names

```python
import tp_lib

# CSV with non-standard column names
results = tp_lib.project_gnss(
    gnss_file="journey.csv",
    gnss_crs="EPSG:4326",
    network_file="network.geojson",
    lat_col="lat",
    lon_col="lon",
    time_col="time_utc",
)
```

---

### Example 5: Custom Warning Threshold

```python
import tp_lib
import sys

# Lower threshold for stricter quality control
results = tp_lib.project_gnss(
    gnss_file="journey.csv",
    gnss_crs="EPSG:31370",
    network_file="network.geojson",
    warning_threshold=10.0,  # Warn if >10m from track
)

# Check for warnings (printed to stderr)
max_distance = results["projection_distance_meters"].max()
if max_distance > 10.0:
    print(f"WARNING: Max projection distance {max_distance:.2f}m", file=sys.stderr)
```

---

### Example 6: Error Handling

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
    print(f"Processing error: {e}")
except FileNotFoundError as e:
    print(f"File error: {e}")
```

---

## DataFrame Schema (output_format="dataframe")

| Column | Type | Description |
|--------|------|-------------|
| `original_lat` | float64 | Original GNSS latitude |
| `original_lon` | float64 | Original GNSS longitude |
| `original_time` | datetime64[ns, tz] | Original timestamp with timezone |
| `projected_lat` | float64 | Projected latitude on netelement |
| `projected_lon` | float64 | Projected longitude on netelement |
| `netelement_id` | object (str) | Netelement ID |
| `measure_meters` | float64 | Distance along netelement from start |
| `projection_distance_meters` | float64 | Perpendicular distance from GNSS to projected point |
| `crs` | object (str) | Coordinate Reference System |
| `[metadata columns]` | various | Additional columns from input CSV |

---

## GeoDataFrame Schema (output_format="geodataframe")

**Geometry Column**: `geometry` (Point with projected coordinates)

**Properties** (same as DataFrame):
- `original_lat`, `original_lon`, `original_time`
- `netelement_id`, `measure_meters`, `projection_distance_meters`
- `crs`
- Metadata columns

**CRS**: EPSG:4326 (WGS84)

---

## Exception Hierarchy

```python
class ProjectionError(Exception):
    """Base exception for projection errors."""

class ValidationError(ProjectionError, ValueError):
    """Input validation failed."""

class ProcessingError(ProjectionError, RuntimeError):
    """Processing operation failed."""

class CrsError(ProcessingError):
    """CRS transformation failed."""
```

**Raised Exceptions**:

| Exception | Scenarios |
|-----------|-----------|
| `ValueError` | Invalid CRS, missing CSV columns, invalid coordinates, missing timezone |
| `RuntimeError` | CRS transformation failed, projection failed, no netelements in network |
| `FileNotFoundError` | Input file not found |
| `PermissionError` | Input file not readable |

---

## Advanced Usage

### Custom Processing Pipeline

```python
import tp_lib
import pandas as pd

# Load GNSS data
gnss_df = pd.read_csv("journey.csv")

# Filter by time range
gnss_filtered = gnss_df[
    (gnss_df["timestamp"] >= "2025-12-09T14:00:00")
    & (gnss_df["timestamp"] <= "2025-12-09T15:00:00")
]

# Save filtered data
gnss_filtered.to_csv("journey_filtered.csv", index=False)

# Project filtered data
results = tp_lib.project_gnss(
    gnss_file="journey_filtered.csv",
    gnss_crs="EPSG:31370",
    network_file="network.geojson",
)

# Analyze projection quality
quality_metrics = {
    "mean_distance": results["projection_distance_meters"].mean(),
    "max_distance": results["projection_distance_meters"].max(),
    "std_distance": results["projection_distance_meters"].std(),
}
print(quality_metrics)
```

---

### Integration with PyProj

```python
import tp_lib
import pyproj

# Transform results to different CRS
results = tp_lib.project_gnss(
    gnss_file="journey.csv",
    gnss_crs="EPSG:31370",
    network_file="network.geojson",
)

# Transform to Belgian Lambert 2008
transformer = pyproj.Transformer.from_crs("EPSG:4326", "EPSG:3812", always_xy=True)

results["projected_x"], results["projected_y"] = transformer.transform(
    results["projected_lon"].values,
    results["projected_lat"].values,
)

print(results[["projected_x", "projected_y"]].head())
```

---

### Batch Processing Multiple Files

```python
import tp_lib
import pandas as pd
from pathlib import Path

# Process all CSV files in directory
input_dir = Path("gnss_data")
output_dir = Path("projected_data")
output_dir.mkdir(exist_ok=True)

for csv_file in input_dir.glob("*.csv"):
    print(f"Processing {csv_file.name}...")
    
    results = tp_lib.project_gnss(
        gnss_file=str(csv_file),
        gnss_crs="EPSG:31370",
        network_file="network.geojson",
    )
    
    # Save results
    output_file = output_dir / f"{csv_file.stem}_projected.csv"
    results.to_csv(output_file, index=False)
    
    print(f"  Processed {len(results)} positions")
```

---

### Visualization with Matplotlib

```python
import tp_lib
import matplotlib.pyplot as plt

results = tp_lib.project_gnss(
    gnss_file="journey.csv",
    gnss_crs="EPSG:4326",
    network_file="network.geojson",
)

# Plot projection distance over time
fig, ax = plt.subplots(figsize=(12, 6))
ax.plot(results["original_time"], results["projection_distance_meters"])
ax.axhline(y=50.0, color="r", linestyle="--", label="Warning Threshold")
ax.set_xlabel("Time")
ax.set_ylabel("Projection Distance (m)")
ax.set_title("GNSS Projection Quality Over Time")
ax.legend()
plt.tight_layout()
plt.savefig("projection_quality.png")
```

---

## Performance Considerations

### Memory Usage

For large datasets (>10,000 positions), use `output_format="dataframe"` to leverage Pandas' efficient memory representation:

```python
import tp_lib

# Efficient for large datasets
results_df = tp_lib.project_gnss(
    gnss_file="large_journey.csv",
    gnss_crs="EPSG:31370",
    network_file="network.geojson",
    output_format="dataframe",  # Columnar format
)

# Less efficient (list of objects)
results_list = tp_lib.project_gnss(
    gnss_file="large_journey.csv",
    gnss_crs="EPSG:31370",
    network_file="network.geojson",
    output_format="list",
)
```

### Parallel Processing

For multiple independent files, use `multiprocessing`:

```python
import tp_lib
from multiprocessing import Pool
from pathlib import Path

def process_file(csv_file):
    return tp_lib.project_gnss(
        gnss_file=str(csv_file),
        gnss_crs="EPSG:31370",
        network_file="network.geojson",
    )

if __name__ == "__main__":
    csv_files = list(Path("gnss_data").glob("*.csv"))
    
    with Pool(processes=4) as pool:
        results = pool.map(process_file, csv_files)
    
    print(f"Processed {len(results)} files")
```

---

## Type Hints

Full type annotations available via `typing` module:

```python
from typing import Union, Literal
import pandas as pd
import geopandas as gpd

def project_gnss(
    gnss_file: str,
    gnss_crs: str,
    network_file: str,
    *,
    output_format: Literal["list", "dataframe", "geodataframe"] = "dataframe",
    warning_threshold: float = 50.0,
    lat_col: str = "latitude",
    lon_col: str = "longitude",
    time_col: str = "timestamp",
) -> Union[list[ProjectedPosition], pd.DataFrame, gpd.GeoDataFrame]:
    ...
```

---

## Contract Stability

**Version**: 1.0.0  
**Stability**: Stable

**Breaking Changes** (require major version bump):
- Change function signatures
- Remove parameters
- Change return types
- Rename functions or classes

**Non-Breaking Changes** (minor version bump):
- Add new optional parameters
- Add new functions or classes
- Add new DataFrame columns
- Improve error messages

**Backward Compatibility**:
- All 1.x.x versions maintain API compatibility with 1.0.0
- New DataFrame columns may be added, but existing columns remain stable

---

## Testing

### Unit Tests (pytest)

```python
import tp_lib
import pytest

def test_basic_projection():
    results = tp_lib.project_gnss(
        gnss_file="tests/data/journey.csv",
        gnss_crs="EPSG:4326",
        network_file="tests/data/network.geojson",
    )
    
    assert len(results) > 0
    assert "netelement_id" in results.columns

def test_invalid_crs():
    with pytest.raises(ValueError, match="Invalid CRS"):
        tp_lib.project_gnss(
            gnss_file="tests/data/journey.csv",
            gnss_crs="INVALID",
            network_file="tests/data/network.geojson",
        )

def test_file_not_found():
    with pytest.raises(FileNotFoundError):
        tp_lib.project_gnss(
            gnss_file="missing.csv",
            gnss_crs="EPSG:4326",
            network_file="network.geojson",
        )
```

---

## Next Steps

- **CLI Contract**: See [cli.md](./cli.md) for command-line interface
- **Rust API**: See [lib-api.md](./lib-api.md) for Rust public API
- **Data Model**: See [data-model.md](../data-model.md) for entity definitions
- **User Guide**: See [quickstart.md](../quickstart.md) for usage examples
