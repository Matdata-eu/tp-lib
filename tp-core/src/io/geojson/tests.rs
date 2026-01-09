//! Unit tests for GeoJSON I/O

use super::*;
use crate::models::{AssociatedNetElement, PathMetadata, TrainPath};
use chrono::{DateTime, FixedOffset};
use geo::Point;
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

fn create_test_projected_position() -> ProjectedPosition {
    ProjectedPosition {
        original: GnssPosition {
            latitude: 50.8503,
            longitude: 4.3517,
            timestamp: "2024-01-15T10:30:00Z"
                .parse::<DateTime<FixedOffset>>()
                .unwrap(),
            crs: "EPSG:4326".to_string(),
            heading: None,
            distance: None,
            metadata: HashMap::new(),
        },
        projected_coords: Point::new(4.3517, 50.8503),
        netelement_id: "NE001".to_string(),
        measure_meters: 100.5,
        projection_distance_meters: 5.2,
        crs: "EPSG:4326".to_string(),
        intrinsic: None,
    }
}

#[test]
fn test_parse_network_geojson_basic() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r#"{{
        "type": "FeatureCollection",
        "features": [
            {{
                "type": "Feature",
                "properties": {{ "id": "NE001" }},
                "geometry": {{
                    "type": "LineString",
                    "coordinates": [[4.3517, 50.8503], [4.3527, 50.8513]]
                }}
            }}
        ]
    }}"#
    )
    .unwrap();

    let result = parse_network_geojson(file.path().to_str().unwrap());
    assert!(result.is_ok());
    let (netelements, netrelations) = result.unwrap();
    assert_eq!(netelements.len(), 1);
    assert_eq!(netelements[0].id, "NE001");
    assert_eq!(netrelations.len(), 0);
}

#[test]
fn test_parse_network_geojson_with_netrelations() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r#"{{
        "type": "FeatureCollection",
        "features": [
            {{
                "type": "Feature",
                "properties": {{
                    "id": "NE001"
                }},
                "geometry": {{
                    "type": "LineString",
                    "coordinates": [[4.3517, 50.8503], [4.3527, 50.8513]]
                }}
            }},
            {{
                "type": "Feature",
                "properties": {{
                    "type": "netrelation",
                    "id": "NR001",
                    "navigability": "Both",
                    "from": "NE001",
                    "to": "NE002",
                    "positionOnA": 1.0,
                    "positionOnB": 0.0
                }},
                "geometry": null
            }}
        ]
    }}"#
    )
    .unwrap();

    let result = parse_network_geojson(file.path().to_str().unwrap());
    assert!(result.is_ok());
    let (netelements, netrelations) = result.unwrap();
    assert_eq!(netelements.len(), 1);
    assert_eq!(netrelations.len(), 1);
    assert_eq!(netrelations[0].id, "NR001");
}

#[test]
fn test_parse_network_geojson_not_feature_collection() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r#"{{
        "type": "Feature",
        "properties": {{}},
        "geometry": null
    }}"#
    )
    .unwrap();

    let result = parse_network_geojson(file.path().to_str().unwrap());
    assert!(result.is_err());
    if let Err(ProjectionError::InvalidGeometry(msg)) = result {
        assert!(msg.contains("FeatureCollection"));
    } else {
        panic!("Expected InvalidGeometry error");
    }
}

#[test]
fn test_parse_network_geojson_invalid_json() {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "not valid json").unwrap();

    let result = parse_network_geojson(file.path().to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_parse_network_geojson_missing_id() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r#"{{
        "type": "FeatureCollection",
        "features": [
            {{
                "type": "Feature",
                "properties": {{}},
                "geometry": {{
                    "type": "LineString",
                    "coordinates": [[4.3517, 50.8503], [4.3527, 50.8513]]
                }}
            }}
        ]
    }}"#
    )
    .unwrap();

    let result = parse_network_geojson(file.path().to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_parse_network_geojson_non_wgs84_crs() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r#"{{
        "type": "FeatureCollection",
        "crs": {{
            "type": "name",
            "properties": {{ "name": "urn:ogc:def:crs:EPSG::31370" }}
        }},
        "features": []
    }}"#
    )
    .unwrap();

    let result = parse_network_geojson(file.path().to_str().unwrap());
    assert!(result.is_err());
    if let Err(ProjectionError::InvalidCrs(msg)) = result {
        assert!(msg.contains("WGS84"));
    } else {
        panic!("Expected InvalidCrs error");
    }
}

#[test]
fn test_parse_gnss_geojson_basic() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r#"{{
        "type": "FeatureCollection",
        "features": [
            {{
                "type": "Feature",
                "properties": {{
                    "timestamp": "2024-01-15T10:30:00Z"
                }},
                "geometry": {{
                    "type": "Point",
                    "coordinates": [4.3517, 50.8503]
                }}
            }}
        ]
    }}"#
    )
    .unwrap();

    let result = parse_gnss_geojson(file.path().to_str().unwrap(), "EPSG:4326");
    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].latitude, 50.8503);
    assert_eq!(positions[0].longitude, 4.3517);
}

