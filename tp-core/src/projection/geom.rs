//! Geometric projection operations

use crate::errors::ProjectionError;
use crate::models::{GnssPosition, ProjectedPosition};
use geo::algorithm::{ClosestPoint, HaversineDistance};
use geo::{LineString, Point};

#[cfg(test)]
use geo::Coord;

/// Project a point onto the nearest location on a LineString
pub fn project_point_onto_linestring(
    point: &Point<f64>,
    linestring: &LineString<f64>,
) -> Result<Point<f64>, ProjectionError> {
    match linestring.closest_point(point) {
        geo::Closest::SinglePoint(projected) | geo::Closest::Intersection(projected) => {
            Ok(projected)
        }
        geo::Closest::Indeterminate => Err(ProjectionError::InvalidGeometry(
            "Could not find closest point on linestring".to_string(),
        )),
    }
}

/// Calculate the distance along a linestring from its start to a given point
///
/// Uses haversine distance to compute geodesic distance in meters between
/// consecutive points along the linestring up to the projected point.
pub fn calculate_measure_along_linestring(
    linestring: &LineString<f64>,
    point: &Point<f64>,
) -> Result<f64, ProjectionError> {
    if linestring.0.is_empty() {
        return Err(ProjectionError::InvalidGeometry(
            "Cannot calculate measure on empty linestring".to_string(),
        ));
    }

    if linestring.0.len() < 2 {
        return Err(ProjectionError::InvalidGeometry(
            "Linestring must have at least 2 points".to_string(),
        ));
    }

    // Accumulate distance from start
    let mut total_distance = 0.0;
    let mut closest_distance = f64::MAX;
    let mut measure_at_closest = 0.0;

    // Iterate through line segments
    for i in 0..linestring.0.len() - 1 {
        let segment_start = Point::from(linestring.0[i]);
        let segment_end = Point::from(linestring.0[i + 1]);

        // Calculate segment length
        let segment_length = segment_start.haversine_distance(&segment_end);

        // Check if point is closest to this segment
        let segment_linestring = LineString::from(vec![linestring.0[i], linestring.0[i + 1]]);
        let closest_on_segment = match segment_linestring.closest_point(point) {
            geo::Closest::SinglePoint(p) | geo::Closest::Intersection(p) => p,
            geo::Closest::Indeterminate => continue,
        };

        let distance_to_segment = point.haversine_distance(&closest_on_segment);

        if distance_to_segment < closest_distance {
            closest_distance = distance_to_segment;
            // Calculate distance from segment start to projected point
            let distance_along_segment = segment_start.haversine_distance(&closest_on_segment);
            measure_at_closest = total_distance + distance_along_segment;
        }

        total_distance += segment_length;
    }

    Ok(measure_at_closest)
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

    // Calculate projection distance using haversine
    let projection_distance = gnss_point.haversine_distance(&projected);

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
    use chrono::{FixedOffset, TimeZone};

    #[test]
    fn test_project_point_on_line() {
        let linestring =
            LineString::from(vec![Coord { x: 0.0, y: 0.0 }, Coord { x: 10.0, y: 0.0 }]);

        let point = Point::new(5.0, 2.0);
        let projected = project_point_onto_linestring(&point, &linestring);

        assert!(projected.is_ok());
        let result = projected.unwrap();
        // Should be projected onto the line at (5.0, 0.0) approximately
        assert!((result.y()).abs() < 0.1);
    }

    #[test]
    fn test_calculate_measure_empty_linestring() {
        let linestring = LineString::<f64>::new(vec![]);
        let point = Point::new(4.0, 50.0);
        let result = calculate_measure_along_linestring(&linestring, &point);
        assert!(result.is_err());
        if let Err(ProjectionError::InvalidGeometry(msg)) = result {
            assert!(msg.contains("empty"));
        }
    }

    #[test]
    fn test_calculate_measure_single_point_linestring() {
        let linestring = LineString::from(vec![Coord { x: 4.0, y: 50.0 }]);
        let point = Point::new(4.0, 50.0);
        let result = calculate_measure_along_linestring(&linestring, &point);
        assert!(result.is_err());
        if let Err(ProjectionError::InvalidGeometry(msg)) = result {
            assert!(msg.contains("at least 2 points"));
        }
    }

    #[test]
    fn test_calculate_measure_at_start() {
        // Simple linestring: (4.0, 50.0) -> (5.0, 50.0)
        let linestring =
            LineString::from(vec![Coord { x: 4.0, y: 50.0 }, Coord { x: 5.0, y: 50.0 }]);
        let point = Point::new(4.0, 50.0);
        let result = calculate_measure_along_linestring(&linestring, &point);
        assert!(result.is_ok());
        let measure = result.unwrap();
        // Should be very close to 0 (at start)
        assert!(
            measure < 10.0,
            "Measure at start should be near 0, got {}",
            measure
        );
    }

    #[test]
    fn test_calculate_measure_at_end() {
        // Simple linestring: (4.0, 50.0) -> (5.0, 50.0)
        let linestring =
            LineString::from(vec![Coord { x: 4.0, y: 50.0 }, Coord { x: 5.0, y: 50.0 }]);
        let point = Point::new(5.0, 50.0);
        let result = calculate_measure_along_linestring(&linestring, &point);
        assert!(result.is_ok());
        let measure = result.unwrap();

        // Calculate expected total length
        let start = Point::new(4.0, 50.0);
        let end = Point::new(5.0, 50.0);
        let expected_length = start.haversine_distance(&end);

        // Measure should be close to total length
        assert!(
            (measure - expected_length).abs() < 10.0,
            "Measure at end should be near {}, got {}",
            expected_length,
            measure
        );
    }

    #[test]
    fn test_calculate_measure_at_middle() {
        // Simple linestring: (4.0, 50.0) -> (4.5, 50.0) -> (5.0, 50.0)
        let linestring = LineString::from(vec![
            Coord { x: 4.0, y: 50.0 },
            Coord { x: 4.5, y: 50.0 },
            Coord { x: 5.0, y: 50.0 },
        ]);
        let point = Point::new(4.5, 50.0);
        let result = calculate_measure_along_linestring(&linestring, &point);
        assert!(result.is_ok());
        let measure = result.unwrap();

        // Calculate expected distance to middle
        let start = Point::new(4.0, 50.0);
        let middle = Point::new(4.5, 50.0);
        let expected_measure = start.haversine_distance(&middle);

        // Measure should be close to distance to middle point
        assert!(
            (measure - expected_measure).abs() < 10.0,
            "Measure at middle should be near {}, got {}",
            expected_measure,
            measure
        );
    }

    #[test]
    fn test_calculate_measure_point_off_line() {
        // Linestring: (4.0, 50.0) -> (5.0, 50.0)
        let linestring =
            LineString::from(vec![Coord { x: 4.0, y: 50.0 }, Coord { x: 5.0, y: 50.0 }]);
        // Point off the line (perpendicular offset)
        let point = Point::new(4.5, 50.1);
        let result = calculate_measure_along_linestring(&linestring, &point);
        assert!(result.is_ok());
        let measure = result.unwrap();

        // Should project to nearest point on line, which is around middle
        let start = Point::new(4.0, 50.0);
        let projected_approx = Point::new(4.5, 50.0);
        let expected_measure = start.haversine_distance(&projected_approx);

        // Measure should be close to distance to projected point
        assert!(
            (measure - expected_measure).abs() < 1000.0,
            "Measure for point off line should be near {}, got {}",
            expected_measure,
            measure
        );
    }

    #[test]
    fn test_project_gnss_position() {
        // Create a simple linestring
        let linestring =
            LineString::from(vec![Coord { x: 4.0, y: 50.0 }, Coord { x: 5.0, y: 50.0 }]);

        // Create a GNSS position near the middle
        let timestamp = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, 1, 1, 12, 0, 0)
            .unwrap();
        let gnss = GnssPosition::new(4.5, 50.01, timestamp, "EPSG:4326".to_string()).unwrap();

        let result = project_gnss_position(
            &gnss,
            "NE123".to_string(),
            &linestring,
            "EPSG:4326".to_string(),
        );

        assert!(result.is_ok());
        let projected = result.unwrap();

        // Verify basic fields
        assert_eq!(projected.netelement_id, "NE123");
        assert_eq!(projected.crs, "EPSG:4326");

        // Verify projection distance is calculated (should be > 0 since point is off line)
        assert!(
            projected.projection_distance_meters > 0.0,
            "Projection distance should be positive, got {}",
            projected.projection_distance_meters
        );

        // Verify measure is reasonable (somewhere along the linestring, not negative)
        assert!(
            projected.measure_meters >= 0.0,
            "Measure should be non-negative, got {}",
            projected.measure_meters
        );

        // Verify measure is not beyond the end of the linestring
        let start = Point::new(4.0, 50.0);
        let end = Point::new(5.0, 50.0);
        let total_length = start.haversine_distance(&end);
        assert!(
            projected.measure_meters <= total_length + 1.0,
            "Measure should not exceed linestring length {}, got {}",
            total_length,
            projected.measure_meters
        );
    }
}
