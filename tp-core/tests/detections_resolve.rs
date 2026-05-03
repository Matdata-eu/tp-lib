//! T029 [US3] — Coordinate-only punctual detection resolution tests.
//!
//! Covers FR-003a/FR-003b: lat/lon punctual detections snap to the nearest
//! netelement within `cutoff_distance`; beyond cutoff they are discarded
//! with `DiscardReason::OutOfReach`. Missing CRS yields
//! `DetectionError::MissingCrs`.

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use geo::LineString;

use tp_lib_core::detections::prepare_detections_from_loaded;
use tp_lib_core::models::{
    Detection, DetectionStatus, DiscardReason, GeographicLocation, GnssPosition,
    Netelement, PunctualDetection, ResolvedAnchor,
};
use tp_lib_core::DetectionError;

fn ts(secs: i64) -> DateTime<FixedOffset> {
    let dt: DateTime<Utc> = Utc.timestamp_opt(1_700_000_000 + secs, 0).unwrap();
    dt.into()
}

fn single_segment_network() -> Vec<Netelement> {
    vec![Netelement::new(
        "NE_MAIN".to_string(),
        LineString::from(vec![(4.3500, 50.8500), (4.3520, 50.8500)]),
        "EPSG:4326".to_string(),
    )
    .unwrap()]
}

fn gnss_corridor(n: usize) -> Vec<GnssPosition> {
    (0..n)
        .map(|i| {
            let frac = i as f64 / (n - 1).max(1) as f64;
            let lon = 4.3500 + frac * 0.0020;
            GnssPosition::new(50.8500, lon, ts(i as i64), "EPSG:4326".to_string()).unwrap()
        })
        .collect()
}

#[test]
fn coordinate_within_cutoff_resolves_to_punctual_anchor() {
    let netelements = single_segment_network();
    let gnss = gnss_corridor(5);

    // Point essentially on NE_MAIN (lon midpoint, lat exact).
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts(2),
        location: None,
        coordinates: Some(GeographicLocation {
            latitude: 50.8500,
            longitude: 4.3510,
            crs: "EPSG:4326".to_string(),
        }),
        intrinsic: None,
        id: None,
        source: None,
        source_file: "tst".into(),
        source_row: 1,
        metadata: Default::default(),
    });

    let prepared = prepare_detections_from_loaded(vec![det], &gnss, &netelements, 5.0)
        .expect("prepare ok");

    assert_eq!(prepared.anchors.len(), 1);
    match &prepared.anchors[0] {
        ResolvedAnchor::Punctual {
            netelement_id,
            intrinsic,
            ..
        } => {
            assert_eq!(netelement_id, "NE_MAIN");
            assert!(
                (0.0..=1.0).contains(intrinsic),
                "intrinsic out of range: {}",
                intrinsic
            );
        }
        other => panic!("expected punctual anchor, got {:?}", other),
    }
    assert!(matches!(
        prepared.records[0].status,
        DetectionStatus::Resolved { .. } | DetectionStatus::Applied { .. }
    ));
}

#[test]
fn coordinate_beyond_cutoff_is_discarded_out_of_reach() {
    let netelements = single_segment_network();
    let gnss = gnss_corridor(5);

    // Point well off NE_MAIN (~1km north).
    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts(2),
        location: None,
        coordinates: Some(GeographicLocation {
            latitude: 50.8600,
            longitude: 4.3510,
            crs: "EPSG:4326".to_string(),
        }),
        intrinsic: None,
        id: None,
        source: None,
        source_file: "tst".into(),
        source_row: 1,
        metadata: Default::default(),
    });

    let prepared = prepare_detections_from_loaded(vec![det], &gnss, &netelements, 2.5)
        .expect("prepare ok");

    assert_eq!(prepared.anchors.len(), 0, "must not anchor beyond cutoff");
    assert_eq!(prepared.records.len(), 1);
    assert!(matches!(
        prepared.records[0].status,
        DetectionStatus::Discarded {
            reason: DiscardReason::OutOfReach { .. }
        }
    ));
}

#[test]
fn coordinate_missing_crs_returns_missing_crs_error() {
    let netelements = single_segment_network();
    let gnss = gnss_corridor(5);

    let det = Detection::Punctual(PunctualDetection {
        timestamp: ts(2),
        location: None,
        coordinates: Some(GeographicLocation {
            latitude: 50.8500,
            longitude: 4.3510,
            crs: "".to_string(), // empty CRS
        }),
        intrinsic: None,
        id: None,
        source: None,
        source_file: "tst".into(),
        source_row: 1,
        metadata: Default::default(),
    });

    let err = prepare_detections_from_loaded(vec![det], &gnss, &netelements, 2.5)
        .expect_err("missing crs must error");
    assert!(
        matches!(err, DetectionError::MissingCrs { .. }),
        "expected MissingCrs, got {:?}",
        err
    );
}
