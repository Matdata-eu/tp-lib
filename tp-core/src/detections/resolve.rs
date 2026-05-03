//! Detection resolution (topological / coordinate → ResolvedAnchor) (T018, T030).
//!
//! Converts validated, time-filtered [`Detection`] values into
//! [`ResolvedAnchor`] values plus per-detection [`DetectionRecord`] provenance
//! entries.
//!
//! Topological detections (those carrying a `netelement_id`) resolve directly.
//! Coordinate-only punctual detections are reprojected to the network CRS and
//! matched to the nearest netelement via a linear scan
//! (`crate::path::candidate::calculate_closest_point_on_linestring`). A
//! detection whose nearest netelement is farther than `cutoff_distance_m`
//! is discarded with [`DiscardReason::OutOfReach`].

use chrono::{DateTime, FixedOffset};
use geo::Point;

use crate::crs::transform::CrsTransformer;
use crate::models::{
    Detection, DetectionKind, DetectionRecord, DetectionStatus, DiscardReason, GnssPosition,
    LinearDetection, Netelement, PunctualDetection, ResolvedAnchor, TimestampOrRange,
};
use crate::path::candidate::calculate_closest_point_on_linestring;

use super::error::DetectionError;

/// Per-detection resolution outcome.
#[derive(Debug, Clone)]
pub struct ResolutionOutcome {
    /// Successfully built anchors (sorted later by [`crate::detections::prepare_detections`]).
    pub anchors: Vec<ResolvedAnchor>,
    /// One [`DetectionRecord`] per input detection (applied / resolved /
    /// discarded). Caller is responsible for merging this with
    /// pre-existing duplicate / out-of-window records.
    pub records: Vec<DetectionRecord>,
    /// Free-form warnings (e.g. coordinate-only detection beyond cutoff).
    pub warnings: Vec<String>,
}

/// Resolve every kept detection into an anchor (or a discard record).
///
/// `gnss` MUST be sorted by timestamp. `cutoff_distance_m` applies to
/// coordinate-only resolution only.
pub fn resolve_detections(
    detections: Vec<Detection>,
    gnss: &[GnssPosition],
    netelements: &[Netelement],
    cutoff_distance_m: f64,
) -> Result<ResolutionOutcome, DetectionError> {
    let mut out = ResolutionOutcome {
        anchors: Vec::new(),
        records: Vec::new(),
        warnings: Vec::new(),
    };

    if gnss.is_empty() {
        return Ok(out);
    }

    for det in detections.into_iter() {
        match det {
            Detection::Punctual(p) => resolve_punctual(p, gnss, netelements, cutoff_distance_m, &mut out)?,
            Detection::Linear(l) => resolve_linear(l, gnss, &mut out)?,
        }
    }

    Ok(out)
}

fn resolve_punctual(
    p: PunctualDetection,
    gnss: &[GnssPosition],
    netelements: &[Netelement],
    cutoff_distance_m: f64,
    out: &mut ResolutionOutcome,
) -> Result<(), DetectionError> {
    let gnss_index = nearest_gnss_index(gnss, p.timestamp);

    if let Some(loc) = &p.location {
        // Topological — direct anchor.
        out.anchors.push(ResolvedAnchor::Punctual {
            netelement_id: loc.netelement_id.clone(),
            intrinsic: loc.intrinsic,
            gnss_index,
        });
        out.records.push(DetectionRecord {
            source_file: p.source_file.clone(),
            source_row: p.source_row,
            kind: DetectionKind::Punctual,
            timestamp: TimestampOrRange::Single { timestamp: p.timestamp },
            status: DetectionStatus::Applied {
                netelement_id: loc.netelement_id.clone(),
                intrinsic: loc.intrinsic,
            },
            id: p.id.clone(),
            source: p.source.clone(),
            metadata: p.metadata.clone(),
        });
        return Ok(());
    }

    if let Some(coords) = &p.coordinates {
        if coords.crs.trim().is_empty() {
            return Err(DetectionError::MissingCrs {
                source_file: p.source_file.clone(),
                source_row: p.source_row,
            });
        }

        // Reproject (lat, lon) from coords.crs to WGS84 (network CRS) so that
        // distance in metres is comparable with cutoff.
        let projected = reproject_to_wgs84(coords.longitude, coords.latitude, &coords.crs)
            .map_err(|e| DetectionError::Parse {
                source_file: p.source_file.clone(),
                source_row: p.source_row,
                message: format!("CRS reprojection failed ({}): {}", coords.crs, e),
            })?;
        let detection_point = Point::new(projected.0, projected.1);

        // Linear scan for nearest netelement.
        let mut best: Option<(usize, f64, f64)> = None; // (idx, distance_m, intrinsic)
        for (idx, ne) in netelements.iter().enumerate() {
            let (distance, intrinsic, _) =
                calculate_closest_point_on_linestring(&detection_point, &ne.geometry).map_err(
                    |e| DetectionError::Parse {
                        source_file: p.source_file.clone(),
                        source_row: p.source_row,
                        message: format!("projection error: {}", e),
                    },
                )?;
            match best {
                None => best = Some((idx, distance, intrinsic)),
                Some((_, d, _)) if distance < d => best = Some((idx, distance, intrinsic)),
                _ => {}
            }
        }

        let (best_idx, best_dist, best_intrinsic) = match best {
            Some(b) => b,
            None => {
                // No netelements — treat as out-of-reach with sentinel.
                out.records.push(DetectionRecord {
                    source_file: p.source_file.clone(),
                    source_row: p.source_row,
                    kind: DetectionKind::Punctual,
                    timestamp: TimestampOrRange::Single { timestamp: p.timestamp },
                    status: DetectionStatus::Discarded {
                        reason: DiscardReason::OutOfReach {
                            nearest_distance_m: f64::INFINITY,
                            cutoff_m: cutoff_distance_m,
                        },
                    },
                    id: p.id.clone(),
                    source: p.source.clone(),
                    metadata: p.metadata.clone(),
                });
                return Ok(());
            }
        };

        if best_dist <= cutoff_distance_m {
            let netelement_id = netelements[best_idx].id.clone();
            out.anchors.push(ResolvedAnchor::Punctual {
                netelement_id: netelement_id.clone(),
                intrinsic: best_intrinsic,
                gnss_index,
            });
            out.records.push(DetectionRecord {
                source_file: p.source_file.clone(),
                source_row: p.source_row,
                kind: DetectionKind::Punctual,
                timestamp: TimestampOrRange::Single { timestamp: p.timestamp },
                status: DetectionStatus::Resolved {
                    netelement_id,
                    distance_m: best_dist,
                },
                id: p.id.clone(),
                source: p.source.clone(),
                metadata: p.metadata.clone(),
            });
        } else {
            out.warnings.push(format!(
                "detection at {}:{} discarded (nearest netelement {:.2} m > cutoff {:.2} m)",
                p.source_file, p.source_row, best_dist, cutoff_distance_m
            ));
            out.records.push(DetectionRecord {
                source_file: p.source_file.clone(),
                source_row: p.source_row,
                kind: DetectionKind::Punctual,
                timestamp: TimestampOrRange::Single { timestamp: p.timestamp },
                status: DetectionStatus::Discarded {
                    reason: DiscardReason::OutOfReach {
                        nearest_distance_m: best_dist,
                        cutoff_m: cutoff_distance_m,
                    },
                },
                id: p.id.clone(),
                source: p.source.clone(),
                metadata: p.metadata.clone(),
            });
        }
        return Ok(());
    }

    // Neither location nor coordinates — should have been caught at load.
    Err(DetectionError::InvalidSchema(format!(
        "punctual detection at {}:{} missing both `location` and `coordinates`",
        p.source_file, p.source_row
    )))
}

