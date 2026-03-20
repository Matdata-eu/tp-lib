//! Network topology graph representation
//!
//! Builds a petgraph DiGraph from netelements and netrelations to enable
//! efficient path traversal, navigability checking, and shortest-path routing.

use crate::errors::ProjectionError;
use crate::models::{NetRelation, Netelement};
use geo::HaversineLength;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Cache for shortest-path queries between netelement sides.
///
/// Stores `(from_ne_id, from_pos, to_ne_id, to_pos) → Option<f64>`.
/// Lazily populated to avoid recomputing Dijkstra for repeated queries.
pub type ShortestPathCache = HashMap<(String, u8, String, u8), Option<f64>>;

/// Look up or compute the shortest-path distance between two netelement sides,
/// caching the result for future queries.
pub fn cached_shortest_path_distance(
    cache: &mut ShortestPathCache,
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    from: &NetelementSide,
    to: &NetelementSide,
) -> Option<f64> {
    let key = (
        from.netelement_id.clone(),
        from.position,
        to.netelement_id.clone(),
        to.position,
    );

    if let Some(&cached) = cache.get(&key) {
        return cached;
    }

    let result = shortest_path_distance(graph, node_map, from, to);
    cache.insert(key, result);
    result
}

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
#[allow(clippy::type_complexity)]
pub fn build_topology_graph(
    netelements: &[Netelement],
    netrelations: &[NetRelation],
) -> Result<
    (
        DiGraph<NetelementSide, f64>,
        HashMap<NetelementSide, NodeIndex>,
    ),
    ProjectionError,
> {
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
    // Weight = netelement's haversine length in meters
    for netelement in netelements {
        let start_side = NetelementSide::new(netelement.id.clone(), 0)?;
        let end_side = NetelementSide::new(netelement.id.clone(), 1)?;

        let start_node = node_map[&start_side];
        let end_node = node_map[&end_side];

        let length = netelement.geometry.haversine_length();

        // Forward edge: start→end
        graph.add_edge(start_node, end_node, length);

        // Backward edge: end→start
        graph.add_edge(end_node, start_node, length);
    }

    // Step 3: Create external edges from netrelations
    // Weight = 0.0 (netelement connection crossing has negligible distance)
    for netrelation in netrelations {
        // Validate netrelation
        netrelation.validate()?;

        // Get nodes for connection points
        let from_side = NetelementSide::new(
            netrelation.from_netelement_id.clone(),
            netrelation.position_on_a,
        )?;
        let to_side = NetelementSide::new(
            netrelation.to_netelement_id.clone(),
            netrelation.position_on_b,
        )?;

        // Check if nodes exist in graph (skip if referencing non-existent netelements)
        if !node_map.contains_key(&from_side) || !node_map.contains_key(&to_side) {
            continue;
        }

        let from_node = node_map[&from_side];
        let to_node = node_map[&to_side];

        // Add edges based on navigability
        if netrelation.is_navigable_forward() {
            graph.add_edge(from_node, to_node, 0.0);
        }

        if netrelation.is_navigable_backward() {
            graph.add_edge(to_node, from_node, 0.0);
        }
    }

    Ok((graph, node_map))
}

/// Compute the shortest-path distance between two netelement sides.
///
/// Uses a direction-aware Dijkstra that prevents consecutive external-edge
/// (connection) traversals.  At netelement connections, multiple netelement sides
/// may connect at the same point via zero-weight external edges.  Standard
/// Dijkstra can "shortcut" through a connection (e.g. NE_B → connection → NE_C)
/// without traversing the connecting netelement, which would represent a
/// physically impossible direction reversal for a train.
///
/// Returns `None` if no path exists (disconnected components).
pub fn shortest_path_distance(
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    from: &NetelementSide,
    to: &NetelementSide,
) -> Option<f64> {
    let &from_idx = node_map.get(from)?;
    let &to_idx = node_map.get(to)?;

    direction_aware_dijkstra(graph, from_idx, to_idx).map(|(cost, _)| cost)
}

