//! Probability calculation module
//!
//! Implements exponential decay probability formulas for distance and heading,
//! and transition probability for HMM-based map matching.

/// Calculate probability based on distance using exponential decay.
///
/// # Arguments
///
/// * `distance_meters` - Distance from GNSS position to closest point on segment (meters)
/// * `distance_scale` - Decay scale parameter (meters). At distance = scale, probability ≈ 0.368
///
/// # Returns
///
/// Probability value in range [0, 1]
///
/// # Examples
///
/// ```ignore
/// let p = calculate_distance_probability(50.0, 100.0);
/// assert!((p - 0.6065).abs() < 0.001); // exp(-0.5) ≈ 0.6065
///
/// let p_at_scale = calculate_distance_probability(100.0, 100.0);
/// assert!((p_at_scale - 0.368).abs() < 0.001); // exp(-1.0) ≈ 0.368
/// ```
pub fn calculate_distance_probability(distance_meters: f64, distance_scale: f64) -> f64 {
    if distance_scale <= 0.0 {
        if distance_meters <= 0.0 {
            1.0
        } else {
            0.0
        }
    } else {
        (-distance_meters / distance_scale).exp()
    }
}

/// Calculate probability based on heading difference using exponential decay.
///
/// # Arguments
///
/// * `heading_difference_degrees` - Absolute angular difference between GNSS heading and segment heading (degrees)
/// * `heading_scale` - Decay scale parameter (degrees). At diff = scale, probability ≈ 0.368
/// * `heading_cutoff_degrees` - Maximum heading difference to accept (degrees). Above this, probability = 0
///
/// # Returns
///
/// Probability value in range [0, 1]. Returns 0 if difference exceeds cutoff.
///
/// # Examples
///
/// ```ignore
/// let p = calculate_heading_probability(30.0, 45.0, 90.0);
/// assert!((p - (-30.0 / 45.0).exp()).abs() < 0.001); // exp(-0.667) ≈ 0.513
///
/// let p_rejected = calculate_heading_probability(100.0, 45.0, 90.0);
/// assert_eq!(p_rejected, 0.0); // exceeds cutoff
/// ```
pub fn calculate_heading_probability(
    heading_difference_degrees: f64,
    heading_scale: f64,
    heading_cutoff_degrees: f64,
) -> f64 {
    if heading_difference_degrees > heading_cutoff_degrees {
        0.0
    } else if heading_scale <= 0.0 {
        if heading_difference_degrees <= 0.0 {
            1.0
        } else {
            0.0
        }
    } else {
        (-heading_difference_degrees / heading_scale).exp()
    }
}

/// Calculate combined probability from distance and heading components.
///
/// Assumes independence, so combined probability is the product of components.
///
/// # Arguments
///
/// * `distance_probability` - Probability from distance (0-1)
/// * `heading_probability` - Probability from heading (0-1)
///
/// # Returns
///
/// Combined probability in range [0, 1]
///
/// # Examples
///
/// ```ignore
/// let combined = calculate_combined_probability(0.8, 0.9);
/// assert!((combined - 0.72).abs() < 0.001); // 0.8 * 0.9 = 0.72
/// ```
pub fn calculate_combined_probability(distance_probability: f64, heading_probability: f64) -> f64 {
    distance_probability * heading_probability
}

/// Calculate transition probability between two map-matched candidates
/// using the Newson & Krumm (2009) formula.
///
/// The transition probability penalises candidates whose shortest-path
/// distance through the network deviates from the great-circle distance
/// between the two GNSS observations.
///
/// # Formula
///
/// `exp(-|route_distance - great_circle_distance| / beta)`
///
/// # Arguments
///
/// * `route_distance` - Shortest-path distance through the network (meters)
/// * `great_circle_distance` - Great-circle (haversine) distance between the two GNSS positions (meters)
/// * `beta` - Scale parameter controlling tolerance for route/GC mismatch (meters)
///
/// # Returns
///
/// Probability in (0, 1]. Returns 1.0 when route distance equals great-circle distance.
pub fn calculate_transition_probability(
    route_distance: f64,
    great_circle_distance: f64,
    beta: f64,
) -> f64 {
    if beta <= 0.0 {
        // Degenerate: only exact matches get probability 1
        if (route_distance - great_circle_distance).abs() < 1e-9 {
            1.0
        } else {
            0.0
        }
    } else {
        (-(route_distance - great_circle_distance).abs() / beta).exp()
    }
}

