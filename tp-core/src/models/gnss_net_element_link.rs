//! Link between a GNSS position and a candidate netelement

use crate::errors::ProjectionError;
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
