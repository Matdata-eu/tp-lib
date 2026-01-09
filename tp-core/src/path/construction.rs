//! Path construction module
//!
//! Implements bidirectional path construction algorithms (forward and backward)
//! with navigability validation and path reversal logic.

use crate::errors::ProjectionError;
use crate::models::{AssociatedNetElement, NetRelation, TrainPath};
use std::collections::HashMap;

/// Represents a path under construction with associated metadata
#[derive(Debug, Clone)]
pub struct PathConstruction {
    /// Ordered sequence of netrelements in path
    pub segments: Vec<AssociatedNetElement>,
    /// Probability score for the path (0-1)
    pub probability: f64,
    /// Total length of path (meters)
    pub length_meters: f64,
    /// Whether path reaches from start to end positions
    pub is_complete: bool,
}

impl PathConstruction {
    /// Create a new path construction starting with a netelement
    pub fn new(
        initial_segment: AssociatedNetElement,
        probability: f64,
        length_meters: f64,
    ) -> Self {
        Self {
            segments: vec![initial_segment],
            probability,
            length_meters,
            is_complete: false,
        }
    }

    /// Add a segment to the path
    pub fn add_segment(&mut self, segment: AssociatedNetElement, _length: f64) {
        self.segments.push(segment);
    }

    /// Reverse the path (for backward path conversion)
    pub fn reverse(&mut self) {
        self.segments.reverse();

        // Swap intrinsic coordinates on each segment
        for segment in &mut self.segments {
            segment.start_intrinsic = 1.0 - segment.start_intrinsic;
            segment.end_intrinsic = 1.0 - segment.end_intrinsic;
            // Swap start and end
            std::mem::swap(&mut segment.start_intrinsic, &mut segment.end_intrinsic);
        }
    }

    /// Convert to final TrainPath structure
    pub fn to_train_path(self) -> Result<TrainPath, ProjectionError> {
        use chrono::Utc;
        TrainPath::new(self.segments, self.probability, Some(Utc::now()), None)
    }
}

/// Construct path in forward direction starting from initial netelement
///
/// Uses graph traversal with navigability constraints (via netrelations).
/// Stops when reaching an end point or when only low-probability continuations exist.
///
/// # Arguments
///
/// * `start_netelement_id` - ID of netelement at first GNSS position
/// * `netelement_map` - Map of netelement ID to netelement with probabilities
/// * `netrelations` - Available navigable connections
/// * `probability_threshold` - Minimum probability to continue (default 0.25)
///
/// # Returns
///
/// Forward path from start, marked incomplete if path terminates early
pub fn construct_forward_path(
    start_netelement_id: &str,
    netelement_map: &HashMap<String, (f64, AssociatedNetElement)>,
    netrelations: &[NetRelation],
    probability_threshold: f64,
) -> Result<PathConstruction, ProjectionError> {
    // Start with the initial netelement
    let (prob, segment) = netelement_map.get(start_netelement_id).ok_or_else(|| {
        ProjectionError::PathCalculationFailed {
            reason: format!("Netelement not found: {}", start_netelement_id),
        }
    })?;

    let mut path = PathConstruction::new(segment.clone(), *prob, 100.0);
    let mut current_id = start_netelement_id.to_string();
    let mut visited = std::collections::HashSet::new();
    visited.insert(current_id.clone());

    // Build index of netrelations for quick lookup
    let mut from_relations: HashMap<String, Vec<&NetRelation>> = HashMap::new();
    for rel in netrelations {
        from_relations
            .entry(rel.from_netelement_id.clone())
            .or_default()
            .push(rel);
    }

    // Traverse forward following netrelations
    loop {
        // Find all outgoing connections from current netelement
        let connections = from_relations.get(&current_id);

        if connections.is_none() || connections.unwrap().is_empty() {
            // No outgoing connections - path complete
            path.is_complete = true;
            break;
        }

        // Filter for navigable connections with valid targets
        let mut candidates: Vec<(&str, f64)> = Vec::new();

        for rel in connections.unwrap() {
            // Check if relation allows forward navigation
            if !rel.navigable_forward {
                continue;
            }

            // Get target netelement
            let target_id = &rel.to_netelement_id;

            // Skip if already visited (avoid cycles)
            if visited.contains(target_id) {
                continue;
            }

            // Get probability for target netelement
            if let Some((target_prob, _)) = netelement_map.get(target_id) {
                candidates.push((target_id.as_str(), *target_prob));
            }
        }

        if candidates.is_empty() {
            // No navigable connections with valid targets
            path.is_complete = true;
            break;
        }

        // Apply probability threshold, unless it's the only option
        let viable_candidates: Vec<_> = if candidates.len() == 1 {
            // Only one option - take it regardless of probability
            candidates.clone()
        } else {
            // Multiple options - filter by threshold
            candidates
                .iter()
                .filter(|(_, prob)| *prob >= probability_threshold)
                .copied()
                .collect()
        };

        if viable_candidates.is_empty() {
            // No candidates meet threshold - path terminates
            path.is_complete = false;
            break;
        }

        // Select highest probability candidate
        let (next_id, _next_prob) = viable_candidates
            .iter()
            .max_by(|(_, p1), (_, p2)| p1.partial_cmp(p2).unwrap())
            .unwrap();

        // Add next segment to path
        let (_, next_segment) = netelement_map.get(*next_id).unwrap();
        path.add_segment(next_segment.clone(), 100.0);
        path.length_meters += 100.0; // Placeholder length

        // Update current position
        current_id = next_id.to_string();
        visited.insert(current_id.clone());

        // Safety limit to prevent infinite loops
        if visited.len() > 1000 {
            return Err(ProjectionError::PathCalculationFailed {
                reason: "Path construction exceeded maximum segment count (1000)".to_string(),
            });
        }
    }

    Ok(path)
}

