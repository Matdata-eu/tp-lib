# Data Model: GNSS Track Axis Projection

**Feature**: 001-gnss-projection | **Date**: 2025-12-12

## Overview

This document defines the core data structures for projecting GNSS positions onto railway track netelements. The model separates input data (GNSS positions, railway network), processing state (spatial index), and output (projected positions).

---

## Core Entities

### 1. GnssPosition

**Purpose**: Represents a single raw GNSS measurement from a train journey.

**Rust Definition**:
```rust
use chrono::{DateTime, FixedOffset};

pub struct GnssPosition {
    /// Latitude in decimal degrees
    pub latitude: f64,
    
    /// Longitude in decimal degrees
    pub longitude: f64,
    
    /// Timestamp with timezone offset (e.g., 2025-12-09T14:30:00+01:00)
    pub timestamp: DateTime<FixedOffset>,
    
    /// Coordinate Reference System (e.g., "EPSG:4326" for WGS84)
    pub crs: String,
    
    /// Additional metadata from CSV (preserved for output)
    pub metadata: HashMap<String, String>,
}
```

**Validation Rules**:
- `latitude`: -90.0 ≤ lat ≤ 90.0
- `longitude`: -180.0 ≤ lon ≤ 180.0
- `timestamp`: Must include timezone (no naive datetime)
- `crs`: Must be valid EPSG code (e.g., "EPSG:4326")
- `metadata`: Optional, preserves extra CSV columns

**Example**:
```rust
GnssPosition {
    latitude: 50.8503,
    longitude: 4.3517,
    timestamp: DateTime::parse_from_rfc3339("2025-12-09T14:30:00+01:00").unwrap(),
    crs: "EPSG:4326".to_string(),
    metadata: [("train_id", "123")].into(),
}
```

---

### 2. Netelement

**Purpose**: Represents a railway track segment (netelement) from the network topology.

**Rust Definition**:
```rust
use geo::LineString;

pub struct Netelement {
    /// Unique identifier from GeoJSON properties (e.g., "NE-12345")
    pub id: String,
    
    /// Track centerline geometry (sequence of (lon, lat) points)
    pub geometry: LineString<f64>,
    
    /// Coordinate Reference System (GeoJSON RFC 7946 mandates WGS84)
    pub crs: String,
}
```

**Constraints**:
- `id`: Non-empty, unique within network
- `geometry`: ≥2 points, valid LineString (no self-intersections for MVP)
- `crs`: Must be "EPSG:4326" (WGS84) per GeoJSON RFC 7946

**Example**:
```rust
Netelement {
    id: "NE-12345".to_string(),
    geometry: LineString::from(vec![
        (4.3517, 50.8503), // Brussels Central
        (4.3610, 50.8450), // Next point south
    ]),
    crs: "EPSG:4326".to_string(),
}
```

---

### 3. ProjectedPosition

**Purpose**: Result of projecting a GNSS position onto the railway network. Enriches original data with track-aligned coordinates and metadata.

**Rust Definition**:
```rust
use geo::Point;

pub struct ProjectedPosition {
    /// Original GNSS position (preserved per FR-013)
    pub original: GnssPosition,
    
    /// Projected coordinates on netelement (lon, lat)
    pub projected_coords: Point<f64>,
    
    /// ID of netelement where position was projected
    pub netelement_id: String,
    
    /// Distance along netelement from start in meters
    pub measure_meters: f64,
    
    /// Perpendicular distance from GNSS to projected point in meters
    /// (diagnostic metadata per FR-019)
    pub projection_distance_meters: f64,
    
    /// Coordinate Reference System of projected_coords
    pub crs: String,
}
```

**Invariants**:
- `original`: Never modified from input
- `projected_coords`: Lies on `netelement_id` geometry
- `measure_meters`: 0.0 ≤ measure ≤ length(netelement geometry)
- `projection_distance_meters`: ≥0.0, triggers warning if >threshold (default 50m)
- `crs`: Matches network CRS (typically "EPSG:4326")

**Example**:
```rust
ProjectedPosition {
    original: GnssPosition { /* ... */ },
    projected_coords: Point::new(4.3518, 50.8504),
    netelement_id: "NE-12345".to_string(),
    measure_meters: 1234.56,
    projection_distance_meters: 2.3,
    crs: "EPSG:4326".to_string(),
}
```

---

### 4. RailwayNetwork

**Purpose**: Container for all netelements with spatial indexing for efficient queries.

