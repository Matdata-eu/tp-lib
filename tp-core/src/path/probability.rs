//! Probability calculation module
//!
//! Implements exponential decay probability formulas for distance and heading,
//! netelement-level probability aggregation, and coverage factor calculation.

/// Configuration for probability calculations
#[derive(Debug, Clone)]
pub struct ProbabilityConfig {
    /// Distance scale for exponential decay (meters)
    pub distance_scale: f64,
    /// Heading scale for exponential decay (degrees)
    pub heading_scale: f64,
    /// Maximum heading difference to accept (degrees)
    pub heading_cutoff: f64,
}

impl Default for ProbabilityConfig {
    fn default() -> Self {
        Self {
            distance_scale: 100.0, // 100m scale
            heading_scale: 45.0,   // 45° scale
            heading_cutoff: 90.0,  // reject if >90° difference
        }
    }
}

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

/// Netelement probability calculation with averaging.
///
/// When multiple GNSS positions are assigned to the same netelement,
/// the probability is the average of their individual probabilities.
///
/// # Arguments
///
/// * `position_probabilities` - Slice of probabilities for positions on this segment
///
/// # Returns
///
/// Average probability, or 0.0 if no positions
pub fn calculate_netelement_probability(position_probabilities: &[f64]) -> f64 {
    if position_probabilities.is_empty() {
        0.0
    } else {
        let sum: f64 = position_probabilities.iter().sum();
        sum / position_probabilities.len() as f64
    }
}

/// Apply coverage correction to netelement probability.
///
/// If GNSS positions are continuous along the netelement, the probability
/// is weighted higher than if they are sparse.
///
/// # Arguments
///
/// * `base_probability` - Probability before coverage correction
/// * `coverage_factor` - Coverage factor in range [0, 1]
///   - 1.0: continuous coverage (full distance)
///   - 0.5: 50% of segment covered by GNSS points
///   - 0.0: no coverage
///
/// # Returns
///
/// Coverage-corrected probability
pub fn apply_coverage_correction(base_probability: f64, coverage_factor: f64) -> f64 {
    // Weight by coverage: better coverage increases confidence
    base_probability * (0.5 + 0.5 * coverage_factor)
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
    fn test_netelement_probability_averaging() {
        // Average of [0.8, 0.9, 0.7] should be 0.8
        let probs = vec![0.8, 0.9, 0.7];
        let avg = calculate_netelement_probability(&probs);
        assert!((avg - 0.8).abs() < 0.0001);
    }

    #[test]
    fn test_netelement_probability_empty() {
        // Empty slice should give 0 probability
        let probs = vec![];
        let avg = calculate_netelement_probability(&probs);
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn test_coverage_correction_full_coverage() {
        // Full coverage (1.0) should increase probability most
        let corrected = apply_coverage_correction(0.5, 1.0);
        assert!((corrected - 0.5).abs() < 0.0001); // 0.5 * (0.5 + 0.5*1.0) = 0.5
    }

    #[test]
    fn test_coverage_correction_no_coverage() {
        // No coverage (0.0) should reduce probability
        let corrected = apply_coverage_correction(0.5, 0.0);
        assert!((corrected - 0.25).abs() < 0.0001); // 0.5 * (0.5 + 0.5*0.0) = 0.25
    }

    #[test]
    fn test_coverage_correction_half_coverage() {
        // Half coverage (0.5) should give intermediate result
        let corrected = apply_coverage_correction(0.5, 0.5);
        assert!((corrected - 0.375).abs() < 0.0001); // 0.5 * (0.5 + 0.5*0.5) = 0.375
    }
}