/// Construct path in backward direction starting from final netelement
///
/// Similar to forward path construction but starting from the end.
/// Result is returned in reverse order (suitable for reversal to forward direction).
///
/// # Arguments
///
/// * `end_netelement_id` - ID of netelement at last GNSS position
/// * `netelement_map` - Map of netelement ID to netelement with probabilities
/// * `netrelations` - Available navigable connections
/// * `probability_threshold` - Minimum probability to continue
///
/// # Returns
///
/// Backward path from end (in reverse order), marked incomplete if early termination
pub fn construct_backward_path(
    end_netelement_id: &str,
    netelement_map: &HashMap<String, (f64, AssociatedNetElement)>,
    netrelations: &[NetRelation],
    probability_threshold: f64,
) -> Result<PathConstruction, ProjectionError> {
    // Start with the end netelement
    let (prob, segment) = netelement_map.get(end_netelement_id).ok_or_else(|| {
        ProjectionError::PathCalculationFailed {
            reason: format!("Netelement not found: {}", end_netelement_id),
        }
    })?;

    let mut path = PathConstruction::new(segment.clone(), *prob, 100.0);
    let mut current_id = end_netelement_id.to_string();
    let mut visited = std::collections::HashSet::new();
    visited.insert(current_id.clone());

    // Build index of netrelations for backward traversal (to -> from)
    let mut to_relations: HashMap<String, Vec<&NetRelation>> = HashMap::new();
    for rel in netrelations {
        to_relations
            .entry(rel.to_netelement_id.clone())
            .or_default()
            .push(rel);
    }

    // Traverse backward following netrelations in reverse
    loop {
        // Find all incoming connections to current netelement
        let connections = to_relations.get(&current_id);

        if connections.is_none() || connections.unwrap().is_empty() {
            // No incoming connections - path complete
            path.is_complete = true;
            break;
        }

        // Filter for navigable connections with valid sources
        let mut candidates: Vec<(&str, f64)> = Vec::new();

        for rel in connections.unwrap() {
            // Check if relation allows backward navigation
            if !rel.navigable_backward {
                continue;
            }

            // Get source netelement
            let source_id = &rel.from_netelement_id;

            // Skip if already visited (avoid cycles)
            if visited.contains(source_id) {
                continue;
            }

            // Get probability for source netelement
            if let Some((source_prob, _)) = netelement_map.get(source_id) {
                candidates.push((source_id.as_str(), *source_prob));
            }
        }

        if candidates.is_empty() {
            // No navigable connections with valid sources
            path.is_complete = true;
            break;
        }

        // Apply probability threshold, unless it's the only option
        let viable_candidates: Vec<_> = if candidates.len() == 1 {
            // Only one option - take it regardless of probability
            candidates.clone()
        } else {
            // Multiple options - filter by threshold
            candidates
                .iter()
                .filter(|(_, prob)| *prob >= probability_threshold)
                .copied()
                .collect()
        };

        if viable_candidates.is_empty() {
            // No candidates meet threshold - path terminates
            path.is_complete = false;
            break;
        }

        // Select highest probability candidate
        let (prev_id, _prev_prob) = viable_candidates
            .iter()
            .max_by(|(_, p1), (_, p2)| p1.partial_cmp(p2).unwrap())
            .unwrap();

        // Add previous segment to path (it will be reversed later)
        let (_, prev_segment) = netelement_map.get(*prev_id).unwrap();
        path.add_segment(prev_segment.clone(), 100.0);
        path.length_meters += 100.0; // Placeholder length

        // Update current position
        current_id = prev_id.to_string();
        visited.insert(current_id.clone());

        // Safety limit to prevent infinite loops
        if visited.len() > 1000 {
            return Err(ProjectionError::PathCalculationFailed {
                reason: "Path construction exceeded maximum segment count (1000)".to_string(),
            });
        }
    }

    Ok(path)
}

