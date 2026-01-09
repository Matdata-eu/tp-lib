//! Train path representation models

use crate::errors::ProjectionError;
use chrono::{DateTime, Utc};
use geo::Point;
use serde::{Deserialize, Serialize};

/// Link between a GNSS position and a candidate netelement
///
/// Created during path calculation to evaluate which netelements are potential
/// matches for each GNSS position. Multiple links exist per GNSS position.
/// This is an **intermediate calculation model**, not part of final output.
///
/// # Examples
///
/// ```
/// use tp_lib_core::GnssNetElementLink;
/// use geo::Point;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let link = GnssNetElementLink::new(
///     5,
///     "NE_A".to_string(),
///     Point::new(4.3517, 50.8503),
///     3.2,
///     0.45,
///     Some(5.3),
///     0.89,
/// )?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GnssNetElementLink {
    /// Index of the GNSS position in the input data
    pub gnss_index: usize,

    /// ID of the candidate netelement
    pub netelement_id: String,

    /// Projected point on the netelement (closest point to GNSS position)
    pub projected_point: Point<f64>,

    /// Distance from GNSS position to projected point in meters
    pub distance_meters: f64,

    /// Intrinsic coordinate on the netelement (0.0 to 1.0)
    /// 0.0 = start of segment, 1.0 = end of segment
    pub intrinsic_coordinate: f64,

    /// Angular difference between GNSS heading and netelement direction (degrees)
    /// None if GNSS position has no heading information
    pub heading_difference: Option<f64>,

    /// Probability score for this link (0.0 to 1.0)
    /// Calculated from distance and heading probability
    pub probability: f64,
}

impl GnssNetElementLink {
    /// Create a new GNSS-netelement link with validation
    pub fn new(
        gnss_index: usize,
        netelement_id: String,
        projected_point: Point<f64>,
        distance_meters: f64,
        intrinsic_coordinate: f64,
        heading_difference: Option<f64>,
        probability: f64,
    ) -> Result<Self, ProjectionError> {
        let link = Self {
            gnss_index,
            netelement_id,
            projected_point,
            distance_meters,
            intrinsic_coordinate,
            heading_difference,
            probability,
        };

        link.validate()?;
        Ok(link)
    }

    /// Validate link fields
    fn validate(&self) -> Result<(), ProjectionError> {
        // Netelement ID must be non-empty
        if self.netelement_id.is_empty() {
            return Err(ProjectionError::InvalidGeometry(
                "GnssNetElementLink netelement_id must not be empty".to_string(),
            ));
        }

        // Distance must be non-negative
        if self.distance_meters < 0.0 {
            return Err(ProjectionError::InvalidGeometry(format!(
                "distance_meters must be non-negative, got {}",
                self.distance_meters
            )));
        }

        // Intrinsic coordinate must be in [0, 1]
        if !(0.0..=1.0).contains(&self.intrinsic_coordinate) {
            return Err(ProjectionError::InvalidGeometry(format!(
                "intrinsic_coordinate must be in [0, 1], got {}",
                self.intrinsic_coordinate
            )));
        }

        // Heading difference must be in [0, 180] if present
        if let Some(heading_diff) = self.heading_difference {
            if !(0.0..=180.0).contains(&heading_diff) {
                return Err(ProjectionError::InvalidGeometry(format!(
                    "heading_difference must be in [0, 180], got {}",
                    heading_diff
                )));
            }
        }

        // Probability must be in [0, 1]
        if !(0.0..=1.0).contains(&self.probability) {
            return Err(ProjectionError::InvalidGeometry(format!(
                "Probability must be in [0, 1], got {}",
                self.probability
            )));
        }

        Ok(())
    }

    /// Check if this is a high-probability candidate (>= threshold)
    pub fn is_high_probability(&self, threshold: f64) -> bool {
        self.probability >= threshold
    }

    /// Check if distance is within acceptable range
    pub fn is_within_distance(&self, max_distance_meters: f64) -> bool {
        self.distance_meters <= max_distance_meters
    }
}

/// Represents a netelement within a calculated train path
///
/// Contains the netelement ID, probability score, and projection details for
/// GNSS positions associated with this segment in the path.
///
/// # Examples
///
/// ```
/// use tp_lib_core::AssociatedNetElement;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let segment = AssociatedNetElement::new(
///     "NE_A".to_string(),
///     0.87,
///     0.25,
///     0.78,
///     5,
///     12,
/// )?;
///
/// // This segment spans from 25% to 78% along netelement NE_A
/// // and is associated with GNSS positions 5-12 in the input data
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssociatedNetElement {
    /// ID of the netelement (track segment)
    pub netelement_id: String,

    /// Aggregate probability score for this segment in the path (0.0 to 1.0)
    /// Calculated from distance/heading probability and coverage correction
    pub probability: f64,

    /// Intrinsic coordinate where the path enters this segment (0.0 to 1.0)
    /// 0.0 = start of segment, 1.0 = end of segment
    pub start_intrinsic: f64,

    /// Intrinsic coordinate where the path exits this segment (0.0 to 1.0)
    pub end_intrinsic: f64,

    /// Index of the first GNSS position associated with this segment
    pub gnss_start_index: usize,

    /// Index of the last GNSS position associated with this segment
    pub gnss_end_index: usize,
}

