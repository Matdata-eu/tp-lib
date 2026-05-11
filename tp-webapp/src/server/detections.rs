//! `GET /api/detections` route handler (T034, US4 / 004-train-detections).
//!
//! Read-only endpoint exposing `WebAppState.detection_provenance` partitioned
//! into applied-punctual, applied-linear, and discarded arrays for the path
//! review webapp's detection overlay. The webapp does not accept new
//! detections; this endpoint is purely visualisation of what the
//! library/CLI already produced (per spec clarification Q2).

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use tp_lib_core::{DetectionKind, DetectionRecord, DetectionStatus};

use crate::server::routes::SharedState;

/// Serialise a [`DetectionRecord`] together with its index in the
/// `PathResult.detection_provenance` vector.
fn record_to_json(index: usize, record: &DetectionRecord) -> Result<Value, serde_json::Error> {
    let mut value = serde_json::to_value(record)?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert("provenance_index".to_owned(), json!(index));
    }
    Ok(value)
}

/// Partition `detection_provenance` into three arrays and return as JSON.
///
/// Response shape:
/// ```jsonc
/// {
///   "punctual":  [<applied punctual records>],
///   "linear":    [<applied linear records>],
///   "discarded": [<discarded records, both kinds>]
/// }
/// ```
///
/// Each entry includes a `provenance_index` field pointing back into the
/// original `PathResult.detection_provenance` vector.
pub async fn get_detections(State(state): State<SharedState>) -> Response {
    let state = state.read().await;

    let mut punctual: Vec<Value> = Vec::new();
    let mut linear: Vec<Value> = Vec::new();
    let mut discarded: Vec<Value> = Vec::new();

    for (index, record) in state.detection_provenance.iter().enumerate() {
        let value = match record_to_json(index, record) {
            Ok(value) => value,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal" })),
                )
                    .into_response();
            }
        };
        match (&record.status, record.kind) {
            (DetectionStatus::Discarded { .. }, _) => discarded.push(value),
            (_, DetectionKind::Punctual) => punctual.push(value),
            (_, DetectionKind::Linear) => linear.push(value),
        }
    }

    Json(json!({
        "punctual": punctual,
        "linear": linear,
        "discarded": discarded,
    }))
    .into_response()
}
