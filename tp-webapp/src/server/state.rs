//! Shared application state for the webapp server

use std::path::PathBuf;
use tokio::sync::oneshot;
use tp_lib_core::{DetectionRecord, GnssPosition, RailwayNetwork, TrainPath};

/// Signals whether the user confirmed or aborted the integrated review.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmResult {
    Confirmed,
    Aborted,
}

/// Distinguishes standalone (save-to-file) from integrated (confirm/abort) mode.
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Standalone,
    Integrated,
}

/// Shared mutable state held behind `Arc<RwLock<WebAppState>>`.
pub struct WebAppState {
    /// The loaded railway network (read-only after startup).
    pub network: RailwayNetwork,

    /// The current train path (edited by the user via the webapp).
    pub path: TrainPath,

    /// Optional GNSS positions for map overlay.
    pub gnss: Option<Vec<GnssPosition>>,

    /// Whether the server is in standalone or integrated mode.
    pub mode: AppMode,

    /// Target file for POST /api/save (standalone mode only).
    /// When `None`, a default name is derived from the current timestamp.
    pub output_path: Option<PathBuf>,

    /// One-shot sender used to unblock `run_webapp_integrated` (integrated mode only).
    /// Taken (set to `None`) when the first confirm/abort request arrives.
    pub confirm_tx: Option<oneshot::Sender<ConfirmResult>>,

    /// Detection provenance records loaded from the source `PathResult` (US4).
    /// Empty when the path was computed without detection inputs.
    pub detection_provenance: Vec<DetectionRecord>,
}
