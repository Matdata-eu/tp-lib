//! Path construction module
//!
//! Implements bidirectional path construction algorithms (forward and backward)
//! with navigability validation and path reversal logic.

use crate::errors::ProjectionError;
use crate::models::{AssociatedNetElement, NetRelation, TrainPath};
use std::collections::HashMap;

/// Maximum number of segments allowed in a path (safety limit)
const MAX_PATH_SEGMENTS: usize = 1000;

/// Placeholder length value for segments (meters)
const PLACEHOLDER_SEGMENT_LENGTH: f64 = 100.0;

/// Maximum number of bridge hops allowed when traversing through netelements
/// not present in the netelement map (no direct GNSS evidence).
const MAX_BRIDGE_HOPS: usize = 10;

/// Build a map from each netelement ID to its directly reachable neighbours,
/// following ALL directed edges in the railway graph:
/// - Forward edges: `from=A, navigable_forward=true → A can reach B`
/// - Backward edges: `to=B, navigable_backward=true → B can reach A`
fn build_outgoing_index(netrelations: &[NetRelation]) -> HashMap<String, Vec<String>> {
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for rel in netrelations {
        if rel.navigable_forward {
            outgoing
                .entry(rel.from_netelement_id.clone())
                .or_default()
                .push(rel.to_netelement_id.clone());
        }
        if rel.navigable_backward {
            outgoing
                .entry(rel.to_netelement_id.clone())
                .or_default()
                .push(rel.from_netelement_id.clone());
        }
    }
    outgoing
}

/// Build a map from each netelement ID to its directly reachable predecessors
/// (netelements that could have preceded it in the train's journey):
/// - Forward edges: `from=A, navigable_forward=true → A preceded B`
/// - Backward edges: `to=B, navigable_backward=true → B preceded A`
fn build_incoming_index(netrelations: &[NetRelation]) -> HashMap<String, Vec<String>> {
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    for rel in netrelations {
        if rel.navigable_forward {
            incoming
                .entry(rel.to_netelement_id.clone())
                .or_default()
                .push(rel.from_netelement_id.clone());
        }
        if rel.navigable_backward {
            incoming
                .entry(rel.from_netelement_id.clone())
                .or_default()
                .push(rel.to_netelement_id.clone());
        }
    }
    incoming
}

/// Find a path from `start_id` (not in `netelement_map`) to the nearest reachable
/// netelement that IS in `netelement_map`, following the topology index up to
/// `max_hops` hops.
///
/// Returns `(chain, probability)` where `chain` is the sequence of netelement IDs
/// from `start_id` up to and including the map netelement, and `probability` is
/// the map netelement's probability.
///
/// Returns `None` if no map netelement is reachable within the hop limit.
fn find_bridge_path(
    start_id: &str,
    already_visited: &std::collections::HashSet<String>,
    netelement_map: &HashMap<String, (f64, AssociatedNetElement)>,
    nav_index: &HashMap<String, Vec<String>>,
    max_hops: usize,
) -> Option<(Vec<String>, f64)> {
    // Level-order BFS: explore hop by hop, collecting ALL map members found at
    // the same hop distance, then return the one with the highest probability.
    // This ensures that at netelement connections where multiple branches are reachable via a
    // bridge netelement, the most-probable branch is always selected.
    let mut frontier: Vec<(String, Vec<String>)> =
        vec![(start_id.to_string(), vec![start_id.to_string()])];
    let mut bfs_visited = std::collections::HashSet::new();
    bfs_visited.insert(start_id.to_string());

    for _ in 0..max_hops {
        let mut found: Vec<(Vec<String>, f64)> = Vec::new();
        let mut next_frontier: Vec<(String, Vec<String>)> = Vec::new();

        for (current, chain) in &frontier {
            let neighbors = nav_index
                .get(current.as_str())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            for next in neighbors {
                if already_visited.contains(next) || bfs_visited.contains(next.as_str()) {
                    continue;
                }
                let mut new_chain = chain.clone();
                new_chain.push(next.clone());
                if let Some((prob, _)) = netelement_map.get(next.as_str()) {
                    // Map member found at this depth — collect it.
                    found.push((new_chain, *prob));
                } else if bfs_visited.insert(next.clone()) {
                    next_frontier.push((next.clone(), new_chain));
                }
            }
        }

        // Return the highest-probability map member found at this hop distance.
        if !found.is_empty() {
            return found
                .into_iter()
                .max_by(|(_, p1), (_, p2)| p1.partial_cmp(p2).unwrap());
        }

        if next_frontier.is_empty() {
            break;
        }
        frontier = next_frontier;
    }

    None
}

