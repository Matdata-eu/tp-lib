//! Unit tests for lib.rs public API

use super::*;
use geo::LineString;

#[test]
fn test_railway_network_creation() {
    let netelements = vec![Netelement {
        id: "NE001".to_string(),
        geometry: LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
        crs: "EPSG:4326".to_string(),
    }];

    let network = RailwayNetwork::new(netelements);
    assert!(network.is_ok());
    let network = network.unwrap();
    assert_eq!(network.netelement_count(), 1);
}

#[test]
fn test_railway_network_empty_netelements() {
    let netelements: Vec<Netelement> = vec![];
    let network = RailwayNetwork::new(netelements);
    assert!(network.is_err());
}

#[test]
fn test_railway_network_find_nearest() {
    let netelements = vec![
        Netelement {
            id: "NE001".to_string(),
            geometry: LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
            crs: "EPSG:4326".to_string(),
        },
        Netelement {
            id: "NE002".to_string(),
            geometry: LineString::from(vec![(4.37, 50.87), (4.38, 50.88)]),
            crs: "EPSG:4326".to_string(),
        },
    ];

    let network = RailwayNetwork::new(netelements).unwrap();
    let point = Point::new(4.355, 50.855);
    let result = network.find_nearest(&point);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0); // Should find NE001
}

#[test]
fn test_railway_network_get_by_index() {
    let netelements = vec![Netelement {
        id: "NE001".to_string(),
        geometry: LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
        crs: "EPSG:4326".to_string(),
    }];

    let network = RailwayNetwork::new(netelements).unwrap();
    let ne = network.get_by_index(0);
    assert!(ne.is_some());
    assert_eq!(ne.unwrap().id, "NE001");
}

#[test]
fn test_railway_network_get_by_invalid_index() {
    let netelements = vec![Netelement {
        id: "NE001".to_string(),
        geometry: LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
        crs: "EPSG:4326".to_string(),
    }];

    let network = RailwayNetwork::new(netelements).unwrap();
    let ne = network.get_by_index(999);
    assert!(ne.is_none());
}

#[test]
fn test_railway_network_netelements() {
    let netelements = vec![
        Netelement {
            id: "NE001".to_string(),
            geometry: LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
            crs: "EPSG:4326".to_string(),
        },
        Netelement {
            id: "NE002".to_string(),
            geometry: LineString::from(vec![(4.37, 50.87), (4.38, 50.88)]),
            crs: "EPSG:4326".to_string(),
        },
    ];

    let network = RailwayNetwork::new(netelements).unwrap();
    let all = network.netelements();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].id, "NE001");
    assert_eq!(all[1].id, "NE002");
}

#[test]
fn test_projection_config_default() {
    let config = ProjectionConfig::default();
    assert_eq!(config.projection_distance_warning_threshold, 50.0);
    assert!(!config.suppress_warnings);
}

#[test]
fn test_projection_config_custom() {
    let config = ProjectionConfig {
        projection_distance_warning_threshold: 100.0,
        suppress_warnings: true,
    };
    assert_eq!(config.projection_distance_warning_threshold, 100.0);
    assert!(config.suppress_warnings);
}

#[test]
fn test_project_gnss_basic() {
    use chrono::DateTime;

    let netelements = vec![Netelement {
        id: "NE001".to_string(),
        geometry: LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
        crs: "EPSG:4326".to_string(),
    }];

    let network = RailwayNetwork::new(netelements).unwrap();

    let positions = vec![GnssPosition {
        latitude: 50.855,
        longitude: 4.355,
        timestamp: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z").unwrap(),
        crs: "EPSG:4326".to_string(),
        heading: None,
        distance: None,
        metadata: std::collections::HashMap::new(),
    }];

    let config = ProjectionConfig {
        projection_distance_warning_threshold: 1000.0,
        suppress_warnings: true,
    };

    let result = project_gnss(&positions, &network, &config);
    assert!(result.is_ok());
    let projected = result.unwrap();
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0].netelement_id, "NE001");
}

#[test]
fn test_project_gnss_empty_positions() {
    let netelements = vec![Netelement {
        id: "NE001".to_string(),
        geometry: LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
        crs: "EPSG:4326".to_string(),
    }];

    let network = RailwayNetwork::new(netelements).unwrap();
    let positions: Vec<GnssPosition> = vec![];
    let config = ProjectionConfig::default();

    let result = project_gnss(&positions, &network, &config);
    assert!(result.is_ok());
    let projected = result.unwrap();
    assert_eq!(projected.len(), 0);
}

#[test]
fn test_project_gnss_multiple_positions() {
    use chrono::DateTime;

    let netelements = vec![Netelement {
        id: "NE001".to_string(),
        geometry: LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
        crs: "EPSG:4326".to_string(),
    }];

    let network = RailwayNetwork::new(netelements).unwrap();

    let positions = vec![
        GnssPosition {
            latitude: 50.851,
            longitude: 4.351,
            timestamp: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z").unwrap(),
            crs: "EPSG:4326".to_string(),
            heading: None,
            distance: None,
            metadata: std::collections::HashMap::new(),
        },
        GnssPosition {
            latitude: 50.859,
            longitude: 4.359,
            timestamp: DateTime::parse_from_rfc3339("2024-01-15T10:30:01Z").unwrap(),
            crs: "EPSG:4326".to_string(),
            heading: None,
            distance: None,
            metadata: std::collections::HashMap::new(),
        },
    ];

    let config = ProjectionConfig {
        projection_distance_warning_threshold: 1000.0,
        suppress_warnings: true,
    };

    let result = project_gnss(&positions, &network, &config);
    assert!(result.is_ok());
    let projected = result.unwrap();
    assert_eq!(projected.len(), 2);
}

#[test]
fn test_projection_error_types() {
    // Test that ProjectionError types exist and can be constructed
    let _err = ProjectionError::EmptyNetwork;
    let _err = ProjectionError::InvalidCoordinate("test".to_string());
    let _err = ProjectionError::InvalidTimestamp("test".to_string());
    let _err = ProjectionError::MissingTimezone("test".to_string());
}

#[test]
fn test_result_type_alias() {
    // Test that Result type alias works
    let ok_result: Result<i32> = Ok(42);
    assert!(ok_result.is_ok());
    assert_eq!(ok_result.unwrap(), 42);

    let err_result: Result<i32> = Err(ProjectionError::EmptyNetwork);
    assert!(err_result.is_err());
}
