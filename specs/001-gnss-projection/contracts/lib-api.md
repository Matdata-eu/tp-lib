# Rust Library API Contract: GNSS Track Axis Projection

**Feature**: 001-gnss-projection | **Version**: 1.0.0 | **Date**: 2025-12-12

This document defines the stable Rust public API for the GNSS track axis projection library. This contract ensures backward compatibility and clear integration patterns.

---

## Public API Surface

### Core Module: `tp_lib_core::projection`

```rust
pub mod projection {
    pub use crate::models::{GnssPosition, Netelement, ProjectedPosition};
    pub use crate::errors::ProjectionError;
    
    /// Project GNSS positions onto railway network netelements
    pub fn project_gnss(
        gnss_positions: &[GnssPosition],
        network: &RailwayNetwork,
        config: ProjectionConfig,
    ) -> Result<Vec<ProjectedPosition>, ProjectionError>;
    
    /// Build railway network with spatial index
    pub fn build_network(netelements: Vec<Netelement>) -> Result<RailwayNetwork, ProjectionError>;
}
```

---

## Data Types

### 1. GnssPosition

```rust
use chrono::{DateTime, FixedOffset};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct GnssPosition {
    /// Latitude in decimal degrees (-90.0 to 90.0)
    pub latitude: f64,
    
    /// Longitude in decimal degrees (-180.0 to 180.0)
    pub longitude: f64,
    
    /// Timestamp with timezone offset
    pub timestamp: DateTime<FixedOffset>,
    
    /// Coordinate Reference System (e.g., "EPSG:4326")
    pub crs: String,
    
    /// Additional metadata (preserved in output)
    pub metadata: HashMap<String, String>,
}

impl GnssPosition {
    /// Create new GNSS position with validation
    pub fn new(
        latitude: f64,
        longitude: f64,
        timestamp: DateTime<FixedOffset>,
        crs: String,
    ) -> Result<Self, ProjectionError>;
    
    /// Parse from CSV record
    pub fn from_csv_record(
        record: &csv::StringRecord,
        lat_col: &str,
        lon_col: &str,
        time_col: &str,
        crs: String,
    ) -> Result<Self, ProjectionError>;
}
```

---

### 2. Netelement

```rust
use geo::LineString;

#[derive(Debug, Clone, PartialEq)]
pub struct Netelement {
    /// Unique identifier (from GeoJSON Feature.id)
    pub id: String,
    
    /// Track centerline geometry
    pub geometry: LineString<f64>,
    
    /// Coordinate Reference System
    pub crs: String,
}

impl Netelement {
    /// Create new netelement with validation
    pub fn new(
        id: String,
        geometry: LineString<f64>,
        crs: String,
    ) -> Result<Self, ProjectionError>;
    
    /// Parse from GeoJSON Feature
    pub fn from_geojson_feature(
        feature: &geojson::Feature,
    ) -> Result<Self, ProjectionError>;
}
```

---

### 3. ProjectedPosition

```rust
use geo::Point;

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedPosition {
    /// Original GNSS position (preserved)
    pub original: GnssPosition,
    
    /// Projected coordinates on netelement
    pub projected_coords: Point<f64>,
    
    /// Netelement ID where position was projected
    pub netelement_id: String,
    
    /// Distance along netelement from start (meters)
    pub measure_meters: f64,
    
    /// Perpendicular distance from GNSS to projected point (meters)
    pub projection_distance_meters: f64,
    
    /// CRS of projected coordinates
    pub crs: String,
}

impl ProjectedPosition {
    /// Convert to CSV record
    pub fn to_csv_record(&self) -> Vec<String>;
    
    /// Convert to GeoJSON Feature
    pub fn to_geojson_feature(&self) -> geojson::Feature;
}
```

---

### 4. RailwayNetwork

```rust
use rstar::RTree;

pub struct RailwayNetwork {
    netelements: Vec<Netelement>,
    spatial_index: RTree<NetelementIndexEntry>,
}

impl RailwayNetwork {
    /// Build network from netelements
    pub fn new(netelements: Vec<Netelement>) -> Result<Self, ProjectionError>;
    
    /// Find nearest netelement to point
    pub fn find_nearest(&self, point: &Point<f64>) -> Option<&Netelement>;
    
    /// Get netelement by ID
    pub fn get_by_id(&self, id: &str) -> Option<&Netelement>;
    
    /// Get all netelements
    pub fn netelements(&self) -> &[Netelement];
}
```

---

### 5. ProjectionConfig

```rust
#[derive(Debug, Clone)]
pub struct ProjectionConfig {
    /// Distance threshold for diagnostic warnings (meters)
    pub warning_threshold: f64,
    
    /// Enable CRS transformation
    pub transform_crs: bool,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 50.0,
            transform_crs: true,
        }
    }
}
```

---

## Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectionError {
    #[error("Invalid CRS: {0}")]
    InvalidCrs(String),
    
    #[error("CRS transformation failed: {0}")]
    TransformFailed(#[from] proj::ProjError),
    
    #[error("Invalid coordinate: lat={lat}, lon={lon}")]
    InvalidCoordinate { lat: f64, lon: f64 },
    
    #[error("Missing timezone in timestamp: {0}")]
    MissingTimezone(String),
    
    #[error("No netelements in network")]
    EmptyNetwork,
    
    #[error("Invalid netelement geometry: {0}")]
    InvalidGeometry(String),
    
    #[error("CSV parsing error: {0}")]
    CsvError(#[from] csv::Error),
    
    #[error("GeoJSON parsing error: {0}")]
    GeoJsonError(#[from] geojson::Error),
    
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
```

---

## Core Functions

### `project_gnss`

**Signature**:
```rust
pub fn project_gnss(
    gnss_positions: &[GnssPosition],
    network: &RailwayNetwork,
    config: ProjectionConfig,
) -> Result<Vec<ProjectedPosition>, ProjectionError>
```

**Purpose**: Project GNSS positions onto railway network netelements.

**Algorithm**:
1. For each GNSS position:
   - Transform CRS if needed (GNSS CRS → Network CRS)
   - Find nearest netelement using spatial index
   - Project point onto netelement geometry
   - Calculate measure along netelement
   - Emit warning if projection distance > threshold
2. Return Vec<ProjectedPosition> (1:1 correspondence with input)

**Invariants**:
- Output length equals input length
- Original GNSS data preserved in ProjectedPosition.original
- Projection distance >= 0.0
- Measure >= 0.0 and <= netelement length

**Example**:
```rust
use tp_lib_core::projection::{project_gnss, build_network, ProjectionConfig};
use tp_lib_core::models::{GnssPosition, Netelement};
use chrono::DateTime;

// Create GNSS positions
let gnss_positions = vec![
    GnssPosition::new(
        50.8503,
        4.3517,
        DateTime::parse_from_rfc3339("2025-12-09T14:30:00+01:00")?,
        "EPSG:4326".to_string(),
    )?,
];

// Create network
let netelements = vec![
    Netelement::new(
        "NE-12345".to_string(),
        LineString::from(vec![(4.3517, 50.8503), (4.3610, 50.8450)]),
        "EPSG:4326".to_string(),
    )?,
];
let network = build_network(netelements)?;

// Project
let config = ProjectionConfig::default();
let results = project_gnss(&gnss_positions, &network, config)?;

// Access results
for result in results {
    println!("Netelement: {}", result.netelement_id);
    println!("Measure: {:.2} m", result.measure_meters);
}
```

---

### `build_network`

**Signature**:
```rust
pub fn build_network(netelements: Vec<Netelement>) -> Result<RailwayNetwork, ProjectionError>
```

**Purpose**: Build railway network with spatial index for efficient queries.

**Algorithm**:
1. Validate netelements (non-empty, unique IDs, valid geometries)
2. Build R-tree spatial index from netelement bounding boxes
3. Return RailwayNetwork with O(log n) nearest-neighbor queries

**Example**:
```rust
use tp_lib_core::projection::build_network;
use tp_lib_core::models::Netelement;
use geo::LineString;

let netelements = vec![
    Netelement::new(
        "NE-1".to_string(),
        LineString::from(vec![(4.0, 50.0), (5.0, 51.0)]),
        "EPSG:4326".to_string(),
    )?,
];

let network = build_network(netelements)?;
```

---

## I/O Utilities

### CSV Parsing

```rust
pub mod io {
    use crate::models::GnssPosition;
    use std::path::Path;
    
    /// Parse GNSS positions from CSV file
    pub fn read_gnss_csv(
        path: &Path,
        lat_col: &str,
        lon_col: &str,
        time_col: &str,
        crs: String,
    ) -> Result<Vec<GnssPosition>, ProjectionError>;
}
```

**Example**:
```rust
use tp_lib_core::io::read_gnss_csv;
use std::path::Path;

let positions = read_gnss_csv(
    Path::new("journey.csv"),
    "latitude",
    "longitude",
    "timestamp",
    "EPSG:4326".to_string(),
)?;
```

---

### GeoJSON Parsing

```rust
pub mod io {
    use crate::models::Netelement;
    use std::path::Path;
    
    /// Parse netelements from GeoJSON file
    pub fn read_network_geojson(
        path: &Path,
    ) -> Result<Vec<Netelement>, ProjectionError>;
}
```

**Example**:
```rust
use tp_lib_core::io::read_network_geojson;
use std::path::Path;

let netelements = read_network_geojson(Path::new("network.geojson"))?;
```

---

### Output Formatting

```rust
pub mod io {
    use crate::models::ProjectedPosition;
    use std::io::Write;
    
    /// Write projected positions to CSV
    pub fn write_csv<W: Write>(
        writer: W,
        positions: &[ProjectedPosition],
    ) -> Result<(), ProjectionError>;
    
    /// Write projected positions to GeoJSON
    pub fn write_geojson<W: Write>(
        writer: W,
        positions: &[ProjectedPosition],
    ) -> Result<(), ProjectionError>;
}
```

**Example**:
```rust
use tp_lib_core::io::write_csv;
use std::fs::File;

let file = File::create("output.csv")?;
write_csv(file, &results)?;
```

---

## CRS Transformation

```rust
pub mod crs {
    use geo::Point;
    use proj::Proj;
    
    pub struct CrsTransformer {
        transform: Proj,
    }
    
    impl CrsTransformer {
        /// Create transformer between two CRS
        pub fn new(source_crs: &str, target_crs: &str) -> Result<Self, ProjectionError>;
        
        /// Transform point from source to target CRS
        pub fn transform(&self, point: Point<f64>) -> Result<Point<f64>, ProjectionError>;
    }
}
```

**Example**:
```rust
use tp_lib_core::crs::CrsTransformer;
use geo::Point;

let transformer = CrsTransformer::new("EPSG:3812", "EPSG:4326")?;
let point_lambert = Point::new(649328.0, 665262.0);
let point_wgs84 = transformer.transform(point_lambert)?;

println!("WGS84: lon={}, lat={}", point_wgs84.x(), point_wgs84.y());
```

---

## Usage Examples

### Example 1: End-to-End Pipeline

```rust
use tp_lib_core::projection::{build_network, project_gnss, ProjectionConfig};
use tp_lib_core::io::{read_gnss_csv, read_network_geojson, write_csv};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read input files
    let gnss_positions = read_gnss_csv(
        Path::new("journey.csv"),
        "latitude",
        "longitude",
        "timestamp",
        "EPSG:31370".to_string(),
    )?;
    
    let netelements = read_network_geojson(Path::new("network.geojson"))?;
    
    // Build network
    let network = build_network(netelements)?;
    
    // Project
    let config = ProjectionConfig {
        warning_threshold: 50.0,
        transform_crs: true,
    };
    let results = project_gnss(&gnss_positions, &network, config)?;
    
    // Write output
    let output = std::fs::File::create("output.csv")?;
    write_csv(output, &results)?;
    
    Ok(())
}
```

---

### Example 2: Custom Processing

```rust
use tp_lib_core::projection::{build_network, ProjectionConfig};
use tp_lib_core::models::{GnssPosition, Netelement};
use geo::{Point, LineString};

fn project_single_position(
    position: &GnssPosition,
    network: &RailwayNetwork,
) -> Option<ProjectedPosition> {
    // Find nearest netelement
    let point = Point::new(position.longitude, position.latitude);
    let netelement = network.find_nearest(&point)?;
    
    // Project onto netelement
    let projected = project_point_onto_linestring(&point, &netelement.geometry);
    
    // Calculate measure
    let measure = calculate_measure(&netelement.geometry, &projected);
    
    Some(ProjectedPosition {
        original: position.clone(),
        projected_coords: projected,
        netelement_id: netelement.id.clone(),
        measure_meters: measure,
        projection_distance_meters: point.euclidean_distance(&projected),
        crs: netelement.crs.clone(),
    })
}
```

---

## Contract Stability

**Version**: 1.0.0  
**Stability**: Stable

**Breaking Changes** (require major version bump):
- Change function signatures
- Remove public types or functions
- Rename public API elements
- Change error types

**Non-Breaking Changes** (minor version bump):
- Add new public functions
- Add new optional fields to structs
- Add new error variants
- Improve performance

**Backward Compatibility**:
- All 1.x.x versions maintain API compatibility with 1.0.0
- Internal implementation may change without API breakage

---

## Testing Contract

### Unit Tests

**Test 1: Basic Projection**
```rust
#[test]
fn test_basic_projection() {
    let position = GnssPosition::new(
        50.0, 4.0,
        DateTime::parse_from_rfc3339("2025-12-09T14:30:00+01:00").unwrap(),
        "EPSG:4326".to_string(),
    ).unwrap();
    
    let netelement = Netelement::new(
        "NE-1".to_string(),
        LineString::from(vec![(4.0, 50.0), (4.1, 50.1)]),
        "EPSG:4326".to_string(),
    ).unwrap();
    
    let network = build_network(vec![netelement]).unwrap();
    let config = ProjectionConfig::default();
    
    let results = project_gnss(&[position], &network, config).unwrap();
    
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].netelement_id, "NE-1");
}
```

**Test 2: Invalid Coordinate**
```rust
#[test]
fn test_invalid_coordinate() {
    let result = GnssPosition::new(
        91.0, // Invalid latitude
        4.0,
        DateTime::parse_from_rfc3339("2025-12-09T14:30:00+01:00").unwrap(),
        "EPSG:4326".to_string(),
    );
    
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ProjectionError::InvalidCoordinate { .. }));
}
```

---

## Next Steps

- **CLI Contract**: See [cli.md](./cli.md) for command-line interface
- **Python API**: See [python-api.md](./python-api.md) for Python bindings
- **Data Model**: See [data-model.md](../data-model.md) for entity definitions
- **Implementation**: See [plan.md](../plan.md) for technical architecture
