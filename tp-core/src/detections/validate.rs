//! Detection validation (T012).
//!
//! Performs cross-detection structural & semantic validation:
//! - linear time ordering (`t_from <= t_to`)
//! - netelement existence (FATAL `UnknownNetelement`)
//! - intrinsic ∈ [0, 1] (FATAL `InvalidIntrinsic`)
//! - same-timestamp + same netelement → recoverable duplicate
//! - same-timestamp + different netelement → FATAL `ConflictingDetections`

use std::collections::HashMap;

use crate::models::{
    Detection, DetectionKind, DetectionRecord, DetectionStatus, DiscardReason, Netelement,
    TimestampOrRange,
};

use super::error::DetectionError;

/// Output of [`validate_detections`]: surviving detections plus duplicate
/// discard records.
#[derive(Debug, Clone, Default)]
pub struct ValidationOutcome {
    pub kept: Vec<Detection>,
    pub duplicate_records: Vec<DetectionRecord>,
}

/// Validate a set of detections against the available netelements.
pub fn validate_detections(
    detections: Vec<Detection>,
    netelements: &[Netelement],
) -> Result<ValidationOutcome, DetectionError> {
    let known_ids: std::collections::HashSet<&str> =
        netelements.iter().map(|n| n.id.as_str()).collect();

    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    let mut kept: Vec<Detection> = Vec::with_capacity(detections.len());
    let mut duplicate_records = Vec::new();

    for det in detections.into_iter() {
        match &det {
            Detection::Punctual(p) => {
                if let Some(loc) = &p.location {
                    if !known_ids.contains(loc.netelement_id.as_str()) {
                        return Err(DetectionError::UnknownNetelement {
                            source_file: p.source_file.clone(),
                            source_row: p.source_row,
                            netelement_id: loc.netelement_id.clone(),
                        });
                    }
                    if !(0.0..=1.0).contains(&loc.intrinsic) {
                        return Err(DetectionError::InvalidIntrinsic {
                            source_file: p.source_file.clone(),
                            source_row: p.source_row,
                            value: loc.intrinsic,
                        });
                    }
                }

                let ts_key = p.timestamp.to_rfc3339();
                let ne_key = p
                    .location
                    .as_ref()
                    .map(|l| l.netelement_id.clone())
                    .unwrap_or_default();

                if !ne_key.is_empty() {
                    for (prior_ts, prior_ne) in seen.keys() {
                        if prior_ts == &ts_key && prior_ne != &ne_key && !prior_ne.is_empty() {
                            return Err(DetectionError::ConflictingDetections {
                                timestamp: p.timestamp,
                                netelement_a: prior_ne.clone(),
                                netelement_b: ne_key.clone(),
                            });
                        }
                    }
                }

                let key = (ts_key, ne_key);
                if !key.1.is_empty() {
                    if let Some(&kept_index) = seen.get(&key) {
                        duplicate_records.push(DetectionRecord {
                            source_file: p.source_file.clone(),
                            source_row: p.source_row,
                            kind: DetectionKind::Punctual,
                            timestamp: TimestampOrRange::Single {
                                timestamp: p.timestamp,
                            },
                            status: DetectionStatus::Discarded {
                                reason: DiscardReason::DuplicateOfPriorDetection { kept_index },
                            },
                            id: p.id.clone(),
                            source: p.source.clone(),
                            metadata: p.metadata.clone(),
                        });
                        continue;
                    }
                    seen.insert(key, kept.len());
                }
                kept.push(det);
            }
            Detection::Linear(l) => {
                if l.t_to < l.t_from {
                    return Err(DetectionError::InvalidTimeRange {
                        source_file: l.source_file.clone(),
                        source_row: l.source_row,
                        t_from: l.t_from,
                        t_to: l.t_to,
                    });
                }
                if !known_ids.contains(l.netelement_id.as_str()) {
                    return Err(DetectionError::UnknownNetelement {
                        source_file: l.source_file.clone(),
                        source_row: l.source_row,
                        netelement_id: l.netelement_id.clone(),
                    });
                }
                if !(0.0..=1.0).contains(&l.start_intrinsic) {
                    return Err(DetectionError::InvalidIntrinsic {
                        source_file: l.source_file.clone(),
                        source_row: l.source_row,
                        value: l.start_intrinsic,
                    });
                }
                if !(0.0..=1.0).contains(&l.end_intrinsic) {
                    return Err(DetectionError::InvalidIntrinsic {
                        source_file: l.source_file.clone(),
                        source_row: l.source_row,
                        value: l.end_intrinsic,
                    });
                }
                kept.push(det);
            }
        }
    }

    Ok(ValidationOutcome {
        kept,
        duplicate_records,
    })
}
