//! Unit-level tests for the webapp API route handlers (T011, T023, T030).
//!
//! These tests spin up a real in-process axum server bound to a random port and
//! exercise every route via `reqwest`. No mocking is used; the full handler
//! stack runs, including state access and JSON serialisation.

use std::sync::Arc;

use chrono::DateTime;
use geo::LineString;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::{oneshot, RwLock};
use tp_lib_core::{AssociatedNetElement, GnssPosition, Netelement, RailwayNetwork, TrainPath};
use tp_webapp::server::{
    build_router,
    state::{AppMode, ConfirmResult, WebAppState},
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn test_netelement(id: &str) -> Netelement {
    Netelement::new(
        id.to_owned(),
        LineString::from(vec![(4.35_f64, 50.85_f64), (4.36, 50.86)]),
        "EPSG:4326".to_owned(),
    )
    .expect("valid netelement")
}

fn test_network() -> RailwayNetwork {
    RailwayNetwork::new(vec![test_netelement("NE001"), test_netelement("NE002")])
        .expect("valid network")
}

fn test_segment(id: &str) -> AssociatedNetElement {
    AssociatedNetElement::new(id.to_owned(), 0.9, 0.0, 1.0, 0, 0).expect("valid segment")
}

fn test_path(segment_ids: &[&str]) -> TrainPath {
    let segs: Vec<AssociatedNetElement> = segment_ids.iter().map(|id| test_segment(id)).collect();
    TrainPath::new(segs, 0.9, None, None).expect("valid train path")
}

fn gnss_pos(lat: f64, lon: f64) -> GnssPosition {
    let ts = DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z").unwrap();
    GnssPosition::new(lat, lon, ts, "EPSG:4326".to_owned()).expect("valid gnss position")
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

fn standalone_state() -> WebAppState {
    WebAppState {
        network: test_network(),
        path: test_path(&["NE001"]),
        gnss: None,
        mode: AppMode::Standalone,
        output_path: None,
        confirm_tx: None,
        detection_provenance: Vec::new(),
    }
}

fn integrated_state(tx: oneshot::Sender<ConfirmResult>) -> WebAppState {
    WebAppState {
        network: test_network(),
        path: test_path(&["NE001"]),
        gnss: None,
        mode: AppMode::Integrated,
        output_path: None,
        confirm_tx: Some(tx),
        detection_provenance: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// T011 — GET /api/network
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_network_is_feature_collection() {
    let (base, _h) = start_server(standalone_state()).await;

    let resp: Value = Client::new()
        .get(format!("{base}/api/network"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["type"], "FeatureCollection");
    assert_eq!(resp["features"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_network_netelement_not_in_path_has_null_properties() {
    let (base, _h) = start_server(standalone_state()).await;

    let resp: Value = Client::new()
        .get(format!("{base}/api/network"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let ne002 = resp["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["properties"]["netelement_id"] == "NE002")
        .expect("NE002 in response");
    assert_eq!(ne002["properties"]["in_path"], false);
    assert!(ne002["properties"]["origin"].is_null());
    assert!(ne002["properties"]["confidence"].is_null());
}

#[tokio::test]
async fn test_get_network_netelement_in_path_has_origin_and_confidence() {
    let state = WebAppState {
        path: test_path(&["NE001"]),
        ..standalone_state()
    };
    let (base, _h) = start_server(state).await;

    let resp: Value = Client::new()
        .get(format!("{base}/api/network"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let ne001 = resp["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["properties"]["netelement_id"] == "NE001")
        .expect("NE001 in response");

    assert_eq!(ne001["properties"]["in_path"], true);
    assert_eq!(ne001["properties"]["origin"], "algorithm");
    assert!(ne001["properties"]["confidence"].as_f64().is_some());
}

// ---------------------------------------------------------------------------
// T011 — GET /api/path
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_path_returns_standalone_mode_and_path_index() {
    let state = WebAppState {
        path: test_path(&["NE001"]),
        ..standalone_state()
    };
    let (base, _h) = start_server(state).await;

    let resp: Value = Client::new()
        .get(format!("{base}/api/path"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["mode"], "standalone");
    let segs = resp["segments"].as_array().unwrap();
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0]["path_index"], 0);
    assert_eq!(segs[0]["netelement_id"], "NE001");
}

#[tokio::test]
async fn test_get_path_returns_integrated_mode() {
    let (tx, _rx) = oneshot::channel::<ConfirmResult>();
    let (base, _h) = start_server(integrated_state(tx)).await;

    let resp: Value = Client::new()
        .get(format!("{base}/api/path"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["mode"], "integrated");
}

// ---------------------------------------------------------------------------
// T011 — PUT /api/path
// ---------------------------------------------------------------------------

fn segment_body(ne_id: &str) -> Value {
    json!({
        "segments": [{
            "netelement_id": ne_id,
            "probability": 0.85,
            "start_intrinsic": 0.0,
            "end_intrinsic": 1.0,
            "gnss_start_index": 0,
            "gnss_end_index": 0,
            "origin": "algorithm"
        }]
    })
}

#[tokio::test]
async fn test_put_path_replaces_segments() {
    let (base, _h) = start_server(standalone_state()).await;

    let resp = Client::new()
        .put(format!("{base}/api/path"))
        .json(&segment_body("NE001"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let json: Value = resp.json().await.unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["segments_count"], 1);
}

#[tokio::test]
async fn test_put_path_422_on_unknown_netelement() {
    let (base, _h) = start_server(standalone_state()).await;

    let resp = Client::new()
        .put(format!("{base}/api/path"))
        .json(&segment_body("NE_UNKNOWN"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 422);
    let json: Value = resp.json().await.unwrap();
    assert_eq!(json["ok"], false);
}

#[tokio::test]
async fn test_put_path_422_on_probability_out_of_range() {
    let (base, _h) = start_server(standalone_state()).await;

    let body = json!({
        "segments": [{
            "netelement_id": "NE001",
            "probability": 1.5,
            "start_intrinsic": 0.0,
            "end_intrinsic": 1.0,
            "gnss_start_index": 0,
            "gnss_end_index": 0,
            "origin": "algorithm"
        }]
    });

    let resp = Client::new()
        .put(format!("{base}/api/path"))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 422);
}

#[tokio::test]
async fn test_put_path_422_on_unknown_origin() {
    let (base, _h) = start_server(standalone_state()).await;

    let body = json!({
        "segments": [{
            "netelement_id": "NE001",
            "probability": 0.9,
            "start_intrinsic": 0.0,
            "end_intrinsic": 1.0,
            "gnss_start_index": 0,
            "gnss_end_index": 0,
            "origin": "manul"  // typo: not a valid origin
        }]
    });

    let resp = Client::new()
        .put(format!("{base}/api/path"))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 422);
    let json: Value = resp.json().await.unwrap();
    assert_eq!(json["ok"], false);
}

// ---------------------------------------------------------------------------
// T011 — POST /api/save
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_post_save_creates_file_and_returns_ok() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output.csv");

    let state = WebAppState {
        path: test_path(&["NE001"]),
        output_path: Some(out.clone()),
        ..standalone_state()
    };
    let (base, _h) = start_server(state).await;

    let resp = Client::new()
        .post(format!("{base}/api/save"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let json: Value = resp.json().await.unwrap();
    assert_eq!(json["ok"], true);
    assert!(json["path"].is_string());
    assert!(out.exists());
}

#[tokio::test]
async fn test_post_save_409_in_integrated_mode() {
    let (tx, _rx) = oneshot::channel::<ConfirmResult>();
    let (base, _h) = start_server(integrated_state(tx)).await;

    let resp = Client::new()
        .post(format!("{base}/api/save"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409);
}

// ---------------------------------------------------------------------------
// T023 — POST /api/confirm and POST /api/abort
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_post_confirm_signals_confirmed_on_channel() {
    let (tx, rx) = oneshot::channel::<ConfirmResult>();
    let (base, _h) = start_server(integrated_state(tx)).await;

    let resp = Client::new()
        .post(format!("{base}/api/confirm"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<Value>().await.unwrap()["ok"], true);
    assert_eq!(rx.await.unwrap(), ConfirmResult::Confirmed);
}

#[tokio::test]
async fn test_post_abort_signals_aborted_on_channel() {
    let (tx, rx) = oneshot::channel::<ConfirmResult>();
    let (base, _h) = start_server(integrated_state(tx)).await;

    let resp = Client::new()
        .post(format!("{base}/api/abort"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<Value>().await.unwrap()["ok"], true);
    assert_eq!(rx.await.unwrap(), ConfirmResult::Aborted);
}

#[tokio::test]
async fn test_post_confirm_409_when_tx_already_consumed() {
    // confirm_tx is None → already consumed
    let state = WebAppState {
        mode: AppMode::Integrated,
        confirm_tx: None,
        detection_provenance: Vec::new(),
        ..standalone_state()
    };
    let (base, _h) = start_server(state).await;

    let resp = Client::new()
        .post(format!("{base}/api/confirm"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409);
}

#[tokio::test]
async fn test_post_confirm_409_in_standalone_mode() {
    let (base, _h) = start_server(standalone_state()).await;

    let resp = Client::new()
        .post(format!("{base}/api/confirm"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409);
}

#[tokio::test]
async fn test_post_abort_409_in_standalone_mode() {
    let (base, _h) = start_server(standalone_state()).await;

    let resp = Client::new()
        .post(format!("{base}/api/abort"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409);
}

#[tokio::test]
async fn test_post_abort_409_already_handled_when_tx_consumed() {
    // confirm_tx is None → session already handled (confirmed or aborted)
    let state = WebAppState {
        mode: AppMode::Integrated,
        confirm_tx: None,
        detection_provenance: Vec::new(),
        ..standalone_state()
    };
    let (base, _h) = start_server(state).await;

    let resp = Client::new()
        .post(format!("{base}/api/abort"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 409);
    let json: Value = resp.json().await.unwrap();
    assert_eq!(json["error"], "already handled");
}

// ---------------------------------------------------------------------------
// T030 — GET /api/gnss
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_gnss_returns_point_features_when_loaded() {
    let state = WebAppState {
        gnss: Some(vec![gnss_pos(50.85, 4.35), gnss_pos(50.86, 4.36)]),
        ..standalone_state()
    };
    let (base, _h) = start_server(state).await;

    let resp: Value = Client::new()
        .get(format!("{base}/api/gnss"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["type"], "FeatureCollection");
    let features = resp["features"].as_array().unwrap();
    assert_eq!(features.len(), 2);
    assert_eq!(features[0]["geometry"]["type"], "Point");
    assert!(features[0]["geometry"]["coordinates"][0].as_f64().is_some());
    assert!(features[0]["geometry"]["coordinates"][1].as_f64().is_some());
}

#[tokio::test]
async fn test_get_gnss_returns_empty_feature_collection_when_not_loaded() {
    let (base, _h) = start_server(standalone_state()).await;

    let resp: Value = Client::new()
        .get(format!("{base}/api/gnss"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["type"], "FeatureCollection");
    assert_eq!(resp["features"].as_array().unwrap().len(), 0);
}
