//! Integration tests for punctual anchor injection (T016, US1).
//!
//! Covers SC-001, FR-012: a topological punctual anchor must override the
//! Viterbi candidate set at the anchored GNSS index so the path follows the
//! anchored netelement, even when GNSS evidence prefers a parallel one.

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use geo::LineString;

use tp_lib_core::models::{
    GnssPosition, NetRelation, Netelement, ResolvedAnchor,
};
use tp_lib_core::{calculate_train_path, PathConfig};

fn ts(secs: i64) -> DateTime<FixedOffset> {
    let dt: DateTime<Utc> = Utc.timestamp_opt(1_700_000_000 + secs, 0).unwrap();
    dt.into()
}

/// Build two parallel netelements `NE_MAIN` and `NE_SIDE`, both spanning the
/// same lat/lon corridor, plus a follow-up `NE_END` reachable from both.
/// GNSS observations sit slightly closer to NE_MAIN — the anchor on NE_SIDE
/// must override that.
fn build_parallel_network() -> (Vec<Netelement>, Vec<NetRelation>) {
    let netelements = vec![
        Netelement::new(
            "NE_MAIN".to_string(),
            LineString::from(vec![(4.3500, 50.8500), (4.3520, 50.8500)]),
            "EPSG:4326".to_string(),
        )
        .unwrap(),
        Netelement::new(
            "NE_SIDE".to_string(),
            // ~25m north of NE_MAIN
            LineString::from(vec![(4.3500, 50.8502), (4.3520, 50.8502)]),
            "EPSG:4326".to_string(),
        )
        .unwrap(),
        Netelement::new(
            "NE_END".to_string(),
            LineString::from(vec![(4.3520, 50.8501), (4.3540, 50.8501)]),
            "EPSG:4326".to_string(),
        )
        .unwrap(),
    ];
    let netrelations = vec![
        NetRelation::new(
            "NR_M_E".to_string(),
            "NE_MAIN".to_string(),
            "NE_END".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap(),
        NetRelation::new(
            "NR_S_E".to_string(),
            "NE_SIDE".to_string(),
            "NE_END".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap(),
    ];
    (netelements, netrelations)
}

fn gnss_along_main(n: usize) -> Vec<GnssPosition> {
    // Sit on NE_MAIN, slightly off-axis.
    (0..n)
        .map(|i| {
            let frac = i as f64 / (n - 1).max(1) as f64;
            let lon = 4.3500 + frac * 0.0020;
            let lat = 50.8500;
            GnssPosition::new(lat, lon, ts(i as i64), "EPSG:4326".to_string()).unwrap()
        })
        .collect()
}

#[test]
fn baseline_no_anchor_picks_main() {
    // Sanity check: without anchors, the GNSS evidence prefers NE_MAIN.
    let (netelements, netrelations) = build_parallel_network();
    let gnss = gnss_along_main(5);
    let config = PathConfig::default();
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config)
        .expect("path calc ok");
    let path = result.path.expect("path returned");
    assert!(
        path.segments.iter().any(|s| s.netelement_id == "NE_MAIN"),
        "baseline should choose NE_MAIN, got: {:?}",
        path.segments.iter().map(|s| &s.netelement_id).collect::<Vec<_>>()
    );
}

#[test]
fn punctual_anchor_overrides_gnss_choice() {
    let (netelements, netrelations) = build_parallel_network();
    let gnss = gnss_along_main(5);

    // Anchor at GNSS index 2 → NE_SIDE.
    let config = PathConfig {
        anchors: vec![ResolvedAnchor::Punctual {
            netelement_id: "NE_SIDE".to_string(),
            intrinsic: 0.5,
            gnss_index: 2,
        }],
        ..PathConfig::default()
    };

    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config)
        .expect("path calc ok with anchor");
    let path = result.path.expect("path returned");

    let chosen: Vec<&str> = path
        .segments
        .iter()
        .map(|s| s.netelement_id.as_str())
        .collect();

    assert!(
        chosen.contains(&"NE_SIDE"),
        "anchor must force NE_SIDE into path; got {:?}",
        chosen
    );
    assert!(
        !chosen.contains(&"NE_MAIN"),
        "NE_MAIN should not appear when anchor forces NE_SIDE; got {:?}",
        chosen
    );
}

