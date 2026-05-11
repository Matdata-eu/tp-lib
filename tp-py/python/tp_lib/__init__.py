"""
Python bindings for GNSS Track Axis Projection Library

This module provides Python access to the high-performance Rust implementation
of GNSS position projection onto railway track axis netelements.

## Example Usage

```python
from tp_lib import project_gnss, ProjectionConfig

# Basic usage with defaults
results = project_gnss(
    gnss_file="data/positions.csv",
    gnss_crs="EPSG:4326",           # WGS84
    network_file="data/network.geojson",
    network_crs="EPSG:4326",
    target_crs="EPSG:31370"         # Belgian Lambert 72
)

# Print results
for pos in results:
    print(f"Position: {pos.netelement_id} at {pos.measure_meters:.2f}m")
    print(f"  Projection distance: {pos.projection_distance_meters:.2f}m")
    print(f"  Coordinates: ({pos.projected_x:.2f}, {pos.projected_y:.2f})")

# Advanced usage with custom configuration
config = ProjectionConfig(
    max_search_radius_meters=500.0,              # Limit search radius
    projection_distance_warning_threshold=30.0,   # Warn if >30m from track
    suppress_warnings=False                       # Show warnings
)

results = project_gnss(
    gnss_file="data/positions.csv",
    gnss_crs="EPSG:4326",
    network_file="data/network.geojson", 
    network_crs="EPSG:4326",
    target_crs="EPSG:31370",
    config=config
)
```

## Input File Formats

### GNSS CSV Format

```csv
latitude,longitude,timestamp
50.8503,4.3517,2025-12-09T14:30:00+01:00
50.8510,4.3525,2025-12-09T14:30:05+01:00
```

Required columns:
- `latitude`: Decimal degrees
- `longitude`: Decimal degrees  
- `timestamp`: RFC3339 format with timezone

### Network GeoJSON Format

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
        "coordinates": [[4.35, 50.85], [4.36, 50.86]]
      }
    }
  ]
}
```

## Classes

- **ProjectedPosition**: Result of projecting a GNSS position onto a netelement
- **ProjectionConfig**: Configuration parameters for projection algorithm

## Functions

- **project_gnss()**: Main projection function
"""

from typing import List, Optional

# Import Rust extension module
from .tp_lib import (
    # Spec 001: GNSS projection
    project_gnss as _project_gnss,
    ProjectionConfig,
    ProjectedPosition,
    # Spec 002: path calculation
    calculate_train_path as _calculate_train_path,
    PathConfig,
    PathResult,
    TrainPath,
    AssociatedNetElement,
    # Spec 004: detections
    prepare_detections as _prepare_detections,
    PreparedDetections,
)

# Re-export for cleaner API
__all__ = [
    # Spec 001
    "project_gnss",
    "ProjectionConfig",
    "ProjectedPosition",
    # Spec 002
    "calculate_train_path",
    "PathConfig",
    "PathResult",
    "TrainPath",
    "AssociatedNetElement",
    # Spec 004
    "prepare_detections",
    "PreparedDetections",
]


def project_gnss(
    gnss_file: str,
    gnss_crs: str,
    network_file: str,
    network_crs: str,
    target_crs: str,
    config: Optional[ProjectionConfig] = None,
) -> List[ProjectedPosition]:
    """
    Project GNSS positions onto railway network elements.
    
    Reads GNSS positions from a CSV file and railway network from a GeoJSON file,
    then projects each position onto the nearest network element (track axis).
    
    Args:
        gnss_file: Path to CSV file with GNSS positions (columns: latitude, longitude, timestamp)
        gnss_crs: CRS of input GNSS coordinates (e.g., "EPSG:4326" for WGS84)
        network_file: Path to GeoJSON file with network LineStrings
        network_crs: CRS of network geometries (e.g., "EPSG:4326")
        target_crs: CRS for output projected coordinates (e.g., "EPSG:31370")
        config: Optional projection configuration (defaults: max_search_radius=1000m, warning_threshold=50m)
    
    Returns:
        List of ProjectedPosition objects, one per input GNSS position
    
    Raises:
        ValueError: Invalid CRS, coordinates, or geometry
        IOError: File reading errors or invalid CSV/GeoJSON format
        RuntimeError: Coordinate transformation failures
    
    Example:
        >>> from tp_lib import project_gnss, ProjectionConfig
        >>> 
        >>> results = project_gnss(
        ...     gnss_file="positions.csv",
        ...     gnss_crs="EPSG:4326",
        ...     network_file="network.geojson",
        ...     network_crs="EPSG:4326",
        ...     target_crs="EPSG:31370",
        ...     config=ProjectionConfig(max_search_radius_meters=500.0)
        ... )
        >>> 
        >>> for pos in results:
        ...     print(f"{pos.netelement_id}: {pos.measure_meters}m")
        NE001: 123.45m
        NE001: 234.56m
    """
    return _project_gnss(
        gnss_file=gnss_file,
        gnss_crs=gnss_crs,
        network_file=network_file,
        network_crs=network_crs,
        target_crs=target_crs,
        config=config,
    )


def calculate_train_path(
    gnss_file: str,
    gnss_crs: str,
    network_file: str,
    config: Optional[PathConfig] = None,
    detections: Optional[PreparedDetections] = None,
) -> PathResult:
    """
    Calculate the most probable train path through the railway network.

    Uses a Hidden Markov Model (Viterbi algorithm) to find the most likely sequence of
    network elements given the GNSS trace. Optionally anchored by prepared detections.

    Args:
        gnss_file: Path to CSV file with GNSS positions (columns: latitude, longitude, timestamp).
        gnss_crs: CRS of input GNSS coordinates (e.g., "EPSG:4326").
        network_file: Path to GeoJSON file with netelements and netrelations.
        config: Optional PathConfig with HMM tuning parameters (uses defaults if None).
        detections: Optional PreparedDetections to anchor path calculation.

    Returns:
        PathResult with attributes ``path``, ``mode``, ``projected_positions``, ``warnings``,
        and method ``detection_provenance()``.

    Raises:
        ValueError: Invalid CRS, geometry, or configuration values.
        IOError: File reading errors.
        RuntimeError: Path calculation failures.
    """
    return _calculate_train_path(
        gnss_file=gnss_file,
        gnss_crs=gnss_crs,
        network_file=network_file,
        config=config,
        detections=detections,
    )


def prepare_detections(
    detections_file: str,
    kind: str,
    gnss_file: str,
    gnss_crs: str,
    network_file: str,
    cutoff_distance: float = 2.5,
) -> PreparedDetections:
    """
    Load, validate, time-filter, and resolve train detections from a file.

    Produces a :class:`PreparedDetections` containing resolved anchors ready to inject
    into :func:`calculate_train_path` plus per-detection provenance records.

    Args:
        detections_file: Path to CSV or GeoJSON detections file.
        kind: ``"punctual"`` or ``"linear"``.
        gnss_file: Path to GNSS CSV file (columns: latitude, longitude, timestamp).
        gnss_crs: CRS of GNSS coordinates (e.g., "EPSG:4326").
        network_file: Path to network GeoJSON file.
        cutoff_distance: Max distance (meters) for coordinate-only resolution. Default 2.5.

    Returns:
        PreparedDetections with ``anchor_count``, ``warnings``, and ``records()``.
    """
    return _prepare_detections(
        detections_file=detections_file,
        kind=kind,
        gnss_file=gnss_file,
        gnss_crs=gnss_crs,
        network_file=network_file,
        cutoff_distance=cutoff_distance,
    )
