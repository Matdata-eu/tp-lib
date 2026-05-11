//! Detection error types (T006).
//!
//! Thin domain error per Constitution VIII (typed, fail-fast).

use chrono::{DateTime, FixedOffset};
use thiserror::Error;

/// Errors produced by the detections pipeline (load/validate/filter/resolve).
///
/// `ConflictingDetections` and `UnknownNetelement` are FATAL per data-model.md
/// and abort path calculation. All other variants surface either as parse-time
/// failures or as recoverable `DiscardReason`s in `DetectionRecord`.
#[derive(Debug, Error)]
pub enum DetectionError {
    /// Input file extension is not `.csv`, `.geojson`, or `.json`.
    #[error("unsupported detections file extension: {0:?}")]
    UnsupportedExtension(String),

    /// Required column / property missing or malformed schema.
    #[error("invalid detections schema: {0}")]
    InvalidSchema(String),

    /// Generic parser failure (CSV row, GeoJSON feature).
    #[error("detection parse error at {source_file}:{source_row}: {message}")]
    Parse {
        source_file: String,
        source_row: usize,
        message: String,
    },

    /// Timestamp could not be parsed or lacked a timezone offset.
    #[error("invalid timestamp at {source_file}:{source_row}: {message}")]
    InvalidTimestamp {
        source_file: String,
        source_row: usize,
        message: String,
    },

    /// `intrinsic` / `start_intrinsic` / `end_intrinsic` not in `[0, 1]`.
    #[error("invalid intrinsic value {value} at {source_file}:{source_row} (must be in [0, 1])")]
    InvalidIntrinsic {
        source_file: String,
        source_row: usize,
        value: f64,
    },

    /// Coordinate row supplied without a `crs` column / property.
    #[error(
        "missing crs at {source_file}:{source_row}: coordinate detections require an explicit CRS"
    )]
    MissingCrs {
        source_file: String,
        source_row: usize,
    },

    /// Two punctual detections at the same timestamp resolve to different netelements (FATAL, D4).
    #[error(
        "conflicting detections at {timestamp}: netelement '{netelement_a}' vs '{netelement_b}'"
    )]
    ConflictingDetections {
        timestamp: DateTime<FixedOffset>,
        netelement_a: String,
        netelement_b: String,
    },

    /// Linear detection has `t_from > t_to`.
    #[error(
        "invalid time range at {source_file}:{source_row}: t_from ({t_from}) is after t_to ({t_to})"
    )]
    InvalidTimeRange {
        source_file: String,
        source_row: usize,
        t_from: DateTime<FixedOffset>,
        t_to: DateTime<FixedOffset>,
    },

    /// `netelement_id` does not exist in the supplied railway network (FATAL, FR-006).
    #[error("unknown netelement '{netelement_id}' at {source_file}:{source_row}")]
    UnknownNetelement {
        source_file: String,
        source_row: usize,
        netelement_id: String,
    },

    /// Internal invariant violation: a detection resolved twice.
    #[error("duplicate resolution for detection at {source_file}:{source_row}")]
    DuplicateResolution {
        source_file: String,
        source_row: usize,
    },

    /// Wrapped `std::io::Error`.
    #[error("detection IO error: {0}")]
    Io(#[from] std::io::Error),
}