impl AssociatedNetElement {
    /// Create a new associated netelement with validation
    pub fn new(
        netelement_id: String,
        probability: f64,
        start_intrinsic: f64,
        end_intrinsic: f64,
        gnss_start_index: usize,
        gnss_end_index: usize,
    ) -> Result<Self, ProjectionError> {
        let element = Self {
            netelement_id,
            probability,
            start_intrinsic,
            end_intrinsic,
            gnss_start_index,
            gnss_end_index,
        };

        element.validate()?;
        Ok(element)
    }

    /// Validate associated netelement fields
    fn validate(&self) -> Result<(), ProjectionError> {
        // Netelement ID must be non-empty
        if self.netelement_id.is_empty() {
            return Err(ProjectionError::InvalidGeometry(
                "AssociatedNetElement netelement_id must not be empty".to_string(),
            ));
        }

        // Probability must be in [0, 1]
        if !(0.0..=1.0).contains(&self.probability) {
            return Err(ProjectionError::InvalidGeometry(format!(
                "Probability must be in [0, 1], got {}",
                self.probability
            )));
        }

        // Intrinsic coordinates must be in [0, 1]
        if !(0.0..=1.0).contains(&self.start_intrinsic) {
            return Err(ProjectionError::InvalidGeometry(format!(
                "start_intrinsic must be in [0, 1], got {}",
                self.start_intrinsic
            )));
        }

        if !(0.0..=1.0).contains(&self.end_intrinsic) {
            return Err(ProjectionError::InvalidGeometry(format!(
                "end_intrinsic must be in [0, 1], got {}",
                self.end_intrinsic
            )));
        }

        // Start index must be <= end index
        if self.gnss_start_index > self.gnss_end_index {
            return Err(ProjectionError::InvalidGeometry(format!(
                "gnss_start_index ({}) must be <= gnss_end_index ({})",
                self.gnss_start_index, self.gnss_end_index
            )));
        }

        Ok(())
    }

    /// Calculate length of path segment as fraction of total netelement
    pub fn fractional_length(&self) -> f64 {
        (self.end_intrinsic - self.start_intrinsic).abs()
    }

    /// Calculate the fractional coverage of this segment (0.0 to 1.0)
    /// Same as fractional_length, representing what portion of the netelement is covered
    pub fn fractional_coverage(&self) -> f64 {
        self.fractional_length()
    }
}

/// Algorithm configuration and diagnostic metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathMetadata {
    /// Distance scale parameter used for probability calculation
    pub distance_scale: f64,

    /// Heading scale parameter used for probability calculation
    pub heading_scale: f64,

    /// Cutoff distance for candidate selection (meters)
    pub cutoff_distance: f64,

    /// Heading difference cutoff (degrees)
    pub heading_cutoff: f64,

    /// Probability threshold for path segment inclusion
    pub probability_threshold: f64,

    /// Resampling distance applied (meters), None if disabled
    pub resampling_distance: Option<f64>,

    /// Whether fallback mode was used
    pub fallback_mode: bool,

    /// Number of candidate paths evaluated
    pub candidate_paths_evaluated: usize,

    /// Whether path existed in both directions (bidirectional validation)
    pub bidirectional_path: bool,
}

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
    use chrono::Utc;

    #[test]
    fn test_gnss_link_valid() {
        let link = GnssNetElementLink::new(
            5,
            "NE_A".to_string(),
            Point::new(4.3517, 50.8503),
            3.2,
            0.45,
            Some(5.3),
            0.89,
        );

        assert!(link.is_ok());
    }

    #[test]
    fn test_gnss_link_invalid_probability() {
        let link = GnssNetElementLink::new(
            5,
            "NE_A".to_string(),
            Point::new(4.3517, 50.8503),
            3.2,
            0.45,
            Some(5.3),
            1.5, // Invalid: > 1.0
        );

        assert!(link.is_err());
    }

    #[test]
    fn test_associated_netelement_valid() {
        let segment =
            AssociatedNetElement::new("NE_A".to_string(), 0.87, 0.25, 0.78, 5, 12);

        assert!(segment.is_ok());
        let seg = segment.unwrap();
        assert_eq!(seg.fractional_length(), 0.53);
    }

    #[test]
    fn test_associated_netelement_invalid_indices() {
        let segment = AssociatedNetElement::new(
            "NE_A".to_string(),
            0.87,
            0.25,
            0.78,
            15, // Invalid: start > end
            12,
        );

        assert!(segment.is_err());
    }

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
