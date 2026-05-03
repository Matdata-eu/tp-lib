//! Integration tests for time-range filter (T005).
//!
//! Covers FR-010, FR-011: detections whose timestamps fall outside the GNSS
//! observation window are discarded.

use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};

use tp_lib_core::detections::filter::filter_detections_by_time;
use tp_lib_core::models::{
    Detection, DetectionStatus, DiscardReason, GnssPosition, LinearDetection, PunctualDetection,
    TopologicalLocation,
};

fn ts(s: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(s).expect("rfc3339")
}

fn gnss(t: &str) -> GnssPosition {
    GnssPosition {
        latitude: 50.0,
        longitude: 4.0,
        timestamp: ts(t),
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    }
}

fn punctual(t: &str) -> Detection {
    Detection::Punctual(PunctualDetection {
        timestamp: ts(t),
        location: Some(TopologicalLocation {
            netelement_id: "NE-1".into(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "tst".into(),
        source_row: 1,
        metadata: Default::default(),
    })
}

fn linear(from: &str, to: &str) -> Detection {
    Detection::Linear(LinearDetection {
        t_from: ts(from),
        t_to: ts(to),
        netelement_id: "NE-1".into(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "tst".into(),
        source_row: 1,
        metadata: Default::default(),
    })
}

fn window() -> Vec<GnssPosition> {
    vec![
        gnss("2026-05-01T08:00:00+00:00"),
        gnss("2026-05-01T08:30:00+00:00"),
    ]
}

#[test]
fn punctual_strictly_before_window_discarded() {
    let out = filter_detections_by_time(vec![punctual("2026-05-01T07:00:00+00:00")], &window());
    assert_eq!(out.kept.len(), 0);
    assert_eq!(out.discard_records.len(), 1);
    match &out.discard_records[0].status {
        DetectionStatus::Discarded {
            reason: DiscardReason::OutOfTimeRange { gnss_first, gnss_last },
        } => {
            assert_eq!(*gnss_first, ts("2026-05-01T08:00:00+00:00"));
            assert_eq!(*gnss_last, ts("2026-05-01T08:30:00+00:00"));
        }
        other => panic!("unexpected status: {:?}", other),
    }
}

#[test]
fn punctual_strictly_after_window_discarded() {
    let out = filter_detections_by_time(vec![punctual("2026-05-01T09:00:00+00:00")], &window());
    assert_eq!(out.kept.len(), 0);
    assert_eq!(out.discard_records.len(), 1);
}

#[test]
fn punctual_inside_window_kept() {
    let out = filter_detections_by_time(vec![punctual("2026-05-01T08:15:00+00:00")], &window());
    assert_eq!(out.kept.len(), 1);
    assert_eq!(out.discard_records.len(), 0);
}

#[test]
fn linear_partially_overlapping_discarded() {
    // t_from inside, t_to after window → not fully contained.
    let out = filter_detections_by_time(
        vec![linear(
            "2026-05-01T08:20:00+00:00",
            "2026-05-01T09:00:00+00:00",
        )],
        &window(),
    );
    assert_eq!(out.kept.len(), 0);
    assert_eq!(out.discard_records.len(), 1);
}

#[test]
fn linear_fully_contained_kept() {
    let out = filter_detections_by_time(
        vec![linear(
            "2026-05-01T08:05:00+00:00",
            "2026-05-01T08:25:00+00:00",
        )],
        &window(),
    );
    assert_eq!(out.kept.len(), 1);
    assert_eq!(out.discard_records.len(), 0);
}

#[test]
fn linear_t_to_inside_t_from_before_discarded() {
    // T024: t_to inside window but t_from before → not fully contained.
    let out = filter_detections_by_time(
        vec![linear(
            "2026-05-01T07:30:00+00:00",
            "2026-05-01T08:15:00+00:00",
        )],
        &window(),
    );
    assert_eq!(out.kept.len(), 0);
    assert_eq!(out.discard_records.len(), 1);
}

#[test]
fn linear_endpoints_on_boundary_kept() {
    // T024: both endpoints exactly on window boundaries → accepted.
    let out = filter_detections_by_time(
        vec![linear(
            "2026-05-01T08:00:00+00:00",
            "2026-05-01T08:30:00+00:00",
        )],
        &window(),
    );
    assert_eq!(out.kept.len(), 1);
    assert_eq!(out.discard_records.len(), 0);
}

#[test]
fn warning_emitted_per_discarded() {
    let out = filter_detections_by_time(
        vec![
            punctual("2026-05-01T07:00:00+00:00"),
            punctual("2026-05-01T09:00:00+00:00"),
        ],
        &window(),
    );
    assert_eq!(out.warnings.len(), 2);
}
