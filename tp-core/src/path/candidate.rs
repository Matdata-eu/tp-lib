//! Candidate netelement selection for GNSS positions
//!
//! Finds candidate netelements that could match each GNSS position based on
//! spatial proximity and heading alignment.

use crate::errors::ProjectionError;
use crate::models::{GnssPosition, Netelement};
use geo::{LineString, Point};

/// Candidates with intrinsic coordinate closer than this to 0.0 or 1.0 are
/// rejected.  Projections at the geometric endpoints indicate the GNSS point
/// is more likely located on an adjacent netelement.
const EDGE_INTRINSIC_THRESHOLD: f64 = 1e-6;

/// A candidate netelement for a GNSS position
#[derive(Debug, Clone)]
pub struct CandidateNetElement {
    /// ID of the candidate netelement
    pub netelement_id: String,

    /// Distance from GNSS position to closest point on linestring (meters)
    pub distance_meters: f64,

    /// Intrinsic coordinate on the netelement (0.0 to 1.0)
    /// where 0.0 = start of segment, 1.0 = end of segment
    pub intrinsic_coordinate: f64,

    /// Projected point on the netelement
    pub projected_point: Point<f64>,
}

/// Find candidate netelements for a GNSS position
///
/// Returns netelements within cutoff_distance, sorted by distance.
///
/// # Arguments
///
/// * `gnss_pos` - The GNSS position to find candidates for
/// * `netelements` - All available network netelements
/// * `cutoff_distance` - Maximum distance for candidate inclusion (meters)
/// * `max_candidates` - Maximum number of candidates to return
///
/// # Returns
///
/// Vector of candidate netelements sorted by distance (closest first)
pub fn find_candidate_netelements(
    gnss_pos: &GnssPosition,
    netelements: &[Netelement],
    cutoff_distance: f64,
    max_candidates: usize,
) -> Result<Vec<CandidateNetElement>, ProjectionError> {
    let gnss_point = Point::new(gnss_pos.longitude, gnss_pos.latitude);
    let mut candidates = Vec::new();

    // Search all netelements for candidates within cutoff
    for netelement in netelements {
        // Calculate distance to netelement
        let (distance, intrinsic, proj_point) =
            calculate_closest_point_on_linestring(&gnss_point, &netelement.geometry)?;

        // Include if within cutoff distance
        if distance <= cutoff_distance {
            candidates.push(CandidateNetElement {
                netelement_id: netelement.id.clone(),
                distance_meters: distance,
                intrinsic_coordinate: intrinsic,
                projected_point: proj_point,
            });
        }
    }

    // Prefer interior projections over edge projections (intrinsic at 0 or 1).
    // Edge projections indicate the GNSS point is past the netelement boundary
    // and may belong to an adjacent netelement.  If at least one non-edge
    // candidate exists, remove edge candidates.
    //
    // Fallback: if *all* candidates are edge projections, none are removed.
    // This prevents the position from having zero candidates when the GNSS
    // point sits exactly at a netelement connection boundary.
    let has_interior = candidates.iter().any(|c| {
        c.intrinsic_coordinate >= EDGE_INTRINSIC_THRESHOLD
            && c.intrinsic_coordinate <= 1.0 - EDGE_INTRINSIC_THRESHOLD
    });
    if has_interior {
        candidates.retain(|c| {
            c.intrinsic_coordinate >= EDGE_INTRINSIC_THRESHOLD
                && c.intrinsic_coordinate <= 1.0 - EDGE_INTRINSIC_THRESHOLD
        });
    }

    // Sort by distance (closest first)
    candidates.sort_by(|a, b| a.distance_meters.partial_cmp(&b.distance_meters).unwrap());

    // Return top max_candidates
    candidates.truncate(max_candidates);

    Ok(candidates)
}

