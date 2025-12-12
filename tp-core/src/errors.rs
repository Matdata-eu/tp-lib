//! Error types for projection operations

use thiserror::Error;

/// Errors that can occur during GNSS projection operations
#[derive(Error, Debug)]
pub enum ProjectionError {
    #[error("Invalid CRS: {0}")]
    InvalidCrs(String),

    #[error("CRS transformation failed: {0}")]
    TransformFailed(String),

    #[error("Invalid coordinate: {0}")]
    InvalidCoordinate(String),

    #[error("Missing timezone information: {0}")]
    MissingTimezone(String),

    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(String),

    #[error("Empty railway network")]
    EmptyNetwork,

    #[error("Invalid geometry: {0}")]
    InvalidGeometry(String),

    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),

    #[error("GeoJSON error: {0}")]
    GeoJsonError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