/// Create a bridge `AssociatedNetElement` for a topology-required segment that has
/// no direct GNSS evidence.  Uses probability 1.0 (topologically certain) to avoid
/// artificially reducing path probability.
fn make_bridge_segment(netelement_id: String) -> Result<AssociatedNetElement, ProjectionError> {
    AssociatedNetElement::new(netelement_id, 1.0, 0.0, 1.0, 0, 0)
}

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
/// Uses graph traversal following only forward-navigable edges (navigable_forward=true).
/// Stops when no more forward edges exist or when only low-probability continuations exist.
///
/// # Arguments
///
/// * `start_netelement_id` - ID of netelement at first GNSS position
/// * `netelement_map` - Map of netelement ID to netelement with probabilities
/// * `netrelations` - Available navigable connections
/// * `probability_threshold` - Minimum probability to continue (default 0.02)
///
/// # Returns
///
/// Forward path from start, marked complete if the topology ends
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

    // Build bidirectional outgoing index: for each netelement, all netelements
    // reachable via any directed edge (navigable_forward OR navigable_backward).
    let outgoing = build_outgoing_index(netrelations);

    // Traverse forward following the network topology
    loop {
        let neighbors = outgoing
            .get(&current_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Collect (target_id, probability, bridge_chain) for every reachable candidate.
        // A candidate may be a direct map member or reachable via a bridge BFS.
        let mut candidates: Vec<(String, f64, Vec<String>)> = Vec::new();

        for neighbor_id in neighbors {
            if visited.contains(neighbor_id.as_str()) {
                continue;
            }
            if let Some((target_prob, _)) = netelement_map.get(neighbor_id.as_str()) {
                candidates.push((neighbor_id.clone(), *target_prob, vec![]));
            } else {
                // Neighbour not in map: BFS to the nearest map netelement.
                if let Some((chain, final_prob)) = find_bridge_path(
                    neighbor_id,
                    &visited,
                    netelement_map,
                    &outgoing,
                    MAX_BRIDGE_HOPS,
                ) {
                    let target_id = chain.last().unwrap().clone();
                    let bridge_ids: Vec<String> = chain[..chain.len() - 1].to_vec();
                    candidates.push((target_id, final_prob, bridge_ids));
                }
            }
        }

        if candidates.is_empty() {
            // No navigable connections with reachable targets – path complete.
            path.is_complete = true;
            break;
        }

        // Apply probability threshold, unless it's the only option
        let viable_candidates: Vec<_> = if candidates.len() == 1 {
            candidates.clone()
        } else {
            candidates
                .iter()
                .filter(|(_, prob, _)| *prob >= probability_threshold)
                .cloned()
                .collect()
        };

        if viable_candidates.is_empty() {
            // No candidates meet threshold - path terminates
            path.is_complete = false;
            break;
        }

        // Select the highest-probability candidate
        let (next_id, _next_prob, bridge_ids) = viable_candidates
            .iter()
            .max_by(|(_, p1, _), (_, p2, _)| p1.partial_cmp(p2).unwrap())
            .unwrap();

        // Add bridge segments first (topology-required, no direct GNSS evidence)
        for bridge_id in bridge_ids {
            let bridge_seg = make_bridge_segment(bridge_id.clone())?;
            path.add_segment(bridge_seg, PLACEHOLDER_SEGMENT_LENGTH);
            path.length_meters += PLACEHOLDER_SEGMENT_LENGTH;
            visited.insert(bridge_id.clone());
        }

        // Add the map netelement
        let (_, next_segment) = netelement_map.get(next_id.as_str()).unwrap();
        path.add_segment(next_segment.clone(), PLACEHOLDER_SEGMENT_LENGTH);
        path.length_meters += PLACEHOLDER_SEGMENT_LENGTH;

        current_id = next_id.clone();
        visited.insert(current_id.clone());

        // Safety limit to prevent infinite loops
        if visited.len() > MAX_PATH_SEGMENTS {
            return Err(ProjectionError::PathCalculationFailed {
                reason: format!(
                    "Path construction exceeded maximum segment count ({})",
                    MAX_PATH_SEGMENTS
                ),
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

    // Build incoming navigation index: for each netelement, all netelements
    // that could have preceded it via any directed edge (forward OR backward).
    let incoming = build_incoming_index(netrelations);

    // Traverse backward following netrelations in reverse
    loop {
        let predecessors = incoming
            .get(&current_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Collect (source_id, probability, bridge_chain) for every reachable
        // predecessor, using bridge BFS when the immediate predecessor is not
        // in the map.
        let mut candidates: Vec<(String, f64, Vec<String>)> = Vec::new();

        for pred_id in predecessors {
            if visited.contains(pred_id.as_str()) {
                continue;
            }
            if let Some((source_prob, _)) = netelement_map.get(pred_id.as_str()) {
                candidates.push((pred_id.clone(), *source_prob, vec![]));
            } else {
                // Predecessor not in map: bridge through it.
                if let Some((chain, final_prob)) = find_bridge_path(
                    pred_id,
                    &visited,
                    netelement_map,
                    &incoming,
                    MAX_BRIDGE_HOPS,
                ) {
                    let source_id = chain.last().unwrap().clone();
                    let bridge_ids: Vec<String> = chain[..chain.len() - 1].to_vec();
                    candidates.push((source_id, final_prob, bridge_ids));
                }
            }
        }

        if candidates.is_empty() {
            // No navigable connections with valid sources
            path.is_complete = true;
            break;
        }

        // Apply probability threshold, unless it's the only option
        let viable_candidates: Vec<_> = if candidates.len() == 1 {
            candidates.clone()
        } else {
            candidates
                .iter()
                .filter(|(_, prob, _)| *prob >= probability_threshold)
                .cloned()
                .collect()
        };

        if viable_candidates.is_empty() {
            // No candidates meet threshold - path terminates
            path.is_complete = false;
            break;
        }

        // Select highest probability candidate
        let (prev_id, _prev_prob, bridge_ids) = viable_candidates
            .iter()
            .max_by(|(_, p1, _), (_, p2, _)| p1.partial_cmp(p2).unwrap())
            .unwrap();

        // Add bridge segments first (topology-required, no direct GNSS evidence)
        for bridge_id in bridge_ids {
            let bridge_seg = make_bridge_segment(bridge_id.clone())?;
            path.add_segment(bridge_seg, PLACEHOLDER_SEGMENT_LENGTH);
            path.length_meters += PLACEHOLDER_SEGMENT_LENGTH;
            visited.insert(bridge_id.clone());
        }

        // Add the map netelement (will be reversed later)
        let (_, prev_segment) = netelement_map.get(prev_id.as_str()).unwrap();
        path.add_segment(prev_segment.clone(), PLACEHOLDER_SEGMENT_LENGTH);
        path.length_meters += PLACEHOLDER_SEGMENT_LENGTH;

        current_id = prev_id.clone();
        visited.insert(current_id.clone());

        // Safety limit to prevent infinite loops
        if visited.len() > MAX_PATH_SEGMENTS {
            return Err(ProjectionError::PathCalculationFailed {
                reason: format!(
                    "Path construction exceeded maximum segment count ({})",
                    MAX_PATH_SEGMENTS
                ),
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
