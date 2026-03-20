//! Spacing calculation for GNSS positions
//!
//! Provides utilities for calculating mean spacing between consecutive GNSS positions,
//! used for resampling optimization.

use crate::models::GnssPosition;

/// Calculate mean spacing between consecutive GNSS positions
///
/// Uses distance column values when available (from wheel sensors),
/// otherwise falls back to geometric distance calculation.
/// This is used for resampling to determine optimal sampling interval.
///
/// # Arguments
///
/// * `gnss_positions` - Slice of GNSS positions in temporal order
///
/// # Returns
///
/// Mean distance in meters between consecutive positions, or 0.0 if fewer than 2 positions
///
/// # Examples
///
/// ```
/// use tp_lib_core::GnssPosition;
/// use chrono::Utc;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let positions = vec![
///     GnssPosition::new(50.8503, 4.3502, Utc::now().into(), "EPSG:4326".to_string())?,
///     GnssPosition::new(50.8513, 4.3512, Utc::now().into(), "EPSG:4326".to_string())?,
/// ];
///
/// let mean_spacing = tp_lib_core::calculate_mean_spacing(&positions);
/// assert!(mean_spacing > 0.0);
/// # Ok(())
/// # }
/// ```
pub fn calculate_mean_spacing(gnss_positions: &[GnssPosition]) -> f64 {
    if gnss_positions.len() < 2 {
        return 0.0;
    }

    let mut total_distance = 0.0;
    let mut count = 0;

    for i in 0..gnss_positions.len() - 1 {
        let curr = &gnss_positions[i];
        let next = &gnss_positions[i + 1];

        // Use distance column if available (T119, T128)
        let spacing = if let (Some(curr_dist), Some(next_dist)) = (curr.distance, next.distance) {
            // Distance column is cumulative, so calculate the difference
            (next_dist - curr_dist).abs()
        } else {
            // Fall back to geometric distance calculation (T128)
            use geo::{HaversineDistance, Point};
            let p1 = Point::new(curr.longitude, curr.latitude);
            let p2 = Point::new(next.longitude, next.latitude);
            p1.haversine_distance(&p2)
        };

        total_distance += spacing;
        count += 1;
    }

    if count > 0 {
        total_distance / count as f64
    } else {
        0.0
    }
}

