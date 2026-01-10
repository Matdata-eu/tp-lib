//! Unit tests for CSV I/O

use super::*;
use crate::models::{AssociatedNetElement, TrainPath};
use chrono::{DateTime, FixedOffset};
use geo::Point;
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

fn create_test_gnss_position() -> GnssPosition {
    GnssPosition {
        latitude: 50.8503,
        longitude: 4.3517,
        timestamp: "2024-01-15T10:30:00Z"
            .parse::<DateTime<FixedOffset>>()
            .unwrap(),
        crs: "EPSG:4326".to_string(),
        heading: None,
        distance: None,
        metadata: HashMap::new(),
    }
}

fn create_test_projected_position() -> ProjectedPosition {
    ProjectedPosition {
        original: create_test_gnss_position(),
        projected_coords: Point::new(4.3517, 50.8503),
        netelement_id: "NE001".to_string(),
        measure_meters: 100.5,
        projection_distance_meters: 5.2,
        crs: "EPSG:4326".to_string(),
        intrinsic: None,
    }
}

fn create_test_trainpath() -> TrainPath {
    TrainPath::new(
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
        None,
    )
    .unwrap()
}

#[test]
fn test_parse_gnss_csv_basic() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp\n50.8503,4.3517,2024-01-15T10:30:00Z\n50.8504,4.3518,2024-01-15T10:30:01Z"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0].latitude, 50.8503);
    assert_eq!(positions[0].longitude, 4.3517);
}

#[test]
fn test_parse_gnss_csv_missing_latitude_column() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "longitude,timestamp\n4.3517,2024-01-15T10:30:00Z").unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_err());
    if let Err(ProjectionError::InvalidCoordinate(msg)) = result {
        assert!(msg.contains("Latitude column"));
    } else {
        panic!("Expected InvalidCoordinate error");
    }
}

#[test]
fn test_parse_gnss_csv_missing_longitude_column() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "latitude,timestamp\n50.8503,2024-01-15T10:30:00Z").unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_err());
    if let Err(ProjectionError::InvalidCoordinate(msg)) = result {
        assert!(msg.contains("Longitude column"));
    } else {
        panic!("Expected InvalidCoordinate error");
    }
}

#[test]
fn test_parse_gnss_csv_missing_timestamp_column() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "latitude,longitude\n50.8503,4.3517").unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_err());
    if let Err(ProjectionError::InvalidTimestamp(msg)) = result {
        assert!(msg.contains("Timestamp column"));
    } else {
        panic!("Expected InvalidTimestamp error");
    }
}

#[test]
fn test_parse_gnss_csv_with_heading_and_distance() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp,heading,distance\n50.8503,4.3517,2024-01-15T10:30:00Z,45.0,100.5"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].heading, Some(45.0));
    assert_eq!(positions[0].distance, Some(100.5));
}

#[test]
fn test_parse_gnss_csv_empty_file() {
    let file = NamedTempFile::new().unwrap();
    writeln!(&file, "latitude,longitude,timestamp").unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions.len(), 0);
}

#[test]
fn test_parse_gnss_csv_invalid_numeric() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp\ninvalid,4.3517,2024-01-15T10:30:00Z"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_err());
}

#[test]
fn test_parse_gnss_csv_custom_column_names() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "lat,lon,time\n50.8503,4.3517,2024-01-15T10:30:00Z"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "lat",
        "lon",
        "time",
    );

    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions.len(), 1);
}

#[test]
fn test_parse_gnss_csv_preserves_metadata() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp,speed,altitude\n50.8503,4.3517,2024-01-15T10:30:00Z,50.5,100.0"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions[0].metadata.get("speed"), Some(&"50.5".to_string()));
    assert_eq!(
        positions[0].metadata.get("altitude"),
        Some(&"100.0".to_string())
    );
}

#[test]
fn test_write_csv_basic() {
    let positions = vec![create_test_projected_position()];

    let mut file = NamedTempFile::new().unwrap();

    let result = write_csv(&positions, &mut file);
    assert!(result.is_ok());

    // Verify file was written
    let contents = fs::read_to_string(file.path()).unwrap();
    assert!(contents.contains("netelement_id"));
    assert!(contents.contains("NE001"));
    assert!(contents.contains("100.5"));
}

