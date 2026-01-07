//! Unit tests for CRS transformation functionality
//!
//! Note: These tests require the `crs-transform` feature to be enabled.
//! Without it, CrsTransformer uses identity transformation.

use geo::Point;
use tp_core::crs::CrsTransformer;

#[test]
fn test_identity_transform() {
    // WGS84 to WGS84 should be identity
    let transformer = CrsTransformer::new("EPSG:4326".to_string(), "EPSG:4326".to_string())
        .expect("Failed to create transformer");

    let point = Point::new(4.3517, 50.8503); // Brussels
    let transformed = transformer
        .transform(point)
        .expect("Failed to transform point");

    // Should be essentially the same (within floating point precision)
    assert!((transformed.x() - point.x()).abs() < 1e-6);
    assert!((transformed.y() - point.y()).abs() < 1e-6);
}

#[test]
#[cfg(feature = "crs-transform")]
fn test_wgs84_to_belgian_lambert_72() {
    // Transform from WGS84 to Belgian Lambert 72 (EPSG:31370)
    let transformer = CrsTransformer::new("EPSG:4326".to_string(), "EPSG:31370".to_string())
        .expect("Failed to create transformer");

    // Brussels Central Station in WGS84
    let wgs84 = Point::new(4.3571, 50.8458);

    let lambert = transformer
        .transform(wgs84)
        .expect("Failed to transform to Lambert 72");

    // Expected Lambert 72 coordinates (approximate)
    // X (Easting): ~148,000 - 150,000
    // Y (Northing): ~169,000 - 171,000
    assert!(
        lambert.x() > 148_000.0 && lambert.x() < 150_000.0,
        "Lambert X coordinate out of expected range: {}",
        lambert.x()
    );
    assert!(
        lambert.y() > 169_000.0 && lambert.y() < 171_000.0,
        "Lambert Y coordinate out of expected range: {}",
        lambert.y()
    );
}

#[test]
#[cfg(feature = "crs-transform")]
fn test_wgs84_to_belgian_lambert_2008() {
    // Transform from WGS84 to Belgian Lambert 2008 (EPSG:3812)
    let transformer = CrsTransformer::new("EPSG:4326".to_string(), "EPSG:3812".to_string())
        .expect("Failed to create transformer");

    // Brussels Central Station in WGS84
    let wgs84 = Point::new(4.3571, 50.8458);

    let lambert = transformer
        .transform(wgs84)
        .expect("Failed to transform to Lambert 2008");

    // Expected Lambert 2008 coordinates (approximate)
    // X (Easting): ~648,000 - 650,000
    // Y (Northing): ~668,000 - 671,000
    assert!(
        lambert.x() > 648_000.0 && lambert.x() < 650_000.0,
        "Lambert 2008 X coordinate out of expected range: {}",
        lambert.x()
    );
    assert!(
        lambert.y() > 668_000.0 && lambert.y() < 671_000.0,
        "Lambert 2008 Y coordinate out of expected range: {}",
        lambert.y()
    );
}

#[test]
#[cfg(feature = "crs-transform")]
fn test_belgian_lambert_72_to_wgs84() {
    // Transform from Belgian Lambert 72 to WGS84
    let transformer = CrsTransformer::new("EPSG:31370".to_string(), "EPSG:4326".to_string())
        .expect("Failed to create transformer");

    // Brussels Central Station in Lambert 72 (approximate)
    let lambert = Point::new(148_500.0, 169_500.0);

    let wgs84 = transformer
        .transform(lambert)
        .expect("Failed to transform to WGS84");

    // Should be close to Brussels area (lon ~4.3, lat ~50.8)
    assert!(
        (wgs84.x() - 4.35).abs() < 0.02,
        "WGS84 longitude out of expected range: {}",
        wgs84.x()
    );
    assert!(
        (wgs84.y() - 50.84).abs() < 0.02,
        "WGS84 latitude out of expected range: {}",
        wgs84.y()
    );
}