/// Select a resampled subset of GNSS positions for path calculation
///
/// Takes every Nth position based on the resampling distance and mean spacing.
/// This reduces computational load while maintaining path structure accuracy.
///
/// # Arguments
///
/// * `gnss_positions` - Full set of GNSS positions in temporal order
/// * `resampling_distance` - Target distance between resampled positions (meters)
///
/// # Returns
///
/// Indices of positions to use for path calculation. Returns all indices if
/// resampling is not beneficial (fewer than 3 positions, or step size < 2).
///
/// # Examples
///
/// ```
/// use tp_lib_core::GnssPosition;
/// use chrono::Utc;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Create 100 positions at 1m spacing
/// let positions: Vec<GnssPosition> = (0..100)
///     .map(|i| {
///         let mut pos = GnssPosition::new(
///             50.85 + i as f64 * 0.00001,
///             4.35,
///             Utc::now().into(),
///             "EPSG:4326".to_string()
///         ).unwrap();
///         pos.distance = Some(i as f64); // 1m spacing
///         pos
///     })
///     .collect();
///
/// // Resample at 10m intervals
/// let indices = tp_lib_core::select_resampled_subset(&positions, 10.0);
/// // Approximately 10-12 positions selected (includes first and last)
/// assert!(indices.len() >= 10 && indices.len() <= 12);
/// assert_eq!(indices[0], 0); // First position always included
/// assert_eq!(*indices.last().unwrap(), 99); // Last position always included
/// # Ok(())
/// # }
/// ```
pub fn select_resampled_subset(
    gnss_positions: &[GnssPosition],
    resampling_distance: f64,
) -> Vec<usize> {
    if gnss_positions.len() < 3 || resampling_distance <= 0.0 {
        // Not enough positions or invalid distance - return all indices
        return (0..gnss_positions.len()).collect();
    }

    let mean_spacing = calculate_mean_spacing(gnss_positions);

    if mean_spacing <= 0.0 {
        // Can't determine spacing - return all indices
        return (0..gnss_positions.len()).collect();
    }

    // Calculate step size (how many positions to skip)
    let step_size = (resampling_distance / mean_spacing).ceil() as usize;

    if step_size < 2 {
        // Resampling not beneficial - return all indices
        return (0..gnss_positions.len()).collect();
    }

    // Select every Nth position using step_by
    // Always include first position (index 0) and try to include last
    let mut indices: Vec<usize> = (0..gnss_positions.len()).step_by(step_size).collect();

    // Ensure last position is included if not already there
    let last_idx = gnss_positions.len() - 1;
    if indices.last() != Some(&last_idx) && last_idx > 0 {
        indices.push(last_idx);
    }

    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_mean_spacing_with_distance_column() {
        // T121: Test with distance column values (cumulative distance from wheel sensors)
        let positions = vec![
            GnssPosition::with_heading_distance(
                50.8503,
                4.3502,
                Utc::now().into(),
                "EPSG:4326".to_string(),
                None,
                Some(0.0), // Start at 0
            )
            .unwrap(),
            GnssPosition::with_heading_distance(
                50.8513,
                4.3512,
                Utc::now().into(),
                "EPSG:4326".to_string(),
                None,
                Some(10.0), // 10m from start
            )
            .unwrap(),
            GnssPosition::with_heading_distance(
                50.8523,
                4.3522,
                Utc::now().into(),
                "EPSG:4326".to_string(),
                None,
                Some(23.0), // 23m from start
            )
            .unwrap(),
        ];

        let mean = calculate_mean_spacing(&positions);
        // Spacings: 10.0 - 0.0 = 10.0, 23.0 - 10.0 = 13.0
        // Mean: (10.0 + 13.0) / 2 = 11.5
        assert!(
            (mean - 11.5).abs() < 0.001,
            "Expected mean 11.5, got {}",
            mean
        );
    }

    #[test]
    fn test_mean_spacing_without_distance_column() {
        // T122: Test with geometric distance calculation
        let positions = vec![
            GnssPosition::new(50.8503, 4.3502, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.8513, 4.3512, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.8523, 4.3522, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        let mean = calculate_mean_spacing(&positions);
        // Should calculate geometric distance using Haversine
        assert!(mean > 0.0, "Mean spacing should be positive");
        assert!(mean < 5000.0, "Mean spacing should be reasonable (< 5km)");
    }

    #[test]
    fn test_mean_spacing_single_position() {
        let positions =
            vec![
                GnssPosition::new(50.8503, 4.3502, Utc::now().into(), "EPSG:4326".to_string())
                    .unwrap(),
            ];

        let mean = calculate_mean_spacing(&positions);
        assert_eq!(mean, 0.0, "Single position should return 0.0");
    }

    #[test]
    fn test_mean_spacing_empty() {
        let positions: Vec<GnssPosition> = vec![];

        let mean = calculate_mean_spacing(&positions);
        assert_eq!(mean, 0.0, "Empty positions should return 0.0");
    }

    #[test]
    fn test_mean_spacing_mixed_distance_values() {
        // Some positions have distance, some don't
        let positions = vec![
            GnssPosition::with_heading_distance(
                50.8503,
                4.3502,
                Utc::now().into(),
                "EPSG:4326".to_string(),
                None,
                Some(10.0),
            )
            .unwrap(),
            GnssPosition::new(50.8513, 4.3512, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::with_heading_distance(
                50.8523,
                4.3522,
                Utc::now().into(),
                "EPSG:4326".to_string(),
                None,
                Some(11.0),
            )
            .unwrap(),
        ];

        let mean = calculate_mean_spacing(&positions);
        // Second position uses geometric distance, third uses distance value 11.0
        assert!(
            mean > 0.0,
            "Mean spacing should be positive with mixed data"
        );
    }

    // T135: Unit tests for resampled subset selection
    #[test]
    fn test_select_resampled_subset_basic() {
        // Create 100 positions at 1m spacing
        let positions: Vec<GnssPosition> = (0..100)
            .map(|i| {
                let mut pos = GnssPosition::new(
                    50.85 + i as f64 * 0.00001,
                    4.35,
                    Utc::now().into(),
                    "EPSG:4326".to_string(),
                )
                .unwrap();
                pos.distance = Some(i as f64); // 1m spacing
                pos
            })
            .collect();

        // Resample at 10m intervals
        let indices = select_resampled_subset(&positions, 10.0);

        // Should select approximately every 10th position
        // With 100 positions at 1m spacing, we expect ~10 positions
        assert!(
            indices.len() >= 10 && indices.len() <= 12,
            "Should select ~10 positions, got {}",
            indices.len()
        );

        // First and last should be included
        assert_eq!(indices[0], 0, "First position should be included");
        assert_eq!(
            indices[indices.len() - 1],
            99,
            "Last position should be included"
        );
    }

    #[test]
    fn test_select_resampled_subset_no_resampling_needed() {
        // Create 10 positions at 10m spacing
        let positions: Vec<GnssPosition> = (0..10)
            .map(|i| {
                let mut pos = GnssPosition::new(
                    50.85 + i as f64 * 0.0001,
                    4.35,
                    Utc::now().into(),
                    "EPSG:4326".to_string(),
                )
                .unwrap();
                pos.distance = Some(i as f64 * 10.0); // 10m spacing
                pos
            })
            .collect();

        // Resample at 10m intervals (same as data spacing)
        let indices = select_resampled_subset(&positions, 10.0);

        // Should return all indices (step size < 2)
        assert_eq!(
            indices.len(),
            10,
            "Should return all positions when resampling not beneficial"
        );
    }

    #[test]
    fn test_select_resampled_subset_too_few_positions() {
        // Only 2 positions
        let positions = vec![
            GnssPosition::new(50.8503, 4.3502, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.8513, 4.3512, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        let indices = select_resampled_subset(&positions, 10.0);

        // Should return all indices (too few to resample)
        assert_eq!(indices.len(), 2, "Should return all positions when too few");
        assert_eq!(indices, vec![0, 1]);
    }

    #[test]
    fn test_select_resampled_subset_invalid_distance() {
        let positions: Vec<GnssPosition> = (0..10)
            .map(|i| {
                GnssPosition::new(
                    50.85 + i as f64 * 0.00001,
                    4.35,
                    Utc::now().into(),
                    "EPSG:4326".to_string(),
                )
                .unwrap()
            })
            .collect();

        // Invalid resampling distance
        let indices = select_resampled_subset(&positions, 0.0);
        assert_eq!(
            indices.len(),
            10,
            "Should return all positions for invalid distance"
        );

        let indices = select_resampled_subset(&positions, -5.0);
        assert_eq!(
            indices.len(),
            10,
            "Should return all positions for negative distance"
        );
    }

    #[test]
    fn test_select_resampled_subset_ensures_last_position() {
        // Create positions where last won't naturally be selected
        let positions: Vec<GnssPosition> = (0..99)
            .map(|i| {
                let mut pos = GnssPosition::new(
                    50.85 + i as f64 * 0.00001,
                    4.35,
                    Utc::now().into(),
                    "EPSG:4326".to_string(),
                )
                .unwrap();
                pos.distance = Some(i as f64); // 1m spacing
                pos
            })
            .collect();

        let indices = select_resampled_subset(&positions, 10.0);

        // Last position (index 98) should be included
        assert_eq!(
            indices[indices.len() - 1],
            98,
            "Last position should always be included"
        );
    }
}