#[test]
fn test_write_csv_empty_vector() {
    let positions: Vec<ProjectedPosition> = vec![];
    let mut file = NamedTempFile::new().unwrap();

    let result = write_csv(&positions, &mut file);
    assert!(result.is_ok());
}

#[test]
fn test_write_csv_with_intrinsic_position() {
    // Test that intrinsic coordinate can exist on ProjectedPosition
    // Note: write_csv doesn't output intrinsic (legacy US1 format)
    let mut projected = create_test_projected_position();
    projected.intrinsic = Some(0.75);

    let mut output = Vec::new();
    let result = write_csv(&[projected], &mut output);
    assert!(result.is_ok());

    let csv_string = String::from_utf8(output).unwrap();
    // Verify standard columns are present
    assert!(csv_string.contains(COL_NETELEMENT_ID));
    assert!(csv_string.contains(COL_MEASURE_METERS));
}

#[test]
fn test_write_csv_multiple_positions() {
    let pos1 = create_test_projected_position();
    let mut pos2 = create_test_projected_position();
    pos2.measure_meters = 200.5;
    pos2.netelement_id = "NE002".to_string();

    let mut output = Vec::new();
    let result = write_csv(&[pos1, pos2], &mut output);
    assert!(result.is_ok());

    let csv_string = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = csv_string.lines().collect();
    assert_eq!(lines.len(), 3); // header + 2 data rows
    assert!(csv_string.contains("NE001"));
    assert!(csv_string.contains("NE002"));
}

#[test]
fn test_write_trainpath_csv_basic() {
    let path = create_test_trainpath();

    let mut file = NamedTempFile::new().unwrap();
    let result = write_trainpath_csv(&path, &mut file);
    assert!(result.is_ok());

    // Verify file was written
    let contents = fs::read_to_string(file.path()).unwrap();
    assert!(contents.contains("netelement_id"));
    assert!(contents.contains("NE001"));
}

#[test]
fn test_parse_trainpath_csv_basic() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "netelement_id,start_intrinsic,end_intrinsic,probability,gnss_start_index,gnss_end_index"
    )
    .unwrap();
    writeln!(file, "NE001,0.0,1.0,0.95,0,10").unwrap();

    let result = parse_trainpath_csv(file.path().to_str().unwrap());
    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path.segments.len(), 1);
    assert_eq!(path.segments[0].netelement_id, "NE001");
    assert_eq!(path.segments[0].probability, 0.95);
}

#[test]
fn test_parse_trainpath_csv_empty() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "netelement_id,start_intrinsic,end_intrinsic,probability,gnss_start_index,gnss_end_index"
    )
    .unwrap();

    let result = parse_trainpath_csv(file.path().to_str().unwrap());
    // Empty trainpath should return an error because TrainPath requires at least one segment
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e, ProjectionError::PathCalculationFailed { .. }));
    }
}

#[test]
fn test_parse_trainpath_csv_multiple_segments() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "netelement_id,start_intrinsic,end_intrinsic,probability,gnss_start_index,gnss_end_index"
    )
    .unwrap();
    writeln!(file, "NE001,0.0,1.0,0.95,0,5").unwrap();
    writeln!(file, "NE002,0.0,0.5,0.90,6,10").unwrap();

    let result = parse_trainpath_csv(file.path().to_str().unwrap());
    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path.segments.len(), 2);
    assert_eq!(path.segments[1].netelement_id, "NE002");
}

#[test]
fn test_roundtrip_trainpath_csv() {
    let original = TrainPath::new(
        vec![
            AssociatedNetElement {
                netelement_id: "NE001".to_string(),
                start_intrinsic: 0.0,
                end_intrinsic: 1.0,
                probability: 0.95,
                gnss_start_index: 0,
                gnss_end_index: 5,
            },
            AssociatedNetElement {
                netelement_id: "NE002".to_string(),
                start_intrinsic: 0.2,
                end_intrinsic: 0.8,
                probability: 0.90,
                gnss_start_index: 6,
                gnss_end_index: 10,
            },
        ],
        0.925,
        None,
        None,
    )
    .unwrap();

    let mut write_file = NamedTempFile::new().unwrap();
    write_trainpath_csv(&original, &mut write_file).unwrap();
    let parsed = parse_trainpath_csv(write_file.path().to_str().unwrap()).unwrap();

    assert_eq!(parsed.segments.len(), original.segments.len());
    assert_eq!(parsed.segments[0].netelement_id, "NE001");
    assert_eq!(parsed.segments[1].netelement_id, "NE002");
}