#[test]
fn multiple_anchors_sorted_by_first_index() {
    use tp_lib_core::detections::prepare_detections_from_loaded;
    use tp_lib_core::models::{Detection, PunctualDetection, TopologicalLocation};

    let (netelements, _netrelations) = build_parallel_network();
    let gnss = gnss_along_main(5);

    // Two detections in reverse timestamp order.
    let det_late = Detection::Punctual(PunctualDetection {
        timestamp: ts(4),
        location: Some(TopologicalLocation {
            netelement_id: "NE_END".to_string(),
            intrinsic: 0.5,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "test.csv".to_string(),
        source_row: 2,
        metadata: Default::default(),
    });
    let det_early = Detection::Punctual(PunctualDetection {
        timestamp: ts(0),
        location: Some(TopologicalLocation {
            netelement_id: "NE_SIDE".to_string(),
            intrinsic: 0.0,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "test.csv".to_string(),
        source_row: 1,
        metadata: Default::default(),
    });

    let prepared = prepare_detections_from_loaded(
        vec![det_late, det_early],
        &gnss,
        &netelements,
        2.5,
    )
    .expect("prepare ok");

    assert_eq!(prepared.anchors.len(), 2);
    // Sorted ascending by first_index.
    assert!(
        prepared.anchors[0].first_index() <= prepared.anchors[1].first_index(),
        "anchors must be sorted by first_index"
    );
}

// ---------- T023 [US2] Linear anchor cases (FR-013, SC-002) ----------

#[test]
fn linear_anchor_window_forces_netelement() {
    // Linear anchor across indices [1..=3] forces NE_SIDE even though GNSS
    // sits closer to NE_MAIN.
    let (netelements, netrelations) = build_parallel_network();
    let gnss = gnss_along_main(5);

    let config = PathConfig {
        anchors: vec![ResolvedAnchor::Linear {
            netelement_id: "NE_SIDE".to_string(),
            start_intrinsic: 0.0,
            end_intrinsic: 1.0,
            gnss_range: 1..=3,
        }],
        ..PathConfig::default()
    };

    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config)
        .expect("path calc ok with linear anchor");
    let path = result.path.expect("path returned");

    let chosen: Vec<&str> = path
        .segments
        .iter()
        .map(|s| s.netelement_id.as_str())
        .collect();

    assert!(
        chosen.contains(&"NE_SIDE"),
        "linear anchor must force NE_SIDE; got {:?}",
        chosen
    );
}

#[test]
fn linear_anchor_window_broader_than_presence_succeeds() {
    // D5: linear window may be larger than the actual netelement traversal.
    // Setting gnss_range = 0..=4 (the entire trajectory) still succeeds
    // even though the train would normally only be on NE_SIDE for some of
    // those indices.
    let (netelements, netrelations) = build_parallel_network();
    let gnss = gnss_along_main(5);

    let config = PathConfig {
        anchors: vec![ResolvedAnchor::Linear {
            netelement_id: "NE_SIDE".to_string(),
            start_intrinsic: 0.0,
            end_intrinsic: 1.0,
            gnss_range: 0..=4,
        }],
        ..PathConfig::default()
    };

    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config)
        .expect("path calc ok with broad linear anchor");
    let path = result.path.expect("path returned");
    assert!(path.segments.iter().any(|s| s.netelement_id == "NE_SIDE"));
}

#[test]
fn linear_anchor_out_of_window_discarded() {
    use chrono::TimeZone;
    use tp_lib_core::detections::prepare_detections_from_loaded;
    use tp_lib_core::models::{Detection, DetectionStatus, DiscardReason, LinearDetection};

    let (netelements, _) = build_parallel_network();
    let gnss = gnss_along_main(5);

    // Linear detection entirely before the GNSS window.
    let before: chrono::DateTime<chrono::FixedOffset> =
        chrono::Utc.timestamp_opt(1_600_000_000, 0).unwrap().into();
    let before2: chrono::DateTime<chrono::FixedOffset> =
        chrono::Utc.timestamp_opt(1_600_000_100, 0).unwrap().into();

    let det = Detection::Linear(LinearDetection {
        t_from: before,
        t_to: before2,
        netelement_id: "NE_SIDE".to_string(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "tst".into(),
        source_row: 1,
        metadata: Default::default(),
    });

    let prepared = prepare_detections_from_loaded(vec![det], &gnss, &netelements, 2.5)
        .expect("prepare ok");

    assert_eq!(prepared.anchors.len(), 0, "out-of-window must not anchor");
    assert_eq!(prepared.records.len(), 1);
    assert!(matches!(
        prepared.records[0].status,
        DetectionStatus::Discarded {
            reason: DiscardReason::OutOfTimeRange { .. }
        }
    ));
}

#[test]
fn linear_and_punctual_anchors_combined() {
    use tp_lib_core::detections::prepare_detections_from_loaded;
    use tp_lib_core::models::{Detection, LinearDetection, PunctualDetection, TopologicalLocation};

    let (netelements, netrelations) = build_parallel_network();
    let gnss = gnss_along_main(5);

    let punc = Detection::Punctual(PunctualDetection {
        timestamp: ts(0),
        location: Some(TopologicalLocation {
            netelement_id: "NE_SIDE".into(),
            intrinsic: 0.0,
        }),
        coordinates: None,
        intrinsic: None,
        id: None,
        source: None,
        source_file: "tst".into(),
        source_row: 1,
        metadata: Default::default(),
    });
    let lin = Detection::Linear(LinearDetection {
        t_from: ts(2),
        t_to: ts(3),
        netelement_id: "NE_SIDE".into(),
        start_intrinsic: 0.5,
        end_intrinsic: 1.0,
        id: None,
        source: None,
        source_file: "tst".into(),
        source_row: 2,
        metadata: Default::default(),
    });

    let prepared = prepare_detections_from_loaded(vec![punc, lin], &gnss, &netelements, 2.5)
        .expect("prepare ok");
    assert_eq!(prepared.anchors.len(), 2, "both anchors retained");

    let config = PathConfig {
        anchors: prepared.anchors,
        ..PathConfig::default()
    };
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config)
        .expect("path calc ok with combined anchors");
    let path = result.path.expect("path");
    let chosen: Vec<&str> = path
        .segments
        .iter()
        .map(|s| s.netelement_id.as_str())
        .collect();
    assert!(chosen.contains(&"NE_SIDE"));
}


