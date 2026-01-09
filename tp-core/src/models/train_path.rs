//! Continuous train path through the rail network

use crate::errors::ProjectionError;
use crate::models::{AssociatedNetElement, PathMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a continuous train path through the rail network
///
/// A TrainPath is an ordered sequence of netelements (track segments) that
/// the train traversed, calculated from GNSS coordinates and network topology.
///
/// # Examples
///
/// ```
/// use tp_lib_core::{TrainPath, AssociatedNetElement};
/// use chrono::Utc;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let segments = vec![
///     AssociatedNetElement::new(
///         "NE_A".to_string(), 0.87, 0.0, 1.0, 0, 10
///     )?,
///     AssociatedNetElement::new(
///         "NE_B".to_string(), 0.92, 0.0, 1.0, 11, 18
///     )?,
/// ];
///
/// let path = TrainPath::new(
///     segments,
///     0.89,
///     Some(Utc::now()),
///     None,
/// )?;
///
/// assert_eq!(path.segments.len(), 2);
/// assert_eq!(path.overall_probability, 0.89);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainPath {
    /// Ordered sequence of netelements in the path
    /// Order represents the direction of travel from first to last GNSS position
    pub segments: Vec<AssociatedNetElement>,

    /// Overall probability score for this path (0.0 to 1.0)
    /// Calculated as length-weighted average of segment probabilities,
    /// averaged between forward and backward path calculations
    pub overall_probability: f64,

    /// Timestamp when this path was calculated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calculated_at: Option<DateTime<Utc>>,

    /// Algorithm configuration metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PathMetadata>,
}

impl TrainPath {
    /// Create a new train path with validation
    pub fn new(
        segments: Vec<AssociatedNetElement>,
        overall_probability: f64,
        calculated_at: Option<DateTime<Utc>>,
        metadata: Option<PathMetadata>,
    ) -> Result<Self, ProjectionError> {
        let path = Self {
            segments,
            overall_probability,
            calculated_at,
            metadata,
        };

        path.validate()?;
        Ok(path)
    }

    /// Validate train path
    fn validate(&self) -> Result<(), ProjectionError> {
        // Must have at least one segment
        if self.segments.is_empty() {
            return Err(ProjectionError::PathCalculationFailed {
                reason: "TrainPath must have at least one segment".to_string(),
            });
        }

        // Overall probability must be in [0, 1]
        if !(0.0..=1.0).contains(&self.overall_probability) {
            return Err(ProjectionError::InvalidGeometry(format!(
                "overall_probability must be in [0, 1], got {}",
                self.overall_probability
            )));
        }

        // Validate segment continuity (GNSS indices should be continuous or overlapping)
        for i in 0..self.segments.len() - 1 {
            let current = &self.segments[i];
            let next = &self.segments[i + 1];

            // Next segment should start at or after current segment's last position
            if next.gnss_start_index < current.gnss_start_index {
                return Err(ProjectionError::PathCalculationFailed {
                    reason: format!(
                        "Segment GNSS indices not continuous: segment {} ends at {}, segment {} starts at {}",
                        i, current.gnss_end_index, i + 1, next.gnss_start_index
                    ),
                });
            }
        }

        Ok(())
    }

    /// Calculate total path length (sum of fractional lengths)
    pub fn total_fractional_length(&self) -> f64 {
        self.segments
            .iter()
            .map(|s| s.fractional_length())
            .sum()
    }

    /// Get netelement IDs in traversal order
    pub fn netelement_ids(&self) -> Vec<&str> {
        self.segments
            .iter()
            .map(|s| s.netelement_id.as_str())
            .collect()
    }

    /// Total number of GNSS positions in path
    pub fn total_gnss_positions(&self) -> usize {
        if self.segments.is_empty() {
            return 0;
        }

        let first = &self.segments[0];
        let last = &self.segments[self.segments.len() - 1];

        last.gnss_end_index - first.gnss_start_index + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_train_path_valid() {
        let segments = vec![
            AssociatedNetElement::new("NE_A".to_string(), 0.87, 0.0, 1.0, 0, 10)
                .unwrap(),
            AssociatedNetElement::new("NE_B".to_string(), 0.92, 0.0, 1.0, 11, 18)
                .unwrap(),
        ];

        let path = TrainPath::new(segments, 0.89, Some(Utc::now()), None);

        assert!(path.is_ok());
        let p = path.unwrap();
        assert_eq!(p.segments.len(), 2);
        assert_eq!(p.total_gnss_positions(), 19);
    }

    #[test]
    fn test_train_path_empty_segments() {
        let path = TrainPath::new(vec![], 0.89, Some(Utc::now()), None);

        assert!(path.is_err());
    }

    #[test]
    fn test_train_path_invalid_probability() {
        let segments = vec![AssociatedNetElement::new(
            "NE_A".to_string(),
            0.87,
            0.0,
            1.0,
            0,
            10,
        )
        .unwrap()];

        let path = TrainPath::new(segments, 1.5, Some(Utc::now()), None); // Invalid: > 1.0

        assert!(path.is_err());
    }
}