/// Return the sequence of graph nodes along the shortest direction-aware path.
///
/// Used by bridge insertion to recover intermediate netelements.
pub fn shortest_path_route(
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    from: &NetelementSide,
    to: &NetelementSide,
) -> Option<Vec<NodeIndex>> {
    let &from_idx = node_map.get(from)?;
    let &to_idx = node_map.get(to)?;

    direction_aware_dijkstra(graph, from_idx, to_idx).map(|(_, path)| path)
}

/// Direction-aware Dijkstra that prevents U-turns.
///
/// The state is expanded to `(NodeIndex, arrived_via_external)`.  When
/// `arrived_via_external` is true, only internal edges (source and target
/// belong to the same netelement) may be followed.  This forces the algorithm
/// to traverse the connecting netelement before taking another external edge,
/// modelling the physical constraint that a train cannot cross from one branch
/// to another at a netelement connection without traversing the connecting segment.
fn direction_aware_dijkstra(
    graph: &DiGraph<NetelementSide, f64>,
    from_idx: NodeIndex,
    to_idx: NodeIndex,
) -> Option<(f64, Vec<NodeIndex>)> {
    if from_idx == to_idx {
        return Some((0.0, vec![from_idx]));
    }

    #[derive(Clone, PartialEq)]
    struct State {
        cost: f64,
        node: NodeIndex,
        via_external: bool,
    }

    impl Eq for State {}

    impl PartialOrd for State {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            other.cost.partial_cmp(&self.cost) // min-heap
        }
    }

    impl Ord for State {
        fn cmp(&self, other: &Self) -> Ordering {
            self.partial_cmp(other).unwrap_or(Ordering::Equal)
        }
    }

    type StateKey = (NodeIndex, bool);

    let mut dist: HashMap<StateKey, f64> = HashMap::new();
    let mut prev: HashMap<StateKey, StateKey> = HashMap::new();
    let mut heap = BinaryHeap::new();

    let start_key: StateKey = (from_idx, false);
    dist.insert(start_key, 0.0);
    heap.push(State {
        cost: 0.0,
        node: from_idx,
        via_external: false,
    });

    let mut reached_target: Option<(f64, StateKey)> = None;

    while let Some(State {
        cost,
        node,
        via_external,
    }) = heap.pop()
    {
        let key: StateKey = (node, via_external);

        if let Some(&best) = dist.get(&key) {
            if cost > best {
                continue;
            }
        }

        if node == to_idx {
            reached_target = Some((cost, key));
            break;
        }

        let source_ne = &graph[node].netelement_id;

        for edge in graph.edges_directed(node, petgraph::Direction::Outgoing) {
            let next = edge.target();
            let w = *edge.weight();
            let target_ne = &graph[next].netelement_id;
            let edge_is_external = source_ne != target_ne;

            // CONSTRAINT: after an external edge, only internal edges allowed.
            if via_external && edge_is_external {
                continue;
            }

            let new_cost = cost + w;
            let next_key: StateKey = (next, edge_is_external);

            if dist.get(&next_key).map_or(true, |&d| new_cost < d) {
                dist.insert(next_key, new_cost);
                prev.insert(next_key, key);
                heap.push(State {
                    cost: new_cost,
                    node: next,
                    via_external: edge_is_external,
                });
            }
        }
    }

    let (cost, target_key) = reached_target?;

    // Reconstruct path from predecessor map.
    let mut path_keys = vec![target_key];
    let mut current = target_key;
    while let Some(&predecessor) = prev.get(&current) {
        path_keys.push(predecessor);
        current = predecessor;
    }
    path_keys.reverse();

    let path: Vec<NodeIndex> = path_keys.iter().map(|(node, _)| *node).collect();

    Some((cost, path))
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
            geometry: LineString::new(vec![Coord { x: 0.0, y: 0.0 }, Coord { x: 1.0, y: 1.0 }]),
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
            1,     // Connect end of A
            0,     // to start of B
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

    #[test]
    fn test_shortest_path_same_netelement() {
        let netelements = vec![create_test_netelement("NE_A")];
        let (graph, node_map) = build_topology_graph(&netelements, &[]).unwrap();

        let from = NetelementSide::new("NE_A".to_string(), 0).unwrap();
        let to = NetelementSide::new("NE_A".to_string(), 1).unwrap();

        let dist = shortest_path_distance(&graph, &node_map, &from, &to);
        assert!(dist.is_some());
        assert!(dist.unwrap() > 0.0);
    }

    #[test]
    fn test_shortest_path_across_netelement_connection() {
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
        ];

        let netrelation = NetRelation::new(
            "NR001".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1, 0, true, false,
        )
        .unwrap();

        let (graph, node_map) = build_topology_graph(&netelements, &[netrelation]).unwrap();

        // Route: NE_A:0 → NE_A:1 → (connection 0.0) → NE_B:0 → NE_B:1
        let from = NetelementSide::new("NE_A".to_string(), 0).unwrap();
        let to = NetelementSide::new("NE_B".to_string(), 1).unwrap();

        let dist = shortest_path_distance(&graph, &node_map, &from, &to);
        assert!(dist.is_some());
        // Should be length(NE_A) + 0 + length(NE_B) — both have same geometry
        let ne_a_len = netelements[0].geometry.haversine_length();
        let ne_b_len = netelements[1].geometry.haversine_length();
        let expected = ne_a_len + ne_b_len;
        assert!((dist.unwrap() - expected).abs() < 0.1);
    }

    #[test]
    fn test_shortest_path_disconnected() {
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
        ];
        // No netrelation → disconnected
        let (graph, node_map) = build_topology_graph(&netelements, &[]).unwrap();

        let from = NetelementSide::new("NE_A".to_string(), 0).unwrap();
        let to = NetelementSide::new("NE_B".to_string(), 0).unwrap();

        assert!(shortest_path_distance(&graph, &node_map, &from, &to).is_none());
    }

    #[test]
    fn test_direction_aware_dijkstra_no_u_turns() {
        // Y-shaped junction: NE_A and NE_B both connect to NE_X at position 0.
        //
        //   NE_A:0 ── NE_A:1 ──ext──► NE_X:0 ── NE_X:1 ──ext──► NE_B:0 ── NE_B:1
        //                                ▲                                      │
        //                                └────────────ext─────────────────────  ┘
        //
        // Without U-turn prevention the shortcut NE_A:1→NE_X:0→NE_B:1 would
        // skip NE_X entirely (two consecutive external edges).
        // The direction-aware Dijkstra must force traversal through NE_X.

        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_X"),
            create_test_netelement("NE_B"),
        ];

        let netrelations = vec![
            // NE_A:1 ↔ NE_X:0
            NetRelation::new(
                "NR1".to_string(),
                "NE_A".to_string(),
                "NE_X".to_string(),
                1, 0, true, true,
            )
            .unwrap(),
            // NE_X:1 ↔ NE_B:0
            NetRelation::new(
                "NR2".to_string(),
                "NE_X".to_string(),
                "NE_B".to_string(),
                1, 0, true, true,
            )
            .unwrap(),
            // NE_B:1 ↔ NE_X:0  (creates the U-turn shortcut)
            NetRelation::new(
                "NR3".to_string(),
                "NE_B".to_string(),
                "NE_X".to_string(),
                1, 0, true, true,
            )
            .unwrap(),
        ];

        let (graph, node_map) = build_topology_graph(&netelements, &netrelations).unwrap();

        let from = NetelementSide::new("NE_A".to_string(), 0).unwrap();
        let to = NetelementSide::new("NE_B".to_string(), 0).unwrap();

        let path = shortest_path_route(&graph, &node_map, &from, &to);
        assert!(path.is_some(), "A path should exist");

        let path = path.unwrap();

        // Verify no U-turns: no two consecutive edges may both be external.
        // An edge is external when its source and target belong to different netelements.
        for window in path.windows(3) {
            let a = &graph[window[0]];
            let b = &graph[window[1]];
            let c = &graph[window[2]];

            let ab_external = a.netelement_id != b.netelement_id;
            let bc_external = b.netelement_id != c.netelement_id;

            assert!(
                !(ab_external && bc_external),
                "U-turn detected: {}:{} → {}:{} → {}:{} has consecutive external edges",
                a.netelement_id, a.position,
                b.netelement_id, b.position,
                c.netelement_id, c.position,
            );
        }
    }
}