/// Compare forward and backward paths to validate bidirectional agreement
///
/// If both forward and backward paths exist and have good probability,
/// verify they represent the same logical path (possibly reversed).
///
/// # Arguments
///
/// * `forward_path` - Path constructed forward from start
/// * `backward_path` - Path constructed backward from end
///
/// # Returns
///
/// true if paths agree bidirectionally (same segments in same order)
pub fn validate_bidirectional_agreement(
    forward_path: &PathConstruction,
    backward_path: &PathConstruction,
) -> bool {
    // Forward should have same segments as reversed backward
    if forward_path.segments.len() != backward_path.segments.len() {
        return false;
    }

    // Check each segment matches (allowing for some position differences)
    for (fwd, bwd) in forward_path
        .segments
        .iter()
        .zip(backward_path.segments.iter().rev())
    {
        if fwd.netelement_id != bwd.netelement_id {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_construction_creation() -> Result<(), ProjectionError> {
        let segment = AssociatedNetElement::new("elem1".to_string(), 0.9, 0.0, 1.0, 0, 10)?;

        let path = PathConstruction::new(segment, 0.9, 100.0);

        assert_eq!(path.segments.len(), 1);
        assert_eq!(path.probability, 0.9);
        assert_eq!(path.length_meters, 100.0);
        assert!(!path.is_complete);
        Ok(())
    }

    #[test]
    fn test_path_add_segment() -> Result<(), ProjectionError> {
        let segment1 = AssociatedNetElement::new("elem1".to_string(), 0.9, 0.0, 1.0, 0, 5)?;
        let segment2 = AssociatedNetElement::new("elem2".to_string(), 0.85, 0.0, 1.0, 6, 10)?;

        let mut path = PathConstruction::new(segment1, 0.9, 100.0);
        path.add_segment(segment2, 150.0);

        assert_eq!(path.segments.len(), 2);
        Ok(())
    }

    #[test]
    fn test_path_reversal() -> Result<(), ProjectionError> {
        let segment = AssociatedNetElement::new("elem1".to_string(), 0.8, 0.2, 0.9, 0, 5)?;

        let mut path = PathConstruction::new(segment, 0.8, 100.0);
        path.reverse();

        // Intrinsic coordinates should be swapped and inverted
        assert!((path.segments[0].start_intrinsic - 0.1).abs() < 0.001);
        assert!((path.segments[0].end_intrinsic - 0.8).abs() < 0.001);
        Ok(())
    }

    #[test]
    fn test_bidirectional_agreement_same_path() -> Result<(), ProjectionError> {
        let seg1 = AssociatedNetElement::new("elem1".to_string(), 0.9, 0.0, 1.0, 0, 5)?;
        let seg2 = AssociatedNetElement::new("elem2".to_string(), 0.85, 0.0, 1.0, 6, 10)?;

        let mut fwd = PathConstruction::new(seg1.clone(), 0.9, 100.0);
        fwd.add_segment(seg2.clone(), 100.0);

        let mut bwd = PathConstruction::new(seg2, 0.85, 100.0);
        bwd.add_segment(seg1, 100.0);

        assert!(validate_bidirectional_agreement(&fwd, &bwd));
        Ok(())
    }

    #[test]
    fn test_bidirectional_agreement_different_paths() -> Result<(), ProjectionError> {
        let seg1 = AssociatedNetElement::new("elem1".to_string(), 0.9, 0.0, 1.0, 0, 5)?;
        let seg2 = AssociatedNetElement::new("elem2".to_string(), 0.85, 0.0, 1.0, 6, 10)?;
        let seg3 = AssociatedNetElement::new("elem3".to_string(), 0.75, 0.0, 1.0, 11, 15)?;

        let mut fwd = PathConstruction::new(seg1.clone(), 0.9, 100.0);
        fwd.add_segment(seg2.clone(), 100.0);

        let mut bwd = PathConstruction::new(seg3, 0.85, 100.0);
        bwd.add_segment(seg1, 100.0);

        assert!(!validate_bidirectional_agreement(&fwd, &bwd));
        Ok(())
    }
}
