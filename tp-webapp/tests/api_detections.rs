//! Contract tests for `GET /api/detections` (T033, US4 / 004-train-detections).
//!
//! Read-only endpoint exposing the `PathResult.detection_provenance` of the
//! currently-loaded path. Webapp does not accept new detections; it only
//! visualises what the CLI/library already produced.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::DateTime;
use geo::LineString;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::sync::RwLock;
use tp_lib_core::{
    AssociatedNetElement, DetectionKind, DetectionRecord, DetectionStatus, DiscardReason,
    Netelement, RailwayNetwork, TimestampOrRange, TrainPath,
};
use tp_webapp::server::{
    build_router,
    state::{AppMode, WebAppState},
};

fn ts(s: &str) -> chrono::DateTime<chrono::FixedOffset> {
    DateTime::parse_from_rfc3339(s).unwrap()
}

fn ne(id: &str) -> Netelement {
    Netelement::new(
        id.to_owned(),
        LineString::from(vec![(4.35_f64, 50.85_f64), (4.36, 50.86)]),
        "EPSG:4326".to_owned(),
    )
    .unwrap()
}

fn network() -> RailwayNetwork {
    RailwayNetwork::new(vec![ne("NE001"), ne("NE002")]).unwrap()
}

fn path() -> TrainPath {
    TrainPath::new(
        vec![AssociatedNetElement::new("NE001".to_owned(), 0.9, 0.0, 1.0, 0, 0).unwrap()],
        0.9,
        None,
        None,
    )
    .unwrap()
}

fn punctual_applied() -> DetectionRecord {
    DetectionRecord {
        source_file: "punctual.csv".to_owned(),
        source_row: 1,
        kind: DetectionKind::Punctual,
        timestamp: TimestampOrRange::Single {
            timestamp: ts("2026-05-01T08:15:30+02:00"),
        },
        status: DetectionStatus::Applied {
            netelement_id: "NE001".to_owned(),
            intrinsic: 0.5,
        },
        id: Some("beacon-7".to_owned()),
        source: Some("BTM-A1".to_owned()),
        metadata: BTreeMap::new(),
    }
}

fn linear_applied() -> DetectionRecord {
    DetectionRecord {
        source_file: "linear.csv".to_owned(),
        source_row: 2,
        kind: DetectionKind::Linear,
        timestamp: TimestampOrRange::Range {
            t_from: ts("2026-05-01T08:18:00+02:00"),
            t_to: ts("2026-05-01T08:19:00+02:00"),
        },
        status: DetectionStatus::Applied {
            netelement_id: "NE002".to_owned(),
            intrinsic: 0.5,
        },
        id: Some("bsec-7".to_owned()),
        source: Some("block-B7".to_owned()),
        metadata: BTreeMap::new(),
    }
}

fn discarded_out_of_reach() -> DetectionRecord {
    DetectionRecord {
        source_file: "punctual.csv".to_owned(),
        source_row: 3,
        kind: DetectionKind::Punctual,
        timestamp: TimestampOrRange::Single {
            timestamp: ts("2026-05-01T08:20:00+02:00"),
        },
        status: DetectionStatus::Discarded {
            reason: DiscardReason::OutOfReach {
                nearest_distance_m: 42.0,
                cutoff_m: 2.5,
            },
        },
        id: Some("beacon-9".to_owned()),
        source: None,
        metadata: BTreeMap::new(),
    }
}

fn state_with(records: Vec<DetectionRecord>) -> WebAppState {
    WebAppState {
        network: network(),
        path: path(),
        gnss: None,
        mode: AppMode::Standalone,
        output_path: None,
        confirm_tx: None,
        detection_provenance: records,
    }
}

async fn start_server(state: WebAppState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let shared = Arc::new(RwLock::new(state));
    let router = build_router(shared);
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (format!("http://127.0.0.1:{port}"), handle)
}

#[tokio::test]
async fn empty_provenance_returns_empty_arrays() {
    let (base, _h) = start_server(state_with(Vec::new())).await;
    let resp = Client::new()
        .get(format!("{base}/api/detections"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["punctual"].as_array().unwrap().is_empty());
    assert!(body["linear"].as_array().unwrap().is_empty());
    assert!(body["discarded"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn applied_punctual_in_punctual_array() {
    let (base, _h) = start_server(state_with(vec![punctual_applied()])).await;
    let body: Value = Client::new()
        .get(format!("{base}/api/detections"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let punctual = body["punctual"].as_array().unwrap();
    assert_eq!(punctual.len(), 1);
    let entry = &punctual[0];
    assert_eq!(entry["id"], "beacon-7");
    assert_eq!(entry["source"], "BTM-A1");
    assert_eq!(entry["kind"], "punctual");
    assert_eq!(entry["status"]["status"], "applied");
    assert_eq!(entry["status"]["netelement_id"], "NE001");
    assert_eq!(entry["provenance_index"], 0);
    assert!(body["linear"].as_array().unwrap().is_empty());
    assert!(body["discarded"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn applied_linear_in_linear_array() {
    let (base, _h) = start_server(state_with(vec![linear_applied()])).await;
    let body: Value = Client::new()
        .get(format!("{base}/api/detections"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let linear = body["linear"].as_array().unwrap();
    assert_eq!(linear.len(), 1);
    let entry = &linear[0];
    assert_eq!(entry["kind"], "linear");
    assert_eq!(entry["status"]["status"], "applied");
    assert_eq!(entry["status"]["netelement_id"], "NE002");
    assert!(body["punctual"].as_array().unwrap().is_empty());
    assert!(body["discarded"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn discarded_in_discarded_array_with_reason() {
    let (base, _h) = start_server(state_with(vec![discarded_out_of_reach()])).await;
    let body: Value = Client::new()
        .get(format!("{base}/api/detections"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let discarded = body["discarded"].as_array().unwrap();
    assert_eq!(discarded.len(), 1);
    let entry = &discarded[0];
    assert_eq!(entry["status"]["status"], "discarded");
    assert_eq!(entry["status"]["reason"]["kind"], "out_of_reach");
    assert_eq!(entry["status"]["reason"]["cutoff_m"], 2.5);
    assert!(body["punctual"].as_array().unwrap().is_empty());
    assert!(body["linear"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn mixed_records_partitioned_correctly_with_indexes() {
    let records = vec![
        punctual_applied(),
        discarded_out_of_reach(),
        linear_applied(),
    ];
    let (base, _h) = start_server(state_with(records)).await;
    let body: Value = Client::new()
        .get(format!("{base}/api/detections"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body["punctual"].as_array().unwrap().len(), 1);
    assert_eq!(body["linear"].as_array().unwrap().len(), 1);
    assert_eq!(body["discarded"].as_array().unwrap().len(), 1);
    assert_eq!(body["punctual"][0]["provenance_index"], 0);
    assert_eq!(body["discarded"][0]["provenance_index"], 1);
    assert_eq!(body["linear"][0]["provenance_index"], 2);
}
