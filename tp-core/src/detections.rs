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

use std::collections::HashMap;
use std::path::Path;

use crate::models::{
    Detection, DetectionKind, DetectionRecord, GnssPosition, Netelement, ResolvedAnchor,
};

/// Output of [`prepare_detections`].
#[derive(Debug, Clone, Default)]
pub struct PreparedDetections {
    /// Anchors ready to inject into the Viterbi pipeline (sorted by first
    /// affected GNSS index).
    pub anchors: Vec<ResolvedAnchor>,
    /// Per-detection provenance (applied / resolved / discarded), preserving
    /// original input order.
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
    let input_order: Vec<(String, usize)> = detections
        .iter()
        .map(|d| (d.source_file().to_owned(), d.source_row()))
        .collect();

    // 1. Validation (FATAL on conflicting / unknown / out-of-range).
    let validated = validate::validate_detections(detections, netelements)?;
    let kept_keys: Vec<(String, usize)> = validated
        .kept
        .iter()
        .map(|d| (d.source_file().to_owned(), d.source_row()))
        .collect();
    all_records.extend(validated.duplicate_records);

    // 2. Time-range filter.
    let filtered = filter::filter_detections_by_time(validated.kept, gnss);
    all_records.extend(filtered.discard_records);
    warnings.extend(filtered.warnings);

    // 3. Resolution (topological + coordinate-only).
    let resolution =
        resolve::resolve_detections(filtered.kept, gnss, netelements, cutoff_distance_m)?;
    all_records.extend(resolution.records);
    warnings.extend(resolution.warnings);

    // Sort anchors by first affected GNSS index.
    let mut anchors = resolution.anchors;
    anchors.sort_by_key(|a| a.first_index());

    // Rebuild records in original input order so duplicate `kept_index`
    // references remain meaningful for `detection_provenance`.
    let mut by_key: HashMap<(String, usize), DetectionRecord> = HashMap::new();
    for rec in all_records {
        by_key.insert((rec.source_file.clone(), rec.source_row), rec);
    }
    let mut records: Vec<DetectionRecord> = Vec::with_capacity(input_order.len());
    for key in input_order {
        if let Some(rec) = by_key.remove(&key) {
            records.push(rec);
        }
    }
    let mut leftovers: Vec<DetectionRecord> = by_key.into_values().collect();
    leftovers.sort_by(|a, b| {
        a.source_file
            .cmp(&b.source_file)
            .then(a.source_row.cmp(&b.source_row))
    });
    records.extend(leftovers);

    let kept_lookup: HashMap<(String, usize), usize> = kept_keys
        .into_iter()
        .enumerate()
        .map(|(idx, key)| (key, idx))
        .collect();
    let mut kept_index_to_provenance_index: Vec<Option<usize>> = vec![None; kept_lookup.len()];
    for (provenance_idx, rec) in records.iter().enumerate() {
        if let Some(&kept_idx) = kept_lookup.get(&(rec.source_file.clone(), rec.source_row)) {
            kept_index_to_provenance_index[kept_idx] = Some(provenance_idx);
        }
    }
    for rec in &mut records {
        if let crate::models::DetectionStatus::Discarded {
            reason: crate::models::DiscardReason::DuplicateOfPriorDetection { kept_index },
        } = &mut rec.status
        {
            if let Some(Some(mapped)) = kept_index_to_provenance_index.get(*kept_index) {
                *kept_index = *mapped;
            }
        }
    }

    Ok(PreparedDetections {
        anchors,
        records,
        warnings,
    })
}
