//! Path calculation metadata

use serde::{Deserialize, Serialize};

/// Algorithm configuration and diagnostic metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMetadata {
    /// Distance scale parameter used for probability calculation
    pub distance_scale: f64,

    /// Heading scale parameter used for probability calculation
    pub heading_scale: f64,

    /// Cutoff distance for candidate selection (meters)
    pub cutoff_distance: f64,

    /// Heading difference cutoff (degrees)
    pub heading_cutoff: f64,

    /// Probability threshold for path segment inclusion
    pub probability_threshold: f64,

    /// Resampling distance applied (meters), None if disabled
    pub resampling_distance: Option<f64>,

    /// Whether fallback mode was used
    pub fallback_mode: bool,

    /// Number of candidate paths evaluated
    pub candidate_paths_evaluated: usize,

    /// Whether path existed in both directions (bidirectional validation)
    pub bidirectional_path: bool,
}
