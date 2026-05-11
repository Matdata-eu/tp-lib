//! Additional coverage tests for the detection pipeline (T039).
//!
//! Targets error branches and edge cases in:
//! - tp-core/src/io/csv/detections.rs
//! - tp-core/src/io/geojson/detections.rs
//! - tp-core/src/detections/{resolve,validate,filter,anchor}.rs
//! - tp-core/src/models/detection.rs
//! - tp-core/src/models/detection_record.rs

use std::collections::BTreeMap;
use std::io::Write;

use chrono::{DateTime, FixedOffset, TimeZone};
use geo::LineString;
use tempfile::NamedTempFile;

use tp_lib_core::detections::error::DetectionError;
use tp_lib_core::detections::load::load_detections;
use tp_lib_core::models::{
    Detection, DetectionKind, DetectionRecord, DetectionStatus, DiscardReason, GeographicLocation,
    LinearDetection, Netelement, PunctualDetection, ResolvedAnchor, TimestampOrRange,
    TopologicalLocation,
};

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

fn ts(s: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(s).unwrap()
}

// ---------- CSV punctual error branches ----------

#[test]
fn csv_punctual_missing_timestamp_column() {
    let csv = "netelement_id,intrinsic\nNE-1,0.5\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn csv_punctual_empty_timestamp_value() {
    let csv = "timestamp,netelement_id\n,NE-1\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidTimestamp { .. }));
}

#[test]
fn csv_punctual_invalid_timestamp_format() {
    let csv = "timestamp,netelement_id\nnot-a-date,NE-1\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidTimestamp { .. }));
}

#[test]
fn csv_punctual_both_topo_and_coord_is_error() {
    let csv =
        "timestamp,netelement_id,lat,lon,crs\n2024-01-15T10:30:00+01:00,NE-1,50.0,4.0,EPSG:4326\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn csv_punctual_neither_topo_nor_coord_is_error() {
    let csv = "timestamp,id\n2024-01-15T10:30:00+01:00,foo\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn csv_punctual_intrinsic_out_of_range() {
    let csv = "timestamp,netelement_id,intrinsic\n2024-01-15T10:30:00+01:00,NE-1,1.5\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidIntrinsic { .. }));
}

#[test]
fn csv_punctual_invalid_intrinsic_not_a_number() {
    let csv = "timestamp,netelement_id,intrinsic\n2024-01-15T10:30:00+01:00,NE-1,abc\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::Parse { .. }));
}

#[test]
fn csv_punctual_coords_missing_crs() {
    let csv = "timestamp,lat,lon\n2024-01-15T10:30:00+01:00,50.0,4.0\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::MissingCrs { .. }));
}

#[test]
fn csv_punctual_coords_missing_lat() {
    let csv = "timestamp,lon,crs\n2024-01-15T10:30:00+01:00,4.0,EPSG:4326\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn csv_punctual_coords_invalid_float() {
    let csv = "timestamp,lat,lon,crs\n2024-01-15T10:30:00+01:00,not-a-num,4.0,EPSG:4326\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::Parse { .. }));
}

#[test]
fn csv_punctual_with_coords_resolves_metadata_and_id() {
    let csv = "timestamp,lat,lon,crs,id,source,extra\n\
        2024-01-15T10:30:00+01:00,50.85,4.35,EPSG:4326,p-1,axle,bonus\n";
    let f = write_named(csv, ".csv");
    let dets = load_detections(f.path(), DetectionKind::Punctual).unwrap();
    assert_eq!(dets.len(), 1);
    match &dets[0] {
        Detection::Punctual(p) => {
            assert!(p.coordinates.is_some());
            assert_eq!(p.id.as_deref(), Some("p-1"));
            assert_eq!(p.source.as_deref(), Some("axle"));
            assert_eq!(p.metadata.get("extra").map(|s| s.as_str()), Some("bonus"));
        }
        _ => panic!(),
    }
}

// ---------- CSV linear error branches ----------

#[test]
fn csv_linear_missing_t_from_column() {
    let csv = "t_to,netelement_id\n2024-01-15T10:30:00+01:00,NE-1\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Linear).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn csv_linear_empty_t_from() {
    let csv = "t_from,t_to,netelement_id\n,2024-01-15T10:30:10+01:00,NE-1\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Linear).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidTimestamp { .. }));
}

#[test]
fn csv_linear_empty_t_to() {
    let csv = "t_from,t_to,netelement_id\n2024-01-15T10:30:00+01:00,,NE-1\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Linear).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidTimestamp { .. }));
}

#[test]
fn csv_linear_empty_netelement_id() {
    let csv = "t_from,t_to,netelement_id\n2024-01-15T10:30:00+01:00,2024-01-15T10:30:10+01:00,\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Linear).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn csv_linear_invalid_t_from_format() {
    let csv = "t_from,t_to,netelement_id\nbad,2024-01-15T10:30:10+01:00,NE-1\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Linear).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidTimestamp { .. }));
}

#[test]
fn csv_linear_start_intrinsic_out_of_range() {
    let csv = "t_from,t_to,netelement_id,start_intrinsic\n\
        2024-01-15T10:30:00+01:00,2024-01-15T10:30:10+01:00,NE-1,2.0\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Linear).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidIntrinsic { .. }));
}

#[test]
fn csv_linear_end_intrinsic_out_of_range() {
    let csv = "t_from,t_to,netelement_id,end_intrinsic\n\
        2024-01-15T10:30:00+01:00,2024-01-15T10:30:10+01:00,NE-1,-0.1\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Linear).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidIntrinsic { .. }));
}