/// Calculate closest point on a linestring to a point
///
/// Returns (distance, intrinsic_coordinate, projected_point)
///
/// # Arguments
///
/// * `point` - The reference point (GNSS position)
/// * `linestring` - The linestring to find closest point on
///
/// # Returns
///
/// Tuple of:
/// - distance: Distance from point to closest point on linestring (meters)
/// - intrinsic: Intrinsic coordinate on linestring (0.0 to 1.0)
/// - projected_point: The closest point on the linestring
fn calculate_closest_point_on_linestring(
    point: &Point<f64>,
    linestring: &LineString<f64>,
) -> Result<(f64, f64, Point<f64>), ProjectionError> {
    use geo::algorithm::haversine_distance::HaversineDistance;

    let coords = &linestring.0;

    // Use cos(lat) scaling so the dot-product projection is metrically
    // correct for geographic (WGS 84) coordinates — same approach used by
    // `calculate_heading_at_point` and `project_point_onto_linestring`.
    let cos_lat = point.y().to_radians().cos();

    let mut min_dist_sq = f64::INFINITY;
    let mut best_seg: usize = 0;
    let mut best_t: f64 = 0.0;
    let mut closest_point = coords[0];

    for i in 0..coords.len() - 1 {
        let p1 = &coords[i];
        let p2 = &coords[i + 1];

        let dx = (p2.x - p1.x) * cos_lat;
        let dy = p2.y - p1.y;
        let len_sq = dx * dx + dy * dy;

        let t = if len_sq > 0.0 {
            let px = (point.x() - p1.x) * cos_lat;
            let py = point.y() - p1.y;
            ((px * dx + py * dy) / len_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Interpolate back in degree space.
        let proj_x = p1.x + t * (p2.x - p1.x);
        let proj_y = p1.y + t * (p2.y - p1.y);

        let ex = (point.x() - proj_x) * cos_lat;
        let ey = point.y() - proj_y;
        let dist_sq = ex * ex + ey * ey;

        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
            closest_point = geo::Coord { x: proj_x, y: proj_y };
            best_seg = i;
            best_t = t;
        }
    }

    let closest_pt = Point::new(closest_point.x, closest_point.y);

    // Distance in meters via haversine (accurate for any latitude).
    let distance = point.haversine_distance(&closest_pt);

    // Compute length-based intrinsic coordinate (0..1) using haversine
    // segment lengths instead of a uniform vertex-index parameterization.
    let mut length_before = 0.0;
    for i in 0..best_seg {
        let a = Point::new(coords[i].x, coords[i].y);
        let b = Point::new(coords[i + 1].x, coords[i + 1].y);
        length_before += a.haversine_distance(&b);
    }
    let seg_start = Point::new(coords[best_seg].x, coords[best_seg].y);
    let seg_end = Point::new(coords[best_seg + 1].x, coords[best_seg + 1].y);
    let seg_length = seg_start.haversine_distance(&seg_end);
    length_before += best_t * seg_length;

    let total_length: f64 = (0..coords.len() - 1)
        .map(|i| {
            let a = Point::new(coords[i].x, coords[i].y);
            let b = Point::new(coords[i + 1].x, coords[i + 1].y);
            a.haversine_distance(&b)
        })
        .sum();

    let intrinsic = if total_length > 0.0 {
        (length_before / total_length).clamp(0.0, 1.0)
    } else {
        0.0
    };

    Ok((distance, intrinsic, closest_pt))
}

/// Calculate heading at a projected point on a linestring
///
/// Returns the direction (bearing) of the linestring at the given point.
/// This is used to compare with GNSS heading to filter incompatible candidates.
///
/// # Arguments
///
/// * `point` - The point on the linestring
/// * `linestring` - The linestring
///
/// # Returns
///
/// Heading in degrees (0-360), where:
/// - 0° = North
/// - 90° = East
/// - 180° = South
/// - 270° = West
pub fn calculate_heading_at_point(
    point: &Point<f64>,
    linestring: &LineString<f64>,
) -> Result<f64, ProjectionError> {
    let coords: Vec<_> = linestring.points().collect();

    if coords.len() < 2 {
        return Ok(0.0);
    }

    // Find the segment closest to the point by perpendicular (point-to-segment)
    // distance, not just distance to the start vertex.
    let cos_lat = point.y().to_radians().cos();
    let px = point.x() * 111_320.0 * cos_lat;
    let py = point.y() * 111_320.0;

    let mut closest_segment_idx = 0;
    let mut closest_distance = f64::MAX;

    for i in 0..coords.len() - 1 {
        let ax = coords[i].x() * 111_320.0 * cos_lat;
        let ay = coords[i].y() * 111_320.0;
        let bx = coords[i + 1].x() * 111_320.0 * cos_lat;
        let by = coords[i + 1].y() * 111_320.0;

        let dx = bx - ax;
        let dy = by - ay;
        let seg_len_sq = dx * dx + dy * dy;

        // Project point onto the segment, clamped to [0, 1].
        let t = if seg_len_sq < 1e-18 {
            0.0
        } else {
            ((px - ax) * dx + (py - ay) * dy) / seg_len_sq
        }
        .clamp(0.0, 1.0);

        let proj_x = ax + t * dx;
        let proj_y = ay + t * dy;
        let dist = ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt();

        if dist < closest_distance {
            closest_distance = dist;
            closest_segment_idx = i;
        }
    }

    // Get segment direction using haversine bearing (correctly accounts
    // for the cos(latitude) scaling of longitude at any latitude).
    let p1 = coords[closest_segment_idx];
    let p2 = coords[closest_segment_idx + 1];

    Ok(haversine_bearing(
        &Point::new(p1.x(), p1.y()),
        &Point::new(p2.x(), p2.y()),
    ))
}

/// Estimate headings from neighboring GNSS positions.
///
/// For each interior position `x`, computes the haversine bearing from position
/// `x-1` to `x+1`.  The estimate is discarded when:
/// - `x` is the first or last position (no symmetric neighbors),
/// - the distances `x-1 → x` and `x → x+1` differ by ≥ 20% of the larger,
/// - the backward bearing (`x-1` → `x`) and forward bearing (`x` → `x+1`)
///   diverge by ≥ 5° (lateral-jump / junction guard),
/// - consecutive estimated headings change by ≥ 5° (continuity check).
///
/// Returns a `Vec` with the same length as `positions`; entries that fail any
/// guard are `None`.
pub fn estimate_headings_from_neighbors(positions: &[&GnssPosition]) -> Vec<Option<f64>> {
    let n = positions.len();
    let mut headings: Vec<Option<f64>> = vec![None; n];

    if n < 3 {
        return headings;
    }

    // Pass 1: compute raw estimated headings for interior positions.
    for x in 1..n - 1 {
        let prev = Point::new(positions[x - 1].longitude, positions[x - 1].latitude);
        let curr = Point::new(positions[x].longitude, positions[x].latitude);
        let next = Point::new(positions[x + 1].longitude, positions[x + 1].latitude);

        let d_prev = haversine_distance(&prev, &curr);
        let d_next = haversine_distance(&curr, &next);

        // Distance symmetry guard: reject if ratio difference ≥ 20%.
        let max_d = d_prev.max(d_next);
        if max_d < 1e-9 {
            continue; // degenerate (coincident points)
        }
        if (d_prev - d_next).abs() / max_d >= 0.20 {
            continue;
        }

        // Bearing deviation guard: the backward bearing (x-1 → x) and the
        // forward bearing (x → x+1) should agree on a stable trajectory.
        // A divergence ≥ 5° indicates a lateral GNSS jump or a junction turn;
        // in either case the smoothed heading is unreliable.
        let h_back = haversine_bearing(&prev, &curr);
        let h_fwd = haversine_bearing(&curr, &next);
        if directional_heading_difference(h_back, h_fwd) >= 5.0 {
            continue;
        }

        headings[x] = Some(haversine_bearing(&prev, &next));
    }

    // Pass 2: heading-continuity check (< 5° between consecutive estimates).
    // Compare each heading against its immediate neighbor's raw (pre-rejection)
    // heading to avoid cascading rejections.
    let raw_headings = headings.clone();
    for x in 2..n - 1 {
        if let Some(h) = headings[x] {
            if let Some(prev_h) = raw_headings[x - 1] {
                if heading_difference(h, prev_h) >= 5.0 {
                    headings[x] = None;
                }
            }
        }
    }

    headings
}

/// Haversine distance between two WGS-84 points, in metres.
fn haversine_distance(a: &Point<f64>, b: &Point<f64>) -> f64 {
    let (lat1, lon1) = (a.y().to_radians(), a.x().to_radians());
    let (lat2, lon2) = (b.y().to_radians(), b.x().to_radians());
    let dlat = lat2 - lat1;
    let dlon = lon2 - lon1;
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * 6_371_000.0 * h.sqrt().asin()
}

/// Haversine initial bearing from point `a` to point `b`, in degrees [0, 360).
pub(crate) fn haversine_bearing(a: &Point<f64>, b: &Point<f64>) -> f64 {
    let (lat1, lon1) = (a.y().to_radians(), a.x().to_radians());
    let (lat2, lon2) = (b.y().to_radians(), b.x().to_radians());
    let dlon = lon2 - lon1;
    let x = dlon.sin() * lat2.cos();
    let y = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
    (x.atan2(y).to_degrees() + 360.0) % 360.0
}

/// Calculate the difference between two headings on a bidirectional track.
///
/// Railway tracks can be traveled in either direction, so a heading and its
/// opposite (180° apart) are considered equivalent.  Returns a value in
/// [0, 90] where 0 = perfectly aligned (same or opposite direction) and
/// 90 = perpendicular.
///
/// # Arguments
///
/// * `heading1` - First heading in degrees (0-360)
/// * `heading2` - Second heading in degrees (0-360)
///
/// # Returns
///
/// Angular difference in degrees (0-90)
pub fn heading_difference(heading1: f64, heading2: f64) -> f64 {
    let diff = (heading1 - heading2).abs() % 360.0;

    // Smallest circular angle in [0, 180]
    let diff = if diff > 180.0 { 360.0 - diff } else { diff };

    // Bidirectional equivalence: 0° ↔ 180° are both "aligned"
    if diff > 90.0 {
        180.0 - diff
    } else {
        diff
    }
}

/// Calculate the directional difference between two headings.
///
/// Unlike [`heading_difference`], this does NOT apply bidirectional
/// equivalence: 0° and 180° are considered opposite (difference = 180°).
/// Returns a value in [0, 180].
///
/// Used for turn-angle penalties at netelement connections where the direction of
/// travel matters (the train cannot make a U-turn).
pub(crate) fn directional_heading_difference(heading1: f64, heading2: f64) -> f64 {
    let diff = (heading1 - heading2).abs() % 360.0;
    if diff > 180.0 { 360.0 - diff } else { diff }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::LineString;

    #[test]
    fn test_heading_difference_same_direction() {
        let diff = heading_difference(45.0, 45.0);
        assert_eq!(diff, 0.0);
    }

    #[test]
    fn test_heading_difference_opposite_direction() {
        // Opposite direction is equivalent on a bidirectional track
        let diff = heading_difference(0.0, 180.0);
        assert_eq!(diff, 0.0);
    }

    #[test]
    fn test_heading_difference_wraparound() {
        // 350° and 10° are 20° apart (not 340°)
        let diff = heading_difference(350.0, 10.0);
        assert_eq!(diff, 20.0);
    }

    #[test]
    fn test_heading_difference_perpendicular() {
        let diff = heading_difference(0.0, 90.0);
        assert_eq!(diff, 90.0);
    }

    #[test]
    fn test_heading_difference_near_opposite() {
        // 170° difference from north → only 10° from the opposite direction
        let diff = heading_difference(0.0, 170.0);
        assert_eq!(diff, 10.0);
    }

    #[test]
    fn test_candidate_selection_within_cutoff() {
        // Create a simple linestring
        let linestring = LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]);
        let netelement =
            Netelement::new("NE_TEST".to_string(), linestring, "EPSG:4326".to_string()).unwrap();

        // Create a GNSS position near the linestring
        let gnss = GnssPosition::new(
            50.8502,
            4.3502,
            chrono::Utc::now().into(),
            "EPSG:4326".to_string(),
        )
        .unwrap();

        // Find candidates
        let candidates = find_candidate_netelements(&gnss, &[netelement], 50.0, 10).unwrap();

        // Should find the netelement
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].netelement_id, "NE_TEST");
        assert!(candidates[0].distance_meters < 50.0);
    }

    #[test]
    fn test_candidate_selection_beyond_cutoff() {
        let linestring = LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]);
        let netelement =
            Netelement::new("NE_TEST".to_string(), linestring, "EPSG:4326".to_string()).unwrap();

        // Create a GNSS position far from the linestring
        let gnss = GnssPosition::new(
            50.90,
            4.40,
            chrono::Utc::now().into(),
            "EPSG:4326".to_string(),
        )
        .unwrap();

        // Find candidates with small cutoff
        let candidates = find_candidate_netelements(&gnss, &[netelement], 1.0, 10).unwrap();

        // Should not find the netelement
        assert_eq!(candidates.len(), 0);
    }

    #[test]
    fn test_candidate_max_candidates_limit() {
        // Create three netelements
        let netelements = vec![
            Netelement::new(
                "NE_1".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_2".to_string(),
                LineString::from(vec![(4.3502, 50.8502), (4.3512, 50.8512)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_3".to_string(),
                LineString::from(vec![(4.3503, 50.8503), (4.3513, 50.8513)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        let gnss = GnssPosition::new(
            50.8502,
            4.3502,
            chrono::Utc::now().into(),
            "EPSG:4326".to_string(),
        )
        .unwrap();

        // Find candidates with max_candidates = 2
        let candidates = find_candidate_netelements(&gnss, &netelements, 500.0, 2).unwrap();

        // Should return at most 2 candidates
        assert!(candidates.len() <= 2);
    }

    // ── Edge rejection tests ─────────────────────────────────────────────

    #[test]
    fn test_edge_projection_at_start_kept_as_fallback() {
        // Linestring running east.  GNSS point placed exactly at the start
        // (lat/lon matching the first coordinate) → intrinsic ≈ 0.0.
        // With no interior candidates, the edge candidate is kept as fallback.
        let linestring = LineString::from(vec![(4.350, 50.850), (4.360, 50.850)]);
        let netelement =
            Netelement::new("NE_EDGE".to_string(), linestring, "EPSG:4326".to_string()).unwrap();

        let gnss = GnssPosition::new(
            50.850,
            4.350,
            chrono::Utc::now().into(),
            "EPSG:4326".to_string(),
        )
        .unwrap();

        let candidates = find_candidate_netelements(&gnss, &[netelement], 500.0, 10).unwrap();
        assert_eq!(
            candidates.len(),
            1,
            "Edge candidate kept as fallback when no interior candidates exist"
        );
    }

    #[test]
    fn test_edge_projection_at_end_kept_as_fallback() {
        // GNSS point placed exactly at the end of the linestring → intrinsic ≈ 1.0
        // With no interior candidates, the edge candidate is kept as fallback.
        let linestring = LineString::from(vec![(4.350, 50.850), (4.360, 50.850)]);
        let netelement =
            Netelement::new("NE_EDGE".to_string(), linestring, "EPSG:4326".to_string()).unwrap();

        let gnss = GnssPosition::new(
            50.850,
            4.360,
            chrono::Utc::now().into(),
            "EPSG:4326".to_string(),
        )
        .unwrap();

        let candidates = find_candidate_netelements(&gnss, &[netelement], 500.0, 10).unwrap();
        assert_eq!(
            candidates.len(),
            1,
            "Edge candidate kept as fallback when no interior candidates exist"
        );
    }

    #[test]
    fn test_mid_range_projection_accepted() {
        // GNSS point near the midpoint of the linestring → intrinsic ≈ 0.5 → accepted.
        let linestring = LineString::from(vec![(4.350, 50.850), (4.360, 50.850)]);
        let netelement =
            Netelement::new("NE_MID".to_string(), linestring, "EPSG:4326".to_string()).unwrap();

        let gnss = GnssPosition::new(
            50.850,
            4.355,
            chrono::Utc::now().into(),
            "EPSG:4326".to_string(),
        )
        .unwrap();

        let candidates = find_candidate_netelements(&gnss, &[netelement], 500.0, 10).unwrap();
        assert_eq!(candidates.len(), 1);
        assert!(
            candidates[0].intrinsic_coordinate > 0.1 && candidates[0].intrinsic_coordinate < 0.9,
            "Midpoint candidate should have intrinsic near 0.5"
        );
    }

    #[test]
    fn test_edge_candidate_removed_when_interior_exists() {
        // Two netelements: one projects to the midpoint (interior), one to its
        // endpoint (edge).  The edge candidate is removed.
        let ne_interior = Netelement::new(
            "NE_INT".to_string(),
            LineString::from(vec![(4.350, 50.850), (4.360, 50.850)]),
            "EPSG:4326".to_string(),
        )
        .unwrap();
        let ne_edge = Netelement::new(
            "NE_EDGE".to_string(),
            LineString::from(vec![(4.353, 50.851), (4.355, 50.851)]),
            "EPSG:4326".to_string(),
        )
        .unwrap();

        // GNSS at midpoint of NE_INT, but past the end of NE_EDGE
        let gnss = GnssPosition::new(
            50.850,
            4.355,
            chrono::Utc::now().into(),
            "EPSG:4326".to_string(),
        )
        .unwrap();

        let candidates =
            find_candidate_netelements(&gnss, &[ne_interior, ne_edge], 500.0, 10).unwrap();
        assert!(
            candidates.iter().all(|c| c.netelement_id != "NE_EDGE"
                || (c.intrinsic_coordinate > 1e-6 && c.intrinsic_coordinate < 1.0 - 1e-6)),
            "Edge candidates should be removed when interior candidates exist"
        );
    }

    // ── Heading estimation tests ─────────────────────────────────────────

    fn make_gnss(lat: f64, lon: f64) -> GnssPosition {
        GnssPosition::new(lat, lon, chrono::Utc::now().into(), "EPSG:4326".to_string()).unwrap()
    }

    #[test]
    fn test_estimate_headings_straight_north() {
        // Three points along the same meridian heading due north, equally spaced.
        let positions = vec![
            make_gnss(50.000, 4.000),
            make_gnss(50.001, 4.000),
            make_gnss(50.002, 4.000),
        ];
        let refs: Vec<&GnssPosition> = positions.iter().collect();
        let headings = estimate_headings_from_neighbors(&refs);

        assert!(headings[0].is_none(), "First position should be None");
        assert!(headings[2].is_none(), "Last position should be None");
        let h = headings[1].expect("Middle position should have estimated heading");
        // Bearing from position 0 to position 2 should be ≈ 0° (north)
        assert!(h < 1.0 || h > 359.0, "Expected ~0° north, got {h}");
    }

    #[test]
    fn test_estimate_headings_straight_east() {
        // Three points along the same latitude heading due east, equally spaced.
        let positions = vec![
            make_gnss(50.000, 4.000),
            make_gnss(50.000, 4.001),
            make_gnss(50.000, 4.002),
        ];
        let refs: Vec<&GnssPosition> = positions.iter().collect();
        let headings = estimate_headings_from_neighbors(&refs);

        let h = headings[1].expect("Middle position should have estimated heading");
        assert!(
            (h - 90.0).abs() < 1.0,
            "Expected ~90° east, got {h}"
        );
    }

    #[test]
    fn test_estimate_headings_endpoints_none() {
        let positions = vec![
            make_gnss(50.000, 4.000),
            make_gnss(50.001, 4.000),
        ];
        let refs: Vec<&GnssPosition> = positions.iter().collect();
        let headings = estimate_headings_from_neighbors(&refs);
        assert!(headings.iter().all(|h| h.is_none()), "With < 3 positions all should be None");
    }

    #[test]
    fn test_estimate_headings_unequal_spacing() {
        // Distance from p0→p1 is much larger than p1→p2 → ratio > 20% → None
        let positions = vec![
            make_gnss(50.000, 4.000),
            make_gnss(50.010, 4.000), // ~1.1 km north
            make_gnss(50.0101, 4.000), // ~11 m further north
        ];
        let refs: Vec<&GnssPosition> = positions.iter().collect();
        let headings = estimate_headings_from_neighbors(&refs);
        assert!(
            headings[1].is_none(),
            "Unequal spacing should reject estimated heading"
        );
    }

    #[test]
    fn test_estimate_headings_continuity_rejection() {
        // Five points: first three go north, then a sharp 90° turn east.
        // The position after the turn should fail the 5° continuity check.
        let positions = vec![
            make_gnss(50.000, 4.000),
            make_gnss(50.001, 4.000), // heading calc: bearing from 0→2 ≈ north
            make_gnss(50.002, 4.000), // heading calc: bearing from 1→3 ≈ NE (sharp change)
            make_gnss(50.002, 4.001), // heading calc: bearing from 2→4 ≈ east
            make_gnss(50.002, 4.002),
        ];
        let refs: Vec<&GnssPosition> = positions.iter().collect();
        let headings = estimate_headings_from_neighbors(&refs);

        // Position 1: should have a heading (north)
        assert!(headings[1].is_some(), "Position 1 heading should be valid");
        // Positions 2 or 3 should have None due to the sharp turn
        let has_rejection = headings[2].is_none() || headings[3].is_none();
        assert!(has_rejection, "Sharp turn should cause at least one heading rejection");
    }
}