/// Check whether a candidate's projected point is near a netelement endpoint.
///
/// Computes the haversine distance from the projected point to the nearest
/// geometric endpoint of the netelement. If that distance is within
/// `edge_zone_distance` meters, the candidate is "near the edge" and may
/// transition to an adjacent netelement.
///
/// Used as an optimization: candidates that are well inside a netelement
/// (far from both endpoints) cannot plausibly transition to a different
/// netelement, so Dijkstra routing can be skipped for them.
pub fn is_near_netelement_edge(
    projected_point: &geo::Point<f64>,
    netelement_geometry: &geo::LineString<f64>,
    edge_zone_distance: f64,
) -> bool {
    use geo::HaversineDistance;

    let coords = netelement_geometry.coords().collect::<Vec<_>>();
    if coords.is_empty() {
        return true; // Degenerate — treat as edge
    }

    let start = geo::Point::new(coords[0].x, coords[0].y);
    let end = geo::Point::new(coords[coords.len() - 1].x, coords[coords.len() - 1].y);

    let dist_to_start = projected_point.haversine_distance(&start);
    let dist_to_end = projected_point.haversine_distance(&end);

    dist_to_start.min(dist_to_end) <= edge_zone_distance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_probability_at_scale() {
        // At distance = scale, probability should be exp(-1) ≈ 0.368
        let p = calculate_distance_probability(100.0, 100.0);
        assert!((p - 0.36787944).abs() < 0.0001);
    }

    #[test]
    fn test_distance_probability_zero_distance() {
        // At zero distance, probability should be 1.0
        let p = calculate_distance_probability(0.0, 100.0);
        assert_eq!(p, 1.0);
    }

    #[test]
    fn test_distance_probability_large_distance() {
        // Large distance should give small probability
        let p = calculate_distance_probability(1000.0, 100.0);
        assert!(p < 0.001);
    }

    #[test]
    fn test_heading_probability_at_scale() {
        // At heading_diff = scale, probability should be exp(-1) ≈ 0.368
        let p = calculate_heading_probability(45.0, 45.0, 90.0);
        assert!((p - 0.36787944).abs() < 0.0001);
    }

    #[test]
    fn test_heading_probability_zero_difference() {
        // At zero heading difference, probability should be 1.0
        let p = calculate_heading_probability(0.0, 45.0, 90.0);
        assert_eq!(p, 1.0);
    }

    #[test]
    fn test_heading_probability_exceeds_cutoff() {
        // Beyond cutoff, probability should be 0
        let p = calculate_heading_probability(100.0, 45.0, 90.0);
        assert_eq!(p, 0.0);
    }

    #[test]
    fn test_heading_probability_at_cutoff_boundary() {
        // At exactly the cutoff, should be accepted (not > cutoff)
        let p = calculate_heading_probability(90.0, 45.0, 90.0);
        assert!(p > 0.0); // Should be accepted
        let p_exact = calculate_heading_probability(90.0, 45.0, 90.0);
        let expected = (-90.0_f64 / 45.0_f64).exp();
        assert!((p_exact - expected).abs() < 0.0001);
    }

    #[test]
    fn test_combined_probability() {
        // Combined should be product of components
        let combined = calculate_combined_probability(0.8, 0.75);
        assert!((combined - 0.6).abs() < 0.0001); // 0.8 * 0.75 = 0.6
    }

    #[test]
    fn test_combined_probability_zero_component() {
        // If any component is zero, combined should be zero
        let combined = calculate_combined_probability(0.8, 0.0);
        assert_eq!(combined, 0.0);
    }

    #[test]
    fn test_transition_probability_perfect_match() {
        // route == gc → exp(0) = 1.0
        let p = calculate_transition_probability(100.0, 100.0, 50.0);
        assert!((p - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_transition_probability_at_beta() {
        // |route - gc| = beta → exp(-1) ≈ 0.368
        let p = calculate_transition_probability(150.0, 100.0, 50.0);
        assert!((p - 0.36787944).abs() < 0.0001);
    }

    #[test]
    fn test_transition_probability_large_mismatch() {
        // Large mismatch → very small probability
        let p = calculate_transition_probability(500.0, 100.0, 50.0);
        assert!(p < 0.001);
    }

    #[test]
    fn test_transition_probability_symmetric() {
        // route < gc should give same result as route > gc with same |diff|
        let p1 = calculate_transition_probability(80.0, 100.0, 50.0);
        let p2 = calculate_transition_probability(120.0, 100.0, 50.0);
        assert!((p1 - p2).abs() < 1e-9);
    }

    #[test]
    fn test_near_edge_at_start() {
        // Point near the first coordinate of the line
        let line = geo::LineString::from(vec![(3.0, 50.0), (3.001, 50.0), (3.002, 50.0)]);
        let point = geo::Point::new(3.0001, 50.0);
        assert!(is_near_netelement_edge(&point, &line, 50.0));
    }

    #[test]
    fn test_near_edge_at_end() {
        // Point near the last coordinate of the line
        let line = geo::LineString::from(vec![(3.0, 50.0), (3.001, 50.0), (3.002, 50.0)]);
        let point = geo::Point::new(3.0019, 50.0);
        assert!(is_near_netelement_edge(&point, &line, 50.0));
    }

    #[test]
    fn test_not_near_edge_interior() {
        // Point in the middle of a long enough line — not near either endpoint
        // 3.0 to 3.01 ≈ ~715m at lat 50, so midpoint is ~357m from each end
        let line = geo::LineString::from(vec![(3.0, 50.0), (3.005, 50.0), (3.01, 50.0)]);
        let point = geo::Point::new(3.005, 50.0);
        assert!(!is_near_netelement_edge(&point, &line, 50.0));
    }

    #[test]
    fn test_near_edge_empty_geometry() {
        // Degenerate geometry — should return true (safe default)
        let line = geo::LineString::from(Vec::<(f64, f64)>::new());
        let point = geo::Point::new(3.0, 50.0);
        assert!(is_near_netelement_edge(&point, &line, 50.0));
    }
}
