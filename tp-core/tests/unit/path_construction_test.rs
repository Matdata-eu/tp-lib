//! Unit tests for path construction module
//!
//! Tests for graph representation and path building algorithms.

use geo::{Coord, LineString};
use tp_lib_core::models::{NetRelation, Netelement};
use tp_lib_core::path::{build_topology_graph, validate_netrelation_references, NetelementSide};

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_netelement(id: &str) -> Netelement {
        Netelement {
            id: id.to_string(),
            geometry: LineString::new(vec![Coord { x: 0.0, y: 0.0 }, Coord { x: 1.0, y: 1.0 }]),
            crs: "EPSG:4326".to_string(),
        }
    }

    // Foundational Graph Tests (T020-T022)

    #[test]
    fn test_netelement_side_node_creation() {
        // T020: Test NetelementSide node creation
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
        ];
        let netrelations = vec![];

        let result = build_topology_graph(&netelements, &netrelations);
        assert!(result.is_ok());

        let (graph, node_map) = result.unwrap();

        // Should have 4 nodes (2 netelements × 2 ends each)
        assert_eq!(graph.node_count(), 4);

        // Verify each netelement has start and end nodes
        let ne_a_start = NetelementSide::new("NE_A".to_string(), 0).unwrap();
        let ne_a_end = NetelementSide::new("NE_A".to_string(), 1).unwrap();
        let ne_b_start = NetelementSide::new("NE_B".to_string(), 0).unwrap();
        let ne_b_end = NetelementSide::new("NE_B".to_string(), 1).unwrap();

        assert!(node_map.contains_key(&ne_a_start));
        assert!(node_map.contains_key(&ne_a_end));
        assert!(node_map.contains_key(&ne_b_start));
        assert!(node_map.contains_key(&ne_b_end));
    }

    #[test]
    fn test_internal_edge_creation() {
        // T021: Test internal edge creation (start→end, end→start)
        let netelements = vec![create_test_netelement("NE_A")];
        let netrelations = vec![];

        let result = build_topology_graph(&netelements, &netrelations);
        assert!(result.is_ok());

        let (graph, node_map) = result.unwrap();

        // Should have 2 internal edges (bidirectional)
        assert_eq!(graph.edge_count(), 2);

        let start_side = NetelementSide::new("NE_A".to_string(), 0).unwrap();
        let end_side = NetelementSide::new("NE_A".to_string(), 1).unwrap();

        let start_node = node_map[&start_side];
        let end_node = node_map[&end_side];

        // Check forward edge exists (start→end)
        assert!(graph.contains_edge(start_node, end_node));

        // Check backward edge exists (end→start)
        assert!(graph.contains_edge(end_node, start_node));
    }

    #[test]
    fn test_netrelation_connection_edge_creation() {
        // T022: Test netrelation connection edge creation
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
        ];

        // Create bidirectional netrelation connecting end of A to start of B
        let netrelation = NetRelation::new(
            "NR001".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,    // position_on_a = end
            0,    // position_on_b = start
            true, // navigable forward
            true, // navigable backward
        )
        .unwrap();

        let netrelations = vec![netrelation];

        let result = build_topology_graph(&netelements, &netrelations);
        assert!(result.is_ok());

        let (graph, node_map) = result.unwrap();

        // Should have 4 internal edges + 2 external edges = 6 total
        assert_eq!(graph.edge_count(), 6);

        let ne_a_end = NetelementSide::new("NE_A".to_string(), 1).unwrap();
        let ne_b_start = NetelementSide::new("NE_B".to_string(), 0).unwrap();

        let a_end_node = node_map[&ne_a_end];
        let b_start_node = node_map[&ne_b_start];

        // Check forward connection exists (A end → B start)
        assert!(graph.contains_edge(a_end_node, b_start_node));

        // Check backward connection exists (B start → A end)
        assert!(graph.contains_edge(b_start_node, a_end_node));
    }

    #[test]
    fn test_netrelation_unidirectional_edge() {
        // T022: Test unidirectional netrelation (only forward navigable)
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
        ];

        let netrelation = NetRelation::new(
            "NR001".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,     // position_on_a = end
            0,     // position_on_b = start
            true,  // navigable forward
            false, // NOT navigable backward
        )
        .unwrap();

        let netrelations = vec![netrelation];

        let result = build_topology_graph(&netelements, &netrelations);
        assert!(result.is_ok());

        let (graph, node_map) = result.unwrap();

        // Should have 4 internal edges + 1 external edge = 5 total
        assert_eq!(graph.edge_count(), 5);

        let ne_a_end = NetelementSide::new("NE_A".to_string(), 1).unwrap();
        let ne_b_start = NetelementSide::new("NE_B".to_string(), 0).unwrap();

        let a_end_node = node_map[&ne_a_end];
        let b_start_node = node_map[&ne_b_start];

        // Check forward connection exists (A end → B start)
        assert!(graph.contains_edge(a_end_node, b_start_node));

        // Check backward connection does NOT exist (B start → A end)
        assert!(!graph.contains_edge(b_start_node, a_end_node));
    }

    // Validation Tests (T026, T026b)

    #[test]
    fn test_netrelation_valid_bidirectional() {
        // T026: Valid bidirectional netrelation
        let relation = NetRelation::new(
            "NR001".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1, // position_on_a: end of A
            0, // position_on_b: start of B
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
    fn test_netrelation_valid_unidirectional() {
        // T026: Valid unidirectional netrelation
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
    fn test_netrelation_invalid_position_on_a() {
        // T026: position_on_a must be 0 or 1
        let relation = NetRelation::new(
            "NR003".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            2, // Invalid: > 1
            0,
            true,
            false,
        );

        assert!(relation.is_err());
    }

    #[test]
    fn test_netrelation_invalid_position_on_b() {
        // T026: position_on_b must be 0 or 1
        let relation = NetRelation::new(
            "NR004".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            5, // Invalid: > 1
            true,
            false,
        );

        assert!(relation.is_err());
    }

    #[test]
    fn test_netrelation_self_reference() {
        // T026: Cannot connect to itself
        let relation = NetRelation::new(
            "NR005".to_string(),
            "NE_A".to_string(),
            "NE_A".to_string(), // Invalid: same as from
            1,
            0,
            true,
            false,
        );

        assert!(relation.is_err());
    }

    #[test]
    fn test_netrelation_empty_id() {
        // T026: ID must be non-empty
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
    fn test_netrelation_empty_from_id() {
        // T026: from_netelement_id must be non-empty
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
    fn test_netrelation_empty_to_id() {
        // T026: to_netelement_id must be non-empty
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

    // T026a-T026b: Test invalid netelement reference handling

    #[test]
    fn test_validate_netrelation_references_all_valid() {
        // T026a: All references are valid
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
            create_test_netelement("NE_C"),
        ];

        let netrelations = vec![
            NetRelation::new(
                "NR001".to_string(),
                "NE_A".to_string(),
                "NE_B".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
            NetRelation::new(
                "NR002".to_string(),
                "NE_B".to_string(),
                "NE_C".to_string(),
                1,
                0,
                true,
                false,
            )
            .unwrap(),
        ];

        let invalid = validate_netrelation_references(&netelements, &netrelations);
        assert_eq!(invalid.len(), 0);
    }

    #[test]
    fn test_validate_netrelation_references_invalid_from() {
        // T026b: from_netelement_id references non-existent netelement
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
        ];

        let netrelations = vec![NetRelation::new(
            "NR001".to_string(),
            "NE_MISSING".to_string(), // Invalid reference
            "NE_B".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        let invalid = validate_netrelation_references(&netelements, &netrelations);
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0], "NR001");
    }

    #[test]
    fn test_validate_netrelation_references_invalid_to() {
        // T026b: to_netelement_id references non-existent netelement
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
        ];

        let netrelations = vec![NetRelation::new(
            "NR001".to_string(),
            "NE_A".to_string(),
            "NE_MISSING".to_string(), // Invalid reference
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        let invalid = validate_netrelation_references(&netelements, &netrelations);
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0], "NR001");
    }

    #[test]
    fn test_validate_netrelation_references_both_invalid() {
        // T026b: Both from and to reference non-existent netelements
        let netelements = vec![create_test_netelement("NE_A")];

        let netrelations = vec![NetRelation::new(
            "NR001".to_string(),
            "NE_MISSING1".to_string(), // Invalid
            "NE_MISSING2".to_string(), // Invalid
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        let invalid = validate_netrelation_references(&netelements, &netrelations);
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0], "NR001");
    }

    #[test]
    fn test_validate_netrelation_references_mixed() {
        // T026b: Mix of valid and invalid references
        let netelements = vec![
            create_test_netelement("NE_A"),
            create_test_netelement("NE_B"),
        ];

        let netrelations = vec![
            NetRelation::new(
                "NR001".to_string(),
                "NE_A".to_string(),
                "NE_B".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
            NetRelation::new(
                "NR002".to_string(),
                "NE_A".to_string(),
                "NE_MISSING".to_string(), // Invalid
                1,
                0,
                true,
                false,
            )
            .unwrap(),
            NetRelation::new(
                "NR003".to_string(),
                "NE_MISSING".to_string(), // Invalid
                "NE_A".to_string(),
                1,
                0,
                true,
                false,
            )
            .unwrap(),
        ];

        let invalid = validate_netrelation_references(&netelements, &netrelations);
        assert_eq!(invalid.len(), 2);
        assert!(invalid.contains(&"NR002".to_string()));
        assert!(invalid.contains(&"NR003".to_string()));
    }

    #[test]
    fn test_validate_netrelation_references_empty_collections() {
        // T026b: Empty collections should return empty result
        let netelements = vec![];
        let netrelations = vec![];

        let invalid = validate_netrelation_references(&netelements, &netrelations);
        assert_eq!(invalid.len(), 0);
    }

    // US1 Phase 4 Tests (T072-T074, T083)
    // T072: Test forward path construction
    // T073: Test backward path construction and reversal
    // T074: Test bidirectional agreement detection
    // T083: Test early termination detection

    // US2 Tests (T101)
    // T101: Test intrinsic coordinate calculation

    #[test]
    fn test_intrinsic_coordinate_calculation() {
        // T101: Verify intrinsic coordinates are correctly calculated (0-1 range)
        use chrono::Utc;
        use tp_lib_core::models::{AssociatedNetElement, GnssPosition, TrainPath};
        use tp_lib_core::{project_onto_path, PathConfig};

        // Create a simple linear network
        let netelements = vec![Netelement {
            id: "NE_A".to_string(),
            geometry: LineString::new(vec![
                Coord {
                    x: 4.350,
                    y: 50.850,
                },
                Coord {
                    x: 4.360,
                    y: 50.860,
                },
            ]),
            crs: "EPSG:4326".to_string(),
        }];

        // Create a path with one segment
        let segments =
            vec![AssociatedNetElement::new("NE_A".to_string(), 0.85, 0.0, 1.0, 0, 2).unwrap()];

        let path = TrainPath::new(segments, 0.85, Some(Utc::now()), None).unwrap();

        // Create GNSS positions at start, middle, and end
        let gnss_positions = vec![
            GnssPosition::new(50.850, 4.350, Utc::now().into(), "EPSG:4326".to_string()).unwrap(), // Start
            GnssPosition::new(50.855, 4.355, Utc::now().into(), "EPSG:4326".to_string()).unwrap(), // Middle
            GnssPosition::new(50.860, 4.360, Utc::now().into(), "EPSG:4326".to_string()).unwrap(), // End
        ];

        let config = PathConfig::default();
        let result = project_onto_path(&gnss_positions, &path, &netelements, &config);

        assert!(result.is_ok(), "Projection should succeed");
        let projected = result.unwrap();

        assert_eq!(projected.len(), 3, "Should have 3 projected positions");

        // Verify intrinsic coordinates exist and are in valid range [0, 1]
        for proj in &projected {
            assert!(
                proj.intrinsic.is_some(),
                "Intrinsic coordinate should be set"
            );
            let intrinsic = proj.intrinsic.unwrap();
            assert!(
                (0.0..=1.0).contains(&intrinsic),
                "Intrinsic coordinate {} should be in range [0, 1]",
                intrinsic
            );
        }

        // Verify ordering: start should have lower intrinsic than end
        let start_intrinsic = projected[0].intrinsic.unwrap();
        let end_intrinsic = projected[2].intrinsic.unwrap();
        assert!(
            start_intrinsic < end_intrinsic,
            "Start intrinsic ({}) should be less than end intrinsic ({})",
            start_intrinsic,
            end_intrinsic
        );
    }

    // US6 Tests (T147)
    // T147: Test fallback detection logic
    // Note: Full fallback behavior is covered by integration tests (T137-T139)
    // This unit test verifies the fallback logic components
    #[test]
    fn test_fallback_detection_logic() {
        use chrono::Utc;
        use tp_lib_core::models::GnssPosition;
        use tp_lib_core::path::{PathCalculationMode, PathConfig};
        use tp_lib_core::calculate_train_path;

        // Create GNSS positions far from network to trigger "no candidates" scenario
        // This is the most reliable way to trigger fallback
        let gnss_positions = vec![
            GnssPosition::new(50.950, 4.450, Utc::now().into(), "EPSG:4326".to_string()).unwrap(), // Very far
            GnssPosition::new(50.951, 4.451, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        // Create network far away from GNSS positions
        let netelements = vec![
            Netelement {
                id: "NE_A".to_string(),
                geometry: LineString::new(vec![
                    Coord { x: 4.350, y: 50.850 }, // ~10km away from GNSS
                    Coord { x: 4.351, y: 50.851 },
                ]),
                crs: "EPSG:4326".to_string(),
            },
        ];

        let netrelations: Vec<NetRelation> = vec![];

        // Use small cutoff distance to ensure GNSS is beyond reach
        let mut config = PathConfig::default();
        config.cutoff_distance = 100.0; // 100m - GNSS is ~10km away

        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);

        assert!(result.is_ok(), "Fallback should succeed");
        let path_result = result.unwrap();

        // T147: Verify fallback mode was triggered
        assert_eq!(
            path_result.mode,
            PathCalculationMode::FallbackIndependent,
            "Should use fallback mode when GNSS beyond cutoff distance"
        );

        // Verify no path was calculated
        assert!(
            path_result.path.is_none(),
            "Path should be None in fallback mode"
        );

        // Verify warnings indicate fallback was used
        assert!(
            path_result
                .warnings
                .iter()
                .any(|w| w.contains("No continuous path found")),
            "Should warn about path calculation failure, got: {:?}",
            path_result.warnings
        );
        assert!(
            path_result
                .warnings
                .iter()
                .any(|w| w.to_lowercase().contains("fallback") || w.to_lowercase().contains("falling back")),
            "Should warn about using fallback mode, got: {:?}",
            path_result.warnings
        );
    }

    #[test]
    fn test_fallback_with_path_only_mode() {
        use chrono::Utc;
        use tp_lib_core::models::GnssPosition;
        use tp_lib_core::path::{PathCalculationMode, PathConfig};
        use tp_lib_core::calculate_train_path;

        // GNSS far from network to trigger fallback
        let gnss_positions =
            vec![GnssPosition::new(50.950, 4.450, Utc::now().into(), "EPSG:4326".to_string()).unwrap()];

        let netelements = vec![create_test_netelement("NE_A")]; // At default location ~4.35, 50.85

        let netrelations: Vec<NetRelation> = vec![];

        let mut config = PathConfig::default();
        config.path_only = true;
        config.cutoff_distance = 100.0; // GNSS is ~10km away

        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);

        assert!(result.is_ok(), "Path-only fallback should succeed");
        let path_result = result.unwrap();

        // Verify fallback mode
        assert_eq!(
            path_result.mode,
            PathCalculationMode::FallbackIndependent,
            "Should use fallback mode when GNSS beyond cutoff"
        );

        // Verify no projected positions in path-only mode
        assert!(
            path_result.projected_positions.is_empty(),
            "Path-only mode should skip fallback projection"
        );

        // Verify warnings indicate path-only skipped projection
        assert!(
            path_result
                .warnings
                .iter()
                .any(|w| w.contains("Path-only mode")),
            "Should warn about path-only mode"
        );
    }
}
