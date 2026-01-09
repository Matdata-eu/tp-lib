//! Projected position result data model

use geo::Point;
use serde::{Deserialize, Serialize};
use crate::models::GnssPosition;

/// Represents a GNSS position projected onto a railway netelement
///
/// A `ProjectedPosition` is the result of projecting a GNSS measurement onto the
/// nearest railway track segment. It preserves the original GNSS data and adds:
///
/// - Projected coordinates on the track centerline
/// - Measure (distance along track from netelement start)
/// - Projection distance (perpendicular distance from original to projected point)
/// - Netelement assignment
///
/// # Use Cases
///
/// - Calculate train progress along tracks
/// - Analyze position accuracy and quality
/// - Detect track deviations or sensor errors
/// - Generate linear referencing for asset management
///
/// # Examples
///
/// ```rust,no_run
/// use tp_lib_core::{parse_gnss_csv, parse_network_geojson, RailwayNetwork};
/// use tp_lib_core::{project_gnss, ProjectionConfig};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load and project data
/// let netelements = parse_network_geojson("network.geojson")?;
/// let network = RailwayNetwork::new(netelements)?;
/// let positions = parse_gnss_csv("gnss.csv", "EPSG:4326", "latitude", "longitude", "timestamp")?;
/// 
/// let config = ProjectionConfig::default();
/// let projected = project_gnss(&positions, &network, &config)?;
///
/// // Analyze results
/// for pos in projected {
///     println!("Track position: {}m on {}", pos.measure_meters, pos.netelement_id);
///     println!("Projection accuracy: {:.2}m", pos.projection_distance_meters);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectedPosition {
    /// Original GNSS measurement (preserved)
    pub original: GnssPosition,
    
    /// Projected coordinates on the track axis
    pub projected_coords: Point<f64>,
    
    /// ID of the netelement this position was projected onto
    pub netelement_id: String,
    
    /// Distance along the netelement from start (in meters)
    pub measure_meters: f64,
    
    /// Distance between original GNSS position and projected position (in meters)
    pub projection_distance_meters: f64,
    
    /// Coordinate Reference System of the projected coordinates
    pub crs: String,
}

impl ProjectedPosition {
    /// Create a new projected position
    pub fn new(
        original: GnssPosition,
        projected_coords: Point<f64>,
        netelement_id: String,
        measure_meters: f64,
        projection_distance_meters: f64,
        crs: String,
    ) -> Self {
        Self {
            original,
            projected_coords,
            netelement_id,
            measure_meters,
            projection_distance_meters,
            crs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, FixedOffset};
    use std::collections::HashMap;

    #[test]
    fn test_projected_position_creation() {
        let timestamp = FixedOffset::east_opt(3600)
            .unwrap()
            .with_ymd_and_hms(2025, 12, 9, 14, 30, 0)
            .unwrap();
        
        let original = GnssPosition {
            latitude: 50.8503,
            longitude: 4.3517,
            timestamp,
            crs: "EPSG:4326".to_string(),
            metadata: HashMap::new(),
            heading: None,
            distance: None,
        };
        
        let projected = ProjectedPosition::new(
            original.clone(),
            Point::new(4.3517, 50.8503),
            "NE001".to_string(),
            100.5,
            2.3,
            "EPSG:4326".to_string(),
        );
        
        assert_eq!(projected.netelement_id, "NE001");
        assert_eq!(projected.measure_meters, 100.5);
        assert_eq!(projected.projection_distance_meters, 2.3);
    }
}
