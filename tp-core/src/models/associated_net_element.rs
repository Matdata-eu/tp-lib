//! Netelement within a calculated train path

use crate::errors::ProjectionError;
use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_associated_netelement_valid() {
        let segment = AssociatedNetElement::new("NE_A".to_string(), 0.87, 0.25, 0.78, 5, 12);

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
}