#[test]
fn test_parse_gnss_geojson_missing_timestamp() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r#"{{
        "type": "FeatureCollection",
        "features": [
            {{
                "type": "Feature",
                "properties": {{}},
                "geometry": {{
                    "type": "Point",
                    "coordinates": [4.3517, 50.8503]
                }}
            }}
        ]
    }}"#
    )
    .unwrap();

    let result = parse_gnss_geojson(file.path().to_str().unwrap(), "EPSG:4326");
    assert!(result.is_err());
}

#[test]
fn test_parse_gnss_geojson_not_point() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r#"{{
        "type": "FeatureCollection",
        "features": [
            {{
                "type": "Feature",
                "properties": {{ "timestamp": "2024-01-15T10:30:00Z" }},
                "geometry": {{
                    "type": "LineString",
                    "coordinates": [[4.3517, 50.8503], [4.3527, 50.8513]]
                }}
            }}
        ]
    }}"#
    )
    .unwrap();

    let result = parse_gnss_geojson(file.path().to_str().unwrap(), "EPSG:4326");
    assert!(result.is_err());
}

#[test]
fn test_parse_gnss_geojson_with_metadata() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r#"{{
        "type": "FeatureCollection",
        "features": [
            {{
                "type": "Feature",
                "properties": {{
                    "timestamp": "2024-01-15T10:30:00Z",
                    "speed": 50.5,
                    "altitude": 100
                }},
                "geometry": {{
                    "type": "Point",
                    "coordinates": [4.3517, 50.8503]
                }}
            }}
        ]
    }}"#
    )
    .unwrap();

    let result = parse_gnss_geojson(file.path().to_str().unwrap(), "EPSG:4326");
    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions[0].metadata.get("speed"), Some(&"50.5".to_string()));
}

#[test]
fn test_write_geojson_basic() {
    let positions = vec![create_test_projected_position()];

    let mut file = NamedTempFile::new().unwrap();
    let result = write_geojson(&positions, &mut file);
    assert!(result.is_ok());

    // Verify file was written and is valid JSON
    let contents = std::fs::read_to_string(file.path()).unwrap();
    assert!(contents.contains("FeatureCollection"));
    assert!(contents.contains("NE001"));
}

#[test]
fn test_write_geojson_empty() {
    let positions: Vec<ProjectedPosition> = vec![];
    let mut file = NamedTempFile::new().unwrap();
    let result = write_geojson(&positions, &mut file);
    assert!(result.is_ok());
}

#[test]
fn test_parse_netrelations_geojson_basic() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        r#"{{
        "type": "FeatureCollection",
        "features": [
            {{
                "type": "Feature",
                "properties": {{
                    "type": "netrelation",
                    "id": "NR001",
                    "navigability": "Both",
                    "from": "NE001",
                    "to": "NE002",
                    "positionOnA": 1.0,
                    "positionOnB": 0.0
                }},
                "geometry": null
            }}
        ]
    }}"#
    )
    .unwrap();

    let result = parse_netrelations_geojson(file.path().to_str().unwrap());
    assert!(result.is_ok());
    let netrelations = result.unwrap();
    assert_eq!(netrelations.len(), 1);
    assert_eq!(netrelations[0].id, "NR001");
}

#[test]
fn test_write_trainpath_geojson_basic() {
    let mut netelements_map = HashMap::new();
    netelements_map.insert(
        "NE001".to_string(),
        Netelement {
            id: "NE001".to_string(),
            geometry: LineString::new(vec![
                Coord {
                    x: 4.3517,
                    y: 50.8503,
                },
                Coord {
                    x: 4.3527,
                    y: 50.8513,
                },
            ]),
            crs: "EPSG:4326".to_string(),
        },
    );

    let path = TrainPath::new(
        vec![AssociatedNetElement {
            netelement_id: "NE001".to_string(),
            start_intrinsic: 0.0,
            end_intrinsic: 1.0,
            probability: 0.95,
            gnss_start_index: 0,
            gnss_end_index: 10,
        }],
        0.95,
        None,
        Some(PathMetadata {
            distance_scale: 50.0,
            heading_scale: 45.0,
            cutoff_distance: 100.0,
            heading_cutoff: 90.0,
            probability_threshold: 0.001,
            resampling_distance: None,
            fallback_mode: false,
            candidate_paths_evaluated: 1,
            bidirectional_path: true,
            diagnostic_info: None,
        }),
    )
    .unwrap();

    let mut file = NamedTempFile::new().unwrap();
    let result = write_trainpath_geojson(&path, &netelements_map, &mut file);
    assert!(result.is_ok());

    // Verify file was written
    let contents = std::fs::read_to_string(file.path()).unwrap();
    assert!(contents.contains("FeatureCollection"));
    assert!(contents.contains("NE001"));
}