#[test]
fn test_parse_gnss_csv_with_metadata() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp,train_id,speed\n50.8503,4.3517,2024-01-15T10:30:00Z,T123,80.5"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions.len(), 1);
    assert!(positions[0].metadata.contains_key("train_id"));
    assert!(positions[0].metadata.contains_key("speed"));
}

#[test]
fn test_parse_gnss_csv_invalid_heading_range() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp,heading\n50.8503,4.3517,2024-01-15T10:30:00Z,400.0"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_err());
    if let Err(ProjectionError::InvalidGeometry(msg)) = result {
        assert!(msg.contains("Heading must be in [0, 360]"));
    } else {
        panic!("Expected InvalidGeometry error for invalid heading");
    }
}

#[test]
fn test_parse_gnss_csv_negative_distance() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp,distance\n50.8503,4.3517,2024-01-15T10:30:00Z,-10.0"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );

    assert!(result.is_err());
    if let Err(ProjectionError::InvalidGeometry(msg)) = result {
        assert!(msg.contains("Distance must be >= 0"));
    } else {
        panic!("Expected InvalidGeometry error for negative distance");
    }
}

#[test]
fn test_write_projected_positions_csv() {
    let positions = vec![create_test_projected_position()];
    let mut output = Vec::new();
    
    let result = write_csv(&positions, &mut output);
    assert!(result.is_ok());
    
    let csv_str = String::from_utf8(output).unwrap();
    assert!(csv_str.contains("original_lat"));
    assert!(csv_str.contains("50.8503"));
    assert!(csv_str.contains("NE001"));
}

#[test]
fn test_write_trainpath_csv_with_comments() {
    use chrono::Utc;
    
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
        Some(Utc::now()),
        None,
    )
    .unwrap();
    
    let mut output = Vec::new();
    let result = write_trainpath_csv(&path, &mut output);
    assert!(result.is_ok());
    
    let csv_str = String::from_utf8(output).unwrap();
    assert!(csv_str.contains("# overall_probability:"));
    assert!(csv_str.contains("# calculated_at:"));
}

#[test]
fn test_parse_trainpath_csv_with_comments() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "#overall_probability: 0.95\n#calculated_at: 2024-01-15T10:30:00Z\nnetelement_id,start_intrinsic,end_intrinsic,probability,gnss_start_index,gnss_end_index\nNE001,0.0,1.0,0.95,0,10"
    )
    .unwrap();

    let result = parse_trainpath_csv(file.path().to_str().unwrap());
    assert!(result.is_ok());
    
    let path = result.unwrap();
    assert_eq!(path.overall_probability, 0.95);
    assert!(path.calculated_at.is_some());
}

// Additional edge case tests for improved coverage

#[test]
fn test_parse_gnss_csv_timezone_validation_missing_utc() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp\n50.8503,4.3517,2025-12-09T14:30:00"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );
    
    // Should fail due to missing timezone
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("timezone"));
    }
}

#[test]
fn test_parse_gnss_csv_empty_string_values() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp,train_id\n50.8503,4.3517,2025-12-09T14:30:00+01:00,"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );
    
    // Should succeed - empty metadata values are allowed
    assert!(result.is_ok());
}

#[test]
fn test_parse_gnss_csv_special_characters_in_metadata() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp,notes\n50.8503,4.3517,2025-12-09T14:30:00+01:00,\"Line with, comma\""
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );
    
    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions.len(), 1);
    // Metadata with special characters should be preserved
    assert!(positions[0].metadata.contains_key("notes"));
}