fn resolve_linear(
    l: LinearDetection,
    gnss: &[GnssPosition],
    out: &mut ResolutionOutcome,
) -> Result<(), DetectionError> {
    // gnss_range = all indices i where gnss[i].timestamp ∈ [t_from, t_to]
    let mut first: Option<usize> = None;
    let mut last: Option<usize> = None;
    for (i, g) in gnss.iter().enumerate() {
        if g.timestamp >= l.t_from && g.timestamp <= l.t_to {
            if first.is_none() {
                first = Some(i);
            }
            last = Some(i);
        }
    }

    match (first, last) {
        (Some(lo), Some(hi)) => {
            out.anchors.push(ResolvedAnchor::Linear {
                netelement_id: l.netelement_id.clone(),
                start_intrinsic: l.start_intrinsic,
                end_intrinsic: l.end_intrinsic,
                gnss_range: lo..=hi,
            });
            out.records.push(DetectionRecord {
                source_file: l.source_file.clone(),
                source_row: l.source_row,
                kind: DetectionKind::Linear,
                timestamp: TimestampOrRange::Range {
                    t_from: l.t_from,
                    t_to: l.t_to,
                },
                status: DetectionStatus::Applied {
                    netelement_id: l.netelement_id.clone(),
                    // For linear records, surface the start_intrinsic as a
                    // representative value (end_intrinsic kept on the anchor).
                    intrinsic: l.start_intrinsic,
                },
                id: l.id.clone(),
                source: l.source.clone(),
                metadata: l.metadata.clone(),
            });
        }
        _ => {
            // No GNSS index falls inside the window — discard.
            let gnss_first = gnss.first().unwrap().timestamp;
            let gnss_last = gnss.last().unwrap().timestamp;
            out.records.push(DetectionRecord {
                source_file: l.source_file.clone(),
                source_row: l.source_row,
                kind: DetectionKind::Linear,
                timestamp: TimestampOrRange::Range {
                    t_from: l.t_from,
                    t_to: l.t_to,
                },
                status: DetectionStatus::Discarded {
                    reason: DiscardReason::OutOfTimeRange {
                        gnss_first,
                        gnss_last,
                    },
                },
                id: l.id.clone(),
                source: l.source.clone(),
                metadata: l.metadata.clone(),
            });
            out.warnings.push(format!(
                "linear detection at {}:{} window has no GNSS samples — discarded",
                l.source_file, l.source_row,
            ));
        }
    }

    Ok(())
}

/// Pick the GNSS index with timestamp closest to `t` (ties → earlier index).
fn nearest_gnss_index(gnss: &[GnssPosition], t: DateTime<FixedOffset>) -> usize {
    debug_assert!(!gnss.is_empty());
    let mut best_idx = 0usize;
    let mut best_diff = (gnss[0].timestamp - t).num_milliseconds().abs();
    for (i, g) in gnss.iter().enumerate().skip(1) {
        let diff = (g.timestamp - t).num_milliseconds().abs();
        if diff < best_diff {
            best_diff = diff;
            best_idx = i;
        }
    }
    best_idx
}

/// Reproject `(lon, lat)` from `crs` (EPSG code) into WGS84 lon/lat (degrees).
///
/// Returns `(lon, lat)`.
fn reproject_to_wgs84(lon: f64, lat: f64, crs: &str) -> Result<(f64, f64), String> {
    if crs.eq_ignore_ascii_case("EPSG:4326") {
        return Ok((lon, lat));
    }
    let xform = CrsTransformer::new(crs.to_string(), "EPSG:4326".to_string())
        .map_err(|e| format!("{}", e))?;
    let pt = xform
        .transform(Point::new(lon, lat))
        .map_err(|e| format!("{}", e))?;
    Ok((pt.x(), pt.y()))
}
