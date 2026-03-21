//! Integration tests for the full webapp workflows (T012, T024, T031).
//!
//! Each test exercises a complete user story end-to-end through HTTP:
//!   US1 — standalone review + save
//!   US2 — integrated mode confirm / abort
//!   US3 — GNSS overlay

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
// Shared helpers
// ---------------------------------------------------------------------------

fn ne(id: &str) -> Netelement {
    Netelement::new(
        id.to_owned(),
        LineString::from(vec![(4.35_f64, 50.85_f64), (4.36, 50.86)]),
        "EPSG:4326".to_owned(),
    )
    .expect("valid netelement")
}

fn network() -> RailwayNetwork {
    RailwayNetwork::new(vec![ne("NE001"), ne("NE002")]).expect("valid network")
}

fn path_with(id: &str) -> TrainPath {
    let seg = AssociatedNetElement::new(id.to_owned(), 0.9, 0.0, 1.0, 0, 0).expect("valid seg");
    TrainPath::new(vec![seg], 0.9, None, None).expect("valid path")
}

fn gnss(lat: f64, lon: f64) -> GnssPosition {
    let ts = DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z").unwrap();
    GnssPosition::new(lat, lon, ts, "EPSG:4326".to_owned()).expect("valid gnss")
}

async fn start(state: WebAppState) -> (String, tokio::task::JoinHandle<()>) {
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

// ---------------------------------------------------------------------------
// T012 — US1: standalone review and save workflow
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_us1_standalone_get_network_put_path_save() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("reviewed.csv");

    let state = WebAppState {
        network: network(),
        path: path_with("NE001"),
        gnss: None,
        mode: AppMode::Standalone,
        output_path: Some(out.clone()),
        confirm_tx: None,
    };
    let (base, _h) = start(state).await;
    let c = Client::new();

    // 1. Network loads as GeoJSON FeatureCollection
    let net: Value = c
        .get(format!("{base}/api/network"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(net["type"], "FeatureCollection");
    assert_eq!(net["features"].as_array().unwrap().len(), 2);

    // 2. PUT a segment into the path
    let put = c
        .put(format!("{base}/api/path"))
        .json(&json!({
            "segments": [{
                "netelement_id": "NE001",
                "probability": 0.87,
                "start_intrinsic": 0.0,
                "end_intrinsic": 1.0,
                "gnss_start_index": 0,
                "gnss_end_index": 0,
                "origin": "manual"
            }]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(put.status(), 200);

    // 3. GET /api/path reflects the updated segment
    let path_resp: Value = c
        .get(format!("{base}/api/path"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let segs = path_resp["segments"].as_array().unwrap();
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0]["netelement_id"], "NE001");
    assert_eq!(segs[0]["origin"], "manual");

    // 4. Network now reflects NE001 as in_path
    let net2: Value = c
        .get(format!("{base}/api/network"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let ne001_feat = net2["features"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["properties"]["netelement_id"] == "NE001")
        .unwrap();
    assert_eq!(ne001_feat["properties"]["in_path"], true);

    // 5. POST /api/save writes the CSV file
    let save: Value = c
        .post(format!("{base}/api/save"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(save["ok"], true);
    assert!(out.exists());

    let content = std::fs::read_to_string(&out).unwrap();
    assert!(content.contains("NE001"), "CSV must contain segment id");
}

#[tokio::test]
async fn test_us1_save_is_not_available_in_integrated_mode() {
    let (tx, _rx) = oneshot::channel::<ConfirmResult>();
    let state = WebAppState {
        network: network(),
        path: path_with("NE001"),
        gnss: None,
        mode: AppMode::Integrated,
        output_path: None,
        confirm_tx: Some(tx),
    };
    let (base, _h) = start(state).await;

    let resp = Client::new()
        .post(format!("{base}/api/save"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);
}

// ---------------------------------------------------------------------------
// T024 — US2: integrated mode confirm / abort
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_us2_confirm_flow() {
    let (tx, rx) = oneshot::channel::<ConfirmResult>();
    let state = WebAppState {
        network: network(),
        path: path_with("NE001"),
        gnss: None,
        mode: AppMode::Integrated,
        output_path: None,
        confirm_tx: Some(tx),
    };
    let (base, _h) = start(state).await;
    let c = Client::new();

    // /api/save should be rejected in integrated mode
    let save = c.post(format!("{base}/api/save")).send().await.unwrap();
    assert_eq!(save.status(), 409);

    // /api/confirm should succeed and signal the channel
    let confirm: Value = c
        .post(format!("{base}/api/confirm"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(confirm["ok"], true);

    assert_eq!(rx.await.unwrap(), ConfirmResult::Confirmed);
}

#[tokio::test]
async fn test_us2_abort_flow() {
    let (tx, rx) = oneshot::channel::<ConfirmResult>();
    let state = WebAppState {
        network: network(),
        path: path_with("NE001"),
        gnss: None,
        mode: AppMode::Integrated,
        output_path: None,
        confirm_tx: Some(tx),
    };
    let (base, _h) = start(state).await;

    let abort: Value = Client::new()
        .post(format!("{base}/api/abort"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(abort["ok"], true);

    assert_eq!(rx.await.unwrap(), ConfirmResult::Aborted);
}

#[tokio::test]
async fn test_us2_double_confirm_is_conflict() {
    let (tx, _rx) = oneshot::channel::<ConfirmResult>();
    let state = WebAppState {
        network: network(),
        path: path_with("NE001"),
        gnss: None,
        mode: AppMode::Integrated,
        output_path: None,
        confirm_tx: Some(tx),
    };
    let (base, _h) = start(state).await;
    let c = Client::new();

    let first = c.post(format!("{base}/api/confirm")).send().await.unwrap();
    assert_eq!(first.status(), 200);

    let second = c.post(format!("{base}/api/confirm")).send().await.unwrap();
    assert_eq!(second.status(), 409);
}

// ---------------------------------------------------------------------------
// T031 — US3: GNSS overlay
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_us3_gnss_loaded_visible_on_map() {
    let state = WebAppState {
        network: network(),
        path: path_with("NE001"),
        gnss: Some(vec![gnss(50.85, 4.35), gnss(50.86, 4.36)]),
        mode: AppMode::Standalone,
        output_path: None,
        confirm_tx: None,
    };
    let (base, _h) = start(state).await;

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
    assert_eq!(features.len(), 2, "two GNSS positions should appear");
    for f in features {
        assert_eq!(f["type"], "Feature");
        assert_eq!(f["geometry"]["type"], "Point");
        let coords = &f["geometry"]["coordinates"];
        assert!(coords[0].as_f64().is_some());
        assert!(coords[1].as_f64().is_some());
    }
}

#[tokio::test]
async fn test_us3_gnss_not_loaded_returns_empty_collection() {
    let state = WebAppState {
        network: network(),
        path: path_with("NE001"),
        gnss: None,
        mode: AppMode::Standalone,
        output_path: None,
        confirm_tx: None,
    };
    let (base, _h) = start(state).await;

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