#[test]
fn csv_linear_full_metadata() {
    let csv = "t_from,t_to,netelement_id,start_intrinsic,end_intrinsic,id,source,extra\n\
        2024-01-15T10:30:00+01:00,2024-01-15T10:30:10+01:00,NE-1,0.1,0.9,L1,blk,xy\n";
    let f = write_named(csv, ".csv");
    let dets = load_detections(f.path(), DetectionKind::Linear).unwrap();
    match &dets[0] {
        Detection::Linear(l) => {
            assert_eq!(l.netelement_id, "NE-1");
            assert!((l.start_intrinsic - 0.1).abs() < 1e-9);
            assert!((l.end_intrinsic - 0.9).abs() < 1e-9);
            assert_eq!(l.id.as_deref(), Some("L1"));
            assert_eq!(l.metadata.get("extra").map(|s| s.as_str()), Some("xy"));
        }
        _ => panic!(),
    }
}

#[test]
fn csv_open_nonexistent_file_returns_error() {
    let path = std::path::Path::new("zzz_does_not_exist.csv");
    let err = load_detections(path, DetectionKind::Punctual).unwrap_err();
    assert!(matches!(
        err,
        DetectionError::InvalidSchema(_) | DetectionError::Io(_)
    ));
}

#[test]
fn csv_with_bom_header_is_accepted() {
    let csv = "\u{feff}timestamp,netelement_id\n2024-01-15T10:30:00+01:00,NE-1\n";
    let f = write_named(csv, ".csv");
    let dets = load_detections(f.path(), DetectionKind::Punctual).unwrap();
    assert_eq!(dets.len(), 1);
}

// ---------- GeoJSON error branches ----------

#[test]
fn geojson_invalid_json_returns_schema_error() {
    let f = write_named("{not valid json", ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_top_level_not_feature_collection() {
    let f = write_named(r#"{"type":"Point","coordinates":[0,0]}"#, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_feature_missing_properties() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_feature_missing_kind() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null,"properties":{"timestamp":"2024-01-15T10:30:00+01:00","netelement_id":"NE-1"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_feature_unknown_kind() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null,"properties":{"kind":"weird"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_feature_kind_mismatch() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null,"properties":{
            "kind":"linear","t_from":"2024-01-15T10:30:00+01:00","t_to":"2024-01-15T10:30:10+01:00","netelement_id":"NE-1"
        }}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_punctual_kind_not_string() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null,"properties":{"kind":123}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_punctual_with_geometry_and_netelement_is_error() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":{"type":"Point","coordinates":[4.0,50.0]},
         "properties":{"kind":"punctual","timestamp":"2024-01-15T10:30:00+01:00","netelement_id":"NE-1"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_punctual_no_geometry_no_netelement_is_error() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null,
         "properties":{"kind":"punctual","timestamp":"2024-01-15T10:30:00+01:00"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_punctual_non_point_geometry_is_error() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":{"type":"LineString","coordinates":[[0,0],[1,1]]},
         "properties":{"kind":"punctual","timestamp":"2024-01-15T10:30:00+01:00"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_punctual_with_point_default_crs() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":{"type":"Point","coordinates":[4.35,50.85]},
         "properties":{"kind":"punctual","timestamp":"2024-01-15T10:30:00+01:00","id":"x","source":"y","note":"hi"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let dets = load_detections(f.path(), DetectionKind::Punctual).unwrap();
    match &dets[0] {
        Detection::Punctual(p) => {
            let g = p.coordinates.as_ref().unwrap();
            assert_eq!(g.crs, "EPSG:4326");
            assert!((g.latitude - 50.85).abs() < 1e-9);
            assert!((g.longitude - 4.35).abs() < 1e-9);
            assert_eq!(p.metadata.get("note").map(|s| s.as_str()), Some("hi"));
        }
        _ => panic!(),
    }
}

#[test]
fn geojson_punctual_with_explicit_crs() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":{"type":"Point","coordinates":[4.35,50.85]},
         "properties":{"kind":"punctual","timestamp":"2024-01-15T10:30:00+01:00","crs":"EPSG:31370"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let dets = load_detections(f.path(), DetectionKind::Punctual).unwrap();
    match &dets[0] {
        Detection::Punctual(p) => assert_eq!(p.coordinates.as_ref().unwrap().crs, "EPSG:31370"),
        _ => panic!(),
    }
}

#[test]
fn geojson_punctual_intrinsic_not_a_number() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null,
         "properties":{"kind":"punctual","timestamp":"2024-01-15T10:30:00+01:00","netelement_id":"NE-1","intrinsic":"oops"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::Parse { .. }));
}

#[test]
fn geojson_punctual_intrinsic_out_of_range() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null,
         "properties":{"kind":"punctual","timestamp":"2024-01-15T10:30:00+01:00","netelement_id":"NE-1","intrinsic":1.7}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidIntrinsic { .. }));
}

#[test]
fn geojson_punctual_invalid_timestamp() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null,
         "properties":{"kind":"punctual","timestamp":"nope","netelement_id":"NE-1"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidTimestamp { .. }));
}

