//! Candidate netelement selection for GNSS positions
//!
//! Finds candidate netelements that could match each GNSS position based on
//! spatial proximity and heading alignment.

use crate::errors::ProjectionError;
use crate::models::{GnssPosition, Netelement};
use geo::{LineString, Point};

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
    use geo::algorithm::euclidean_distance::EuclideanDistance;

    // Find the closest point by checking each line segment
    let mut min_distance = f64::INFINITY;
    let mut closest_point = linestring.0[0];
    let mut closest_param = 0.0;

    for i in 0..linestring.0.len() - 1 {
        let p1 = Point::new(linestring.0[i].x, linestring.0[i].y);
        let p2 = Point::new(linestring.0[i + 1].x, linestring.0[i + 1].y);

        // Calculate closest point on this segment
        let dx = p2.x() - p1.x();
        let dy = p2.y() - p1.y();
        let len_sq = dx * dx + dy * dy;

        let t = if len_sq > 0.0 {
            ((point.x() - p1.x()) * dx + (point.y() - p1.y()) * dy) / len_sq
        } else {
            0.0
        };

        let t_clamped = t.clamp(0.0, 1.0);
        let seg_point = Point::new(p1.x() + t_clamped * dx, p1.y() + t_clamped * dy);

        let dist = point.euclidean_distance(&seg_point);
        if dist < min_distance {
            min_distance = dist;
            closest_point = seg_point.0;
            closest_param = (i as f64 + t_clamped) / (linestring.0.len() - 1) as f64;
        }
    }

    let closest_pt = Point::new(closest_point.x, closest_point.y);

    // Calculate distance in meters (rough approximation for small distances)
    let lat_diff = (point.y() - closest_pt.y()) * 111320.0; // 1° ≈ 111.32 km
    let lon_diff = (point.x() - closest_pt.x()) * 111320.0 * (point.y().to_radians()).cos();
    let distance = (lat_diff * lat_diff + lon_diff * lon_diff).sqrt();

    Ok((distance, closest_param, closest_pt))
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

    // Find the segment containing or closest to the point
    let mut closest_segment_idx = 0;
    let mut closest_distance = f64::MAX;

    for (i, segment_start) in coords.iter().enumerate().take(coords.len() - 1) {
        let lat_diff = (point.y() - segment_start.y()) * 111320.0;
        let lon_diff =
            (point.x() - segment_start.x()) * 111320.0 * segment_start.y().to_radians().cos();
        let distance = (lat_diff * lat_diff + lon_diff * lon_diff).sqrt();

        if distance < closest_distance {
            closest_distance = distance;
            closest_segment_idx = i;
        }
    }

    // Get segment direction
    let p1 = coords[closest_segment_idx];
    let p2 = coords[closest_segment_idx + 1];

    // Calculate bearing in degrees
    let lat_diff = p2.y() - p1.y();
    let lon_diff = p2.x() - p1.x();
    let bearing = (lon_diff.atan2(lat_diff).to_degrees() + 360.0) % 360.0;

    Ok(bearing)
}

/// Calculate the difference between two headings
///
/// Considers the circular nature of headings (180° is same as -180°).
/// Returns value in range [0, 180] representing the smaller angle between the two headings.
///
/// # Arguments
///
/// * `heading1` - First heading in degrees (0-360)
/// * `heading2` - Second heading in degrees (0-360)
///
/// # Returns
///
/// Angular difference in degrees (0-180)
pub fn heading_difference(heading1: f64, heading2: f64) -> f64 {
    let diff = (heading1 - heading2).abs();

    // Return the smaller angle (considering 360° wrap-around)
    if diff > 180.0 {
        360.0 - diff
    } else {
        diff
    }
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
        let diff = heading_difference(0.0, 180.0);
        assert_eq!(diff, 180.0);
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
}