**Rust Definition**:
```rust
use rstar::RTree;

pub struct RailwayNetwork {
    /// All netelements in the network
    pub netelements: Vec<Netelement>,
    
    /// Spatial index for O(log n) nearest-neighbor queries
    spatial_index: RTree<NetelementIndexEntry>,
}

/// Internal: Spatial index entry wrapping netelement
struct NetelementIndexEntry {
    netelement_id: String,
    bounding_box: geo::Rect<f64>,
}

impl RailwayNetwork {
    /// Build spatial index from netelements
    pub fn new(netelements: Vec<Netelement>) -> Self {
        // Build R-tree from netelement bounding boxes
        // ...
    }
    
    /// Find nearest netelement to given point (FR-009)
    pub fn find_nearest(&self, point: &Point<f64>) -> Option<&Netelement> {
        // Use R-tree nearest-neighbor query
        // ...
    }
}
```

**Operations**:
- `new(netelements)`: Build spatial index
- `find_nearest(point)`: O(log n) nearest netelement query
- `get_by_id(id)`: O(1) lookup by netelement ID

---

## Entity Relationships

```text
┌─────────────────┐
│  GnssPosition   │
│                 │
│  - latitude     │
│  - longitude    │
│  - timestamp    │
│  - crs          │
│  - metadata     │
└────────┬────────┘
         │
         │ 1:1 (preserved)
         ▼
┌─────────────────────────┐
│  ProjectedPosition      │
│                         │
│  - original (GnssPos)   │
│  - projected_coords     │
│  - netelement_id        │────┐
│  - measure_meters       │    │
│  - projection_distance  │    │ references
│  - crs                  │    │
└─────────────────────────┘    │
                               │
                               │
                               ▼
                    ┌───────────────────┐
                    │   Netelement      │
                    │                   │
                    │   - id (PK)       │
                    │   - geometry      │
                    │   - crs           │
                    └─────────┬─────────┘
                              │
                              │ 1:N (container)
                              ▼
                    ┌───────────────────┐
                    │ RailwayNetwork    │
                    │                   │
                    │ - netelements[]   │
                    │ - spatial_index   │
                    └───────────────────┘
```

**Cardinality**:
- 1 GnssPosition → 1 ProjectedPosition (FR-012: 1:1 correspondence)
- 1 ProjectedPosition → 1 Netelement (FR-009: nearest assignment)
- 1 RailwayNetwork → N Netelements (container relationship)

---

## Workflow Data Flow

### Input Phase
```text
CSV File             GeoJSON File
   │                      │
   ▼                      ▼
[CSV Parser]       [GeoJSON Parser]
   │                      │
   ▼                      ▼
Vec<GnssPosition>   Vec<Netelement>
   │                      │
   │                      ▼
   │              [Build Spatial Index]
   │                      │
   │                      ▼
   │              RailwayNetwork
   │                      │
   └──────────────────────┘
```

### Processing Phase
```text
for each GnssPosition:
    │
    ├─→ [CRS Transform to Network CRS] (if needed)
    │
    ├─→ [Find Nearest Netelement] (spatial index query)
    │
    ├─→ [Project onto Netelement Geometry]
    │
    ├─→ [Calculate Measure Along Track]
    │
    └─→ [Create ProjectedPosition]
        
Result: Vec<ProjectedPosition>
```

### Output Phase
```text
Vec<ProjectedPosition>
   │
   ├─→ [Format as CSV] → stdout
   │
   └─→ [Format as JSON] → stdout
```

---

## CRS Transformation

**Challenge**: GNSS positions may use different CRS than railway network.

**Example Scenario**:
- GNSS CSV: Belgian Lambert 2008 (EPSG:3812)
- Railway Network GeoJSON: WGS84 (EPSG:4326) per RFC 7946

**Solution**: Transform GNSS coordinates to network CRS before projection.

```rust
use proj::Proj;

pub struct CrsTransformer {
    transform: Proj,
}

impl CrsTransformer {
    pub fn new(source_crs: &str, target_crs: &str) -> Result<Self, ProjectionError> {
        let transform = Proj::new_known_crs(source_crs, target_crs, None)
            .map_err(|e| ProjectionError::InvalidCrs(e))?;
        Ok(Self { transform })
    }
    
    pub fn transform(&self, lon: f64, lat: f64) -> Result<(f64, f64), ProjectionError> {
        self.transform.convert((lon, lat))
            .map_err(|e| ProjectionError::TransformFailed(e))
    }
}
```