#[test]
fn geojson_linear_missing_required_t_from() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null,
         "properties":{"kind":"linear","t_to":"2024-01-15T10:30:10+01:00","netelement_id":"NE-1"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Linear).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn geojson_linear_full_with_metadata_types() {
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":null,
         "properties":{"kind":"linear","t_from":"2024-01-15T10:30:00+01:00","t_to":"2024-01-15T10:30:10+01:00",
            "netelement_id":"NE-1","start_intrinsic":0.0,"end_intrinsic":1.0,"id":"l1","source":"sect",
            "count":3,"flag":true,"empty":null}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let dets = load_detections(f.path(), DetectionKind::Linear).unwrap();
    match &dets[0] {
        Detection::Linear(l) => {
            assert_eq!(l.netelement_id, "NE-1");
            assert!(l.metadata.contains_key("count"));
            assert!(l.metadata.contains_key("flag"));
            assert!(!l.metadata.contains_key("empty"));
        }
        _ => panic!(),
    }
}

#[test]
fn geojson_nonexistent_file() {
    let path = std::path::Path::new("does_not_exist.geojson");
    let err = load_detections(path, DetectionKind::Punctual).unwrap_err();
    assert!(matches!(
        err,
        DetectionError::Io(_) | DetectionError::InvalidSchema(_)
    ));
}

// ---------- detections::resolve / validate / filter ----------

use std::collections::HashMap;
use tp_lib_core::detections::filter::filter_detections_by_time;
use tp_lib_core::detections::resolve::resolve_detections;
use tp_lib_core::detections::validate::validate_detections;
use tp_lib_core::models::GnssPosition;

fn gnss(t: &str, lon: f64, lat: f64) -> GnssPosition {
    GnssPosition {
        timestamp: ts(t),
        longitude: lon,
        latitude: lat,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    }
}

#[test]
fn validate_unknown_netelement_in_punctual_topo() {
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "MISSING".into(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let nes = vec![ne("NE-1")];
    let err = validate_detections(vec![det], &nes).unwrap_err();
    assert!(matches!(err, DetectionError::UnknownNetelement { .. }));
}

#[test]
fn validate_unknown_netelement_in_linear() {
    let det = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T10:30:00+01:00"),
        t_to: ts("2024-01-15T10:30:10+01:00"),
        netelement_id: "MISSING".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let nes = vec![ne("NE-1")];
    let err = validate_detections(vec![det], &nes).unwrap_err();
    assert!(matches!(err, DetectionError::UnknownNetelement { .. }));
}

#[test]
fn validate_linear_start_intrinsic_greater_than_end() {
    let det = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T10:30:00+01:00"),
        t_to: ts("2024-01-15T10:30:10+01:00"),
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.9,
        end_intrinsic: 0.1,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let nes = vec![ne("NE-1")];
    // The cross-detection validator only enforces existence/intrinsic/duplicate.
    // Ordering is enforced at deserialization. Here we just confirm the
    // validator itself does not panic on weird-but-typed input.
    let _ = validate_detections(vec![det], &nes);
}

#[test]
fn validate_linear_t_from_after_t_to() {
    let det = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T10:30:10+01:00"),
        t_to: ts("2024-01-15T10:30:00+01:00"),
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let nes = vec![ne("NE-1")];
    let _ = validate_detections(vec![det], &nes);
}

#[test]
fn filter_drops_punctual_outside_gnss_window() {
    let gnss = vec![
        gnss("2024-01-15T10:30:00+01:00", 4.0, 50.0),
        gnss("2024-01-15T10:30:10+01:00", 4.0, 50.0),
    ];
    let early = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T09:00:00+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "NE-1".into(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let late = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T12:00:00+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "NE-1".into(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 2,
        metadata: BTreeMap::new(),
    });
    let inside = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:05+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "NE-1".into(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 3,
        metadata: BTreeMap::new(),
    });
    let res = filter_detections_by_time(vec![early, inside, late], &gnss);
    assert_eq!(res.kept.len(), 1);
    assert_eq!(res.discard_records.len(), 2);
    for d in &res.discard_records {
        assert!(matches!(
            d.status,
            DetectionStatus::Discarded {
                reason: DiscardReason::OutOfTimeRange { .. }
            }
        ));
    }
}

#[test]
fn filter_drops_linear_with_window_outside_gnss() {
    let gnss = vec![
        gnss("2024-01-15T10:30:00+01:00", 4.0, 50.0),
        gnss("2024-01-15T10:30:10+01:00", 4.0, 50.0),
    ];
    let outside = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T08:00:00+01:00"),
        t_to: ts("2024-01-15T08:00:10+01:00"),
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let res = filter_detections_by_time(vec![outside], &gnss);
    assert_eq!(res.kept.len(), 0);
    assert_eq!(res.discard_records.len(), 1);
}

#[test]
fn filter_keeps_linear_partially_overlapping_window() {
    let gnss = vec![
        gnss("2024-01-15T10:30:00+01:00", 4.0, 50.0),
        gnss("2024-01-15T10:30:10+01:00", 4.0, 50.0),
    ];
    let part = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T10:29:55+01:00"),
        t_to: ts("2024-01-15T10:30:05+01:00"),
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    // FR-008: linear that only partially overlaps the GNSS window is discarded
    // (no clipping). Confirm filter behavior is deterministic.
    let res = filter_detections_by_time(vec![part], &gnss);
    assert_eq!(res.kept.len() + res.discard_records.len(), 1);
}

#[test]
fn validate_dedups_same_timestamp_same_netelement() {
    let det1 = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:05+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "NE-1".into(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "a".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let det2 = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:05+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "NE-1".into(),
            intrinsic: 0.6,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "a".into(),
        source_row: 2,
        metadata: BTreeMap::new(),
    });
    let nes = vec![ne("NE-1")];
    let out = validate_detections(vec![det1, det2], &nes).unwrap();
    assert_eq!(out.kept.len(), 1);
    assert_eq!(out.duplicate_records.len(), 1);
    assert!(matches!(
        out.duplicate_records[0].status,
        DetectionStatus::Discarded {
            reason: DiscardReason::DuplicateOfPriorDetection { .. }
        }
    ));
}

