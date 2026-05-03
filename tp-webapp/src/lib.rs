//! Train Path Review Webapp — public API
//!
//! This crate exposes two entry points consumed by `tp-cli`:
//!
//! - [`run_webapp_standalone`]: launch the webapp in standalone mode where the
//!   user reviews and saves an edited path to a CSV file.
//! - [`run_webapp_integrated`]: launch the webapp in integrated mode where the
//!   user confirms or aborts the path; the result is returned to the caller
//!   so the CLI pipeline can continue or exit.

pub mod edit;
pub mod embed;
pub mod server;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tp_lib_core::{GnssPosition, RailwayNetwork, TrainPath};

use crate::server::bind_port;
use crate::server::build_router;
use crate::server::state::{AppMode, ConfirmResult, WebAppState};

pub use crate::server::state::ConfirmResult as WebConfirmResult;

/// Errors that can occur while running the webapp.
#[derive(Debug, thiserror::Error)]
pub enum WebAppError {
    /// No port in the scan range (8765–8774) was available.
    #[error("no available port in range 8765–8774")]
    PortUnavailable,

    /// The tokio runtime failed to start.
    #[error("failed to start async runtime: {0}")]
    RuntimeError(#[from] std::io::Error),

    /// The confirm/abort channel was closed before a result was received
    /// (only relevant in integrated mode).
    #[error("confirm channel closed before result was received")]
    ChannelClosed,

    /// Feature not yet implemented (used during TDD stub phase).
    #[error("not implemented")]
    NotImplemented,
}

/// Default port-scan range used when no explicit port is given.
const DEFAULT_PORTS: std::ops::RangeInclusive<u16> = 8765..=8774;

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Launch the webapp in **standalone** mode.
///
/// The server runs until the process is terminated (Ctrl+C / SIGINT).
/// The user can save the edited path to `output_path` (or a generated default)
/// via `POST /api/save` without stopping the server.
///
/// # Parameters
///
/// - `network` – loaded railway network (read-only after startup)
/// - `path` – initial train path to display and edit
/// - `output_path` – target CSV file for saves; a timestamped default is used when `None`
/// - `gnss` – optional GNSS positions shown as map overlay
/// - `port` – preferred starting port; falls back to the default range when 0 or unavailable
/// - `open_browser` – if `true`, open the default browser automatically after binding
pub fn run_webapp_standalone(
    network: &RailwayNetwork,
    path: TrainPath,
    output_path: Option<PathBuf>,
    gnss: Option<Vec<GnssPosition>>,
    port: u16,
    open_browser: bool,
) -> Result<(), WebAppError> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let port_range = if port == 0 {
            DEFAULT_PORTS
        } else {
            port..=port.saturating_add(9)
        };

        let (listener, bound_port) = bind_port(port_range)?;
        let listener = tokio::net::TcpListener::from_std(listener)?;

        let url = format!("http://127.0.0.1:{}", bound_port);
        println!("Train path review webapp running at {}", url);

        if open_browser {
            let _ = open::that(&url);
        }

        let state = Arc::new(RwLock::new(WebAppState {
            network: network.clone(),
            path,
            gnss,
            mode: AppMode::Standalone,
            output_path,
            confirm_tx: None,
            detection_provenance: Vec::new(),
        }));

        let router = build_router(state);
        axum::serve(listener, router)
            .await
            .map_err(WebAppError::RuntimeError)
    })
}

/// Launch the webapp in **integrated** mode.
///
/// The server runs until the user clicks Confirm or Abort (`POST /api/confirm`
/// or `POST /api/abort`).  The function blocks until one of these is received,
/// then returns the result together with the (possibly edited) path.
///
/// # Parameters
///
/// - `network` – loaded railway network (read-only after startup)
/// - `path` – initial train path from the pipeline's path calculation step
/// - `gnss` – GNSS positions from the pipeline (shown as map overlay under US2 AS-2)
/// - `port` – preferred starting port; falls back to the default range when 0 or unavailable
/// - `open_browser` – if `true`, open the default browser automatically after binding
pub fn run_webapp_integrated(
    network: &RailwayNetwork,
    path: TrainPath,
    gnss: Option<Vec<GnssPosition>>,
    port: u16,
    open_browser: bool,
) -> Result<(ConfirmResult, TrainPath), WebAppError> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let port_range = if port == 0 {
            DEFAULT_PORTS
        } else {
            port..=port.saturating_add(9)
        };

        let (listener, bound_port) = bind_port(port_range)?;
        let listener = tokio::net::TcpListener::from_std(listener)?;

        let url = format!("http://127.0.0.1:{}", bound_port);
        println!("Train path review webapp running at {}", url);

        if open_browser {
            let _ = open::that(&url);
        }

        let (confirm_tx, confirm_rx) = tokio::sync::oneshot::channel::<ConfirmResult>();

        let state = Arc::new(RwLock::new(WebAppState {
            network: network.clone(),
            path,
            gnss,
            mode: AppMode::Integrated,
            output_path: None,
            confirm_tx: Some(confirm_tx),
            detection_provenance: Vec::new(),
        }));

        let router = build_router(state.clone());

        // Use a CancellationToken so we can shut down the server after confirm/abort.
        let cancel = tokio_util::sync::CancellationToken::new();
        let cancel_clone = cancel.clone();

        let serve_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move { cancel_clone.cancelled().await })
                .await
        });

        // Wait for the user's decision.
        let result = confirm_rx.await.map_err(|_| WebAppError::ChannelClosed)?;

        // Trigger graceful shutdown.
        cancel.cancel();
        let _ = serve_handle.await;

        // Return confirmed result + (possibly edited) path snapshot.
        let final_path = state.read().await.path.clone();
        Ok((result, final_path))
    })
}
