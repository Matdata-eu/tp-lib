//! Integration tests for detection loading (T004).
//!
//! Covers FR-001, FR-002, FR-002a, FR-002b, FR-005..FR-007a:
//! extension dispatch, valid CSV/GeoJSON parsing, schema errors,
//! invalid timestamp, conflicting punctual detections, duplicate dedup.

use std::io::Write;

use tempfile::NamedTempFile;

use tp_lib_core::detections::error::DetectionError;
use tp_lib_core::detections::load::load_detections;
use tp_lib_core::detections::validate::validate_detections;
use tp_lib_core::models::{Detection, DetectionKind, DetectionStatus, Netelement};

use chrono::TimeZone;
use geo::LineString;

fn write_named(content: &str, suffix: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new()
        .suffix(suffix)
        .tempfile()
        .expect("create temp file");
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn ne(id: &str) -> Netelement {
    Netelement {
        id: id.to_string(),
        geometry: LineString::from(vec![(0.0, 0.0), (1.0, 0.0)]),
        crs: "EPSG:4326".to_string(),
    }
}

#[test]
fn rejects_unsupported_extension() {
    let f = write_named("hello", ".txt");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::UnsupportedExtension(_)));
}

#[test]
fn loads_valid_punctual_csv() {
    let csv = "\
timestamp,netelement_id,intrinsic,id,source\n\
2026-05-01T08:15:30+02:00,NE-1,0.5,beacon-7,axle-counter-A12\n";
    let f = write_named(csv, ".csv");
    let dets = load_detections(f.path(), DetectionKind::Punctual).expect("parse csv");
    assert_eq!(dets.len(), 1);
    match &dets[0] {
        Detection::Punctual(p) => {
            assert_eq!(p.location.as_ref().unwrap().netelement_id, "NE-1");
            assert!((p.location.as_ref().unwrap().intrinsic - 0.5).abs() < 1e-9);
            assert_eq!(p.id.as_deref(), Some("beacon-7"));
        }
        _ => panic!("expected punctual"),
    }
}

#[test]
fn loads_valid_linear_geojson() {
    let gj = r#"{
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "geometry": null,
                "properties": {
                    "kind": "linear",
                    "t_from": "2026-05-01T08:15:00+02:00",
                    "t_to": "2026-05-01T08:17:30+02:00",
                    "netelement_id": "NE-9001",
                    "source": "track-circuit-A12"
                }
            }
        ]
    }"#;
    let f = write_named(gj, ".geojson");
    let dets = load_detections(f.path(), DetectionKind::Linear).expect("parse geojson");
    assert_eq!(dets.len(), 1);
    match &dets[0] {
        Detection::Linear(l) => {
            assert_eq!(l.netelement_id, "NE-9001");
            assert!(l.t_to >= l.t_from);
        }
        _ => panic!("expected linear"),
    }
}

#[test]
fn invalid_schema_punctual_kind_mismatch() {
    let gj = r#"{
        "type": "FeatureCollection",
        "features": [{"type":"Feature","geometry":null,"properties":{
            "kind":"linear","t_from":"2026-05-01T08:00:00+00:00",
            "t_to":"2026-05-01T08:01:00+00:00","netelement_id":"NE-1"
        }}]
    }"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn invalid_timestamp_rejected() {
    let csv = "timestamp,netelement_id\n2026-05-01 08:75:30,NE-1\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidTimestamp { .. }));
}

#[test]
fn conflicting_punctual_detections_fail_validation() {
    let csv = "\
timestamp,netelement_id\n\
2026-05-01T08:15:30+02:00,NE-1\n\
2026-05-01T08:15:30+02:00,NE-2\n";
    let f = write_named(csv, ".csv");
    let dets = load_detections(f.path(), DetectionKind::Punctual).expect("parse");
    let netelements = vec![ne("NE-1"), ne("NE-2")];
    let err = validate_detections(dets, &netelements).unwrap_err();
    assert!(matches!(err, DetectionError::ConflictingDetections { .. }));
}

#[test]
fn duplicate_detections_are_deduplicated() {
    let csv = "\
timestamp,netelement_id\n\
2026-05-01T08:15:30+02:00,NE-1\n\
2026-05-01T08:15:30+02:00,NE-1\n";
    let f = write_named(csv, ".csv");
    let dets = load_detections(f.path(), DetectionKind::Punctual).expect("parse");
    let netelements = vec![ne("NE-1")];
    let outcome = validate_detections(dets, &netelements).expect("validate");
    assert_eq!(outcome.kept.len(), 1);
    assert_eq!(outcome.duplicate_records.len(), 1);
    assert!(matches!(
        outcome.duplicate_records[0].status,
        DetectionStatus::Discarded { .. }
    ));
}

#[test]
fn linear_invalid_time_range_fails_validation() {
    let _ = chrono::FixedOffset::east_opt(0); // touch chrono import
    let _ = chrono::Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0);

    let gj = r#"{
        "type":"FeatureCollection","features":[{
            "type":"Feature","geometry":null,
            "properties":{"kind":"linear",
                "t_from":"2026-05-01T08:30:00+00:00",
                "t_to":  "2026-05-01T08:00:00+00:00",
                "netelement_id":"NE-1"}
        }]
    }"#;
    let f = write_named(gj, ".geojson");
    let dets = load_detections(f.path(), DetectionKind::Linear).expect("parse");
    let netelements = vec![ne("NE-1")];
    let err = validate_detections(dets, &netelements).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidTimeRange { .. }));
}