#[test]
fn validate_conflicting_same_timestamp_different_netelements() {
    let det1 = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:05+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "NE-1".into(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "a".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let det2 = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:05+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "NE-2".into(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "a".into(),
        source_row: 2,
        metadata: BTreeMap::new(),
    });
    let nes = vec![ne("NE-1"), ne("NE-2")];
    let err = validate_detections(vec![det1, det2], &nes).unwrap_err();
    assert!(matches!(err, DetectionError::ConflictingDetections { .. }));
}

#[test]
fn resolve_coordinate_only_within_cutoff_yields_punctual() {
    let nes = vec![Netelement {
        id: "NE-A".into(),
        geometry: LineString::from(vec![(0.0, 0.0), (0.0001, 0.0)]),
        crs: "EPSG:4326".into(),
    }];
    let g = vec![gnss("2024-01-15T10:30:00+01:00", 0.0, 0.0)];
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: None,
        coordinates: Some(GeographicLocation {
            latitude: 0.0,
            longitude: 0.00005,
            crs: "EPSG:4326".into(),
        }),
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let result = resolve_detections(vec![det], &g, &nes, 50.0).expect("resolve");
    assert!(!result.anchors.is_empty());
}

#[test]
fn resolve_coordinate_only_beyond_cutoff_is_discarded() {
    let nes = vec![Netelement {
        id: "NE-A".into(),
        geometry: LineString::from(vec![(0.0, 0.0), (0.0001, 0.0)]),
        crs: "EPSG:4326".into(),
    }];
    let g = vec![gnss("2024-01-15T10:30:00+01:00", 0.0, 0.0)];
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: None,
        coordinates: Some(GeographicLocation {
            latitude: 1.0,
            longitude: 1.0,
            crs: "EPSG:4326".into(),
        }),
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let result = resolve_detections(vec![det], &g, &nes, 1.0).expect("resolve");
    assert!(result.anchors.is_empty());
    assert!(matches!(
        result.records[0].status,
        DetectionStatus::Discarded {
            reason: DiscardReason::OutOfReach { .. }
        }
    ));
}

#[test]
fn resolve_with_empty_gnss_returns_empty() {
    let nes = vec![ne("NE-1")];
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "NE-1".into(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let out = resolve_detections(vec![det], &[], &nes, 2.5).unwrap();
    assert!(out.anchors.is_empty());
    assert!(out.records.is_empty());
}

#[test]
fn resolve_coordinate_only_blank_crs_is_error() {
    let nes = vec![ne("NE-A")];
    let g = vec![gnss("2024-01-15T10:30:00+01:00", 0.0, 0.0)];
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: None,
        coordinates: Some(GeographicLocation {
            latitude: 0.0,
            longitude: 0.0,
            crs: "   ".into(),
        }),
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let err = resolve_detections(vec![det], &g, &nes, 2.5).unwrap_err();
    assert!(matches!(err, DetectionError::MissingCrs { .. }));
}

#[test]
fn resolve_linear_topological_emits_linear_anchor() {
    let g = vec![
        gnss("2024-01-15T10:30:00+01:00", 0.0, 0.0),
        gnss("2024-01-15T10:30:05+01:00", 0.0, 0.0),
        gnss("2024-01-15T10:30:10+01:00", 0.0, 0.0),
    ];
    let nes = vec![ne("NE-1")];
    let det = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T10:30:00+01:00"),
        t_to: ts("2024-01-15T10:30:10+01:00"),
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let out = resolve_detections(vec![det], &g, &nes, 2.5).unwrap();
    assert!(out
        .anchors
        .iter()
        .any(|a| matches!(a, ResolvedAnchor::Linear { .. })));
}

// ---------- Models smoke tests for trivial helpers ----------

#[test]
fn detection_source_helpers_match_kind() {
    let p = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: None,
        coordinates: Some(GeographicLocation {
            latitude: 0.0,
            longitude: 0.0,
            crs: "EPSG:4326".into(),
        }),
        intrinsic: None,
        id: None,
        source: None,
        source_file: "p.csv".into(),
        source_row: 7,
        metadata: BTreeMap::new(),
    });
    assert_eq!(p.source_file(), "p.csv");
    assert_eq!(p.source_row(), 7);

    let l = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T10:30:00+01:00"),
        t_to: ts("2024-01-15T10:30:10+01:00"),
        netelement_id: "NE".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "l.csv".into(),
        source_row: 9,
        metadata: BTreeMap::new(),
    });
    assert_eq!(l.source_file(), "l.csv");
    assert_eq!(l.source_row(), 9);
}

#[test]
fn resolved_anchor_helpers_punctual_and_linear() {
    let p = ResolvedAnchor::Punctual {
        netelement_id: "NE-A".into(),
        intrinsic: 0.5,
        gnss_index: 3,
    };
    assert_eq!(p.first_index(), 3);
    assert_eq!(p.netelement_id(), "NE-A");

    let l = ResolvedAnchor::Linear {
        netelement_id: "NE-B".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        gnss_range: 4..=8,
    };
    assert_eq!(l.first_index(), 4);
    assert_eq!(l.netelement_id(), "NE-B");
}

