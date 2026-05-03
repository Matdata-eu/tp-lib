//! Integration tests for detection provenance output (T017, US1).
//!
//! Covers FR-017, D9: every input detection must produce exactly one
//! [`DetectionRecord`] in `PreparedDetections.records`, with the correct
//! status (`Applied` / `Resolved` / `Discarded`).

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use geo::LineString;

use tp_lib_core::detections::prepare_detections_from_loaded;
use tp_lib_core::models::{
    Detection, DetectionKind, DetectionStatus, GnssPosition, Netelement,
    PunctualDetection, TopologicalLocation,
};

fn ts(secs: i64) -> DateTime<FixedOffset> {
    let dt: DateTime<Utc> = Utc.timestamp_opt(1_700_000_000 + secs, 0).unwrap();
    dt.into()
}

fn ne(id: &str) -> Netelement {
    Netelement::new(
        id.to_string(),
        LineString::from(vec![(4.3500, 50.8500), (4.3520, 50.8500)]),
        "EPSG:4326".to_string(),
    )
    .unwrap()
}

fn gnss_window(n: usize) -> Vec<GnssPosition> {
    (0..n)
        .map(|i| {
            GnssPosition::new(
                50.8500,
                4.3500 + i as f64 * 0.0005,
                ts(i as i64),
                "EPSG:4326".to_string(),
            )
            .unwrap()
        })
        .collect()
}

#[test]
fn applied_detection_yields_applied_record() {
    let netelements = vec![ne("NE_A")];
    let gnss = gnss_window(5);

    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts(2),
        location: Some(TopologicalLocation {
            netelement_id: "NE_A".to_string(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: Some("d1".to_string()),
        source: None,
        source_file: "topo.csv".to_string(),
        source_row: 1,
        metadata: Default::default(),
    });

    let prepared =
        prepare_detections_from_loaded(vec![det], &gnss, &netelements, 2.5).expect("ok");

    assert_eq!(prepared.records.len(), 1);
    let rec = &prepared.records[0];
    assert_eq!(rec.source_file, "topo.csv");
    assert_eq!(rec.source_row, 1);
    assert_eq!(rec.kind, DetectionKind::Punctual);
    match &rec.status {
        DetectionStatus::Applied {
            netelement_id,
            intrinsic,
        } => {
            assert_eq!(netelement_id, "NE_A");
            assert!((intrinsic - 0.5).abs() < 1e-9);
        }
        other => panic!("expected Applied, got {:?}", other),
    }
    assert_eq!(prepared.anchors.len(), 1);
}

#[test]
fn out_of_window_detection_yields_discarded_record() {
    let netelements = vec![ne("NE_A")];
    let gnss = gnss_window(5);

    // Timestamp far outside GNSS window (1 hour later).
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts(3_600),
        location: Some(TopologicalLocation {
            netelement_id: "NE_A".to_string(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "topo.csv".to_string(),
        source_row: 7,
        metadata: Default::default(),
    });

    let prepared =
        prepare_detections_from_loaded(vec![det], &gnss, &netelements, 2.5).expect("ok");

    assert_eq!(prepared.records.len(), 1);
    let rec = &prepared.records[0];
    assert_eq!(rec.source_row, 7);
    assert!(
        matches!(rec.status, DetectionStatus::Discarded { .. }),
        "expected Discarded, got {:?}",
        rec.status
    );
    assert_eq!(prepared.anchors.len(), 0);
}

#[test]
fn provenance_length_equals_input_count() {
    let netelements = vec![ne("NE_A")];
    let gnss = gnss_window(5);

    let make = |row: usize, secs: i64| {
        Detection::Punctual(PunctualDetection {
            timestamp: ts(secs),
            location: Some(TopologicalLocation {
                netelement_id: "NE_A".to_string(),
                intrinsic: 0.25,
            }),
            coordinates: None,
            intrinsic: None,
            id: None,
            source: None,
            source_file: "topo.csv".to_string(),
            source_row: row,
            metadata: Default::default(),
        })
    };

    let inputs = vec![
        make(1, 0),     // applied
        make(2, 2),     // applied
        make(3, 9_999), // discarded (out-of-window)
    ];

    let prepared =
        prepare_detections_from_loaded(inputs, &gnss, &netelements, 2.5).expect("ok");

    assert_eq!(prepared.records.len(), 3, "one record per input detection");

    let applied = prepared
        .records
        .iter()
        .filter(|r| matches!(r.status, DetectionStatus::Applied { .. }))
        .count();
    let discarded = prepared
        .records
        .iter()
        .filter(|r| matches!(r.status, DetectionStatus::Discarded { .. }))
        .count();
    assert_eq!(applied, 2);
    assert_eq!(discarded, 1);
    assert_eq!(prepared.anchors.len(), 2);
}
