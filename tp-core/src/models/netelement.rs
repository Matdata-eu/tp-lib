//! Netelement (railway track segment) data model

use geo::{LineString, Coord};
use serde::{Deserialize, Serialize};
use crate::errors::ProjectionError;

/// Represents a railway track segment (netelement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Netelement {
    /// Unique identifier for the netelement
    pub id: String,
    
    /// LineString geometry representing the track centerline
    pub geometry: LineString<f64>,
    
    /// Coordinate Reference System (e.g., "EPSG:4326" for WGS84)
    pub crs: String,
}

impl Netelement {
    /// Create a new netelement with validation
    pub fn new(id: String, geometry: LineString<f64>, crs: String) -> Result<Self, ProjectionError> {
        let netelement = Self {
            id,
            geometry,
            crs,
        };
        
        netelement.validate()?;
        Ok(netelement)
    }
    
    /// Validate netelement ID is non-empty
    pub fn validate_id(&self) -> Result<(), ProjectionError> {
        if self.id.is_empty() {
            return Err(ProjectionError::InvalidGeometry(
                "Netelement ID must not be empty".to_string()
            ));
        }
        Ok(())
    }
    
    /// Validate geometry has at least 2 points
    pub fn validate_geometry(&self) -> Result<(), ProjectionError> {
        let count = self.geometry.coords().count();
        if count < 2 {
            return Err(ProjectionError::InvalidGeometry(
                format!("LineString must have at least 2 points, got {}", count)
            ));
        }
        Ok(())
    }
    
    /// Validate all fields
    fn validate(&self) -> Result<(), ProjectionError> {
        self.validate_id()?;
        self.validate_geometry()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_netelement() {
        let coords = vec![
            Coord { x: 4.0, y: 50.0 },
            Coord { x: 4.0, y: 51.0 },
        ];
        let linestring = LineString::from(coords);
        
        let netelement = Netelement::new(
            "NE001".to_string(),
            linestring,
            "EPSG:4326".to_string(),
        );
        
        assert!(netelement.is_ok());
    }

    #[test]
    fn test_empty_id() {
        let coords = vec![
            Coord { x: 4.0, y: 50.0 },
            Coord { x: 4.0, y: 51.0 },
        ];
        let linestring = LineString::from(coords);
        
        let netelement = Netelement::new(
            "".to_string(),  // Invalid
            linestring,
            "EPSG:4326".to_string(),
        );
        
        assert!(netelement.is_err());
    }

    #[test]
    fn test_invalid_geometry() {
        let coords = vec![
            Coord { x: 4.0, y: 50.0 },  // Only 1 point
        ];
        let linestring = LineString::from(coords);
        
        let netelement = Netelement::new(
            "NE001".to_string(),
            linestring,
            "EPSG:4326".to_string(),
        );
        
        assert!(netelement.is_err());
    }
}