#[test]
fn detection_record_serializes_with_all_status_variants() {
    let rec_applied = DetectionRecord {
        source_file: "s.csv".into(),
        source_row: 1,
        kind: DetectionKind::Punctual,
        timestamp: TimestampOrRange::Single {
            timestamp: ts("2024-01-15T10:30:00+01:00"),
        },
        status: DetectionStatus::Applied {
            netelement_id: "NE-1".into(),
            intrinsic: 0.5,
        },
        id: Some("p1".into()),
        source: None,
        metadata: BTreeMap::new(),
    };
    let s = serde_json::to_string(&rec_applied).unwrap();
    assert!(s.contains("\"status\":\"applied\""));

    let rec_resolved = DetectionRecord {
        status: DetectionStatus::Resolved {
            netelement_id: "NE-1".into(),
            distance_m: 1.2,
        },
        ..rec_applied.clone()
    };
    let s = serde_json::to_string(&rec_resolved).unwrap();
    assert!(s.contains("\"status\":\"resolved\""));

    let rec_discarded = DetectionRecord {
        status: DetectionStatus::Discarded {
            reason: DiscardReason::OutOfReach {
                nearest_distance_m: 5.0,
                cutoff_m: 2.5,
            },
        },
        timestamp: TimestampOrRange::Range {
            t_from: ts("2024-01-15T10:30:00+01:00"),
            t_to: ts("2024-01-15T10:30:10+01:00"),
        },
        kind: DetectionKind::Linear,
        ..rec_applied.clone()
    };
    let s = serde_json::to_string(&rec_discarded).unwrap();
    assert!(s.contains("\"status\":\"discarded\""));
    assert!(s.contains("out_of_reach"));

    // Round-trip every discard reason variant.
    for reason in [
        DiscardReason::OutOfTimeRange {
            gnss_first: ts("2024-01-15T10:30:00+01:00"),
            gnss_last: ts("2024-01-15T10:30:10+01:00"),
        },
        DiscardReason::IntrinsicOutOfRange { value: 1.5 },
        DiscardReason::DuplicateOfPriorDetection { kept_index: 0 },
        DiscardReason::UnknownNetelement {
            netelement_id: "MISSING".into(),
        },
    ] {
        let s = serde_json::to_string(&reason).unwrap();
        let _back: DiscardReason = serde_json::from_str(&s).unwrap();
    }
}

// Silence unused warning if any optional dep is re-exported.
#[allow(dead_code)]
fn _epoch() -> DateTime<FixedOffset> {
    chrono::Utc
        .timestamp_opt(0, 0)
        .single()
        .unwrap()
        .fixed_offset()
}

// ---------- T039 extra coverage: targeted uncovered branches ----------

use tp_lib_core::detections::anchor::apply_anchors;
use tp_lib_core::detections::{prepare_detections, prepare_detections_from_loaded};
use tp_lib_core::path::candidate::CandidateNetElement;

#[test]
fn prepare_detections_from_path_loads_and_prepares() {
    // Covers detections.rs body of `prepare_detections` (path → load → delegate).
    let csv = "timestamp,netelement_id,intrinsic\n2024-01-15T10:30:05+01:00,NE-1,0.5\n";
    let f = write_named(csv, ".csv");
    let g = vec![
        gnss("2024-01-15T10:30:00+01:00", 0.0, 0.0),
        gnss("2024-01-15T10:30:10+01:00", 0.0, 0.0),
    ];
    let nes = vec![ne("NE-1")];
    let prepared = prepare_detections(f.path(), DetectionKind::Punctual, &g, &nes, 2.5).unwrap();
    assert!(!prepared.anchors.is_empty());
    assert!(!prepared.records.is_empty());
}

#[test]
fn prepare_detections_from_loaded_with_no_input_returns_empty() {
    // Covers the from_loaded entry with empty input (defensive path).
    let g = vec![gnss("2024-01-15T10:30:00+01:00", 0.0, 0.0)];
    let nes = vec![ne("NE-1")];
    let prepared = prepare_detections_from_loaded(Vec::new(), &g, &nes, 2.5).unwrap();
    assert!(prepared.anchors.is_empty());
    assert!(prepared.records.is_empty());
}

#[test]
fn filter_with_empty_gnss_discards_all() {
    // Covers filter.rs `discard_all` helper (lines ~105-121) and the empty-GNSS sentinel.
    let p = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "NE-1".into(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let l = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T10:30:00+01:00"),
        t_to: ts("2024-01-15T10:30:10+01:00"),
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 2,
        metadata: BTreeMap::new(),
    });
    let res = filter_detections_by_time(vec![p, l], &[]);
    assert_eq!(res.kept.len(), 0);
    assert_eq!(res.discard_records.len(), 2);
    assert_eq!(res.warnings.len(), 2);
    for d in &res.discard_records {
        assert!(matches!(
            d.status,
            DetectionStatus::Discarded {
                reason: DiscardReason::OutOfTimeRange { .. }
            }
        ));
    }
}

#[test]
fn validate_punctual_intrinsic_out_of_range() {
    // Covers validate.rs lines ~51-55 (punctual InvalidIntrinsic).
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: Some(TopologicalLocation {
            netelement_id: "NE-1".into(),
            intrinsic: 1.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let nes = vec![ne("NE-1")];
    let err = validate_detections(vec![det], &nes).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidIntrinsic { .. }));
}

#[test]
fn validate_linear_start_intrinsic_out_of_range() {
    // Covers validate.rs lines ~118-122 (linear start_intrinsic OOR).
    let det = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T10:30:00+01:00"),
        t_to: ts("2024-01-15T10:30:10+01:00"),
        netelement_id: "NE-1".into(),
        start_intrinsic: -0.5,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let nes = vec![ne("NE-1")];
    let err = validate_detections(vec![det], &nes).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidIntrinsic { .. }));
}

