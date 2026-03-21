//! Axum router construction and TCP port binding
//!
//! This module wires all routes behind a shared `Arc<RwLock<WebAppState>>` and
//! provides a helper to find a free port in the configured range.

pub mod routes;
pub mod state;

use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::sync::RwLock;

use crate::embed::static_handler;
use crate::server::routes::{
    get_gnss, get_network, get_path, post_abort, post_confirm, post_path_add, post_path_remove,
    post_save, put_path,
};
use crate::server::state::WebAppState;
use crate::WebAppError;

pub type SharedState = Arc<RwLock<WebAppState>>;

/// Build the axum [`Router`] with all routes wired up.
pub fn build_router(state: SharedState) -> Router {
    Router::new()
        // SPA shell
        .route("/", get(serve_index))
        // API routes
        .route("/api/network", get(get_network))
        .route("/api/path", get(get_path).put(put_path))
        .route("/api/path/add", post(post_path_add))
        .route("/api/path/remove", post(post_path_remove))
        .route("/api/save", post(post_save))
        .route("/api/confirm", post(post_confirm))
        .route("/api/abort", post(post_abort))
        .route("/api/gnss", get(get_gnss))
        // Static assets (JS, CSS, leaflet, etc.)
        .fallback(static_handler)
        .with_state(state)
}

/// Serve `index.html` from the embedded static assets.
async fn serve_index() -> axum::response::Response {
    static_handler(axum::http::Uri::from_static("/index.html")).await
}

/// Try to bind a `TcpListener` on `127.0.0.1` to each port in `ports` in order.
///
/// Returns the first successfully bound listener together with its actual port.
///
/// # Errors
///
/// Returns [`WebAppError::PortUnavailable`] when none of the ports could be bound.
pub fn bind_port(ports: std::ops::RangeInclusive<u16>) -> Result<(TcpListener, u16), WebAppError> {
    for port in ports {
        let addr: SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .expect("valid address");
        match TcpListener::bind(addr) {
            Ok(listener) => {
                listener
                    .set_nonblocking(true)
                    .map_err(WebAppError::RuntimeError)?;
                return Ok((listener, port));
            }
            Err(_) => continue,
        }
    }
    Err(WebAppError::PortUnavailable)
}
