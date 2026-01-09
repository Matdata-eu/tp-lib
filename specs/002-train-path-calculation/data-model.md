# Data Model: Train Path Calculation

**Feature**: 002-train-path-calculation  
**Date**: January 9, 2026  
**Phase**: 1 - Design & Contracts

- [Data Model: Train Path Calculation](#data-model-train-path-calculation)
  - [Overview](#overview)
  - [1. NetRelation (Network Topology Connection)](#1-netrelation-network-topology-connection)
    - [Purpose](#purpose)
    - [Rust Structure](#rust-structure)
    - [GeoJSON Representation](#geojson-representation)
    - [Validation Rules](#validation-rules)
  - [2. Extended GnssPosition (with Heading and Distance)](#2-extended-gnssposition-with-heading-and-distance)
    - [Purpose](#purpose-1)
    - [Rust Structure Extensions](#rust-structure-extensions)
    - [CSV Representation](#csv-representation)
  - [3. GnssNetElementLink (Candidate Projection)](#3-gnssnetelementlink-candidate-projection)
    - [Purpose](#purpose-2)
    - [Rust Structure](#rust-structure-1)
    - [JSON Representation](#json-representation)
  - [4. AssociatedNetElement (Netelement in Path)](#4-associatednetelement-netelement-in-path)
    - [Purpose](#purpose-3)
    - [Rust Structure](#rust-structure-2)
    - [JSON Representation](#json-representation-1)
  - [5. TrainPath (Complete Path Representation)](#5-trainpath-complete-path-representation)
    - [Purpose](#purpose-4)
    - [Rust Structure](#rust-structure-3)
    - [GeoJSON Representation](#geojson-representation-1)
    - [CSV Representation](#csv-representation-1)
  - [Entity Relationships](#entity-relationships)
  - [Validation Summary](#validation-summary)
  - [Backward Compatibility](#backward-compatibility)
    - [Existing Models (Unchanged)](#existing-models-unchanged)
    - [Extended Models](#extended-models)
    - [New Models](#new-models)

## Overview

This document defines the data models required for train path calculation. Models are designed to integrate with existing tp-core data structures (GnssPosition, Netelement) while adding topology and path representation capabilities.

---

## 1. NetRelation (Network Topology Connection)

### Purpose
Represents a navigability connection between two netelements (track segments). Defines whether trains can travel from one segment to another and in which direction(s).

### Rust Structure

```rust
use serde::{Deserialize, Serialize};

/// Represents a navigability connection between two track segments
///
/// A NetRelation defines whether trains can travel from one netelement to another.
/// Navigability may be unidirectional (e.g., one-way track) or bidirectional.
///
/// # Examples
///
/// ```
/// use tp_lib_core::NetRelation;
///
/// // Bidirectional connection: trains can go from A to B and from B to A
/// let relation = NetRelation {
///     id: "NR001".to_string(),
///     from_netelement_id: "NE_A".to_string(),
///     to_netelement_id: "NE_B".to_string(),
///     navigable_forward: true,   // A → B allowed
///     navigable_backward: true,  // B → A allowed
/// };
///
/// // Unidirectional connection: trains can only go from A to B
/// let relation = NetRelation {
///     id: "NR002".to_string(),
///     from_netelement_id: "NE_A".to_string(),
///     to_netelement_id: "NE_B".to_string(),
///     navigable_forward: true,   // A → B allowed
///     navigable_backward: false, // B → A forbidden
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetRelation {
    /// Unique identifier for this netrelation
    pub id: String,

    /// ID of the source netelement (starting track segment)
    pub from_netelement_id: String,

    /// ID of the target netelement (destination track segment)
    pub to_netelement_id: String,

    /// Position on netelementA where the connection applies (0 = start, 1 = end)
    pub position_on_a: u8,

    /// Position on netelementB where the connection applies (0 = start, 1 = end)
    pub position_on_b: u8,

    /// Whether trains can navigate forward (from → to)
    pub navigable_forward: bool,

    /// Whether trains can navigate backward (to → from)
    pub navigable_backward: bool,
}

impl NetRelation {
    /// Create a new netrelation with validation
    pub fn new(
        id: String,
        from_netelement_id: String,
        to_netelement_id: String,
        position_on_a: u8,
        position_on_b: u8,
        navigable_forward: bool,
        navigable_backward: bool,
    ) -> Result<Self, ProjectionError> {
        let relation = Self {
            id,
            from_netelement_id,
            to_netelement_id,
            position_on_a,
            position_on_b,
            navigable_forward,
            navigable_backward,
        };
        
        relation.validate()?;
        Ok(relation)
    }
    
    /// Validate netrelation fields
    fn validate(&self) -> Result<(), ProjectionError> {
        // ID must be non-empty
        if self.id.is_empty() {
            return Err(ProjectionError::InvalidNetRelation(
                "NetRelation ID must not be empty".to_string(),
            ));
        }
        
        // Netelement IDs must be non-empty
        if self.from_netelement_id.is_empty() {
            return Err(ProjectionError::InvalidNetRelation(
                "from_netelement_id must not be empty".to_string(),
            ));
        }
        
        if self.to_netelement_id.is_empty() {
            return Err(ProjectionError::InvalidNetRelation(
                "to_netelement_id must not be empty".to_string(),
            ));
        }
        
        // Position values must be 0 or 1
        if self.position_on_a > 1 {
            return Err(ProjectionError::InvalidNetRelation(
                format!("position_on_a must be 0 or 1, got {}", self.position_on_a),
            ));
        }
        
        if self.position_on_b > 1 {
            return Err(ProjectionError::InvalidNetRelation(
                format!("position_on_b must be 0 or 1, got {}", self.position_on_b),
            ));
        }
        
        // Cannot connect to itself
        if self.from_netelement_id == self.to_netelement_id {
            return Err(ProjectionError::InvalidNetRelation(
                format!(
                    "NetRelation cannot connect netelement to itself: {}",
                    self.from_netelement_id
                ),
            ));
        }
        
        Ok(())
    }
    
    /// Check if navigation is allowed in forward direction (from → to)
    pub fn is_navigable_forward(&self) -> bool {
        self.navigable_forward
    }
    
    /// Check if navigation is allowed in backward direction (to → from)
    pub fn is_navigable_backward(&self) -> bool {
        self.navigable_backward
    }
    
    /// Check if bidirectional (both directions navigable)
    pub fn is_bidirectional(&self) -> bool {
        self.navigable_forward && self.navigable_backward
    }
}
```

### GeoJSON Representation

NetRelations are stored in the same GeoJSON file as netelements, distinguished by a `type` property:

```json
{
  "type": "FeatureCollection",
  "features": [
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
      "geometry": null
    }
  ]
}
```

**Navigability values**:
- `"both"`: Bidirectional (A ↔ B)
- `"AB"`: Unidirectional A → B only
- `"BA"`: Unidirectional B → A only
- `"none"`: No navigation allowed (physical connection exists but trains cannot pass)

**Geometry**:
- Can be `null` (no spatial representation)
- Can be a `Point` geometry representing the connection point between netelementA and netelementB
- Point geometry is useful for visualizing and managing connections in GIS applications (e.g., QGIS)

**Example with Point geometry**:
```json
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
    "coordinates": [4.3518, 50.8504]
  }
}
```

**Note**: The internal Rust model uses `from_netelement_id`/`to_netelement_id` and boolean flags, with parsing logic converting from the external GeoJSON representation.

### Validation Rules

| Rule | Validation | Error Condition |
|------|------------|-----------------|
| ID uniqueness | `id` must be non-empty | Empty string |
| Netelement IDs | `from_netelement_id` and `to_netelement_id` must be non-empty | Empty string |
| Position values | `position_on_a` and `position_on_b` must be 0 or 1 | Value > 1 |
| No self-connection | `from_netelement_id != to_netelement_id` | Same ID |

**Note**: NetRelations where both directions are non-navigable (`navigability: "none"`) are valid and represent physical connection points where trains cannot pass.

---

## 2. Extended GnssPosition (with Heading and Distance)

### Purpose
Extend the existing `GnssPosition` model to include optional heading (direction of travel) and distance (from odometry) data for improved path calculation accuracy.

### Rust Structure Extensions

```rust
// Existing GnssPosition in models/gnss.rs - ADD these fields:

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnssPosition {
    // Existing fields (unchanged)
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<FixedOffset>,
    pub crs: String,
    pub metadata: HashMap<String, String>,
    
    // NEW: Optional heading in degrees (0-360°, 0 = North, 90 = East)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<f64>,
    
    // NEW: Optional distance from previous position (meters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f64>,
}

impl GnssPosition {
    // Extend existing constructor to accept optional heading and distance
    pub fn with_heading_distance(
        latitude: f64,
        longitude: f64,
        timestamp: DateTime<FixedOffset>,
        crs: String,
        heading: Option<f64>,
        distance: Option<f64>,
    ) -> Result<Self, ProjectionError> {
        let position = Self {
            latitude,
            longitude,
            timestamp,
            crs,
            metadata: HashMap::new(),
            heading,
            distance,
        };
        
        position.validate()?;
        position.validate_heading()?;
        Ok(position)
    }
    
    /// Validate heading if present (must be 0-360°)
    pub fn validate_heading(&self) -> Result<(), ProjectionError> {
        if let Some(heading) = self.heading {
            if !(0.0..=360.0).contains(&heading) {
                return Err(ProjectionError::InvalidGeometry(
                    format!("Heading must be in range [0, 360], got {}", heading),
                ));
            }
        }
        Ok(())
    }
    
    /// Check if two headings are opposite
    /// Returns true if headings are closer to 180° apart than to 0° apart
    /// 
    /// Logic: Compare distance to 180° shift vs normal distance
    /// If shifting by 180° gives smaller circular distance, they're opposite
    pub fn is_opposite_heading(h1: f64, h2: f64) -> bool {
        // Calculate normal circular distance
        let diff_normal = (h1 - h2).abs();
        let dist_normal = diff_normal.min(360.0 - diff_normal);
        
        // Calculate distance when one heading is shifted by 180°
        let diff_shifted = (h1 - h2 - 180.0).abs() % 360.0;
        let dist_shifted = diff_shifted.min(360.0 - diff_shifted);
        
        // If shifted distance is smaller, they're opposite
        dist_shifted < dist_normal
    }
    
    /// Calculate angular difference between two headings
    /// Accounts for circular nature of compass bearings
    /// Accounts for possible opposite headings (180° apart)
    pub fn heading_difference(h1: f64, h2: f64) -> f64 {
        // Check if headings are opposite
        if Self::is_opposite_heading(h1, h2) {
            // Opposite headings: return the small angular deviation from exactly 180°
            let diff_shifted = (h1 - h2 - 180.0).abs() % 360.0;
            diff_shifted.min(360.0 - diff_shifted)
        } else {
            // Not opposite: return normal circular distance
            let diff = (h1 - h2).abs();
            diff.min(360.0 - diff)
        }
    }
}
```

### CSV Representation

```csv
timestamp,latitude,longitude,crs,heading,distance
2026-01-09T10:00:00+01:00,50.8503,4.3517,EPSG:4326,45.3,
2026-01-09T10:00:01+01:00,50.8504,4.3518,EPSG:4326,47.1,12.5
2026-01-09T10:00:02+01:00,50.8505,4.3519,EPSG:4326,46.8,11.9
```

**Notes**:
- `heading` and `distance` columns are optional
- Empty values parsed as `None` (e.g., first row has no distance from previous position)
- If columns are missing entirely, all positions have `None` for those fields

---

## 3. GnssNetElementLink (Candidate Projection)

### Purpose
Represents the link between a single GNSS position and a candidate netelement during path calculation (Phases 1-2). Each GNSS position may have multiple candidate links to different netelements, each with its own probability score. This is an **intermediate calculation model**, not part of the final output.

### Rust Structure

```rust
use geo::{Point, Haversine, Length};

/// Link between a GNSS position and a candidate netelement
///
/// Created during path calculation to evaluate which netelements are potential
/// matches for each GNSS position. Multiple links exist per GNSS position.
///
/// # Examples
///
/// ```
/// use tp_lib_core::GnssNetElementLink;
/// use geo::Point;
///
/// let link = GnssNetElementLink {
///     gnss_index: 5,
///     netelement_id: "NE_A".to_string(),
///     projected_point: Point::new(4.3517, 50.8503),
///     distance_meters: 3.2,
///     intrinsic_coordinate: 0.45,
///     heading_difference: Some(5.3),
///     probability: 0.89,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GnssNetElementLink {
    /// Index of the GNSS position in the input data
    pub gnss_index: usize,
    
    /// ID of the candidate netelement
    pub netelement_id: String,
    
    /// Projected point on the netelement (closest point to GNSS position)
    pub projected_point: Point<f64>,
    
    /// Distance from GNSS position to projected point in meters
    pub distance_meters: f64,
    
    /// Intrinsic coordinate on the netelement (0.0 to 1.0)
    /// 0.0 = start of segment, 1.0 = end of segment
    pub intrinsic_coordinate: f64,
    
    /// Angular difference between GNSS heading and netelement direction (degrees)
    /// None if GNSS position has no heading information
    pub heading_difference: Option<f64>,
    
    /// Probability score for this link (0.0 to 1.0)
    /// Calculated from distance and heading probability
    pub probability: f64,
}

impl GnssNetElementLink {
    /// Create a new GNSS-netelement link with validation
    pub fn new(
        gnss_index: usize,
        netelement_id: String,
        projected_point: Point<f64>,
        distance_meters: f64,
        intrinsic_coordinate: f64,
        heading_difference: Option<f64>,
        probability: f64,
    ) -> Result<Self, ProjectionError> {
        let link = Self {
            gnss_index,
            netelement_id,
            projected_point,
            distance_meters,
            intrinsic_coordinate,
            heading_difference,
            probability,
        };
        
        link.validate()?;
        Ok(link)
    }
    
    /// Validate link fields
    fn validate(&self) -> Result<(), ProjectionError> {
        // Netelement ID must be non-empty
        if self.netelement_id.is_empty() {
            return Err(ProjectionError::InvalidGeometry(
                "GnssNetElementLink netelement_id must not be empty".to_string(),
            ));
        }
        
        // Distance must be non-negative
        if self.distance_meters < 0.0 {
            return Err(ProjectionError::InvalidGeometry(
                format!("distance_meters must be non-negative, got {}", self.distance_meters),
            ));
        }
        
        // Intrinsic coordinate must be in [0, 1]
        if !(0.0..=1.0).contains(&self.intrinsic_coordinate) {
            return Err(ProjectionError::InvalidGeometry(
                format!("intrinsic_coordinate must be in [0, 1], got {}", self.intrinsic_coordinate),
            ));
        }
        
        // Heading difference must be in [0, 180] if present
        if let Some(heading_diff) = self.heading_difference {
            if !(0.0..=180.0).contains(&heading_diff) {
                return Err(ProjectionError::InvalidGeometry(
                    format!("heading_difference must be in [0, 180], got {}", heading_diff),
                ));
            }
        }
        
        // Probability must be in [0, 1]
        if !(0.0..=1.0).contains(&self.probability) {
            return Err(ProjectionError::InvalidGeometry(
                format!("Probability must be in [0, 1], got {}", self.probability),
            ));
        }
        
        Ok(())
    }
    
    /// Check if this is a high-probability candidate (>= threshold)
    pub fn is_high_probability(&self, threshold: f64) -> bool {
        self.probability >= threshold
    }
    
    /// Check if distance is within acceptable range
    pub fn is_within_distance(&self, max_distance_meters: f64) -> bool {
        self.distance_meters <= max_distance_meters
    }
}
```

### JSON Representation

```json
{
  "gnss_index": 5,
  "netelement_id": "NE_A",
  "projected_point": {
    "type": "Point",
    "coordinates": [4.3517, 50.8503]
  },
  "distance_meters": 3.2,
  "intrinsic_coordinate": 0.45,
  "heading_difference": 5.3,
  "probability": 0.89
}
```

**Usage in Path Calculation**:
- Phase 1: Create links for all GNSS positions and nearby netelements
- Phase 2: Filter links by probability and distance cutoffs
- Phase 3: Aggregate links to construct candidate paths
- Phase 4: Select best path and convert to AssociatedNetElements

---

## 4. AssociatedNetElement (Netelement in Path)

### Purpose
Represents a track segment (netelement) as part of a calculated train path, including probability score, projection details, and the range of GNSS positions associated with this segment.

### Rust Structure

```rust
/// Represents a netelement within a calculated train path
///
/// Contains the netelement ID, probability score, and projection details for
/// GNSS positions associated with this segment in the path.
///
/// # Examples
///
/// ```
/// use tp_lib_core::AssociatedNetElement;
///
/// let segment = AssociatedNetElement {
///     netelement_id: "NE_A".to_string(),
///     probability: 0.87,
///     start_intrinsic: 0.25,
///     end_intrinsic: 0.78,
///     gnss_start_index: 5,
///     gnss_end_index: 12,
/// };
///
/// // This segment spans from 25% to 78% along netelement NE_A
/// // and is associated with GNSS positions 5-12 in the input data
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssociatedNetElement {
    /// ID of the netelement (track segment)
    pub netelement_id: String,
    
    /// Aggregate probability score for this segment in the path (0.0 to 1.0)
    /// Calculated from distance/heading probability and coverage correction
    pub probability: f64,
    
    /// Intrinsic coordinate where the path enters this segment (0.0 to 1.0)
    /// 0.0 = start of segment, 1.0 = end of segment
    pub start_intrinsic: f64,
    
    /// Intrinsic coordinate where the path exits this segment (0.0 to 1.0)
    pub end_intrinsic: f64,
    
    /// Index of the first GNSS position associated with this segment
    pub gnss_start_index: usize,
    
    /// Index of the last GNSS position associated with this segment
    pub gnss_end_index: usize,
}

impl AssociatedNetElement {
    /// Create a new associated netelement with validation
    pub fn new(
        netelement_id: String,
        probability: f64,
        start_intrinsic: f64,
        end_intrinsic: f64,
        gnss_start_index: usize,
        gnss_end_index: usize,
    ) -> Result<Self, ProjectionError> {
        let element = Self {
            netelement_id,
            probability,
            start_intrinsic,
            end_intrinsic,
            gnss_start_index,
            gnss_end_index,
        };
        
        element.validate()?;
        Ok(element)
    }
    
    /// Validate associated netelement fields
    fn validate(&self) -> Result<(), ProjectionError> {
        // Netelement ID must be non-empty
        if self.netelement_id.is_empty() {
            return Err(ProjectionError::InvalidGeometry(
                "AssociatedNetElement netelement_id must not be empty".to_string(),
            ));
        }
        
        // Probability must be in [0, 1]
        if !(0.0..=1.0).contains(&self.probability) {
            return Err(ProjectionError::InvalidGeometry(
                format!("Probability must be in [0, 1], got {}", self.probability),
            ));
        }
        
        // Intrinsic coordinates must be in [0, 1]
        if !(0.0..=1.0).contains(&self.start_intrinsic) {
            return Err(ProjectionError::InvalidGeometry(
                format!("start_intrinsic must be in [0, 1], got {}", self.start_intrinsic),
            ));
        }
        
        if !(0.0..=1.0).contains(&self.end_intrinsic) {
            return Err(ProjectionError::InvalidGeometry(
                format!("end_intrinsic must be in [0, 1], got {}", self.end_intrinsic),
            ));
        }
        
        // Start index must be <= end index
        if self.gnss_start_index > self.gnss_end_index {
            return Err(ProjectionError::InvalidGeometry(
                format!(
                    "gnss_start_index ({}) must be <= gnss_end_index ({})",
                    self.gnss_start_index, self.gnss_end_index
                ),
            ));
        }
        
        Ok(())
    }
    
    /// Calculate length of path segment as fraction of total netelement
    pub fn fractional_length(&self) -> f64 {
        (self.end_intrinsic - self.start_intrinsic).abs()
    }
    
    /// Calculate the fractional coverage of this segment (0.0 to 1.0)
    /// Same as fractional_length, representing what portion of the netelement is covered
    pub fn fractional_coverage(&self) -> f64 {
        self.fractional_length()
    }
    
    /// Get the length of the associated netelement in meters
    /// Requires the actual NetElement to calculate the geometry length
    pub fn netelement_length(&self) -> Result<f64, ProjectionError> {
        // first get the netelement from some data source (not shown here)
        let netelement = get_netelement_by_id(&self.netelement_id)?;

        netelement.geometry.length::<Haversine>()
    }
}
```

### JSON Representation

```json
{
  "netelement_id": "NE_A",
  "probability": 0.87,
  "start_intrinsic": 0.25,
  "end_intrinsic": 0.78,
  "gnss_start_index": 5,
  "gnss_end_index": 12
}
```

---

## 5. TrainPath (Complete Path Representation)

### Purpose
Represents a calculated continuous path through the rail network, consisting of an ordered sequence of associated netelements with metadata about the path calculation.

### Rust Structure

```rust
use chrono::{DateTime, Utc};

/// Represents a continuous train path through the rail network
///
/// A TrainPath is an ordered sequence of netelements (track segments) that
/// the train traversed, calculated from GNSS coordinates and network topology.
///
/// # Examples
///
/// ```
/// use tp_lib_core::{TrainPath, AssociatedNetElement};
/// use chrono::Utc;
///
/// let segments = vec![
///     AssociatedNetElement::new(
///         "NE_A".to_string(), 0.87, 0.0, 1.0, 0, 10
///     ).unwrap(),
///     AssociatedNetElement::new(
///         "NE_B".to_string(), 0.92, 0.0, 1.0, 11, 18
///     ).unwrap(),
/// ];
///
/// let path = TrainPath::new(
///     segments,
///     0.89,
///     Some(Utc::now()),
///     None,
/// ).unwrap();
///
/// assert_eq!(path.segments.len(), 2);
/// assert_eq!(path.overall_probability, 0.89);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainPath {
    /// Ordered sequence of netelements in the path
    /// Order represents the direction of travel from first to last GNSS position
    pub segments: Vec<AssociatedNetElement>,
    
    /// Overall probability score for this path (0.0 to 1.0)
    /// Calculated as length-weighted average of segment probabilities,
    /// averaged between forward and backward path calculations
    pub overall_probability: f64,
    
    /// Timestamp when this path was calculated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calculated_at: Option<DateTime<Utc>>,
    
    /// Algorithm configuration metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PathMetadata>,
}

/// Algorithm configuration and diagnostic metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMetadata {
    /// Distance scale parameter used for probability calculation
    pub distance_scale: f64,
    
    /// Heading scale parameter used for probability calculation
    pub heading_scale: f64,
    
    /// Cutoff distance for candidate selection (meters)
    pub cutoff_distance: f64,
    
    /// Heading difference cutoff (degrees)
    pub heading_cutoff: f64,
    
    /// Probability threshold for path segment inclusion
    pub probability_threshold: f64,
    
    /// Resampling distance applied (meters), None if disabled
    pub resampling_distance: Option<f64>,
    
    /// Whether fallback mode was used
    pub fallback_mode: bool,
    
    /// Number of candidate paths evaluated
    pub candidate_paths_evaluated: usize,
    
    /// Whether path existed in both directions (bidirectional validation)
    pub bidirectional_path: bool,
}

impl TrainPath {
    /// Create a new train path with validation
    pub fn new(
        segments: Vec<AssociatedNetElement>,
        overall_probability: f64,
        calculated_at: Option<DateTime<Utc>>,
        metadata: Option<PathMetadata>,
    ) -> Result<Self, ProjectionError> {
        let path = Self {
            segments,
            overall_probability,
            calculated_at,
            metadata,
        };
        
        path.validate()?;
        Ok(path)
    }
    
    /// Validate train path
    fn validate(&self) -> Result<(), ProjectionError> {
        // Must have at least one segment
        if self.segments.is_empty() {
            return Err(ProjectionError::PathCalculationFailed {
                reason: "TrainPath must have at least one segment".to_string(),
            });
        }
        
        // Overall probability must be in [0, 1]
        if !(0.0..=1.0).contains(&self.overall_probability) {
            return Err(ProjectionError::InvalidGeometry(
                format!(
                    "overall_probability must be in [0, 1], got {}",
                    self.overall_probability
                ),
            ));
        }
        
        // Validate segment continuity (GNSS indices should be continuous or overlapping)
        for i in 0..self.segments.len() - 1 {
            let current = &self.segments[i];
            let next = &self.segments[i + 1];
            
            // Next segment should start at or after current segment's last position
            if next.gnss_start_index < current.gnss_start_index {
                return Err(ProjectionError::PathCalculationFailed {
                    reason: format!(
                        "Segment GNSS indices not continuous: segment {} ends at {}, segment {} starts at {}",
                        i, current.gnss_end_index, i + 1, next.gnss_start_index
                    ),
                });
            }
        }
        
        Ok(())
    }
    
    /// Calculate total path length (sum of fractional lengths)
    pub fn total_fractional_length(&self) -> f64 {
        self.segments
            .iter()
            .map(|s| s.fractional_length())
            .sum()
    }
    
    /// Get netelement IDs in traversal order
    pub fn netelement_ids(&self) -> Vec<&str> {
        self.segments
            .iter()
            .map(|s| s.netelement_id.as_str())
            .collect()
    }
    
    /// Total number of GNSS positions in path
    pub fn total_gnss_positions(&self) -> usize {
        if self.segments.is_empty() {
            return 0;
        }
        
        let first = &self.segments[0];
        let last = &self.segments[self.segments.len() - 1];
        
        last.gnss_end_index - first.gnss_start_index + 1
    }
}
```

### GeoJSON Representation

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
      "resampling_distance": 10.0,
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
        "netelement_id": "NE_A",
        "probability": 0.87,
        "start_intrinsic": 0.0,
        "end_intrinsic": 1.0,
        "gnss_start_index": 0,
        "gnss_end_index": 10
      },
      "geometry": null
    },
    {
      "type": "Feature",
      "properties": {
        "type": "associated_netelement",
        "netelement_id": "NE_B",
        "probability": 0.92,
        "start_intrinsic": 0.0,
        "end_intrinsic": 0.65,
        "gnss_start_index": 11,
        "gnss_end_index": 18
      },
      "geometry": null
    }
  ]
}
```

### CSV Representation

Simplified tabular format:

```csv
sequence,netelement_id,probability,start_intrinsic,end_intrinsic,gnss_start_index,gnss_end_index
1,NE_A,0.87,0.0,1.0,0,10
2,NE_B,0.92,0.0,0.65,11,18
```

With metadata in separate file or header comments.

---

## Entity Relationships

```
┌─────────────────┐
│  GnssPosition   │
│  (extended)     │
│─────────────────│
│ + heading       │
│ + distance      │
└────────┬────────┘
         │
         │ 1:N (creates during calculation)
         │
         ▼
┌──────────────────────┐
│ GnssNetElementLink   │
│ (intermediate)       │
│──────────────────────│
│ + gnss_index         │
│ + netelement_id      │
│ + projected_point    │
│ + distance_meters    │
│ + intrinsic_coord    │
│ + heading_difference │
│ + probability        │
└──────────┬───────────┘
           │
           │ N:1 (aggregated into)
           │
           ▼
┌──────────────────────┐       ┌─────────────────┐
│ AssociatedNetElement │──────►│   Netelement    │
│ (final path)         │       │   (existing)    │
│──────────────────────│       │─────────────────│
│ + netelement_id      │       │ + geometry      │
│ + probability        │       └─────────────────┘
│ + start_intrinsic    │
│ + end_intrinsic      │◄──┐
│ + gnss_start_index   │   │
│ + gnss_end_index     │   │
└──────────────────────┘   │
         │                 │
         │ N:1 (part of)   │
         │                 │
         ▼                 │
┌──────────────────────┐   │
│     TrainPath        │   │
│──────────────────────│   │
│ + segments           ├───┘
│ + overall_probability│
│ + calculated_at      │
│ + metadata           │
└──────────────────────┘
         │
         │ 1:1
         │
         ▼
┌──────────────────────┐       ┌─────────────────┐
│    PathMetadata      │       │   NetRelation   │
│──────────────────────│       │─────────────────│
│ + algorithm_params   │       │ + from/to       │
│ + diagnostic_info    │       │ + navigable_*   │
└──────────────────────┘       └─────────────────┘
                                        │
                                        │ (defines topology)
                                        │
                                        ▼
                               ┌─────────────────┐
                               │   Netelement    │
                               │   (existing)    │
                               └─────────────────┘
```

**Key Relationships**:

1. **GnssPosition → GnssNetElementLink** (1:N): Each GNSS position is evaluated against multiple candidate netelements during Phase 1-2
2. **GnssNetElementLink → AssociatedNetElement** (N:1): Links are aggregated to form path segments during Phase 3-4
3. **AssociatedNetElement → Netelement** (N:1): Each path segment references one netelement from the network
4. **AssociatedNetElement → TrainPath** (N:1): Multiple segments form one complete path
5. **TrainPath → PathMetadata** (1:1): Each path has diagnostic and configuration metadata
6. **NetRelation → Netelement** (N:2): Defines allowed transitions between netelements for path construction

---

## Validation Summary

| Model | Key Validations |
|-------|-----------------|
| `NetRelation` | Non-empty IDs, no self-connection, navigability can be "none" |
| `GnssPosition` (extended) | Heading in [0, 360°], existing lat/lon/timestamp validations |
| `GnssNetElementLink` | Non-empty ID, distance ≥ 0, intrinsic in [0, 1], heading_diff in [0, 180°], probability in [0, 1] |
| `AssociatedNetElement` | Non-empty ID, probability in [0, 1], intrinsics in [0, 1], start_index ≤ end_index |
| `TrainPath` | Non-empty segments, probability in [0, 1], continuous GNSS indices |

---

## Backward Compatibility

### Existing Models (Unchanged)
- `Netelement`: No changes required
- `ProjectedPosition`: No changes required (may extend in future to include path context)

### Extended Models
- `GnssPosition`: New optional fields (`heading`, `distance`) are backward-compatible
  - Existing code using `GnssPosition` continues to work
  - New fields only populated when data is available
  - Serialization skips `None` values via `#[serde(skip_serializing_if = "Option::is_none")]`

### New Models
- `NetRelation`: New model, no breaking changes to existing code
- `AssociatedNetElement`: New model for path representation
- `TrainPath`: New model for path representation

---

**Phase 1 (Data Model) Complete** | Next: Contracts definition