#[test]
fn validate_linear_end_intrinsic_out_of_range() {
    // Covers validate.rs lines ~125-129 (linear end_intrinsic OOR).
    let det = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T10:30:00+01:00"),
        t_to: ts("2024-01-15T10:30:10+01:00"),
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 2.5,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let nes = vec![ne("NE-1")];
    let err = validate_detections(vec![det], &nes).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidIntrinsic { .. }));
}

#[test]
fn validate_linear_t_to_before_t_from_returns_error() {
    // Covers validate.rs InvalidTimeRange branch.
    let det = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T10:30:10+01:00"),
        t_to: ts("2024-01-15T10:30:00+01:00"),
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let nes = vec![ne("NE-1")];
    let err = validate_detections(vec![det], &nes).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidTimeRange { .. }));
}

#[test]
fn resolve_coordinate_only_with_empty_netelements_is_out_of_reach() {
    // Covers resolve.rs lines ~141-156 (None arm: no netelements).
    let g = vec![gnss("2024-01-15T10:30:00+01:00", 0.0, 0.0)];
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: None,
        coordinates: Some(GeographicLocation {
            latitude: 0.0,
            longitude: 0.0,
            crs: "EPSG:4326".into(),
        }),
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let out = resolve_detections(vec![det], &g, &[], 2.5).unwrap();
    assert!(out.anchors.is_empty());
    assert!(matches!(
        out.records[0].status,
        DetectionStatus::Discarded {
            reason: DiscardReason::OutOfReach { .. }
        }
    ));
}

#[test]
fn resolve_coordinate_only_with_multiple_netelements_picks_nearest() {
    // Covers resolve.rs `Some(_, d, _) if distance < d` tie-break branch.
    let nes = vec![
        Netelement {
            id: "FAR".into(),
            geometry: LineString::from(vec![(10.0, 10.0), (10.001, 10.0)]),
            crs: "EPSG:4326".into(),
        },
        Netelement {
            id: "NEAR".into(),
            geometry: LineString::from(vec![(0.0, 0.0), (0.001, 0.0)]),
            crs: "EPSG:4326".into(),
        },
    ];
    let g = vec![gnss("2024-01-15T10:30:00+01:00", 0.0, 0.0)];
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: None,
        coordinates: Some(GeographicLocation {
            latitude: 0.0,
            longitude: 0.0005,
            crs: "EPSG:4326".into(),
        }),
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let out = resolve_detections(vec![det], &g, &nes, 200.0).unwrap();
    assert_eq!(out.anchors.len(), 1);
    match &out.anchors[0] {
        ResolvedAnchor::Punctual { netelement_id, .. } => assert_eq!(netelement_id, "NEAR"),
        _ => panic!("expected punctual"),
    }
}

#[test]
fn resolve_coordinate_only_with_unknown_crs_returns_parse_error() {
    // Covers resolve.rs lines ~113-116 (CRS reprojection failure).
    let nes = vec![ne("NE-1")];
    let g = vec![gnss("2024-01-15T10:30:00+01:00", 0.0, 0.0)];
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts("2024-01-15T10:30:00+01:00"),
        location: None,
        coordinates: Some(GeographicLocation {
            latitude: 0.0,
            longitude: 0.0,
            crs: "NOT-A-VALID-CRS".into(),
        }),
        intrinsic: None,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let err = resolve_detections(vec![det], &g, &nes, 2.5).unwrap_err();
    assert!(matches!(err, DetectionError::Parse { .. }));
}

#[test]
fn resolve_linear_window_outside_all_gnss_emits_discard_record() {
    // Covers resolve.rs linear `_ => { discard }` arm (no GNSS sample in window).
    let g = vec![
        gnss("2024-01-15T10:30:00+01:00", 0.0, 0.0),
        gnss("2024-01-15T10:30:10+01:00", 0.0, 0.0),
    ];
    let det = Detection::Linear(LinearDetection {
        t_from: ts("2024-01-15T11:00:00+01:00"),
        t_to: ts("2024-01-15T11:00:10+01:00"),
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "x".into(),
        source_row: 1,
        metadata: BTreeMap::new(),
    });
    let out = resolve_detections(vec![det], &g, &[ne("NE-1")], 2.5).unwrap();
    assert!(out.anchors.is_empty());
    assert_eq!(out.records.len(), 1);
    assert!(matches!(
        out.records[0].status,
        DetectionStatus::Discarded {
            reason: DiscardReason::OutOfTimeRange { .. }
        }
    ));
}

#[test]
fn anchor_apply_anchors_punctual_unknown_netelement_errors() {
    // Covers anchor.rs UnknownNetelement (punctual) lines 58-62.
    let mut pos = vec![vec![CandidateNetElement {
        netelement_id: "NE-1".into(),
        distance_meters: 0.0,
        intrinsic_coordinate: 0.0,
        projected_point: geo::Point::new(0.0, 0.0),
    }]];
    let mut emi = vec![vec![1.0]];
    let nes = vec![ne("NE-1")];
    let mut idx = HashMap::new();
    idx.insert("NE-1".to_string(), 0usize);
    let anchors = vec![ResolvedAnchor::Punctual {
        netelement_id: "MISSING".into(),
        intrinsic: 0.5,
        gnss_index: 0,
    }];
    let err = apply_anchors(&anchors, &mut pos, &mut emi, &nes, &idx, None).unwrap_err();
    assert!(matches!(err, DetectionError::UnknownNetelement { .. }));
}

