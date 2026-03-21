//! Axum route handlers for the train path review webapp

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::server::state::{AppMode, ConfirmResult, WebAppState};

pub type SharedState = Arc<RwLock<WebAppState>>;

// ---------------------------------------------------------------------------
// Common response helpers
// ---------------------------------------------------------------------------

fn error_response(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({"ok": false, "error": msg.into()}))).into_response()
}

// ---------------------------------------------------------------------------
// GET /api/network
// ---------------------------------------------------------------------------

pub async fn get_network(State(state): State<SharedState>) -> Response {
    let state = state.read().await;

    let path_ids: std::collections::HashSet<&str> = state
        .path
        .segments
        .iter()
        .map(|s| s.netelement_id.as_str())
        .collect();

    let path_map: std::collections::HashMap<&str, (f64, &tp_lib_core::PathOrigin)> = state
        .path
        .segments
        .iter()
        .map(|s| (s.netelement_id.as_str(), (s.probability, &s.origin)))
        .collect();

    let features: Vec<Value> = state
        .network
        .netelements()
        .iter()
        .map(|ne| {
            let coords: Vec<[f64; 2]> = ne.geometry.0.iter().map(|c| [c.x, c.y]).collect();

            let in_path = path_ids.contains(ne.id.as_str());
            let (origin, confidence) = if in_path {
                let (prob, orig) = path_map[ne.id.as_str()];
                let origin_str = match orig {
                    tp_lib_core::PathOrigin::Algorithm => "algorithm",
                    tp_lib_core::PathOrigin::Manual => "manual",
                };
                (Value::String(origin_str.to_string()), json!(prob))
            } else {
                (Value::Null, Value::Null)
            };

            json!({
                "type": "Feature",
                "geometry": {
                    "type": "LineString",
                    "coordinates": coords
                },
                "properties": {
                    "netelement_id": ne.id,
                    "in_path": in_path,
                    "origin": origin,
                    "confidence": confidence
                }
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({
            "type": "FeatureCollection",
            "features": features
        })),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// GET /api/path
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct PathSegmentResponse {
    netelement_id: String,
    probability: f64,
    start_intrinsic: f64,
    end_intrinsic: f64,
    gnss_start_index: usize,
    gnss_end_index: usize,
    origin: String,
    path_index: usize,
}

#[derive(Serialize)]
struct PathResponse {
    segments: Vec<PathSegmentResponse>,
    overall_probability: f64,
    mode: String,
}

pub async fn get_path(State(state): State<SharedState>) -> Response {
    let state = state.read().await;

    let mode_str = match state.mode {
        AppMode::Standalone => "standalone",
        AppMode::Integrated => "integrated",
    };

    let segments = state
        .path
        .segments
        .iter()
        .enumerate()
        .map(|(i, s)| PathSegmentResponse {
            netelement_id: s.netelement_id.clone(),
            probability: s.probability,
            start_intrinsic: s.start_intrinsic,
            end_intrinsic: s.end_intrinsic,
            gnss_start_index: s.gnss_start_index,
            gnss_end_index: s.gnss_end_index,
            origin: match s.origin {
                tp_lib_core::PathOrigin::Algorithm => "algorithm".to_string(),
                tp_lib_core::PathOrigin::Manual => "manual".to_string(),
            },
            path_index: i,
        })
        .collect();

    let resp = PathResponse {
        segments,
        overall_probability: state.path.overall_probability,
        mode: mode_str.to_string(),
    };

    (StatusCode::OK, Json(resp)).into_response()
}

// ---------------------------------------------------------------------------
// PUT /api/path
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PutPathRequest {
    segments: Vec<InputSegment>,
}

#[derive(Deserialize)]
pub struct InputSegment {
    netelement_id: String,
    probability: f64,
    start_intrinsic: f64,
    end_intrinsic: f64,
    gnss_start_index: usize,
    gnss_end_index: usize,
    origin: String,
}

pub async fn put_path(
    State(state): State<SharedState>,
    Json(body): Json<PutPathRequest>,
) -> Response {
    // Validate all netelement IDs exist in the network
    let state_read = state.read().await;
    let network_ids: std::collections::HashSet<&str> = state_read
        .network
        .netelements()
        .iter()
        .map(|ne| ne.id.as_str())
        .collect();

    for seg in &body.segments {
        if !network_ids.contains(seg.netelement_id.as_str()) {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!(
                    "invalid netelement_id: {} not found in loaded network",
                    seg.netelement_id
                ),
            );
        }
        if !(0.0..=1.0).contains(&seg.probability) {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("probability out of range [0,1]: {}", seg.probability),
            );
        }
        if !(0.0..=1.0).contains(&seg.start_intrinsic) || !(0.0..=1.0).contains(&seg.end_intrinsic)
        {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "start_intrinsic/end_intrinsic must be in [0,1]".to_string(),
            );
        }
    }
    drop(state_read);

    // Build new segments list
    let new_segments: Result<Vec<tp_lib_core::AssociatedNetElement>, String> = body
        .segments
        .into_iter()
        .map(|seg| {
            let origin = parse_origin(&seg.origin)?;
            let mut element = tp_lib_core::AssociatedNetElement::new(
                seg.netelement_id,
                seg.probability,
                seg.start_intrinsic,
                seg.end_intrinsic,
                seg.gnss_start_index,
                seg.gnss_end_index,
            )
            .map_err(|e| e.to_string())?;
            element.origin = origin;
            Ok(element)
        })
        .collect();

    let new_segments = match new_segments {
        Ok(s) => s,
        Err(e) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, e),
    };

    let count = new_segments.len();
    let mut state_write = state.write().await;
    state_write.path.segments = new_segments;

    (
        StatusCode::OK,
        Json(json!({"ok": true, "segments_count": count})),
    )
        .into_response()
}

