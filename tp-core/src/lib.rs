//! TP-Core: Train Positioning Library - Core Engine
//! 
//! This library provides geospatial projection of GNSS positions onto railway track netelements.

pub mod models;
pub mod projection;
pub mod io;
pub mod crs;
pub mod temporal;
pub mod errors;

// Re-export main types for convenience
pub use models::{GnssPosition, Netelement, ProjectedPosition};
pub use errors::ProjectionError;
pub use io::{parse_gnss_csv, parse_network_geojson};

/// Result type alias using ProjectionError
pub type Result<T> = std::result::Result<T, ProjectionError>;

use projection::spatial::{NetworkIndex, find_nearest_netelement};
use projection::geom::project_gnss_position;
use geo::Point;

/// Configuration for GNSS projection operations
#[derive(Debug, Clone)]
pub struct ProjectionConfig {
    /// Threshold distance in meters for emitting warnings about large projection distances
    pub projection_distance_warning_threshold: f64,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            projection_distance_warning_threshold: 50.0,
        }
    }
}

/// Railway network with spatial indexing for efficient projection
pub struct RailwayNetwork {
    index: NetworkIndex,
}

impl RailwayNetwork {
    /// Create a new railway network from netelements
    pub fn new(netelements: Vec<Netelement>) -> Result<Self> {
        let index = NetworkIndex::new(netelements)?;
        Ok(Self { index })
    }
    
    /// Find the nearest netelement to a given point
    pub fn find_nearest(&self, point: &Point<f64>) -> Result<usize> {
        find_nearest_netelement(point, &self.index)
    }
    
    /// Get netelement by index
    pub fn get_by_index(&self, index: usize) -> Option<&Netelement> {
        self.index.netelements().get(index)
    }
    
    /// Get all netelements
    pub fn netelements(&self) -> &[Netelement] {
        self.index.netelements()
    }
}

/// Project GNSS positions onto railway network
pub fn project_gnss(
    positions: &[GnssPosition],
    network: &RailwayNetwork,
    config: &ProjectionConfig,
) -> Result<Vec<ProjectedPosition>> {
    let mut results = Vec::with_capacity(positions.len());
    
    for gnss in positions {
        // Create point from GNSS position
        let gnss_point = Point::new(gnss.longitude, gnss.latitude);
        
        // Find nearest netelement
        let netelement_idx = network.find_nearest(&gnss_point)?;
        let netelement = network.get_by_index(netelement_idx)
            .ok_or_else(|| ProjectionError::InvalidGeometry(
                format!("Netelement index {} out of bounds", netelement_idx)
            ))?;
        
        // Project onto netelement
        let projected = project_gnss_position(
            gnss,
            netelement.id.clone(),
            &netelement.geometry,
            netelement.crs.clone(),
        )?;
        
        // Emit warning if projection distance exceeds threshold
        if projected.projection_distance_meters > config.projection_distance_warning_threshold {
            eprintln!(
                "WARNING: Large projection distance ({:.2}m > {:.2}m threshold) for position at {:?}",
                projected.projection_distance_meters,
                config.projection_distance_warning_threshold,
                gnss.timestamp
            );
        }
        
        results.push(projected);
    }
    
    Ok(results)
}