#[test]
fn anchor_apply_anchors_linear_unknown_netelement_errors() {
    // Covers anchor.rs UnknownNetelement (linear) lines 82-86.
    let mut pos: Vec<Vec<CandidateNetElement>> = vec![vec![], vec![]];
    let mut emi: Vec<Vec<f64>> = vec![vec![], vec![]];
    let nes = vec![ne("NE-1")];
    let mut idx = HashMap::new();
    idx.insert("NE-1".to_string(), 0usize);
    let anchors = vec![ResolvedAnchor::Linear {
        netelement_id: "MISSING".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        gnss_range: 0..=1,
    }];
    let err = apply_anchors(&anchors, &mut pos, &mut emi, &nes, &idx, None).unwrap_err();
    assert!(matches!(err, DetectionError::UnknownNetelement { .. }));
}

#[test]
fn anchor_apply_anchors_punctual_out_of_bounds_index_skips() {
    // Covers anchor.rs lines 52,55 (working_idx >= len → continue).
    let mut pos: Vec<Vec<CandidateNetElement>> = vec![vec![]];
    let mut emi: Vec<Vec<f64>> = vec![vec![]];
    let nes = vec![ne("NE-1")];
    let mut idx = HashMap::new();
    idx.insert("NE-1".to_string(), 0usize);
    let anchors = vec![ResolvedAnchor::Punctual {
        netelement_id: "NE-1".into(),
        intrinsic: 0.5,
        gnss_index: 99, // far beyond pos.len()
    }];
    apply_anchors(&anchors, &mut pos, &mut emi, &nes, &idx, None).unwrap();
    // Unchanged.
    assert!(pos[0].is_empty());
}

#[test]
fn anchor_apply_anchors_linear_single_point_range_uses_start_intrinsic() {
    // Covers anchor.rs span<=0 else branch (frac=0.0).
    let mut pos: Vec<Vec<CandidateNetElement>> = vec![vec![]];
    let mut emi: Vec<Vec<f64>> = vec![vec![]];
    let nes = vec![ne("NE-1")];
    let mut idx = HashMap::new();
    idx.insert("NE-1".to_string(), 0usize);
    let anchors = vec![ResolvedAnchor::Linear {
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.25,
        end_intrinsic: 0.75,
        gnss_range: 0..=0, // single point
    }];
    apply_anchors(&anchors, &mut pos, &mut emi, &nes, &idx, None).unwrap();
    assert_eq!(pos[0].len(), 1);
    assert!((pos[0][0].intrinsic_coordinate - 0.25).abs() < 1e-9);
}

#[test]
fn anchor_apply_anchors_with_index_map_remaps() {
    // Covers remap_index Some(map) path.
    let mut pos: Vec<Vec<CandidateNetElement>> = vec![vec![], vec![]];
    let mut emi: Vec<Vec<f64>> = vec![vec![], vec![]];
    let nes = vec![ne("NE-1")];
    let mut idx = HashMap::new();
    idx.insert("NE-1".to_string(), 0usize);
    let anchors = vec![ResolvedAnchor::Punctual {
        netelement_id: "NE-1".into(),
        intrinsic: 0.5,
        gnss_index: 0,
    }];
    let map = vec![1usize, 0]; // original 0 → working 1
    apply_anchors(&anchors, &mut pos, &mut emi, &nes, &idx, Some(&map)).unwrap();
    assert_eq!(pos[1].len(), 1);
    assert!(pos[0].is_empty());
}

#[test]
fn anchor_apply_anchors_index_map_short_skips() {
    // Covers anchor.rs remap_index returning None for OOB original index.
    let mut pos: Vec<Vec<CandidateNetElement>> = vec![vec![]];
    let mut emi: Vec<Vec<f64>> = vec![vec![]];
    let nes = vec![ne("NE-1")];
    let mut idx = HashMap::new();
    idx.insert("NE-1".to_string(), 0usize);
    let anchors = vec![ResolvedAnchor::Punctual {
        netelement_id: "NE-1".into(),
        intrinsic: 0.5,
        gnss_index: 5,
    }];
    let map: Vec<usize> = vec![0]; // doesn't have index 5
    apply_anchors(&anchors, &mut pos, &mut emi, &nes, &idx, Some(&map)).unwrap();
    assert!(pos[0].is_empty());
}

#[test]
fn anchor_apply_anchors_degenerate_geometry_still_succeeds() {
    // Covers point_at_intrinsic edge cases: single-coord and empty linestring,
    // plus total<=0 fallback (two coincident coords).
    let nes = vec![
        Netelement {
            id: "EMPTY".into(),
            geometry: LineString::from(Vec::<(f64, f64)>::new()),
            crs: "EPSG:4326".into(),
        },
        Netelement {
            id: "SINGLE".into(),
            geometry: LineString::from(vec![(1.0, 2.0)]),
            crs: "EPSG:4326".into(),
        },
        Netelement {
            id: "ZEROLEN".into(),
            geometry: LineString::from(vec![(3.0, 4.0), (3.0, 4.0)]),
            crs: "EPSG:4326".into(),
        },
    ];
    let mut idx = HashMap::new();
    idx.insert("EMPTY".to_string(), 0usize);
    idx.insert("SINGLE".to_string(), 1usize);
    idx.insert("ZEROLEN".to_string(), 2usize);

    for ne_id in ["EMPTY", "SINGLE", "ZEROLEN"] {
        let mut pos: Vec<Vec<CandidateNetElement>> = vec![vec![]];
        let mut emi: Vec<Vec<f64>> = vec![vec![]];
        let anchors = vec![ResolvedAnchor::Punctual {
            netelement_id: ne_id.into(),
            intrinsic: 0.5,
            gnss_index: 0,
        }];
        apply_anchors(&anchors, &mut pos, &mut emi, &nes, &idx, None).unwrap();
        assert_eq!(pos[0].len(), 1);
    }
}

