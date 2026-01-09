//! Basic projection test - FIRST TEST to validate environment setup
//!
//! This test uses hardcoded Point and LineString (no file I/O) to verify:
//! - geo crate is correctly installed and linked
//! - Test framework runs correctly
//! - Basic projection math works

use geo::algorithm::ClosestPoint;
use geo::algorithm::HaversineDistance;
use geo::{Coord, LineString, Point};

#[test]
fn test_project_point_on_linestring() {
    // Create a simple linestring from (50.0, 4.0) to (51.0, 4.0)
    let linestring = LineString::from(vec![
        Coord { x: 4.0, y: 50.0 }, // lon, lat
        Coord { x: 4.0, y: 51.0 },
    ]);

    // Create a point near the middle of the line
    let point = Point::new(4.0, 50.5);

    // Find the closest point on the linestring
    let closest = linestring.closest_point(&point);

    // Verify we got a valid result
    match closest {
        geo::Closest::SinglePoint(projected) | geo::Closest::Intersection(projected) => {
            // Should be very close to the point since it's on the line
            let distance = point.haversine_distance(&projected);
            assert!(distance < 1.0, "Point should be very close to linestring");

            // The projected point should be on the line
            assert!(
                (projected.x() - 4.0_f64).abs() < 0.0001,
                "Projected longitude should be 4.0"
            );
            assert!(
                (projected.y() - 50.5_f64).abs() < 0.1,
                "Projected latitude should be around 50.5"
            );
        }
        geo::Closest::Indeterminate => {
            panic!("Could not find closest point");
        }
    }
}

#[test]
fn test_linestring_creation() {
    // Verify basic LineString creation works
    let coords = vec![Coord { x: 4.0, y: 50.0 }, Coord { x: 4.0, y: 51.0 }];
    let linestring = LineString::from(coords);

    assert_eq!(linestring.coords().count(), 2);
}

#[test]
fn test_point_creation() {
    // Verify basic Point creation works
    let point = Point::new(4.0, 50.0);

    assert_eq!(point.x(), 4.0);
    assert_eq!(point.y(), 50.0);
}
