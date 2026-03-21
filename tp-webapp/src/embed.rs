//! Static asset embedding via `rust-embed`
//!
//! All files from `static/` are compiled into the binary at build time.
//! In debug builds (with the `debug-embed` feature), files are re-read from
//! disk on every request, which speeds up frontend iteration.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

/// Embeds all files from the `static/` directory into the binary.
#[derive(RustEmbed)]
#[folder = "static/"]
pub struct EmbeddedAssets;

/// Axum handler that serves any file from the embedded `static/` directory.
///
/// The URI path is used directly as the asset key (leading `/` is stripped).
/// Returns `404 Not Found` when an asset does not exist.
pub async fn static_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    match EmbeddedAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref())],
                content.data,
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