#[test]
#[cfg(feature = "crs-transform")]
fn test_belgian_lambert_2008_to_wgs84() {
    // Transform from Belgian Lambert 2008 to WGS84
    let transformer = CrsTransformer::new("EPSG:3812".to_string(), "EPSG:4326".to_string())
        .expect("Failed to create transformer");

    // Brussels Central Station in Lambert 2008 (approximate)
    let lambert = Point::new(648_500.0, 668_500.0);

    let wgs84 = transformer
        .transform(lambert)
        .expect("Failed to transform to WGS84");

    // Should be close to Brussels area (lon ~4.3, lat ~50.8)
    assert!(
        (wgs84.x() - 4.35).abs() < 0.02,
        "WGS84 longitude out of expected range: {}",
        wgs84.x()
    );
    assert!(
        (wgs84.y() - 50.83).abs() < 0.02,
        "WGS84 latitude out of expected range: {}",
        wgs84.y()
    );
}

#[test]
#[cfg(feature = "crs-transform")]
fn test_round_trip_transformation() {
    // WGS84 -> Lambert 72 -> WGS84 should be close to identity
    let to_lambert = CrsTransformer::new("EPSG:4326".to_string(), "EPSG:31370".to_string())
        .expect("Failed to create to_lambert transformer");
    let to_wgs84 = CrsTransformer::new("EPSG:31370".to_string(), "EPSG:4326".to_string())
        .expect("Failed to create to_wgs84 transformer");

    let original = Point::new(4.3517, 50.8503); // Brussels

    let lambert = to_lambert
        .transform(original)
        .expect("Failed to transform to Lambert");
    let round_trip = to_wgs84
        .transform(lambert)
        .expect("Failed to transform back to WGS84");

    // Should be very close to original (within 1 meter in WGS84 degrees ~ 0.00001)
    assert!(
        (round_trip.x() - original.x()).abs() < 0.00001,
        "Round trip longitude differs: {} vs {}",
        round_trip.x(),
        original.x()
    );
    assert!(
        (round_trip.y() - original.y()).abs() < 0.00001,
        "Round trip latitude differs: {} vs {}",
        round_trip.y(),
        original.y()
    );
}

#[test]
#[cfg(feature = "crs-transform")]
fn test_invalid_source_crs() {
    // Invalid CRS should return error
    let result = CrsTransformer::new("INVALID:9999".to_string(), "EPSG:4326".to_string());
    assert!(result.is_err());
}

#[test]
#[cfg(feature = "crs-transform")]
fn test_invalid_target_crs() {
    // Invalid target CRS should return error
    let result = CrsTransformer::new("EPSG:4326".to_string(), "INVALID:9999".to_string());
    assert!(result.is_err());
}

#[test]
#[cfg(feature = "crs-transform")]
fn test_transform_multiple_points() {
    // Test transforming multiple points along Brussels-Antwerp railway line
    let transformer = CrsTransformer::new("EPSG:4326".to_string(), "EPSG:31370".to_string())
        .expect("Failed to create transformer");

    let points = vec![
        Point::new(4.3517, 50.8503), // Brussels
        Point::new(4.4042, 51.2194), // Antwerp
        Point::new(4.3777, 51.0353), // Mechelen (halfway)
    ];

    for point in points {
        let transformed = transformer
            .transform(point)
            .expect("Failed to transform point");

        // All should be within Belgian Lambert 72 bounds
        // X: ~0 - 300,000
        // Y: ~0 - 300,000
        assert!(
            transformed.x() > 0.0 && transformed.x() < 300_000.0,
            "Lambert X out of Belgian bounds: {}",
            transformed.x()
        );
        assert!(
            transformed.y() > 0.0 && transformed.y() < 300_000.0,
            "Lambert Y out of Belgian bounds: {}",
            transformed.y()
        );
    }
}

#[test]
#[cfg(feature = "crs-transform")]
fn test_transform_preserves_point_ordering() {
    // Verify that relative positions are preserved after transformation
    let transformer = CrsTransformer::new("EPSG:4326".to_string(), "EPSG:31370".to_string())
        .expect("Failed to create transformer");

    let point1 = Point::new(4.3517, 50.8503); // Brussels (south)
    let point2 = Point::new(4.4042, 51.2194); // Antwerp (north)

    let lambert1 = transformer
        .transform(point1)
        .expect("Failed to transform point1");
    let lambert2 = transformer
        .transform(point2)
        .expect("Failed to transform point2");

    // Antwerp is north of Brussels, so Y should be greater
    assert!(
        lambert2.y() > lambert1.y(),
        "Point ordering not preserved: {} vs {}",
        lambert2.y(),
        lambert1.y()
    );
}
