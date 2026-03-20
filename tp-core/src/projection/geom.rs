//! Geometric projection operations

use crate::errors::ProjectionError;
use crate::models::{GnssPosition, ProjectedPosition};
use geo::algorithm::HaversineDistance;
use geo::{LineString, Point};

#[cfg(test)]
use geo::Coord;

/// Project a point onto the nearest location on a LineString.
///
/// Uses an equirectangular approximation (cos(lat) scaling on longitude) so
/// that the closest-point computation is metrically correct for geographic
/// (WGS 84) coordinates.
pub fn project_point_onto_linestring(
    point: &Point<f64>,
    linestring: &LineString<f64>,
) -> Result<Point<f64>, ProjectionError> {
    let coords = &linestring.0;
    if coords.len() < 2 {
        return Err(ProjectionError::InvalidGeometry(
            "Linestring must have at least 2 points for projection".to_string(),
        ));
    }

    let cos_lat = point.y().to_radians().cos();
    let mut min_dist_sq = f64::INFINITY;
    let mut best = coords[0];

    for i in 0..coords.len() - 1 {
        let p1 = &coords[i];
        let p2 = &coords[i + 1];

        // Work in a locally-scaled coordinate frame where distances
        // approximate true metric distances (up to a constant factor).
        let dx = (p2.x - p1.x) * cos_lat;
        let dy = p2.y - p1.y;
        let len_sq = dx * dx + dy * dy;

        let t = if len_sq > 0.0 {
            let px = (point.x() - p1.x) * cos_lat;
            let py = point.y() - p1.y;
            ((px * dx + py * dy) / len_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Interpolate back in degree space.
        let proj_x = p1.x + t * (p2.x - p1.x);
        let proj_y = p1.y + t * (p2.y - p1.y);

        let ex = (point.x() - proj_x) * cos_lat;
        let ey = point.y() - proj_y;
        let dist_sq = ex * ex + ey * ey;

        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
            best = geo::Coord {
                x: proj_x,
                y: proj_y,
            };
        }
    }

    Ok(Point::from(best))
}

/// Calculate the distance along a linestring from its start to a given point.
///
/// Locates the segment closest to `point` using an equirectangular
/// approximation (cos(lat) scaling), then accumulates haversine distances
/// up to that segment plus the fractional part within it.
pub fn calculate_measure_along_linestring(
    linestring: &LineString<f64>,
    point: &Point<f64>,
) -> Result<f64, ProjectionError> {
    let coords = &linestring.0;
    if coords.is_empty() {
        return Err(ProjectionError::InvalidGeometry(
            "Cannot calculate measure on empty linestring".to_string(),
        ));
    }
    if coords.len() < 2 {
        return Err(ProjectionError::InvalidGeometry(
            "Linestring must have at least 2 points".to_string(),
        ));
    }

    // Find the segment the point projects onto (same metric as
    // project_point_onto_linestring).
    let cos_lat = point.y().to_radians().cos();
    let mut min_dist_sq = f64::INFINITY;
    let mut best_seg: usize = 0;
    let mut best_t: f64 = 0.0;

    for i in 0..coords.len() - 1 {
        let p1 = &coords[i];
        let p2 = &coords[i + 1];

        let dx = (p2.x - p1.x) * cos_lat;
        let dy = p2.y - p1.y;
        let len_sq = dx * dx + dy * dy;

        let t = if len_sq > 0.0 {
            let px = (point.x() - p1.x) * cos_lat;
            let py = point.y() - p1.y;
            ((px * dx + py * dy) / len_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let proj_x = p1.x + t * (p2.x - p1.x);
        let proj_y = p1.y + t * (p2.y - p1.y);
        let ex = (point.x() - proj_x) * cos_lat;
        let ey = point.y() - proj_y;
        let dist_sq = ex * ex + ey * ey;

        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
            best_seg = i;
            best_t = t;
        }
    }

    // Accumulate haversine distances for complete segments before best_seg.
    let mut measure = 0.0;
    for i in 0..best_seg {
        let a = Point::new(coords[i].x, coords[i].y);
        let b = Point::new(coords[i + 1].x, coords[i + 1].y);
        measure += a.haversine_distance(&b);
    }

    // Add fractional distance within the best segment.
    let seg_start = Point::new(coords[best_seg].x, coords[best_seg].y);
    let seg_end = Point::new(coords[best_seg + 1].x, coords[best_seg + 1].y);
    measure += best_t * seg_start.haversine_distance(&seg_end);

    Ok(measure)
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
