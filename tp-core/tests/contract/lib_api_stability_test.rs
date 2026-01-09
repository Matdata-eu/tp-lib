//! Contract tests for API stability
//!
//! These tests verify that the public API signatures haven't changed unexpectedly.
//! They serve as a snapshot of the expected API surface area.

use tp_lib_core::{
    GnssPosition, Netelement, ProjectedPosition, ProjectionConfig, ProjectionError,
    RailwayNetwork,
};

/// Verify GnssPosition struct fields and public API
#[test]
fn test_gnss_position_contract() {
    use chrono::{FixedOffset, TimeZone};
    use std::collections::HashMap;

    // Should be able to construct with all required fields
    let tz = FixedOffset::east_opt(3600).unwrap();
    let dt = tz.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    let pos = GnssPosition {
        latitude: 50.8503,
        longitude: 4.3517,
        timestamp: dt,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };

    // Verify fields are accessible
    assert_eq!(pos.latitude, 50.8503);
    assert_eq!(pos.longitude, 4.3517);
    assert_eq!(pos.crs, "EPSG:4326");
    assert!(pos.metadata.is_empty());
}

/// Verify Netelement struct fields and public API
#[test]
fn test_netelement_contract() {
    use geo::{CoordsIter, LineString};

    let geom = LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]);

    let ne = Netelement {
        id: "NE001".to_string(),
        geometry: geom.clone(),
        crs: "EPSG:4326".to_string(),
    };

    // Verify fields are accessible
    assert_eq!(ne.id, "NE001");
    assert_eq!(ne.crs, "EPSG:4326");
    assert_eq!(ne.geometry.coords_count(), 2);
}

/// Verify ProjectedPosition struct fields and public API
#[test]
fn test_projected_position_contract() {
    use chrono::{FixedOffset, TimeZone};
    use geo::Point;
    use std::collections::HashMap;

    let tz = FixedOffset::east_opt(3600).unwrap();
    let dt = tz.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    let original = GnssPosition {
        latitude: 50.8503,
        longitude: 4.3517,
        timestamp: dt,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };

    let proj = ProjectedPosition {
        original,
        projected_coords: Point::new(4.3517, 50.8503),
        netelement_id: "NE001".to_string(),
        measure_meters: 123.45,
        projection_distance_meters: 2.5,
        crs: "EPSG:4326".to_string(),
    };

    // Verify fields are accessible
    assert_eq!(proj.netelement_id, "NE001");
    assert_eq!(proj.measure_meters, 123.45);
    assert_eq!(proj.projection_distance_meters, 2.5);
    assert_eq!(proj.crs, "EPSG:4326");
}

/// Verify ProjectionConfig struct fields and Default trait
#[test]
fn test_projection_config_contract() {
    // Default values
    let config = ProjectionConfig::default();
    assert_eq!(config.projection_distance_warning_threshold, 50.0);
    assert!(!config.suppress_warnings);

    // Custom values
    let custom = ProjectionConfig {
        projection_distance_warning_threshold: 100.0,
        suppress_warnings: true,
    };
    assert_eq!(custom.projection_distance_warning_threshold, 100.0);
    assert!(custom.suppress_warnings);
}

/// Verify ProjectionError enum variants
#[test]
fn test_projection_error_contract() {
    use std::fmt::Write;

    // Verify all error variants exist and implement Display
    let errors = vec![
        ProjectionError::InvalidCrs("test".to_string()),
        ProjectionError::TransformFailed("test".to_string()),
        ProjectionError::InvalidCoordinate("test".to_string()),
        ProjectionError::MissingTimezone("test".to_string()),
        ProjectionError::InvalidTimestamp("test".to_string()),
        ProjectionError::EmptyNetwork,
        ProjectionError::InvalidGeometry("test".to_string()),
        ProjectionError::GeoJsonError("test".to_string()),
    ];

    for err in errors {
        let mut msg = String::new();
        write!(&mut msg, "{}", err).unwrap();
        assert!(!msg.is_empty(), "Error message should not be empty");
    }
}

/// Verify RailwayNetwork public API
#[test]
fn test_railway_network_contract() {
    use geo::LineString;

    let geom = LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]);
    let ne = Netelement {
        id: "NE001".to_string(),
        geometry: geom,
        crs: "EPSG:4326".to_string(),
    };

    // Should be able to construct from Vec<Netelement>
    let network = RailwayNetwork::new(vec![ne]).unwrap();

    // Verify find_nearest exists and returns Result<usize>
    use geo::Point;
    let point = Point::new(4.355, 50.855);
    let nearest = network.find_nearest(&point);
    assert!(nearest.is_ok());
}

/// Verify main projection function signature
#[test]
fn test_project_gnss_contract() {
    use chrono::{FixedOffset, TimeZone};
    use geo::LineString;
    use std::collections::HashMap;

    let tz = FixedOffset::east_opt(3600).unwrap();
    let dt = tz.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    let pos = GnssPosition {
        latitude: 50.8503,
        longitude: 4.3517,
        timestamp: dt,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };

    let geom = LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]);
    let ne = Netelement {
        id: "NE001".to_string(),
        geometry: geom,
        crs: "EPSG:4326".to_string(),
    };

    let network = RailwayNetwork::new(vec![ne]).unwrap();
    let config = ProjectionConfig::default();

    // Function signature: project_gnss(&[GnssPosition], &RailwayNetwork, &ProjectionConfig) -> Result<Vec<ProjectedPosition>>
    let result = tp_lib_core::project_gnss(&[pos], &network, &config);
    assert!(result.is_ok());
    let projected = result.unwrap();
    assert_eq!(projected.len(), 1);
}

/// Verify I/O functions exist with correct signatures
#[test]
fn test_io_functions_contract() {
    // These functions should exist in the public API
    // We're just checking compilation here - not testing functionality

    // parse_gnss_csv signature
    fn _check_parse_gnss_csv() -> Result<Vec<GnssPosition>, ProjectionError> {
        tp_lib_core::parse_gnss_csv(
            "dummy.csv",
            "EPSG:4326",
            "latitude",
            "longitude",
            "timestamp",
        )
    }

    // parse_gnss_geojson signature
    fn _check_parse_gnss_geojson() -> Result<Vec<GnssPosition>, ProjectionError> {
        tp_lib_core::parse_gnss_geojson("dummy.geojson", "EPSG:4326")
    }

    // parse_network_geojson signature
    fn _check_parse_network_geojson() -> Result<Vec<Netelement>, ProjectionError> {
        tp_lib_core::parse_network_geojson("dummy.geojson")
    }

    // write_csv signature
    fn _check_write_csv<W: std::io::Write>(
        projected: &[ProjectedPosition],
        writer: &mut W,
    ) -> Result<(), ProjectionError> {
        tp_lib_core::write_csv(projected, writer)
    }

    // write_geojson signature
    fn _check_write_geojson<W: std::io::Write>(
        projected: &[ProjectedPosition],
        writer: &mut W,
    ) -> Result<(), ProjectionError> {
        tp_lib_core::write_geojson(projected, writer)
    }
}
