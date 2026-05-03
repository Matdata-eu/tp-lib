//! Anchor injection into the Viterbi candidate / emission lattice (T019, T026).
//!
//! For each [`ResolvedAnchor`], rewrites the candidate set and emission row at
//! the affected GNSS index/range so that the Viterbi pass is *forced* to pass
//! through the anchored netelement at the anchored intrinsic position(s).
//!
//! Punctual anchors collapse `position_candidates[gnss_index]` to a single
//! [`CandidateNetElement`] with emission probability 1.0.
//!
//! Linear anchors restrict every GNSS step in `gnss_range` to candidates
//! lying on the anchored netelement (a single forced candidate per step,
//! intrinsic linearly interpolated between `start_intrinsic` and
//! `end_intrinsic`).

use std::collections::HashMap;

use geo::{LineString, Point};

use crate::models::{Netelement, ResolvedAnchor};
use crate::path::candidate::CandidateNetElement;

use super::error::DetectionError;

/// Apply every anchor to `position_candidates` / `emission_probs`.
///
/// `gnss_index_map` (when `Some`) maps the anchor's *original* GNSS index
/// (referring to the input GNSS array) to the *working* index (after
/// resampling). When `None`, anchors are assumed to use working indices
/// directly.
///
/// Out-of-bounds anchors (`gnss_index >= position_candidates.len()`) are
/// silently skipped — the caller's filter pass should already have removed
/// them, but defensive checks keep us robust against resampling edge cases.
pub fn apply_anchors(
    anchors: &[ResolvedAnchor],
    position_candidates: &mut [Vec<CandidateNetElement>],
    emission_probs: &mut [Vec<f64>],
    netelements: &[Netelement],
    netelement_index: &HashMap<String, usize>,
    gnss_index_map: Option<&[usize]>,
) -> Result<(), DetectionError> {
    debug_assert_eq!(position_candidates.len(), emission_probs.len());

    for anchor in anchors {
        match anchor {
            ResolvedAnchor::Punctual {
                netelement_id,
                intrinsic,
                gnss_index,
            } => {
                let Some(working_idx) = remap_index(*gnss_index, gnss_index_map) else {
                    continue;
                };
                if working_idx >= position_candidates.len() {
                    continue;
                }
                let Some(&ne_idx) = netelement_index.get(netelement_id) else {
                    return Err(DetectionError::UnknownNetelement {
                        source_file: String::new(),
                        source_row: 0,
                        netelement_id: netelement_id.clone(),
                    });
                };
                let ne = &netelements[ne_idx];
                let pt = point_at_intrinsic(&ne.geometry, *intrinsic);
                let forced = CandidateNetElement {
                    netelement_id: netelement_id.clone(),
                    distance_meters: 0.0,
                    intrinsic_coordinate: intrinsic.clamp(0.0, 1.0),
                    projected_point: pt,
                };
                position_candidates[working_idx] = vec![forced];
                emission_probs[working_idx] = vec![1.0];
            }
            ResolvedAnchor::Linear {
                netelement_id,
                start_intrinsic,
                end_intrinsic,
                gnss_range,
            } => {
                let Some(&ne_idx) = netelement_index.get(netelement_id) else {
                    return Err(DetectionError::UnknownNetelement {
                        source_file: String::new(),
                        source_row: 0,
                        netelement_id: netelement_id.clone(),
                    });
                };
                let ne = &netelements[ne_idx];

                let lo = *gnss_range.start();
                let hi = *gnss_range.end();
                let span = hi.saturating_sub(lo) as f64;

                for original_idx in lo..=hi {
                    let Some(working_idx) = remap_index(original_idx, gnss_index_map) else {
                        continue;
                    };
                    if working_idx >= position_candidates.len() {
                        continue;
                    }

                    let frac = if span > 0.0 {
                        (original_idx - lo) as f64 / span
                    } else {
                        0.0
                    };
                    let intrinsic = start_intrinsic + frac * (end_intrinsic - start_intrinsic);
                    let intrinsic = intrinsic.clamp(0.0, 1.0);
                    let pt = point_at_intrinsic(&ne.geometry, intrinsic);

                    let forced = CandidateNetElement {
                        netelement_id: netelement_id.clone(),
                        distance_meters: 0.0,
                        intrinsic_coordinate: intrinsic,
                        projected_point: pt,
                    };
                    position_candidates[working_idx] = vec![forced];
                    emission_probs[working_idx] = vec![1.0];
                }
            }
        }
    }

    Ok(())
}

fn remap_index(original: usize, map: Option<&[usize]>) -> Option<usize> {
    match map {
        None => Some(original),
        Some(m) => m.get(original).copied(),
    }
}

/// Interpolate a [`Point`] at fractional distance `intrinsic` ∈ [0, 1] along
/// `line`, using haversine segment lengths so the result is metric-correct on
/// geographic (WGS84) coordinates.
pub(crate) fn point_at_intrinsic(line: &LineString<f64>, intrinsic: f64) -> Point<f64> {
    use geo::algorithm::haversine_distance::HaversineDistance;

    let coords = &line.0;
    if coords.is_empty() {
        return Point::new(0.0, 0.0);
    }
    if coords.len() == 1 {
        return Point::new(coords[0].x, coords[0].y);
    }

    let t = intrinsic.clamp(0.0, 1.0);

    // Per-segment & total haversine length.
    let mut seg_lens: Vec<f64> = Vec::with_capacity(coords.len() - 1);
    let mut total = 0.0;
    for i in 0..coords.len() - 1 {
        let a = Point::new(coords[i].x, coords[i].y);
        let b = Point::new(coords[i + 1].x, coords[i + 1].y);
        let d = a.haversine_distance(&b);
        seg_lens.push(d);
        total += d;
    }
    if total <= 0.0 {
        return Point::new(coords[0].x, coords[0].y);
    }

    let target = t * total;
    let mut acc = 0.0;
    for (i, &seg_len) in seg_lens.iter().enumerate() {
        if acc + seg_len >= target || i == seg_lens.len() - 1 {
            let local_t = if seg_len > 0.0 {
                ((target - acc) / seg_len).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let p1 = &coords[i];
            let p2 = &coords[i + 1];
            return Point::new(
                p1.x + local_t * (p2.x - p1.x),
                p1.y + local_t * (p2.y - p1.y),
            );
        }
        acc += seg_len;
    }

    Point::new(coords[0].x, coords[0].y)
}