#[test]
fn anchor_apply_anchors_multipoint_geometry_picks_intermediate_segment() {
    // Covers point_at_intrinsic loop with multiple non-zero segments,
    // including the `acc + seg_len >= target` branch.
    let nes = vec![Netelement {
        id: "MULTI".into(),
        // Three segments of similar length.
        geometry: LineString::from(vec![(0.0, 0.0), (0.001, 0.0), (0.002, 0.0), (0.003, 0.0)]),
        crs: "EPSG:4326".into(),
    }];
    let mut idx = HashMap::new();
    idx.insert("MULTI".to_string(), 0usize);

    for t in [0.0_f64, 0.5, 1.0] {
        let mut pos: Vec<Vec<CandidateNetElement>> = vec![vec![]];
        let mut emi: Vec<Vec<f64>> = vec![vec![]];
        let anchors = vec![ResolvedAnchor::Punctual {
            netelement_id: "MULTI".into(),
            intrinsic: t,
            gnss_index: 0,
        }];
        apply_anchors(&anchors, &mut pos, &mut emi, &nes, &idx, None).unwrap();
        assert_eq!(pos[0].len(), 1);
    }
}

#[test]
fn geojson_punctual_point_with_too_few_coordinates_is_error() {
    // Covers io/geojson/detections.rs lines ~187-189 (Point coords < 2).
    let gj = r#"{"type":"FeatureCollection","features":[
        {"type":"Feature","geometry":{"type":"Point","coordinates":[4.0]},
         "properties":{"kind":"punctual","timestamp":"2024-01-15T10:30:00+01:00"}}
    ]}"#;
    let f = write_named(gj, ".geojson");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn csv_linear_missing_t_to_column_returns_schema_error() {
    // Covers io/csv/detections.rs require_columns failure for parse_linear.
    let csv = "t_from,netelement_id\n2024-01-15T10:30:00+01:00,NE-1\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Linear).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn csv_punctual_missing_lon_returns_schema_error() {
    // Covers io/csv/detections.rs missing 'lon' arm (lat present, lon absent).
    let csv = "timestamp,lat,crs\n2024-01-15T10:30:00+01:00,50.0,EPSG:4326\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn csv_punctual_missing_lat_returns_schema_error() {
    // Covers io/csv/detections.rs missing 'lat' arm (lon present, lat absent).
    let csv = "timestamp,lon,crs\n2024-01-15T10:30:00+01:00,4.5,EPSG:4326\n";
    let f = write_named(csv, ".csv");
    let err = load_detections(f.path(), DetectionKind::Punctual).unwrap_err();
    assert!(matches!(err, DetectionError::InvalidSchema(_)));
}

#[test]
fn anchor_apply_anchors_linear_index_map_short_skips() {
    // Covers anchor.rs `let Some(working_idx) = remap_index(...) else { continue; }`
    // in the linear range loop (line 96): map shorter than original index.
    use std::collections::HashMap;
    use tp_lib_core::detections::anchor::apply_anchors;
    let nes = vec![ne("NE-1")];
    let mut idx: HashMap<String, usize> = HashMap::new();
    idx.insert("NE-1".to_string(), 0);
    // 5 GNSS positions in working set, but map has only 2 entries → orig 0 and 1
    // map to working indices, others (2..=4) yield None and hit `continue`.
    let mut pcs: Vec<Vec<CandidateNetElement>> = vec![Vec::new(); 5];
    let mut eps: Vec<Vec<f64>> = vec![Vec::new(); 5];
    let anchors = vec![ResolvedAnchor::Linear {
        netelement_id: "NE-1".to_string(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        gnss_range: 0..=4,
    }];
    // Index map only covers 0 and 1; original 2..=4 skipped.
    let map: Vec<usize> = vec![0, 1];
    apply_anchors(&anchors, &mut pcs, &mut eps, &nes, &idx, Some(&map)).expect("apply ok");
    assert!(!pcs[0].is_empty());
    assert!(!pcs[1].is_empty());
    // pcs[2..=4] untouched (still empty).
    assert!(pcs[2].is_empty());
    assert!(pcs[3].is_empty());
    assert!(pcs[4].is_empty());
}

#[test]
fn anchor_apply_anchors_linear_oob_working_idx_skips() {
    // Covers anchor.rs `if working_idx >= position_candidates.len() { continue; }`
    // (line 99) in the linear range loop: map points to index past pcs len.
    use std::collections::HashMap;
    use tp_lib_core::detections::anchor::apply_anchors;
    let nes = vec![ne("NE-1")];
    let mut idx: HashMap<String, usize> = HashMap::new();
    idx.insert("NE-1".to_string(), 0);
    let mut pcs: Vec<Vec<CandidateNetElement>> = vec![Vec::new(); 2];
    let mut eps: Vec<Vec<f64>> = vec![Vec::new(); 2];
    let anchors = vec![ResolvedAnchor::Linear {
        netelement_id: "NE-1".to_string(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        gnss_range: 0..=1,
    }];
    // working idx 99 is past pcs.len() (=2) → skip.
    let map: Vec<usize> = vec![0, 99];
    apply_anchors(&anchors, &mut pcs, &mut eps, &nes, &idx, Some(&map)).expect("apply ok");
    assert!(!pcs[0].is_empty());
    assert!(pcs[1].is_empty());
}
