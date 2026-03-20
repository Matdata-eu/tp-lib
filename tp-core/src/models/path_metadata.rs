//! Path calculation metadata

use serde::{Deserialize, Serialize};

use crate::models::AssociatedNetElement;

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

    /// Snapshot of segment-level diagnostics (order, intrinsics, probabilities)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_info: Option<PathDiagnosticInfo>,
}

/// Collection of segment-level diagnostics for a calculated path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathDiagnosticInfo {
    /// Ordered diagnostics per segment in the path
    pub segments: Vec<SegmentDiagnostic>,
}

impl PathDiagnosticInfo {
    /// Build diagnostics from associated netelements, preserving traversal order
    pub fn from_segments(segments: &[AssociatedNetElement]) -> Self {
        let segments = segments
            .iter()
            .map(|segment| SegmentDiagnostic {
                netelement_id: segment.netelement_id.clone(),
                probability: segment.probability,
                start_intrinsic: segment.start_intrinsic,
                end_intrinsic: segment.end_intrinsic,
                gnss_start_index: segment.gnss_start_index,
                gnss_end_index: segment.gnss_end_index,
            })
            .collect();

        Self { segments }
    }
}

/// Diagnostic details for a single segment in a train path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentDiagnostic {
    /// ID of the netelement
    pub netelement_id: String,

    /// Probability assigned to this segment
    pub probability: f64,

    /// Intrinsic coordinate where the path enters this segment
    pub start_intrinsic: f64,

    /// Intrinsic coordinate where the path exits this segment
    pub end_intrinsic: f64,

    /// Index of the first GNSS position associated with this segment
    pub gnss_start_index: usize,

    /// Index of the last GNSS position associated with this segment
    pub gnss_end_index: usize,
}

#[cfg(test)]
mod tests;
