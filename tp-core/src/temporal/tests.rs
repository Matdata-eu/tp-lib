//! Unit tests for temporal utilities

use super::*;
use chrono::{Datelike, TimeZone, Timelike};

#[test]
fn test_parse_valid_rfc3339_utc() {
    let result = parse_rfc3339_with_timezone("2024-01-15T10:30:00Z");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.year(), 2024);
    assert_eq!(dt.month(), 1);
    assert_eq!(dt.day(), 15);
    assert_eq!(dt.hour(), 10);
    assert_eq!(dt.minute(), 30);
    assert_eq!(dt.second(), 0);
}

#[test]
fn test_parse_valid_rfc3339_with_positive_offset() {
    let result = parse_rfc3339_with_timezone("2024-01-15T10:30:00+01:00");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.offset().local_minus_utc(), 3600); // +01:00 = 3600 seconds
}

#[test]
fn test_parse_valid_rfc3339_with_negative_offset() {
    let result = parse_rfc3339_with_timezone("2024-01-15T10:30:00-05:00");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.offset().local_minus_utc(), -18000); // -05:00 = -18000 seconds
}

#[test]
fn test_parse_invalid_timestamp() {
    let result = parse_rfc3339_with_timezone("not a timestamp");
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(
            matches!(e, ProjectionError::MissingTimezone(_)),
            "Expected MissingTimezone error"
        );
    }
}

#[test]
fn test_parse_timestamp_missing_timezone() {
    // ISO 8601 without timezone should fail
    let result = parse_rfc3339_with_timezone("2024-01-15T10:30:00");
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_format() {
    let result = parse_rfc3339_with_timezone("15/01/2024 10:30");
    assert!(result.is_err());
}

#[test]
fn test_validate_timezone_present() {
    let dt = FixedOffset::east_opt(3600)
        .unwrap()
        .with_ymd_and_hms(2024, 1, 15, 10, 30, 0)
        .unwrap();
    let result = validate_timezone_present(&dt);
    assert!(result.is_ok());
}

#[test]
fn test_parse_with_fractional_seconds() {
    let result = parse_rfc3339_with_timezone("2024-01-15T10:30:00.123Z");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.timestamp_subsec_millis(), 123);
}

#[test]
fn test_parse_with_various_offsets() {
    let offsets = vec!["+00:00", "+01:00", "+02:30", "-08:00", "-11:30"];
    for offset in offsets {
        let timestamp = format!("2024-01-15T10:30:00{}", offset);
        let result = parse_rfc3339_with_timezone(&timestamp);
        assert!(
            result.is_ok(),
            "Failed to parse timestamp with offset: {}",
            offset
        );
    }
}

#[test]
fn test_parse_leap_second() {
    // RFC3339 allows leap seconds (60)
    let result = parse_rfc3339_with_timezone("2024-06-30T23:59:60Z");
    // chrono may handle this differently, just ensure it doesn't panic
    let _ = result;
}

#[test]
fn test_parse_edge_case_dates() {
    let dates = vec![
        "2024-01-01T00:00:00Z", // Start of year
        "2024-12-31T23:59:59Z", // End of year
        "2024-02-29T12:00:00Z", // Leap year
    ];
    
    for date_str in dates {
        let result = parse_rfc3339_with_timezone(date_str);
        assert!(result.is_ok(), "Failed to parse: {}", date_str);
    }
}