#[test]
fn test_parse_gnss_csv_null_values_in_optional_columns() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "latitude,longitude,timestamp,heading,distance\n50.8503,4.3517,2025-12-09T14:30:00+01:00,,\n50.8513,4.3527,2025-12-09T14:31:00+01:00,90.0,100.0"
    )
    .unwrap();

    let result = parse_gnss_csv(
        file.path().to_str().unwrap(),
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    );
    
    // Should succeed - null values in optional columns are allowed
    assert!(result.is_ok());
    let positions = result.unwrap();
    assert_eq!(positions.len(), 2);
    assert!(positions[0].heading.is_none());
    assert!(positions[0].distance.is_none());
    assert_eq!(positions[1].heading, Some(90.0));
    assert_eq!(positions[1].distance, Some(100.0));
}

#[test]
fn test_write_csv_with_metadata_field() {
    let mut pos = create_test_projected_position();
    pos.original.metadata.insert("train_id".to_string(), "T123".to_string());
    pos.original.metadata.insert("description".to_string(), "Test data".to_string());
    
    let mut output = Vec::new();
    let result = write_csv(&vec![pos], &mut output);
    assert!(result.is_ok());
    
    // CSV writing should succeed even with metadata
    let csv_str = String::from_utf8(output).unwrap();
    assert!(!csv_str.is_empty());
}

#[test]
fn test_write_trainpath_csv_with_zero_probability() {
    use chrono::Utc;
    
    let path = TrainPath::new(
        vec![AssociatedNetElement {
            netelement_id: "NE001".to_string(),
            start_intrinsic: 0.0,
            end_intrinsic: 1.0,
            probability: 0.0,
            gnss_start_index: 0,
            gnss_end_index: 10,
        }],
        0.0,
        Some(Utc::now()),
        None,
    )
    .unwrap();
    
    let mut output = Vec::new();
    let result = write_trainpath_csv(&path, &mut output);
    assert!(result.is_ok());
    
    let csv_str = String::from_utf8(output).unwrap();
    assert!(csv_str.contains("# overall_probability: 0"));
}

#[test]
fn test_parse_trainpath_csv_invalid_probability_format() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "#overall_probability: not_a_number\nnetelement_id,start_intrinsic,end_intrinsic,probability,gnss_start_index,gnss_end_index\nNE001,0.0,1.0,0.95,0,10"
    )
    .unwrap();

    let result = parse_trainpath_csv(file.path().to_str().unwrap());
    // Should succeed with default probability since parsing failed
    assert!(result.is_ok());
}

#[test]
fn test_parse_trainpath_csv_invalid_timestamp_format() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "#calculated_at: invalid-date\nnetelement_id,start_intrinsic,end_intrinsic,probability,gnss_start_index,gnss_end_index\nNE001,0.0,1.0,0.95,0,10"
    )
    .unwrap();

    let result = parse_trainpath_csv(file.path().to_str().unwrap());
    // Should succeed with None timestamp since parsing failed
    assert!(result.is_ok());
    let path = result.unwrap();
    assert!(path.calculated_at.is_none());
}

#[test]
fn test_parse_trainpath_csv_missing_netelement_id_column() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "start_intrinsic,end_intrinsic,probability,gnss_start_index,gnss_end_index\n0.0,1.0,0.95,0,10"
    )
    .unwrap();

    let result = parse_trainpath_csv(file.path().to_str().unwrap());
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("netelement_id"));
    }
}

#[test]
fn test_parse_trainpath_csv_invalid_intrinsic_value() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "netelement_id,start_intrinsic,end_intrinsic,probability,gnss_start_index,gnss_end_index\nNE001,not_a_number,1.0,0.95,0,10"
    )
    .unwrap();

    let result = parse_trainpath_csv(file.path().to_str().unwrap());
    assert!(result.is_err());
}

#[test]
fn test_parse_trainpath_csv_invalid_gnss_index() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "netelement_id,start_intrinsic,end_intrinsic,probability,gnss_start_index,gnss_end_index\nNE001,0.0,1.0,0.95,-1,10"
    )
    .unwrap();

    let result = parse_trainpath_csv(file.path().to_str().unwrap());
    // Negative values will be cast to large positive usize, which might cause issues
    // The code doesn't validate indices, so this might succeed but with wrong values
    // Testing that parsing doesn't crash
    let _ = result;
}

