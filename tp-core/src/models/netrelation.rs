//! Network topology connection model

use crate::errors::ProjectionError;
use serde::{Deserialize, Serialize};

/// Represents a navigability connection between two track segments
///
/// A NetRelation defines whether trains can travel from one netelement to another.
/// Navigability may be unidirectional (e.g., one-way track) or bidirectional.
///
/// # Examples
///
/// ```
/// use tp_lib_core::NetRelation;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Bidirectional connection: trains can go from A to B and from B to A
/// let relation = NetRelation::new(
///     "NR001".to_string(),
///     "NE_A".to_string(),
///     "NE_B".to_string(),
///     1,  // position_on_a: end of A
///     0,  // position_on_b: start of B
///     true,   // A → B allowed
///     true,   // B → A allowed
/// )?;
///
/// assert!(relation.is_bidirectional());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetRelation {
    /// Unique identifier for this netrelation
    pub id: String,

    /// ID of the source netelement (starting track segment)
    pub from_netelement_id: String,

    /// ID of the target netelement (destination track segment)
    pub to_netelement_id: String,

    /// Position on netelementA where the connection applies (0 = start, 1 = end)
    pub position_on_a: u8,

    /// Position on netelementB where the connection applies (0 = start, 1 = end)
    pub position_on_b: u8,

    /// Whether trains can navigate forward (from → to)
    pub navigable_forward: bool,

    /// Whether trains can navigate backward (to → from)
    pub navigable_backward: bool,
}

impl NetRelation {
    /// Create a new netrelation with validation
    pub fn new(
        id: String,
        from_netelement_id: String,
        to_netelement_id: String,
        position_on_a: u8,
        position_on_b: u8,
        navigable_forward: bool,
        navigable_backward: bool,
    ) -> Result<Self, ProjectionError> {
        let relation = Self {
            id,
            from_netelement_id,
            to_netelement_id,
            position_on_a,
            position_on_b,
            navigable_forward,
            navigable_backward,
        };

        relation.validate()?;
        Ok(relation)
    }

    /// Validate netrelation fields
    pub fn validate(&self) -> Result<(), ProjectionError> {
        // ID must be non-empty
        if self.id.is_empty() {
            return Err(ProjectionError::InvalidNetRelation(
                "NetRelation ID must not be empty".to_string(),
            ));
        }

        // Netelement IDs must be non-empty
        if self.from_netelement_id.is_empty() {
            return Err(ProjectionError::InvalidNetRelation(
                "from_netelement_id must not be empty".to_string(),
            ));
        }

        if self.to_netelement_id.is_empty() {
            return Err(ProjectionError::InvalidNetRelation(
                "to_netelement_id must not be empty".to_string(),
            ));
        }

        // Position values must be 0 or 1
        if self.position_on_a > 1 {
            return Err(ProjectionError::InvalidNetRelation(format!(
                "position_on_a must be 0 or 1, got {}",
                self.position_on_a
            )));
        }

        if self.position_on_b > 1 {
            return Err(ProjectionError::InvalidNetRelation(format!(
                "position_on_b must be 0 or 1, got {}",
                self.position_on_b
            )));
        }

        // Cannot connect to itself
        if self.from_netelement_id == self.to_netelement_id {
            return Err(ProjectionError::InvalidNetRelation(format!(
                "NetRelation cannot connect netelement to itself: {}",
                self.from_netelement_id
            )));
        }

        Ok(())
    }

    /// Check if navigation is allowed in forward direction (from → to)
    pub fn is_navigable_forward(&self) -> bool {
        self.navigable_forward
    }

    /// Check if navigation is allowed in backward direction (to → from)
    pub fn is_navigable_backward(&self) -> bool {
        self.navigable_backward
    }

    /// Check if bidirectional (both directions navigable)
    pub fn is_bidirectional(&self) -> bool {
        self.navigable_forward && self.navigable_backward
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_bidirectional() {
        let relation = NetRelation::new(
            "NR001".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            0,
            true,
            true,
        );

        assert!(relation.is_ok());
        let rel = relation.unwrap();
        assert!(rel.is_bidirectional());
        assert!(rel.is_navigable_forward());
        assert!(rel.is_navigable_backward());
    }

    #[test]
    fn test_valid_unidirectional() {
        let relation = NetRelation::new(
            "NR002".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            0,
            true,
            false,
        );

        assert!(relation.is_ok());
        let rel = relation.unwrap();
        assert!(!rel.is_bidirectional());
        assert!(rel.is_navigable_forward());
        assert!(!rel.is_navigable_backward());
    }

    #[test]
    fn test_invalid_position_on_a() {
        let relation = NetRelation::new(
            "NR003".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            2, // Invalid: must be 0 or 1
            0,
            true,
            false,
        );

        assert!(relation.is_err());
    }

    #[test]
    fn test_invalid_position_on_b() {
        let relation = NetRelation::new(
            "NR004".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            5, // Invalid: must be 0 or 1
            true,
            false,
        );

        assert!(relation.is_err());
    }

    #[test]
    fn test_self_reference() {
        let relation = NetRelation::new(
            "NR005".to_string(),
            "NE_A".to_string(),
            "NE_A".to_string(), // Invalid: same as from_netelement_id
            1,
            0,
            true,
            false,
        );

        assert!(relation.is_err());
    }

    #[test]
    fn test_empty_id() {
        let relation = NetRelation::new(
            "".to_string(), // Invalid
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            0,
            true,
            false,
        );

        assert!(relation.is_err());
    }

    #[test]
    fn test_empty_from_id() {
        let relation = NetRelation::new(
            "NR006".to_string(),
            "".to_string(), // Invalid
            "NE_B".to_string(),
            1,
            0,
            true,
            false,
        );

        assert!(relation.is_err());
    }

    #[test]
    fn test_empty_to_id() {
        let relation = NetRelation::new(
            "NR007".to_string(),
            "NE_A".to_string(),
            "".to_string(), // Invalid
            1,
            0,
            true,
            false,
        );

        assert!(relation.is_err());
    }
}
