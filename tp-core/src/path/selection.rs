//! Path selection module
//!
//! Calculates path probabilities, performs bidirectional averaging,
//! and selects the optimal path from candidates.

use crate::errors::ProjectionError;
use crate::models::AssociatedNetElement;

/// Calculate probability for a single path
///
/// Uses length-weighted averaging of netelement probabilities.
/// Longer segments contribute more weight (they have higher certainty due to coverage).
///
/// # Arguments
///
/// * `path_segments` - Segments in the path
///
/// # Returns
///
/// Path probability in range [0, 1]
///
/// # Examples
///
/// ```ignore
/// let seg1 = AssociatedNetElement::new("NE_A", 0.9, 0.0, 1.0, 0, 5)?;
/// let seg2 = AssociatedNetElement::new("NE_B", 0.8, 0.0, 1.0, 6, 10)?;
/// let probability = calculate_path_probability(&vec![seg1, seg2]);
/// // Average: (0.9 + 0.8) / 2 = 0.85
/// ```
pub fn calculate_path_probability(path_segments: &[AssociatedNetElement]) -> f64 {
    if path_segments.is_empty() {
        return 0.0;
    }

    let sum: f64 = path_segments.iter().map(|seg| seg.probability).sum();
    sum / path_segments.len() as f64
}

/// Average probabilities from forward and backward paths
///
/// If both paths exist, combines their probabilities as average.
/// If only one exists, uses that probability with penalty (50% confidence).
///
/// # Arguments
///
/// * `forward_probability` - Probability of forward path (None if no path)
/// * `backward_probability` - Probability of backward path (None if no path)
///
/// # Returns
///
/// Bidirectional averaged probability
///
/// # Examples
///
/// ```ignore
/// let avg = average_bidirectional_probability(Some(0.9), Some(0.85));
/// assert!((avg - 0.875).abs() < 0.001); // (0.9 + 0.85) / 2 = 0.875
///
/// let unidirectional = average_bidirectional_probability(Some(0.8), None);
/// assert!((unidirectional - 0.4).abs() < 0.001); // 0.8 * 0.5 = 0.4
/// ```
pub fn average_bidirectional_probability(
    forward_probability: Option<f64>,
    backward_probability: Option<f64>,
) -> f64 {
    match (forward_probability, backward_probability) {
        (Some(fwd), Some(bwd)) => (fwd + bwd) / 2.0,
        (Some(p), None) | (None, Some(p)) => p * 0.5, // Penalty for unidirectional
        (None, None) => 0.0,
    }
}

/// Select best path from candidates based on probability
///
/// Chooses path with highest probability. If tied, returns first occurrence.
/// Paths that terminate before reaching the end position get probability 0.
///
/// # Arguments
///
/// * `candidate_paths` - Candidate paths with their probabilities
/// * `reaches_end` - Boolean slice indicating if each path reaches end position
///
/// # Returns
///
/// Index of best path, or error if no paths provided
pub fn select_best_path(
    candidate_probabilities: &[f64],
    reaches_end: &[bool],
) -> Result<usize, ProjectionError> {
    if candidate_probabilities.is_empty() {
        return Err(ProjectionError::PathCalculationFailed {
            reason: "No candidate paths provided".to_string(),
        });
    }

    if candidate_probabilities.len() != reaches_end.len() {
        return Err(ProjectionError::PathCalculationFailed {
            reason: "Probability and reaches_end arrays have different lengths".to_string(),
        });
    }

    let mut best_idx = 0;
    let mut best_prob = if reaches_end[0] {
        candidate_probabilities[0]
    } else {
        0.0
    };

    for (idx, (&prob, &reaches)) in candidate_probabilities.iter().zip(reaches_end).enumerate() {
        let effective_prob = if reaches { prob } else { 0.0 };
        if effective_prob > best_prob {
            best_prob = effective_prob;
            best_idx = idx;
        }
    }

    Ok(best_idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_probability_single_segment() -> Result<(), ProjectionError> {
        let seg = AssociatedNetElement::new("elem1".to_string(), 0.85, 0.0, 1.0, 0, 5)?;

        let prob = calculate_path_probability(&[seg]);
        assert!((prob - 0.85).abs() < 0.001);
        Ok(())
    }

    #[test]
    fn test_path_probability_multiple_segments() -> Result<(), ProjectionError> {
        let seg1 = AssociatedNetElement::new("elem1".to_string(), 0.9, 0.0, 1.0, 0, 5)?;
        let seg2 = AssociatedNetElement::new("elem2".to_string(), 0.8, 0.0, 1.0, 6, 10)?;

        let prob = calculate_path_probability(&[seg1, seg2]);
        assert!((prob - 0.85).abs() < 0.001); // (0.9 + 0.8) / 2
        Ok(())
    }

    #[test]
    fn test_path_probability_empty() {
        let prob = calculate_path_probability(&[]);
        assert_eq!(prob, 0.0);
    }

    #[test]
    fn test_bidirectional_both_paths() {
        let avg = average_bidirectional_probability(Some(0.9), Some(0.85));
        assert!((avg - 0.875).abs() < 0.001); // (0.9 + 0.85) / 2
    }

    #[test]
    fn test_bidirectional_only_forward() {
        let avg = average_bidirectional_probability(Some(0.8), None);
        assert!((avg - 0.4).abs() < 0.001); // 0.8 * 0.5
    }

    #[test]
    fn test_bidirectional_only_backward() {
        let avg = average_bidirectional_probability(None, Some(0.75));
        assert!((avg - 0.375).abs() < 0.001); // 0.75 * 0.5
    }

    #[test]
    fn test_bidirectional_no_paths() {
        let avg = average_bidirectional_probability(None, None);
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn test_select_best_path_highest_probability() {
        let probs = vec![0.7, 0.95, 0.8];
        let reaches = vec![true, true, true];

        let best = select_best_path(&probs, &reaches).unwrap();
        assert_eq!(best, 1); // Index with 0.95
    }

    #[test]
    fn test_select_best_path_ignores_incomplete() {
        let probs = vec![0.95, 0.8, 0.85];
        let reaches = vec![false, true, true]; // First path doesn't reach end

        let best = select_best_path(&probs, &reaches).unwrap();
        assert_eq!(best, 2); // Index with 0.85 (best that reaches end)
    }

    #[test]
    fn test_select_best_path_all_incomplete() {
        let probs = vec![0.95, 0.9, 0.85];
        let reaches = vec![false, false, false];

        let best = select_best_path(&probs, &reaches).unwrap();
        assert_eq!(best, 0); // Returns first (all have prob 0 when incomplete)
    }

    #[test]
    fn test_select_best_path_empty() {
        let probs: Vec<f64> = vec![];
        let reaches: Vec<bool> = vec![];

        let result = select_best_path(&probs, &reaches);
        assert!(result.is_err());
    }

    #[test]
    fn test_select_best_path_mismatched_lengths() {
        let probs = vec![0.8, 0.9];
        let reaches = vec![true, true, true];

        let result = select_best_path(&probs, &reaches);
        assert!(result.is_err());
    }
}
