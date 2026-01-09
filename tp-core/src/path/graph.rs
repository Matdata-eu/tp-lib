//! Network topology graph representation
//!
//! Builds a petgraph DiGraph from netelements and netrelations to enable
//! efficient path traversal and navigability checking.

use crate::errors::ProjectionError;
use crate::models::{NetRelation, Netelement};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents one end of a netelement in the topology graph
///
/// Each netelement has two ends: position 0 (start) and position 1 (end).
/// The graph treats each end as a separate node, allowing bidirectional
/// navigation within and between netelements.
///
/// # Examples
///
/// ```
/// use tp_lib_core::path::NetelementSide;
///
/// let start = NetelementSide {
///     netelement_id: "NE_A".to_string(),
///     position: 0,  // Start of segment
/// };
///
/// let end = NetelementSide {
///     netelement_id: "NE_A".to_string(),
///     position: 1,  // End of segment
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetelementSide {
    /// ID of the netelement
    pub netelement_id: String,

    /// Position on the netelement: 0 = start, 1 = end
    pub position: u8,
}

impl NetelementSide {
    /// Create a new netelement side
    pub fn new(netelement_id: String, position: u8) -> Result<Self, ProjectionError> {
        if position > 1 {
            return Err(ProjectionError::InvalidGeometry(format!(
                "NetelementSide position must be 0 or 1, got {}",
                position
            )));
        }

        Ok(Self {
            netelement_id,
            position,
        })
    }

    /// Get the opposite end of this netelement
    pub fn opposite(&self) -> Self {
        Self {
            netelement_id: self.netelement_id.clone(),
            position: 1 - self.position,
        }
    }
}

/// Build a directed graph representing the railway network topology
///
/// Creates a petgraph DiGraph where:
/// - **Nodes** are NetelementSide (each netelement has 2 nodes: start and end)
/// - **Edges** represent navigability:
///   - Internal edges: start→end and end→start within each netelement
///   - External edges: connections between netelements via netrelations
///
/// # Graph Structure Example
///
/// For netelement "NE_A":
/// - Node: NE_A position 0 (start)
/// - Node: NE_A position 1 (end)
/// - Internal edges: start→end (forward), end→start (backward)
///
/// For netrelation connecting NE_A(end) to NE_B(start) with forward navigability:
/// - External edge: NE_A position 1 → NE_B position 0
///
/// # Parameters
///
/// - `netelements`: All track segments in the network
/// - `netrelations`: Navigability connections between segments
///
/// # Returns
///
/// A DiGraph where edges represent valid navigation paths, and a mapping
/// from NetelementSide to NodeIndex for efficient lookups.
///
/// # Examples
///
/// ```no_run
/// use tp_lib_core::path::build_topology_graph;
/// use tp_lib_core::models::{Netelement, NetRelation};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let netelements = vec![/* ... */];
/// let netrelations = vec![/* ... */];
///
/// let (graph, node_map) = build_topology_graph(&netelements, &netrelations)?;
///
/// // Use graph for path finding algorithms
/// # Ok(())
/// # }
/// ```
pub fn build_topology_graph(
    netelements: &[Netelement],
    netrelations: &[NetRelation],
) -> Result<(DiGraph<NetelementSide, ()>, HashMap<NetelementSide, NodeIndex>), ProjectionError> {
    let mut graph = DiGraph::new();
    let mut node_map: HashMap<NetelementSide, NodeIndex> = HashMap::new();

    // Step 1: Create nodes for each netelement side
    for netelement in netelements {
        let start_side = NetelementSide::new(netelement.id.clone(), 0)?;
        let end_side = NetelementSide::new(netelement.id.clone(), 1)?;

        let start_node = graph.add_node(start_side.clone());
        let end_node = graph.add_node(end_side.clone());

        node_map.insert(start_side, start_node);
        node_map.insert(end_side, end_node);
    }

    // Step 2: Create internal edges (bidirectional within each netelement)
    for netelement in netelements {
        let start_side = NetelementSide::new(netelement.id.clone(), 0)?;
        let end_side = NetelementSide::new(netelement.id.clone(), 1)?;

        let start_node = node_map[&start_side];
        let end_node = node_map[&end_side];

        // Forward edge: start→end
        graph.add_edge(start_node, end_node, ());

        // Backward edge: end→start
        graph.add_edge(end_node, start_node, ());
    }

    // Step 3: Create external edges from netrelations
    for netrelation in netrelations {
        // Validate netrelation
        netrelation.validate()?;

        // Get nodes for connection points
        let from_side = NetelementSide::new(
            netrelation.from_netelement_id.clone(),
            netrelation.position_on_a,
        )?;
        let to_side =
            NetelementSide::new(netrelation.to_netelement_id.clone(), netrelation.position_on_b)?;

        // Check if nodes exist in graph (skip if referencing non-existent netelements)
        if !node_map.contains_key(&from_side) || !node_map.contains_key(&to_side) {
            // This will be caught by validate_netrelation_references() in T026a
            continue;
        }

        let from_node = node_map[&from_side];
        let to_node = node_map[&to_side];

        // Add edges based on navigability
        if netrelation.is_navigable_forward() {
            graph.add_edge(from_node, to_node, ());
        }

        if netrelation.is_navigable_backward() {
            graph.add_edge(to_node, from_node, ());
        }
    }

    Ok((graph, node_map))
}

