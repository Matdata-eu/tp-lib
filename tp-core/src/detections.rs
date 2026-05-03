//! Train detections — punctual & linear absolute position anchors.
//!
//! See `specs/004-train-detections/` for the design.

pub mod anchor;
pub mod error;
pub mod filter;
pub mod load;
pub mod resolve;
pub mod validate;

pub use error::DetectionError;

use std::path::Path;

use crate::models::{Detection, DetectionKind, DetectionRecord, GnssPosition, Netelement, ResolvedAnchor};

/// Output of [`prepare_detections`].
#[derive(Debug, Clone, Default)]
pub struct PreparedDetections {
    /// Anchors ready to inject into the Viterbi pipeline (sorted by first
    /// affected GNSS index).
    pub anchors: Vec<ResolvedAnchor>,
    /// Per-detection provenance (applied / resolved / discarded). Sorted by
    /// `(source_file, source_row)`.
    pub records: Vec<DetectionRecord>,
    /// Free-form warnings collected during filter / resolve.
    pub warnings: Vec<String>,
}

/// Load → validate → time-filter → resolve a single detections file.
///
/// `expected_kind` reflects the originating CLI flag.
pub fn prepare_detections(
    path: &Path,
    expected_kind: DetectionKind,
    gnss: &[GnssPosition],
    netelements: &[Netelement],
    cutoff_distance_m: f64,
) -> Result<PreparedDetections, DetectionError> {
    let detections = load::load_detections(path, expected_kind)?;
    prepare_detections_from_loaded(detections, gnss, netelements, cutoff_distance_m)
}

/// Same as [`prepare_detections`] but skips the load step (useful for tests
/// and for combining detections from multiple files).
pub fn prepare_detections_from_loaded(
    detections: Vec<Detection>,
    gnss: &[GnssPosition],
    netelements: &[Netelement],
    cutoff_distance_m: f64,
) -> Result<PreparedDetections, DetectionError> {
    let mut warnings = Vec::new();
    let mut all_records: Vec<DetectionRecord> = Vec::new();

    // 1. Validation (FATAL on conflicting / unknown / out-of-range).
    let validated = validate::validate_detections(detections, netelements)?;
    all_records.extend(validated.duplicate_records);

    // 2. Time-range filter.
    let filtered = filter::filter_detections_by_time(validated.kept, gnss);
    all_records.extend(filtered.discard_records);
    warnings.extend(filtered.warnings);

    // 3. Resolution (topological + coordinate-only).
    let resolution = resolve::resolve_detections(filtered.kept, gnss, netelements, cutoff_distance_m)?;
    all_records.extend(resolution.records);
    warnings.extend(resolution.warnings);

    // Sort anchors by first affected GNSS index.
    let mut anchors = resolution.anchors;
    anchors.sort_by_key(|a| a.first_index());

    // Sort records by (source_file, source_row).
    all_records.sort_by(|a, b| {
        a.source_file
            .cmp(&b.source_file)
            .then(a.source_row.cmp(&b.source_row))
    });

    Ok(PreparedDetections {
        anchors,
        records: all_records,
        warnings,
    })
}
