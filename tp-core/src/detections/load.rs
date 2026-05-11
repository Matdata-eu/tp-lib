//! Detection loading & format dispatch (T011).
//!
//! Public entry point: [`load_detections`].

use std::path::Path;

use crate::io::csv::detections as csv_loader;
use crate::io::geojson::detections as geojson_loader;
use crate::models::{Detection, DetectionKind};

use super::error::DetectionError;

/// Load detections from a file. Format is inferred from the extension:
/// `.csv` → CSV, `.geojson` / `.json` → GeoJSON. Anything else returns
/// [`DetectionError::UnsupportedExtension`].
///
/// `expected_kind` corresponds to the CLI flag (`--punctual-detections` or
/// `--linear-detections`). The parser validates that the file content matches
/// the expected kind.
pub fn load_detections(
    path: &Path,
    expected_kind: DetectionKind,
) -> Result<Vec<Detection>, DetectionError> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "csv" => csv_loader::load(path, expected_kind),
        "geojson" | "json" => geojson_loader::load(path, expected_kind),
        _ => Err(DetectionError::UnsupportedExtension(ext)),
    }
}