/// Validate that all netrelations reference existing netelements
///
/// Checks that `from_netelement_id` and `to_netelement_id` in each netrelation
/// correspond to actual netelements in the network. Returns a list of invalid
/// netrelation IDs that reference non-existent netelements.
///
/// This function is used to detect data quality issues where netrelations
/// reference netelements that don't exist (FR-006a).
///
/// # Parameters
///
/// - `netelements`: All track segments in the network
/// - `netrelations`: Navigability connections to validate
///
/// # Returns
///
/// A vector of netrelation IDs that have invalid references. Empty if all are valid.
///
/// # Examples
///
/// ```no_run
/// use tp_lib_core::path::validate_netrelation_references;
/// use tp_lib_core::models::{Netelement, NetRelation};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let netelements = vec![/* ... */];
/// let netrelations = vec![/* ... */];
///
/// let invalid = validate_netrelation_references(&netelements, &netrelations);
///
/// if !invalid.is_empty() {
///     eprintln!("Warning: {} netrelations reference non-existent netelements", invalid.len());
/// }
/// # Ok(())
/// # }
/// ```
pub fn validate_netrelation_references(
    netelements: &[Netelement],
    netrelations: &[NetRelation],
) -> Vec<String> {
    use std::collections::HashSet;

    // Build set of valid netelement IDs for O(1) lookup
    let netelement_ids: HashSet<&str> = netelements.iter().map(|ne| ne.id.as_str()).collect();

    let mut invalid_netrelations = Vec::new();

    for netrelation in netrelations {
        let from_exists = netelement_ids.contains(netrelation.from_netelement_id.as_str());
        let to_exists = netelement_ids.contains(netrelation.to_netelement_id.as_str());

        if !from_exists || !to_exists {
            invalid_netrelations.push(netrelation.id.clone());
        }
    }

    invalid_netrelations
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Coord, LineString};

    fn create_test_netelement(id: &str) -> Netelement {
        Netelement {
            id: id.to_string(),
            geometry: LineString::new(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 1.0, y: 1.0 },
            ]),
            crs: "EPSG:4326".to_string(),
        }
    }

    #[test]
    fn test_netelement_side_creation() {
        let side = NetelementSide::new("NE_A".to_string(), 0);
        assert!(side.is_ok());

        let side = NetelementSide::new("NE_A".to_string(), 1);
        assert!(side.is_ok());

        let side = NetelementSide::new("NE_A".to_string(), 2);
        assert!(side.is_err());
    }

    #[test]
    fn test_netelement_side_opposite() {
        let start = NetelementSide::new("NE_A".to_string(), 0).unwrap();
        let end = start.opposite();

        assert_eq!(end.position, 1);
        assert_eq!(end.netelement_id, "NE_A");

        let back_to_start = end.opposite();
        assert_eq!(back_to_start.position, 0);
    }

    #[test]
    fn test_build_graph_single_netelement() {
        let netelements = vec![create_test_netelement("NE_A")];
        let netrelations = vec![];

        let result = build_topology_graph(&netelements, &netrelations);
        assert!(result.is_ok());

        let (graph, node_map) = result.unwrap();

        // Should have 2 nodes (start and end)
        assert_eq!(graph.node_count(), 2);

        // Should have 2 internal edges (start→end, end→start)
        assert_eq!(graph.edge_count(), 2);

        // Verify nodes exist
        let start_side = NetelementSide::new("NE_A".to_string(), 0).unwrap();
        let end_side = NetelementSide::new("NE_A".to_string(), 1).unwrap();

        assert!(node_map.contains_key(&start_side));
        assert!(node_map.contains_key(&end_side));
    }

    #[test]
    fn test_build_graph_with_netrelation() {
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
        ];

        let netrelation = NetRelation::new(
            "NR001".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1, // Connect end of A
            0, // to start of B
            true,  // Forward navigable
            false, // Not backward navigable
        )
        .unwrap();

        let netrelations = vec![netrelation];

        let result = build_topology_graph(&netelements, &netrelations);
        assert!(result.is_ok());

        let (graph, _node_map) = result.unwrap();

        // Should have 4 nodes (2 netelements × 2 ends each)
        assert_eq!(graph.node_count(), 4);

        // Should have 4 internal edges + 1 external edge = 5 total
        assert_eq!(graph.edge_count(), 5);
    }

    #[test]
    fn test_build_graph_bidirectional_netrelation() {
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
        ];

        let netrelation = NetRelation::new(
            "NR001".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,    // Connect end of A
            0,    // to start of B
            true, // Forward navigable
            true, // Backward navigable
        )
        .unwrap();

        let netrelations = vec![netrelation];

        let result = build_topology_graph(&netelements, &netrelations);
        assert!(result.is_ok());

        let (graph, _node_map) = result.unwrap();

        // Should have 4 nodes
        assert_eq!(graph.node_count(), 4);

        // Should have 4 internal edges + 2 external edges = 6 total
        assert_eq!(graph.edge_count(), 6);
    }

    #[test]
    fn test_build_graph_missing_netelement_reference() {
        let netelements = vec![create_test_netelement("NE_A")];

        let netrelation = NetRelation::new(
            "NR001".to_string(),
            "NE_A".to_string(),
            "NE_MISSING".to_string(), // References non-existent netelement
            1,
            0,
            true,
            false,
        )
        .unwrap();

        let netrelations = vec![netrelation];

        let result = build_topology_graph(&netelements, &netrelations);
        assert!(result.is_ok()); // Should not fail, just skip invalid reference

        let (graph, _node_map) = result.unwrap();

        // Should have 2 nodes (only NE_A)
        assert_eq!(graph.node_count(), 2);

        // Should have only 2 internal edges (no external edge added)
        assert_eq!(graph.edge_count(), 2);
    }
}
