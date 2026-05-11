//! Detection input types (T007).
//!
//! Types modelling absolute train-position detections (punctual or linear)
//! supplied by the user as anchors for path calculation.
//!
//! See `specs/004-train-detections/data-model.md`.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

/// Topological reference to a position on a netelement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologicalLocation {
    /// Identifier of the netelement.
    pub netelement_id: String,
    /// Position along the netelement, 0.0 = start, 1.0 = end.
    pub intrinsic: f64,
}

/// Geographic (lat/lon + CRS) reference for a punctual detection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeographicLocation {
    pub latitude: f64,
    pub longitude: f64,
    /// Authoritative CRS of `(latitude, longitude)`, e.g. `"EPSG:4326"`.
    pub crs: String,
}

/// A punctual detection: train was at a precise (timestamp, position).
///
/// Either `location` (topological) or `coordinates` (geographic) MUST be
/// supplied — never both. The combination is validated at load time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PunctualDetection {
    pub timestamp: DateTime<FixedOffset>,
    /// Topological position, mutually exclusive with `coordinates`.
    pub location: Option<TopologicalLocation>,
    /// Geographic position, mutually exclusive with `location`.
    pub coordinates: Option<GeographicLocation>,
    /// Optional intrinsic to associate with `coordinates` once resolved
    /// (currently unused at load time; reserved for future enhancements).
    pub intrinsic: Option<f64>,
    /// Optional caller-supplied stable identifier (free-form).
    pub id: Option<String>,
    /// Free-form source label (e.g. `"axle-counter-A12"`).
    pub source: Option<String>,
    /// Provenance: origin file path.
    pub source_file: String,
    /// Provenance: origin row index (CSV) or feature index (GeoJSON).
    pub source_row: usize,
    /// Unknown / extra columns or properties, captured verbatim.
    pub metadata: BTreeMap<String, String>,
}

/// A linear detection: train was somewhere on `netelement_id` between `t_from` and `t_to`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinearDetection {
    pub t_from: DateTime<FixedOffset>,
    pub t_to: DateTime<FixedOffset>,
    pub netelement_id: String,
    pub start_intrinsic: f64,
    pub end_intrinsic: f64,
    pub id: Option<String>,
    pub source: Option<String>,
    pub source_file: String,
    pub source_row: usize,
    pub metadata: BTreeMap<String, String>,
}

/// A detection of either kind, as parsed from input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Detection {
    Punctual(PunctualDetection),
    Linear(LinearDetection),
}

impl Detection {
    pub fn source_file(&self) -> &str {
        match self {
            Detection::Punctual(p) => &p.source_file,
            Detection::Linear(l) => &l.source_file,
        }
    }

    pub fn source_row(&self) -> usize {
        match self {
            Detection::Punctual(p) => p.source_row,
            Detection::Linear(l) => l.source_row,
        }
    }
}

/// A detection successfully resolved into a Viterbi anchor.
///
/// Linear anchors carry a `gnss_range` indexed into the GNSS observation array
/// (covering every index whose timestamp falls within `[t_from, t_to]`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ResolvedAnchor {
    Punctual {
        netelement_id: String,
        intrinsic: f64,
        gnss_index: usize,
    },
    Linear {
        netelement_id: String,
        start_intrinsic: f64,
        end_intrinsic: f64,
        gnss_range: RangeInclusive<usize>,
    },
}

impl ResolvedAnchor {
    /// First GNSS index this anchor affects (used for ordering).
    pub fn first_index(&self) -> usize {
        match self {
            ResolvedAnchor::Punctual { gnss_index, .. } => *gnss_index,
            ResolvedAnchor::Linear { gnss_range, .. } => *gnss_range.start(),
        }
    }

    /// Netelement this anchor pins.
    pub fn netelement_id(&self) -> &str {
        match self {
            ResolvedAnchor::Punctual { netelement_id, .. }
            | ResolvedAnchor::Linear { netelement_id, .. } => netelement_id,
        }
    }
}
