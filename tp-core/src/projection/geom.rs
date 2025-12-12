//! Geometric projection operations

use geo::{Point, LineString, Coord};
use geo::algorithm::ClosestPoint;
use crate::models::{ProjectedPosition, GnssPosition};
use crate::errors::ProjectionError;

/// Project a point onto the nearest location on a LineString
pub fn project_point_onto_linestring(
    point: &Point<f64>,
    linestring: &LineString<f64>,
) -> Result<Point<f64>, ProjectionError> {
    match linestring.closest_point(point) {
        geo::Closest::SinglePoint(projected) | geo::Closest::Intersection(projected) => {
            Ok(projected)
        }
        geo::Closest::Indeterminate => {
            Err(ProjectionError::InvalidGeometry(
                "Could not find closest point on linestring".to_string()
            ))
        }
    }
}

/// Calculate the distance along a linestring from its start to a given point
pub fn calculate_measure_along_linestring(
    linestring: &LineString<f64>,
    point: &Point<f64>,
) -> Result<f64, ProjectionError> {
    // TODO: Implement proper measure calculation using geodesic distance
    // For now, return stub value
    Ok(0.0)
}

/// Project a GNSS position onto a netelement
pub fn project_gnss_position(
    gnss: &GnssPosition,
    netelement_id: String,
    linestring: &LineString<f64>,
    crs: String,
) -> Result<ProjectedPosition, ProjectionError> {
    // Convert GNSS position to Point
    let gnss_point = Point::new(gnss.longitude, gnss.latitude);
    
    // Project onto linestring
    let projected = project_point_onto_linestring(&gnss_point, linestring)?;
    
    // Calculate measure
    let measure = calculate_measure_along_linestring(linestring, &projected)?;
    
    // Calculate projection distance
    let projection_distance = 0.0; // TODO: Calculate actual distance using haversine
    
    Ok(ProjectedPosition::new(
        gnss.clone(),
        projected,
        netelement_id,
        measure,
        projection_distance,
        crs,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_point_on_line() {
        let linestring = LineString::from(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 10.0, y: 0.0 },
        ]);
        
        let point = Point::new(5.0, 2.0);
        let projected = project_point_onto_linestring(&point, &linestring);
        
        assert!(projected.is_ok());
        let result = projected.unwrap();
        // Should be projected onto the line at (5.0, 0.0) approximately
        assert!((result.y()).abs() < 0.1);
    }
}
