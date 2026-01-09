//! Unit tests for GnssPosition model validation

use chrono::{FixedOffset, TimeZone};
use std::collections::HashMap;
use tp_lib_core::models::GnssPosition;

#[test]
fn test_valid_gnss_position() {
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

    // Should be able to create a valid position
    assert_eq!(pos.latitude, 50.8503);
    assert_eq!(pos.longitude, 4.3517);
}

#[test]
fn test_latitude_at_boundaries() {
    let tz = FixedOffset::east_opt(3600).unwrap();
    let dt = tz.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    // Valid: latitude at boundaries (-90, 90)
    let pos_north = GnssPosition {
        latitude: 90.0,
        longitude: 0.0,
        timestamp: dt,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };
    assert_eq!(pos_north.latitude, 90.0);

    let pos_south = GnssPosition {
        latitude: -90.0,
        longitude: 0.0,
        timestamp: dt,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };
    assert_eq!(pos_south.latitude, -90.0);
}

#[test]
fn test_longitude_at_boundaries() {
    let tz = FixedOffset::east_opt(3600).unwrap();
    let dt = tz.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    // Valid: longitude at boundaries (-180, 180)
    let pos_east = GnssPosition {
        latitude: 0.0,
        longitude: 180.0,
        timestamp: dt,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };
    assert_eq!(pos_east.longitude, 180.0);

    let pos_west = GnssPosition {
        latitude: 0.0,
        longitude: -180.0,
        timestamp: dt,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };
    assert_eq!(pos_west.longitude, -180.0);
}

#[test]
fn test_timezone_preservation() {
    // Test different timezone offsets
    let tz_plus = FixedOffset::east_opt(3600).unwrap(); // +01:00
    let dt_plus = tz_plus.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    let pos_plus = GnssPosition {
        latitude: 50.8503,
        longitude: 4.3517,
        timestamp: dt_plus,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };

    assert_eq!(pos_plus.timestamp.timezone(), tz_plus);

    // Test negative timezone
    let tz_minus = FixedOffset::west_opt(18000).unwrap(); // -05:00
    let dt_minus = tz_minus.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    let pos_minus = GnssPosition {
        latitude: 40.7128,
        longitude: -74.0060,
        timestamp: dt_minus,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };

    assert_eq!(pos_minus.timestamp.timezone(), tz_minus);
}

#[test]
fn test_metadata_storage() {
    let tz = FixedOffset::east_opt(3600).unwrap();
    let dt = tz.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    let mut metadata = HashMap::new();
    metadata.insert("train_id".to_string(), "IC1234".to_string());
    metadata.insert("speed_kmh".to_string(), "120".to_string());
    metadata.insert("source".to_string(), "GPS".to_string());

    let pos = GnssPosition {
        latitude: 50.8503,
        longitude: 4.3517,
        timestamp: dt,
        crs: "EPSG:4326".to_string(),
        metadata: metadata.clone(),
        heading: None,
        distance: None,
    };

    assert_eq!(pos.metadata.len(), 3);
    assert_eq!(pos.metadata.get("train_id"), Some(&"IC1234".to_string()));
    assert_eq!(pos.metadata.get("speed_kmh"), Some(&"120".to_string()));
    assert_eq!(pos.metadata.get("source"), Some(&"GPS".to_string()));
}

#[test]
fn test_empty_metadata() {
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

    assert!(pos.metadata.is_empty());
}

#[test]
fn test_various_crs_formats() {
    let tz = FixedOffset::east_opt(3600).unwrap();
    let dt = tz.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    // Test various CRS string formats
    let crs_formats = vec![
        "EPSG:4326",
        "EPSG:31370", // Belgian Lambert 72
        "EPSG:3812",  // Belgian Lambert 2008
        "urn:ogc:def:crs:EPSG::4326",
    ];

    for crs in crs_formats {
        let pos = GnssPosition {
            latitude: 50.8503,
            longitude: 4.3517,
            timestamp: dt,
            crs: crs.to_string(),
            metadata: HashMap::new(),
            heading: None,
            distance: None,
        };
        assert_eq!(pos.crs, crs);
    }
}

#[test]
fn test_utc_timezone() {
    // Test UTC (offset 0)
    let tz = FixedOffset::east_opt(0).unwrap();
    let dt = tz.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    let pos = GnssPosition {
        latitude: 51.5074,
        longitude: -0.1278,
        timestamp: dt,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };

    assert_eq!(pos.timestamp.timezone(), tz);
    assert_eq!(pos.timestamp.offset().local_minus_utc(), 0);
}

#[test]
fn test_extreme_timezone_offsets() {
    // Test extreme timezone offsets
    // UTC+14:00 (Line Islands)
    let tz_max = FixedOffset::east_opt(14 * 3600).unwrap();
    let dt_max = tz_max.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    let pos_max = GnssPosition {
        latitude: -11.0,
        longitude: -157.0,
        timestamp: dt_max,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };
    assert_eq!(pos_max.timestamp.timezone(), tz_max);

    // UTC-12:00 (Baker Island)
    let tz_min = FixedOffset::west_opt(12 * 3600).unwrap();
    let dt_min = tz_min.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

    let pos_min = GnssPosition {
        latitude: 0.0,
        longitude: -176.0,
        timestamp: dt_min,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };
    assert_eq!(pos_min.timestamp.timezone(), tz_min);
}

#[test]
fn test_timestamp_ordering() {
    let tz = FixedOffset::east_opt(3600).unwrap();
    let dt1 = tz.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
    let dt2 = tz.with_ymd_and_hms(2024, 1, 15, 10, 30, 5).unwrap();

    let pos1 = GnssPosition {
        latitude: 50.8503,
        longitude: 4.3517,
        timestamp: dt1,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };

    let pos2 = GnssPosition {
        latitude: 50.8505,
        longitude: 4.3520,
        timestamp: dt2,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    };

    // Verify timestamps are ordered
    assert!(pos1.timestamp < pos2.timestamp);
}
