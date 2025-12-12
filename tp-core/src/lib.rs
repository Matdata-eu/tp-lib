//! TP-Core: Train Positioning Library - Core Engine
//! 
//! This library provides geospatial projection of GNSS positions onto railway track netelements.

pub mod models;
pub mod projection;
pub mod io;
pub mod crs;
pub mod temporal;
pub mod errors;

// Re-export main types for convenience
pub use models::{GnssPosition, Netelement, ProjectedPosition};
pub use errors::ProjectionError;

/// Result type alias using ProjectionError
pub type Result<T> = std::result::Result<T, ProjectionError>;
