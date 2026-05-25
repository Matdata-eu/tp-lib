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

    #[error("Path calculation failed: {reason}")]
    PathCalculationFailed { reason: String },

    #[error("No navigable path found between netelements")]
    NoNavigablePath,

    #[error("Invalid netrelation: {0}")]
    InvalidNetRelation(String),

    /// GNSS input was empty, unparseable, or otherwise unusable before any
    /// retrieval attempt could be made.
    #[error("Invalid GNSS input: {0}")]
    InvalidGnssInput(String),

    /// The RINF SPARQL endpoint could not be reached or returned an unusable response.
    #[error("RINF retrieval failed: {0}")]
    RinfRetrievalFailed(String),

    /// The retrieval region produced zero netelements.
    #[error("RINF coverage missing for area: {0}")]
    RinfMissingCoverage(String),

    /// The retrieval returned netelements but no netrelations, or coarse geometries.
    #[error("RINF topology incomplete: {0}")]
    RinfIncompleteTopology(String),
}
