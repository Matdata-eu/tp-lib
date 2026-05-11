//! Detection provenance record types (T008).
//!
//! `DetectionRecord` is appended to `PathResult.detection_provenance` for
//! every detection ingested (applied or discarded), preserving original input
//! order.

use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

/// Whether a detection was punctual or linear (preserved for provenance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectionKind {
    Punctual,
    Linear,
}

/// Either a single timestamp (punctual) or an inclusive `[from, to]` range (linear).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TimestampOrRange {
    Single {
        timestamp: DateTime<FixedOffset>,
    },
    Range {
        t_from: DateTime<FixedOffset>,
        t_to: DateTime<FixedOffset>,
    },
}

/// Reason a detection was discarded (FR-010, FR-011, FR-009, FR-006, FR-007, FR-007a).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiscardReason {
    /// Timestamp / window outside the GNSS observation window.
    OutOfTimeRange {
        gnss_first: DateTime<FixedOffset>,
        gnss_last: DateTime<FixedOffset>,
    },
    /// Coordinate-only punctual: nearest netelement is farther than the cutoff.
    OutOfReach {
        nearest_distance_m: f64,
        cutoff_m: f64,
    },
    /// Referenced `netelement_id` does not exist in the supplied network.
    /// (Only used for non-fatal warnings; the standard pipeline raises
    /// `DetectionError::UnknownNetelement` instead.)
    UnknownNetelement {
        netelement_id: String,
    },
    /// Intrinsic value out of `[0, 1]`.
    IntrinsicOutOfRange {
        value: f64,
    },
    /// Same timestamp + same netelement as a previously kept detection (FR-007a).
    DuplicateOfPriorDetection {
        kept_index: usize,
    },
}

/// Disposition of an ingested detection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DetectionStatus {
    /// Detection was applied as a Viterbi anchor.
    Applied {
        netelement_id: String,
        intrinsic: f64,
    },
    /// Coordinate-only detection successfully resolved within the cutoff
    /// (subsequently applied).
    Resolved {
        netelement_id: String,
        distance_m: f64,
    },
    /// Detection was discarded; see `reason`.
    Discarded {
        reason: DiscardReason,
    },
}

/// Per-detection provenance record (one per input detection).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectionRecord {
    pub source_file: String,
    pub source_row: usize,
    pub kind: DetectionKind,
    pub timestamp: TimestampOrRange,
    pub status: DetectionStatus,
    pub id: Option<String>,
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}