fn parse_origin(s: &str) -> Result<tp_lib_core::PathOrigin, String> {
    match s {
        "manual" => Ok(tp_lib_core::PathOrigin::Manual),
        "algorithm" => Ok(tp_lib_core::PathOrigin::Algorithm),
        other => Err(format!(
            "unknown origin '{}': expected 'algorithm' or 'manual'",
            other
        )),
    }
}

// ---------------------------------------------------------------------------
// POST /api/path/add  — add one segment via snap-insertion (edit::add_segment)
// POST /api/path/remove — remove a segment by ID (edit::remove_segment)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PathEditRequest {
    netelement_id: String,
}

pub async fn post_path_add(
    State(state): State<SharedState>,
    Json(body): Json<PathEditRequest>,
) -> Response {
    let state_read = state.read().await;

    if !state_read
        .network
        .netelements()
        .iter()
        .any(|ne| ne.id == body.netelement_id)
    {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "invalid netelement_id: {} not found in loaded network",
                body.netelement_id
            ),
        );
    }

    let new_path =
        crate::edit::add_segment(&body.netelement_id, &state_read.network, &state_read.path);
    drop(state_read);

    let mut state_write = state.write().await;
    state_write.path = new_path;

    (StatusCode::OK, Json(json!({"ok": true}))).into_response()
}

pub async fn post_path_remove(
    State(state): State<SharedState>,
    Json(body): Json<PathEditRequest>,
) -> Response {
    let new_path = {
        let state_read = state.read().await;
        crate::edit::remove_segment(&body.netelement_id, &state_read.path)
    };

    let mut state_write = state.write().await;
    state_write.path = new_path;

    (StatusCode::OK, Json(json!({"ok": true}))).into_response()
}

// ---------------------------------------------------------------------------
// POST /api/save
// ---------------------------------------------------------------------------

pub async fn post_save(State(state): State<SharedState>) -> Response {
    let state = state.read().await;

    if state.mode == AppMode::Integrated {
        return error_response(
            StatusCode::CONFLICT,
            "save is not available in integrated mode; use /api/confirm instead",
        );
    }

    let output_path = state.output_path.clone().unwrap_or_else(|| {
        let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
        std::path::PathBuf::from(format!("tp_reviewed_{}.csv", ts))
    });

    let mut file = match std::fs::File::create(&output_path) {
        Ok(f) => f,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to write output file: {}", e),
            )
        }
    };

    if let Err(e) = tp_lib_core::write_trainpath_csv(&state.path, &mut file) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to write output file: {}", e),
        );
    }

    let abs_path = output_path
        .canonicalize()
        .unwrap_or(output_path)
        .to_string_lossy()
        .to_string();

    (StatusCode::OK, Json(json!({"ok": true, "path": abs_path}))).into_response()
}

// ---------------------------------------------------------------------------
// POST /api/confirm
// ---------------------------------------------------------------------------

pub async fn post_confirm(State(state): State<SharedState>) -> Response {
    let mut state = state.write().await;

    if state.mode != AppMode::Integrated {
        return error_response(
            StatusCode::CONFLICT,
            "confirm is not available in standalone mode; use /api/save instead",
        );
    }

    match state.confirm_tx.take() {
        None => error_response(StatusCode::CONFLICT, "already confirmed"),
        Some(tx) => {
            let _ = tx.send(ConfirmResult::Confirmed);
            (StatusCode::OK, Json(json!({"ok": true}))).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// POST /api/abort
// ---------------------------------------------------------------------------

pub async fn post_abort(State(state): State<SharedState>) -> Response {
    let mut state = state.write().await;

    if state.mode != AppMode::Integrated {
        return error_response(
            StatusCode::CONFLICT,
            "abort is not available in standalone mode",
        );
    }

    match state.confirm_tx.take() {
        None => error_response(StatusCode::CONFLICT, "already handled"),
        Some(tx) => {
            let _ = tx.send(ConfirmResult::Aborted);
            (StatusCode::OK, Json(json!({"ok": true}))).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// GET /api/gnss
// ---------------------------------------------------------------------------

pub async fn get_gnss(State(state): State<SharedState>) -> Response {
    let state = state.read().await;

    let features: Vec<Value> = match &state.gnss {
        None => vec![],
        Some(positions) => positions
            .iter()
            .map(|p| {
                json!({
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [p.longitude, p.latitude]
                    },
                    "properties": {}
                })
            })
            .collect(),
    };

    (
        StatusCode::OK,
        Json(json!({
            "type": "FeatureCollection",
            "features": features
        })),
    )
        .into_response()
}