**Workflow with CRS Transform**:
```rust
// 1. Parse GNSS data (Belgian Lambert)
let gnss_positions: Vec<GnssPosition> = parse_csv("journey.csv", "EPSG:3812")?;

// 2. Parse network (WGS84)
let network: RailwayNetwork = parse_geojson("network.geojson")?;

// 3. Create transformer
let transformer = CrsTransformer::new("EPSG:3812", "EPSG:4326")?;

// 4. Project with transformation
let results: Vec<ProjectedPosition> = gnss_positions.iter().map(|pos| {
    // Transform GNSS coords to network CRS
    let (lon_wgs84, lat_wgs84) = transformer.transform(pos.longitude, pos.latitude)?;
    let point_wgs84 = Point::new(lon_wgs84, lat_wgs84);
    
    // Find nearest netelement
    let netelement = network.find_nearest(&point_wgs84)?;
    
    // Project and calculate measure
    // ...
}).collect::<Result<Vec<_>, _>>()?;
```

---

## Memory Representation

### Arrow Columnar Format (Performance Optimization)

**Goal**: Minimize memory allocations, enable SIMD operations.

**Schema for GNSS Positions**:
```rust
use arrow::datatypes::{Schema, Field, DataType};

fn gnss_schema() -> Schema {
    Schema::new(vec![
        Field::new("latitude", DataType::Float64, false),
        Field::new("longitude", DataType::Float64, false),
        Field::new("timestamp", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC")), false),
        Field::new("crs", DataType::Utf8, false),
    ])
}
```

**Schema for Projected Positions**:
```rust
fn projected_schema() -> Schema {
    Schema::new(vec![
        // Original GNSS fields
        Field::new("original_lat", DataType::Float64, false),
        Field::new("original_lon", DataType::Float64, false),
        Field::new("original_time", DataType::Timestamp(TimeUnit::Millisecond, Some("UTC")), false),
        
        // Projection results
        Field::new("projected_lat", DataType::Float64, false),
        Field::new("projected_lon", DataType::Float64, false),
        Field::new("netelement_id", DataType::Utf8, false),
        Field::new("measure_meters", DataType::Float64, false),
        Field::new("projection_distance_meters", DataType::Float64, false),
        Field::new("crs", DataType::Utf8, false),
    ])
}
```

**Trade-off**: Arrow columnar format optimizes batch operations but adds serialization overhead. Use for large datasets (>1000 positions), direct structs for small datasets.

---

## Validation Rules

### Input Validation (Fail-Fast per Constitution VIII)

**GnssPosition**:
- ✅ Latitude in [-90, 90]
- ✅ Longitude in [-180, 180]
- ✅ Timestamp includes timezone
- ✅ CRS is valid EPSG code

**Netelement**:
- ✅ ID is non-empty
- ✅ Geometry has ≥2 points
- ✅ Geometry forms valid LineString

**RailwayNetwork**:
- ✅ At least 1 netelement
- ✅ All netelement IDs unique

### Output Invariants

**ProjectedPosition**:
- ✅ `projected_coords` lies on `netelement_id` geometry (within numerical tolerance)
- ✅ `measure_meters` ∈ [0, length(netelement)]
- ✅ `projection_distance_meters` ≥ 0
- ✅ `original` unchanged from input

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("Invalid CRS: {0}")]
    InvalidCrs(String),
    
    #[error("CRS transformation failed: {0}")]
    TransformFailed(proj::ProjError),
    
    #[error("Invalid coordinate: lat={lat}, lon={lon}")]
    InvalidCoordinate { lat: f64, lon: f64 },
    
    #[error("Missing timezone in timestamp: {0}")]
    MissingTimezone(String),
    
    #[error("No netelements in network")]
    EmptyNetwork,
    
    #[error("Invalid netelement geometry: {0}")]
    InvalidGeometry(String),
}
```

---

## Performance Considerations

### Spatial Index Performance

| Operation | Without R-tree | With R-tree | Speedup |
|-----------|----------------|-------------|---------|
| Find nearest (1000 points × 50 netelements) | O(50,000) | O(1000 × log 50) | ~900× |
| Find nearest (10,000 × 500) | O(5,000,000) | O(10,000 × log 500) | ~5,500× |

**Conclusion**: R-tree mandatory for datasets >1000 positions or >100 netelements.

### Memory Usage

| Dataset Size | Struct-based | Arrow Columnar | Delta |
|--------------|--------------|----------------|-------|
| 1,000 positions | ~200 KB | ~150 KB | -25% |
| 10,000 positions | ~2 MB | ~1.2 MB | -40% |

**Conclusion**: Arrow format reduces memory for large datasets, enables zero-copy Python integration.

---

## Next Steps

1. ✅ Data model defined
2. → Create API contracts (`contracts/cli.md`, `contracts/lib-api.md`, `contracts/python-api.md`)
3. → Write user guide (`quickstart.md`)
4. → Generate implementation tasks (`/speckit.tasks`)
5. → Implement basic projection test (FIRST TEST per Constitution IV)
