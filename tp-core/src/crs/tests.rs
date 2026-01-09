//! Unit tests for CRS transformation

use super::*;
use crate::errors::ProjectionError;
use geo::Point;

#[test]
fn test_wgs84_to_lambert72() {
    let transformer = CrsTransformer::new("EPSG:4326".to_string(), "EPSG:31370".to_string())
        .expect("Failed to create transformer");

    // Brussels coordinates: 50.8503°N, 4.3517°E
    let wgs84_point = Point::new(4.3517, 50.8503);
    let lambert72_point = transformer
        .transform(wgs84_point)
        .expect("Transformation failed");

    // Expected Lambert72 coordinates for Brussels (approximate)
    assert!(
        (lambert72_point.x() - 149_445.0).abs() < 1000.0,
        "X coordinate out of range: {}",
        lambert72_point.x()
    );
    assert!(
        (lambert72_point.y() - 170_154.0).abs() < 1000.0,
        "Y coordinate out of range: {}",
        lambert72_point.y()
    );
}

#[test]
fn test_lambert72_to_wgs84() {
    let transformer = CrsTransformer::new("EPSG:31370".to_string(), "EPSG:4326".to_string())
        .expect("Failed to create transformer");

    // Lambert72 coordinates for Brussels
    let lambert_point = Point::new(149_445.0, 170_154.0);
    let wgs84_point = transformer
        .transform(lambert_point)
        .expect("Transformation failed");

    // Should get back approximately Brussels coordinates
    assert!(
        (wgs84_point.x() - 4.3517).abs() < 0.01,
        "Longitude out of range: {}",
        wgs84_point.x()
    );
    assert!(
        (wgs84_point.y() - 50.8503).abs() < 0.01,
        "Latitude out of range: {}",
        wgs84_point.y()
    );
}

#[test]
fn test_invalid_epsg_code() {
    let result = CrsTransformer::new("EPSG:99999".to_string(), "EPSG:4326".to_string());
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(
            matches!(e, ProjectionError::InvalidCrs(_)),
            "Expected InvalidCrs error"
        );
    }
}

#[test]
fn test_invalid_proj_string() {
    let result = CrsTransformer::new("invalid proj string".to_string(), "EPSG:4326".to_string());
    assert!(result.is_err());
}

#[test]
fn test_epsg_without_prefix() {
    let transformer = CrsTransformer::new("4326".to_string(), "31370".to_string())
        .expect("Should handle EPSG codes without prefix");

    let point = Point::new(4.3517, 50.8503);
    let result = transformer.transform(point);
    assert!(result.is_ok());
}

#[test]
fn test_same_crs_transformation() {
    let transformer = CrsTransformer::new("EPSG:4326".to_string(), "EPSG:4326".to_string())
        .expect("Failed to create transformer");

    let original = Point::new(4.3517, 50.8503);
    let transformed = transformer
        .transform(original)
        .expect("Transformation failed");

    assert!(
        (transformed.x() - original.x()).abs() < 0.000001,
        "X coordinate changed"
    );
    assert!(
        (transformed.y() - original.y()).abs() < 0.000001,
        "Y coordinate changed"
    );
}

#[test]
fn test_roundtrip_transformation() {
    let to_lambert = CrsTransformer::new("EPSG:4326".to_string(), "EPSG:31370".to_string())
        .expect("Failed to create transformer");
    let to_wgs84 = CrsTransformer::new("EPSG:31370".to_string(), "EPSG:4326".to_string())
        .expect("Failed to create transformer");

    let original = Point::new(5.0, 51.0);
    let lambert = to_lambert
        .transform(original)
        .expect("First transformation failed");
    let back_to_wgs84 = to_wgs84
        .transform(lambert)
        .expect("Second transformation failed");

    assert!(
        (back_to_wgs84.x() - original.x()).abs() < 0.0001,
        "Roundtrip X mismatch: {} vs {}",
        back_to_wgs84.x(),
        original.x()
    );
    assert!(
        (back_to_wgs84.y() - original.y()).abs() < 0.0001,
        "Roundtrip Y mismatch: {} vs {}",
        back_to_wgs84.y(),
        original.y()
    );
}

#[test]
fn test_projected_to_projected() {
    // UTM Zone 31N to Lambert72 (both projected)
    let transformer = CrsTransformer::new("EPSG:32631".to_string(), "EPSG:31370".to_string())
        .expect("Failed to create transformer");

    let utm_point = Point::new(500000.0, 5600000.0);
    let result = transformer.transform(utm_point);
    assert!(result.is_ok());
}

#[test]
fn test_geographic_to_geographic() {
    // WGS84 to NAD83 (both geographic)
    let transformer = CrsTransformer::new("EPSG:4326".to_string(), "EPSG:4269".to_string())
        .expect("Failed to create transformer");

    let wgs84_point = Point::new(-75.0, 40.0);
    let nad83_point = transformer
        .transform(wgs84_point)
        .expect("Transformation failed");

    // Should be very close since WGS84 and NAD83 are nearly identical
    assert!(
        (nad83_point.x() - wgs84_point.x()).abs() < 0.01,
        "Unexpected coordinate difference"
    );
}
