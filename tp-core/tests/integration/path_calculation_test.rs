//! Integration tests for path calculation
//!
//! End-to-end tests that validate complete workflows from GNSS data
//! and network topology to calculated paths and projected positions.

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use tp_lib_core::*;

    // US1: Path Calculation Tests (T039-T042)

    // T039: Simple linear path without junctions
    #[test]
    fn test_successful_path_calculation_linear() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathConfig};

        // Create simple linear network: NE_A -> NE_B -> NE_C
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_C".to_string(),
                LineString::from(vec![(4.352, 50.852), (4.353, 50.853)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        // Create netrelations connecting segments
        let netrelations = vec![
            NetRelation::new(
                "NR_AB".to_string(),
                "NE_A".to_string(),
                "NE_B".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
            NetRelation::new(
                "NR_BC".to_string(),
                "NE_B".to_string(),
                "NE_C".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
        ];

        // Create GNSS positions along the linear path
        let gnss_positions = vec![
            GnssPosition::new(50.8503, 4.3502, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.8512, 4.3512, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.8523, 4.3522, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        // Calculate path with default configuration
        let config = PathConfig::default();
        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);

        // Verify successful calculation
        assert!(result.is_ok(), "Path calculation should succeed");
        let path_result = result.unwrap();

        // Verify path exists and is continuous
        assert!(path_result.path.is_some(), "Path should be calculated");
        let path = path_result.path.unwrap();
        assert_eq!(path.segments.len(), 3, "Should have 3 segments");
        assert_eq!(path.segments[0].netelement_id, "NE_A");
        assert_eq!(path.segments[1].netelement_id, "NE_B");
        assert_eq!(path.segments[2].netelement_id, "NE_C");
    }

    // T040: Path with 3 candidate branches at junction
    #[test]
    fn test_path_calculation_with_junction() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathConfig};

        // Create junction network: NE_A connects to NE_B1, NE_B2, NE_B3
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B1".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B2".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.350, 50.852)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B3".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.351, 50.853)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        // Create netrelations for junction
        let netrelations = vec![
            NetRelation::new(
                "NR_AB1".to_string(),
                "NE_A".to_string(),
                "NE_B1".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
            NetRelation::new(
                "NR_AB2".to_string(),
                "NE_A".to_string(),
                "NE_B2".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
            NetRelation::new(
                "NR_AB3".to_string(),
                "NE_A".to_string(),
                "NE_B3".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
        ];

        // Create GNSS positions that clearly follow NE_B1 branch
        let gnss_positions = vec![
            GnssPosition::new(50.8503, 4.3502, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.8515, 4.3515, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        let config = PathConfig::default();
        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);

        assert!(result.is_ok());
        let path_result = result.unwrap();
        let path = path_result.path.unwrap();

        // Should select NE_B1 (highest probability based on position proximity)
        assert!(
            path.segments.iter().any(|s| s.netelement_id == "NE_B1"),
            "Should select NE_B1 branch"
        );
    }

    // T041: Heading filtering
    #[test]
    fn test_heading_filtering() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathConfig};

        // Create network with segment at wrong heading
        let netelements = vec![
            Netelement::new(
                "NE_forward".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_backward".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.350, 50.850)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        let netrelations = vec![NetRelation::new(
            "NR_conn".to_string(),
            "NE_forward".to_string(),
            "NE_backward".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        // GNSS positions with heading indicating forward direction
        let mut gnss_positions = vec![
            GnssPosition::new(50.8502, 4.3502, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.8508, 4.3508, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];
        gnss_positions[0].heading = Some(45.0); // Northeast heading
        gnss_positions[1].heading = Some(45.0);

        let config = PathConfig::default();
        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);

        assert!(result.is_ok());
        let path_result = result.unwrap();

        // Should prefer NE_forward (matching heading) over NE_backward (opposite heading)
        if let Some(path) = path_result.path {
            let forward_prob = path
                .segments
                .iter()
                .find(|s| s.netelement_id == "NE_forward")
                .map(|s| s.probability);
            let backward_prob = path
                .segments
                .iter()
                .find(|s| s.netelement_id == "NE_backward")
                .map(|s| s.probability);

            if let (Some(fw), Some(bw)) = (forward_prob, backward_prob) {
                assert!(
                    fw > bw,
                    "Forward segment should have higher probability than backward"
                );
            }
        }
    }

    // T042: Select highest probability path
    #[test]
    fn test_highest_probability_path_selection() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathConfig};

        // Create network with two possible paths
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            // Path 1: close to GNSS positions
            Netelement::new(
                "NE_B_close".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            // Path 2: far from GNSS positions
            Netelement::new(
                "NE_B_far".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.360, 50.860)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        let netrelations = vec![
            NetRelation::new(
                "NR_A_close".to_string(),
                "NE_A".to_string(),
                "NE_B_close".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
            NetRelation::new(
                "NR_A_far".to_string(),
                "NE_A".to_string(),
                "NE_B_far".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
        ];

        // GNSS positions near the "close" path
        // NE_B_close is at (lon, lat) = (4.351, 50.851) -> (4.352, 50.852)
        // NE_B_far is at (lon, lat) = (4.351, 50.851) -> (4.360, 50.860)
        // So GNSS should be near 4.352, 50.852 (close path endpoint)
        let gnss_positions = vec![
            GnssPosition::new(50.850, 4.350, Utc::now().into(), "EPSG:4326".to_string()).unwrap(), // Near NE_A start
            GnssPosition::new(50.852, 4.352, Utc::now().into(), "EPSG:4326".to_string()).unwrap(), // Near NE_B_close end
        ];

        let config = PathConfig::default();
        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);

        assert!(result.is_ok(), "Path calculation should succeed");
        let path_result = result.unwrap();
        assert!(path_result.path.is_some(), "Path should be calculated");

        let path = path_result.path.unwrap();

        // Should select the closer path (NE_B_close) with higher probability
        assert!(
            path.segments
                .iter()
                .any(|s| s.netelement_id == "NE_B_close"),
            "Should select closer path. Got segments: {:?}",
            path.segments
                .iter()
                .map(|s| &s.netelement_id)
                .collect::<Vec<_>>()
        );
        assert!(
            path.overall_probability > 0.5,
            "Path should have reasonable probability"
        );
    }

    // US2: Projection Tests (T089-T091)

    #[test]
    fn test_project_coordinates_on_path() {
        // T089: Project GNSS coordinates onto calculated path
        // This test is intentionally empty - functionality tested by test_project_coordinates_onto_path()
    }

    // Additional integration tests to be added per user stories

    // T031: NetRelation GeoJSON parsing test
    #[test]
    fn test_netrelation_geojson_parsing() {
        use tp_lib_core::io::parse_netrelations_geojson;

        // Create test GeoJSON file with netrelations
        let test_geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [4.3517, 50.8503]
                    },
                    "properties": {
                        "type": "netrelation",
                        "id": "NR_001",
                        "netelementA": "NE_A",
                        "netelementB": "NE_B",
                        "positionOnA": 1,
                        "positionOnB": 0,
                        "navigability": "both"
                    }
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [4.3520, 50.8505]
                    },
                    "properties": {
                        "type": "netrelation",
                        "id": "NR_002",
                        "netelementA": "NE_B",
                        "netelementB": "NE_C",
                        "positionOnA": 1,
                        "positionOnB": 0,
                        "navigability": "AB"
                    }
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[4.35, 50.85], [4.36, 50.86]]
                    },
                    "properties": {
                        "id": "NE_A"
                    }
                }
            ]
        }"#;

        // Write to temporary file
        let temp_file = std::env::temp_dir().join("test_netrelations.geojson");
        std::fs::write(&temp_file, test_geojson).unwrap();

        // Parse netrelations
        let netrelations = parse_netrelations_geojson(temp_file.to_str().unwrap()).unwrap();

        // Verify results
        assert_eq!(
            netrelations.len(),
            2,
            "Should parse 2 netrelations, skipping netelement"
        );

        let nr1 = &netrelations[0];
        assert_eq!(nr1.id, "NR_001");
        assert_eq!(nr1.from_netelement_id, "NE_A");
        assert_eq!(nr1.to_netelement_id, "NE_B");
        assert_eq!(nr1.position_on_a, 1);
        assert_eq!(nr1.position_on_b, 0);
        assert!(nr1.navigable_forward);
        assert!(nr1.navigable_backward);

        let nr2 = &netrelations[1];
        assert_eq!(nr2.id, "NR_002");
        assert_eq!(nr2.from_netelement_id, "NE_B");
        assert_eq!(nr2.to_netelement_id, "NE_C");
        assert!(nr2.navigable_forward);
        assert!(!nr2.navigable_backward);

        // Clean up
        std::fs::remove_file(temp_file).unwrap();
    }

    // T032: TrainPath serialization roundtrip test
    #[test]
    fn test_trainpath_serialization_roundtrip() {
        use chrono::Utc;
        use geo::LineString;
        use std::collections::HashMap;
        use tp_lib_core::io::{parse_trainpath_csv, write_trainpath_csv, write_trainpath_geojson};
        use tp_lib_core::{AssociatedNetElement, Netelement, TrainPath};

        // Create test TrainPath
        let segments = vec![
            AssociatedNetElement::new("NE_A".to_string(), 0.87, 0.0, 1.0, 0, 10).unwrap(),
            AssociatedNetElement::new("NE_B".to_string(), 0.92, 0.0, 1.0, 11, 18).unwrap(),
        ];

        let original_path = TrainPath::new(segments, 0.89, Some(Utc::now()), None).unwrap();

        // Test CSV roundtrip
        let csv_temp = std::env::temp_dir().join("test_trainpath.csv");
        let mut csv_file = std::fs::File::create(&csv_temp).unwrap();
        write_trainpath_csv(&original_path, &mut csv_file).unwrap();
        drop(csv_file);

        let parsed_path = parse_trainpath_csv(csv_temp.to_str().unwrap()).unwrap();

        // Verify CSV roundtrip
        assert_eq!(parsed_path.segments.len(), 2);
        assert_eq!(parsed_path.overall_probability, 0.89);
        assert_eq!(parsed_path.segments[0].netelement_id, "NE_A");
        assert_eq!(parsed_path.segments[0].probability, 0.87);
        assert_eq!(parsed_path.segments[1].netelement_id, "NE_B");
        assert_eq!(parsed_path.segments[1].probability, 0.92);

        std::fs::remove_file(&csv_temp).unwrap();

        // Test GeoJSON serialization (no roundtrip, just verify it works)
        let mut netelements_map = HashMap::new();
        netelements_map.insert(
            "NE_A".to_string(),
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        );
        netelements_map.insert(
            "NE_B".to_string(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.36, 50.86), (4.37, 50.87)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        );

        let geojson_temp = std::env::temp_dir().join("test_trainpath.geojson");
        let mut geojson_file = std::fs::File::create(&geojson_temp).unwrap();
        write_trainpath_geojson(&original_path, &netelements_map, &mut geojson_file).unwrap();
        drop(geojson_file);

        // Verify GeoJSON file exists and has content
        let geojson_content = std::fs::read_to_string(&geojson_temp).unwrap();
        assert!(geojson_content.contains("\"overall_probability\""));
        assert!(geojson_content.contains("NE_A"));
        assert!(geojson_content.contains("NE_B"));

        std::fs::remove_file(&geojson_temp).unwrap();
    }

    // US2: Project Coordinates onto Path Tests (T089-T092)

    // T089: Project coordinates onto calculated path
    #[test]
    fn test_project_coordinates_onto_path() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{
            AssociatedNetElement, GnssPosition, NetRelation, Netelement, TrainPath,
        };
        use tp_lib_core::{project_onto_path, PathConfig};

        // Create simple network
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.360, 50.860)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.360, 50.860), (4.370, 50.870)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        let _netrelations = [NetRelation::new(
            "NR_AB".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        // Create pre-calculated path
        let segments = vec![
            AssociatedNetElement::new(
                "NE_A".to_string(),
                1.0, // probability
                0.0, // start_intrinsic
                1.0, // end_intrinsic
                0,   // gnss_start_index
                10,  // gnss_end_index
            )
            .unwrap(),
            AssociatedNetElement::new("NE_B".to_string(), 1.0, 0.0, 1.0, 11, 20).unwrap(),
        ];

        let path = TrainPath::new(segments, 0.90, Some(Utc::now()), None).unwrap();

        // Create GNSS positions to project
        let gnss_positions = vec![
            GnssPosition::new(50.8551, 4.3551, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.8652, 4.3652, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        // Project coordinates onto path
        let config = PathConfig::default();
        let result = project_onto_path(&gnss_positions, &path, &netelements, &config);

        // Verify projection result
        assert!(result.is_ok(), "Projection should succeed");
        let projected = result.unwrap();
        assert_eq!(projected.len(), 2, "Should have 2 projected positions");

        // Each projection should have valid intrinsic coordinates (0-1)
        for proj in &projected {
            assert!(
                proj.intrinsic.is_some(),
                "Intrinsic coordinate should be present"
            );
            let intr = proj.intrinsic.unwrap();
            assert!(
                (0.0..=1.0).contains(&intr),
                "Intrinsic coordinate should be between 0 and 1"
            );
        }
    }

    // T090: Coordinates between segments assigned to nearest segment
    #[test]
    fn test_project_coordinates_between_segments() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{AssociatedNetElement, GnssPosition, Netelement, TrainPath};
        use tp_lib_core::{project_onto_path, PathConfig};

        // Create network segments
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.360, 50.860)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.360, 50.860), (4.370, 50.870)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        // Create path
        let segments = vec![
            AssociatedNetElement::new("NE_A".to_string(), 1.0, 0.0, 1.0, 0, 10).unwrap(),
            AssociatedNetElement::new("NE_B".to_string(), 1.0, 0.0, 1.0, 11, 20).unwrap(),
        ];

        let path = TrainPath::new(segments, 0.90, Some(Utc::now()), None).unwrap();

        // Create GNSS position between two segments
        let gnss_positions = vec![
            // Position near junction between NE_A and NE_B
            GnssPosition::new(50.860, 4.360, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        let config = PathConfig::default();
        let result = project_onto_path(&gnss_positions, &path, &netelements, &config);

        // Verify position is assigned to one of the nearest segments
        assert!(result.is_ok(), "Projection should succeed");
        let projected = result.unwrap();
        assert_eq!(projected.len(), 1);

        // Should be assigned to either NE_A or NE_B
        assert!(projected[0].netelement_id == "NE_A" || projected[0].netelement_id == "NE_B");
    }

    // T091: Pre-supplied path skips calculation
    #[test]
    fn test_project_with_presupplied_path() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{AssociatedNetElement, GnssPosition, Netelement, TrainPath};
        use tp_lib_core::{project_onto_path, PathConfig};

        // Network
        let netelements = vec![Netelement::new(
            "NE_A".to_string(),
            LineString::from(vec![(4.350, 50.850), (4.360, 50.860)]),
            "EPSG:4326".to_string(),
        )
        .unwrap()];

        // Pre-calculated path
        let segments =
            vec![AssociatedNetElement::new("NE_A".to_string(), 1.0, 0.0, 1.0, 0, 10).unwrap()];

        let path = TrainPath::new(segments, 0.85, Some(Utc::now()), None).unwrap();

        // GNSS positions
        let gnss_positions =
            vec![
                GnssPosition::new(50.8551, 4.3551, Utc::now().into(), "EPSG:4326".to_string())
                    .unwrap(),
            ];

        // Project without needing to calculate path
        let config = PathConfig::default();
        let result = project_onto_path(&gnss_positions, &path, &netelements, &config);

        // Should succeed using pre-supplied path
        assert!(
            result.is_ok(),
            "Projection with pre-supplied path should succeed"
        );
        let projected = result.unwrap();
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].netelement_id, "NE_A");
    }

    // US3: Path-only export tests (T103-T106)

    // T103: Path-only export in CSV format
    #[test]
    fn test_path_only_export_csv() {
        use chrono::Utc;
        use tp_lib_core::io::{parse_trainpath_csv, write_trainpath_csv};
        use tp_lib_core::{AssociatedNetElement, TrainPath};

        let segments = vec![
            AssociatedNetElement::new("NE_A".to_string(), 0.9, 0.0, 1.0, 0, 5).unwrap(),
            AssociatedNetElement::new("NE_B".to_string(), 0.8, 0.0, 1.0, 6, 12).unwrap(),
        ];

        let path = TrainPath::new(segments, 0.85, Some(Utc::now()), None).unwrap();

        // Use unique temp file name to avoid race conditions with parallel tests
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let csv_temp = std::env::temp_dir().join(format!("test_path_only_{}.csv", unique_id));
        let mut csv_file = std::fs::File::create(&csv_temp).unwrap();
        write_trainpath_csv(&path, &mut csv_file).unwrap();
        drop(csv_file);

        let parsed = parse_trainpath_csv(csv_temp.to_str().unwrap()).unwrap();
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(parsed.segments[0].netelement_id, "NE_A");
        assert_eq!(parsed.segments[1].netelement_id, "NE_B");

        std::fs::remove_file(&csv_temp).unwrap();
    }

    // T104/T105: Path-only export in GeoJSON with segment sequence
    #[test]
    fn test_path_only_export_geojson() {
        use chrono::Utc;
        use geo::LineString;
        use std::collections::HashMap;
        use tp_lib_core::io::write_trainpath_geojson;
        use tp_lib_core::{AssociatedNetElement, Netelement, TrainPath};

        let segments = vec![
            AssociatedNetElement::new("NE_A".to_string(), 0.9, 0.0, 1.0, 0, 5).unwrap(),
            AssociatedNetElement::new("NE_B".to_string(), 0.8, 0.0, 1.0, 6, 12).unwrap(),
        ];
        let path = TrainPath::new(segments, 0.85, Some(Utc::now()), None).unwrap();

        let mut netelements_map = HashMap::new();
        netelements_map.insert(
            "NE_A".to_string(),
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        );
        netelements_map.insert(
            "NE_B".to_string(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.36, 50.86), (4.37, 50.87)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        );

        let geojson_temp = std::env::temp_dir().join("test_path_only.geojson");
        let mut geojson_file = std::fs::File::create(&geojson_temp).unwrap();
        write_trainpath_geojson(&path, &netelements_map, &mut geojson_file).unwrap();
        drop(geojson_file);

        let content = std::fs::read_to_string(&geojson_temp).unwrap();
        assert!(content.contains("\"overall_probability\""));
        assert!(content.contains("NE_A"));
        assert!(content.contains("NE_B"));

        std::fs::remove_file(&geojson_temp).unwrap();
    }

    // US4: Enhanced GNSS Data with Heading and Distance (T112-T123)

    // T112: Heading-enhanced path calculation (placeholder test)
    #[test]
    fn test_heading_enhanced_path_calculation() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, Netelement};
        use tp_lib_core::{calculate_train_path, PathConfig};

        // Create network with two parallel segments
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.360, 50.860)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B_parallel".to_string(),
                LineString::from(vec![(4.350, 50.851), (4.360, 50.861)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        // GNSS positions with heading information (aligned with NE_A)
        let gnss_positions = vec![
            GnssPosition::with_heading_distance(
                50.8503,
                4.3502,
                Utc::now().into(),
                "EPSG:4326".to_string(),
                Some(45.0), // heading northeast, aligns with NE_A
                None,
            )
            .unwrap(),
            GnssPosition::with_heading_distance(
                50.8512,
                4.3512,
                Utc::now().into(),
                "EPSG:4326".to_string(),
                Some(47.0), // similar heading
                None,
            )
            .unwrap(),
        ];

        let netrelations = vec![];
        let config = PathConfig::default();

        // When path calculation is implemented, should prefer NE_A based on heading
        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);

        // For now, just verify no panic with heading data
        let _ = result;
    }

    // T114: GNSS data without heading/distance still works (backward compatibility)
    #[test]
    fn test_backward_compatibility_no_heading_distance() {
        use chrono::Utc;
        use tp_lib_core::models::GnssPosition;

        // Positions created without heading/distance should still work
        let position =
            GnssPosition::new(50.8503, 4.3517, Utc::now().into(), "EPSG:4326".to_string()).unwrap();

        assert!(
            position.heading.is_none(),
            "Heading should be None by default"
        );
        assert!(
            position.distance.is_none(),
            "Distance should be None by default"
        );
    }

    // T114b: CSV parsing with optional heading and distance columns
    #[test]
    fn test_csv_parsing_with_heading_distance() {
        use tp_lib_core::io::parse_gnss_csv;

        // Write test CSV with heading and distance columns
        let csv_content = r#"timestamp,latitude,longitude,crs,heading,distance
2026-01-09T10:00:00+01:00,50.8503,4.3517,EPSG:4326,45.3,
2026-01-09T10:00:01+01:00,50.8504,4.3518,EPSG:4326,47.1,12.5
2026-01-09T10:00:02+01:00,50.8505,4.3519,EPSG:4326,46.8,11.9"#;

        let temp_file = std::env::temp_dir().join("test_heading_distance.csv");
        std::fs::write(&temp_file, csv_content).unwrap();

        // Parse CSV
        let positions = parse_gnss_csv(
            temp_file.to_str().unwrap(),
            "EPSG:4326",
            "latitude",
            "longitude",
            "timestamp",
        )
        .unwrap();

        // Verify heading and distance were parsed
        assert_eq!(positions.len(), 3);

        // First position: heading=45.3, no distance
        assert_eq!(positions[0].heading, Some(45.3));
        assert!(positions[0].distance.is_none());

        // Second position: heading=47.1, distance=12.5
        assert_eq!(positions[1].heading, Some(47.1));
        assert_eq!(positions[1].distance, Some(12.5));

        // Third position: heading=46.8, distance=11.9
        assert_eq!(positions[2].heading, Some(46.8));
        assert_eq!(positions[2].distance, Some(11.9));

        std::fs::remove_file(&temp_file).unwrap();
    }

    // T114c: GeoJSON parsing with optional heading and distance properties
    #[test]
    fn test_geojson_parsing_with_heading_distance() {
        use tp_lib_core::io::parse_gnss_geojson;

        let geojson_content = r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {"type": "Point", "coordinates": [4.3517, 50.8503]},
      "properties": {
        "timestamp": "2026-01-09T10:00:00+01:00",
        "heading": 45.3
      }
    },
    {
      "type": "Feature",
      "geometry": {"type": "Point", "coordinates": [4.3518, 50.8504]},
      "properties": {
        "timestamp": "2026-01-09T10:00:01+01:00",
        "heading": 47.1,
        "distance": 12.5
      }
    }
  ]
}"#;

        let temp_file = std::env::temp_dir().join("test_heading_distance.geojson");
        std::fs::write(&temp_file, geojson_content).unwrap();

        // Parse GeoJSON
        let positions = parse_gnss_geojson(temp_file.to_str().unwrap(), "EPSG:4326").unwrap();

        // Verify heading and distance were parsed
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].heading, Some(45.3));
        assert!(positions[0].distance.is_none());

        assert_eq!(positions[1].heading, Some(47.1));
        assert_eq!(positions[1].distance, Some(12.5));

        std::fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn test_path_only_failure_reports_warnings() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathConfig};

        let gnss_positions =
            vec![
                GnssPosition::new(50.8503, 4.3517, Utc::now().into(), "EPSG:4326".to_string())
                    .unwrap(),
            ];
        let netelements = vec![Netelement::new(
            "NE_X".to_string(),
            LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
            "EPSG:4326".to_string(),
        )
        .unwrap()];
        let netrelations: Vec<NetRelation> = vec![]; // No connections

        let config = PathConfig::builder().path_only(true).build().unwrap();
        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);
        assert!(
            result.is_ok(),
            "Path-only mode should return Ok with warnings"
        );

        let path_result = result.unwrap();
        assert!(
            path_result.path.is_none(),
            "No path calculated in placeholder implementation"
        );
        assert!(
            path_result.projected_positions.is_empty(),
            "Projected positions must be empty in path-only mode"
        );
        assert!(
            !path_result.warnings.is_empty(),
            "Warnings should be populated when calculation is not implemented"
        );
    }

    // T113: Integration test for distance-based spacing calculation
    #[test]
    fn test_calculate_mean_spacing_with_distance() {
        use chrono::Utc;
        use tp_lib_core::calculate_mean_spacing;
        use tp_lib_core::models::GnssPosition;

        // Create GNSS positions with distance column values
        let mut pos1 =
            GnssPosition::new(50.850, 4.350, Utc::now().into(), "EPSG:4326".to_string()).unwrap();
        pos1.distance = Some(0.0); // First position at distance 0

        let mut pos2 =
            GnssPosition::new(50.851, 4.351, Utc::now().into(), "EPSG:4326".to_string()).unwrap();
        pos2.distance = Some(15.0); // Second position 15m from first

        let mut pos3 =
            GnssPosition::new(50.852, 4.352, Utc::now().into(), "EPSG:4326".to_string()).unwrap();
        pos3.distance = Some(30.0); // Third position 15m from second

        let mut pos4 =
            GnssPosition::new(50.853, 4.353, Utc::now().into(), "EPSG:4326".to_string()).unwrap();
        pos4.distance = Some(45.0); // Fourth position 15m from third

        let positions = vec![pos1, pos2, pos3, pos4];

        // Calculate mean spacing
        let mean_spacing = calculate_mean_spacing(&positions);

        // Expected: (15 + 15 + 15) / 3 = 15.0
        assert!(
            (mean_spacing - 15.0).abs() < 0.1,
            "Mean spacing should be 15.0m, got {}",
            mean_spacing
        );
    }

    // T124: Integration test for resampling with 10m interval on 1m-spaced data
    #[test]
    fn test_resampling_reduces_computation() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathConfig};

        // Create 100 GNSS positions at approximately 1m spacing along a linear path
        let gnss_positions: Vec<GnssPosition> = (0..100)
            .map(|i| {
                let lat = 50.850 + i as f64 * 0.000009; // ~1m spacing
                let lon = 4.350 + i as f64 * 0.000012;
                let mut pos =
                    GnssPosition::new(lat, lon, Utc::now().into(), "EPSG:4326".to_string())
                        .unwrap();
                pos.distance = Some(i as f64); // 1m cumulative distance
                pos
            })
            .collect();

        // Create simple linear network: NE_A -> NE_B -> NE_C
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.353, 50.853)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.353, 50.853), (4.356, 50.856)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_C".to_string(),
                LineString::from(vec![(4.356, 50.856), (4.360, 50.860)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        let netrelations = vec![
            NetRelation::new(
                "NR_AB".to_string(),
                "NE_A".to_string(),
                "NE_B".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
            NetRelation::new(
                "NR_BC".to_string(),
                "NE_B".to_string(),
                "NE_C".to_string(),
                1,
                0,
                true,
                true,
            )
            .unwrap(),
        ];

        // Test WITHOUT resampling
        let config_no_resample = PathConfig::builder()
            .resampling_distance(None)
            .build()
            .unwrap();

        let result_no_resample = calculate_train_path(
            &gnss_positions,
            &netelements,
            &netrelations,
            &config_no_resample,
        );
        assert!(
            result_no_resample.is_ok(),
            "Path calculation without resampling should succeed"
        );
        assert!(
            result_no_resample.as_ref().unwrap().warnings.is_empty()
                || !result_no_resample
                    .as_ref()
                    .unwrap()
                    .warnings
                    .iter()
                    .any(|w| w.contains("Resampling")),
            "Should not have resampling warning"
        );

        // Test WITH 10m resampling
        let config_resample = PathConfig::builder()
            .resampling_distance(Some(10.0))
            .build()
            .unwrap();

        let result_resample = calculate_train_path(
            &gnss_positions,
            &netelements,
            &netrelations,
            &config_resample,
        );
        assert!(
            result_resample.is_ok(),
            "Path calculation with resampling should succeed"
        );

        let path_result = result_resample.unwrap();

        // Verify resampling was applied (check for warning message)
        assert!(
            path_result
                .warnings
                .iter()
                .any(|w| w.contains("Resampling applied")),
            "Should have resampling warning indicating it was applied"
        );

        // Verify a path was found
        assert!(
            path_result.path.is_some(),
            "Should calculate a valid path with resampling"
        );

        let path = path_result.path.unwrap();
        assert!(!path.segments.is_empty(), "Path should have segments");
    }

    // T125: Integration test verifying all original positions available for output despite resampling
    #[test]
    fn test_resampling_preserves_original_positions() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathConfig};

        // Create 50 GNSS positions at 1m spacing
        let gnss_positions: Vec<GnssPosition> = (0..50)
            .map(|i| {
                let lat = 50.850 + i as f64 * 0.000009;
                let lon = 4.350 + i as f64 * 0.000012;
                let mut pos =
                    GnssPosition::new(lat, lon, Utc::now().into(), "EPSG:4326".to_string())
                        .unwrap();
                pos.distance = Some(i as f64);
                pos
            })
            .collect();

        // Create simple network
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.355, 50.855)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.355, 50.855), (4.360, 50.860)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        let netrelations = vec![NetRelation::new(
            "NR_AB".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        // Calculate path with 5m resampling (should use ~10 positions for path calculation)
        let config = PathConfig::builder()
            .resampling_distance(Some(5.0))
            .build()
            .unwrap();

        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);
        assert!(
            result.is_ok(),
            "Path calculation with resampling should succeed"
        );

        let path_result = result.unwrap();

        // Verify resampling was applied
        assert!(
            path_result
                .warnings
                .iter()
                .any(|w| w.contains("Resampling applied")),
            "Resampling should be applied"
        );

        // Verify warning shows the reduction
        let resample_warning = path_result
            .warnings
            .iter()
            .find(|w| w.contains("Resampling applied"))
            .unwrap();

        // Should indicate fewer positions used for path calculation
        assert!(
            resample_warning.contains("of 50"),
            "Warning should mention original 50 positions: {}",
            resample_warning
        );

        // The original gnss_positions slice is still intact (not consumed by calculate_train_path)
        // This means all 50 positions can still be used for projection later
        assert_eq!(
            gnss_positions.len(),
            50,
            "Original positions should remain intact for projection phase"
        );

        // Path calculation succeeded with reduced dataset
        assert!(path_result.path.is_some(), "Path should be calculated");

        // Note: US2 project_onto_path will use ALL original positions later
        // For now, we just verify the positions are preserved
    }

    // T137: Integration test for fallback with disconnected network
    #[test]
    fn test_fallback_with_disconnected_network() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathCalculationMode, PathConfig};

        // Create GNSS positions far from any network segments
        let gnss_positions = vec![
            GnssPosition::new(50.950, 4.450, Utc::now().into(), "EPSG:4326".to_string()).unwrap(), // Far away
            GnssPosition::new(50.951, 4.451, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        // Create network segments far from GNSS positions
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        // Connected network, but GNSS is too far away
        let netrelations = vec![NetRelation::new(
            "NR_AB".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        // Use small cutoff distance so GNSS positions are beyond reach
        let config = PathConfig::builder()
            .cutoff_distance(100.0) // Only 100m cutoff - GNSS is ~10km away
            .build()
            .unwrap();

        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);

        assert!(
            result.is_ok(),
            "Should succeed with fallback even when GNSS far from network"
        );

        let path_result = result.unwrap();

        // Verify fallback mode was used
        assert!(
            matches!(path_result.mode, PathCalculationMode::FallbackIndependent),
            "Should use fallback mode when no candidates found, got {:?}",
            path_result.mode
        );

        // Verify no path was calculated
        assert!(
            path_result.path.is_none(),
            "No path should be calculated in fallback mode"
        );

        // Verify fallback warnings
        assert!(
            path_result
                .warnings
                .iter()
                .any(|w| w.contains("No continuous path")),
            "Should warn about no continuous path, got: {:?}",
            path_result.warnings
        );
        assert!(
            path_result
                .warnings
                .iter()
                .any(|w| w.contains("Falling back")),
            "Should warn about falling back, got: {:?}",
            path_result.warnings
        );
    }

    // T138: Integration test verifying fallback notification to user
    #[test]
    fn test_fallback_notification() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathConfig};

        // GNSS far from network to trigger fallback
        let gnss_positions =
            vec![
                GnssPosition::new(50.950, 4.450, Utc::now().into(), "EPSG:4326".to_string())
                    .unwrap(),
            ];

        let netelements = vec![Netelement::new(
            "NE_A".to_string(),
            LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
            "EPSG:4326".to_string(),
        )
        .unwrap()];

        let netrelations: Vec<NetRelation> = vec![];
        let config = PathConfig::builder()
            .cutoff_distance(100.0)
            .build()
            .unwrap();

        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);
        assert!(result.is_ok());

        let path_result = result.unwrap();

        // Verify clear warning messages are provided
        let warnings_text = path_result.warnings.join(" ");
        assert!(
            warnings_text.contains("No continuous path") || warnings_text.contains("no valid path"),
            "Should clearly state no path was found: {}",
            warnings_text
        );
        assert!(
            warnings_text.contains("Falling back") || warnings_text.contains("fallback"),
            "Should explicitly mention fallback: {}",
            warnings_text
        );
        assert!(
            warnings_text.contains("independent") || warnings_text.contains("nearest-segment"),
            "Should explain fallback uses independent projection: {}",
            warnings_text
        );
    }

    // T139: Integration test for fallback ignoring navigability constraints
    #[test]
    fn test_fallback_ignores_navigability() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathCalculationMode, PathConfig};

        // Create GNSS positions
        let gnss_positions = vec![
            GnssPosition::new(50.850, 4.350, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.851, 4.351, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        // Create network with unidirectional constraint
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        // Unidirectional relation (only navigable forward, not backward)
        let netrelations = vec![
            NetRelation::new(
                "NR_AB".to_string(),
                "NE_A".to_string(),
                "NE_B".to_string(),
                1,
                0,
                true,
                false,
            )
            .unwrap(), // navigable_forward=true, navigable_backward=false
        ];

        let config = PathConfig::default();
        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);

        // Path calculation will fail due to navigability constraints
        // Fallback should succeed and project to geometrically nearest regardless of navigability
        assert!(result.is_ok());

        let path_result = result.unwrap();

        // Fallback mode projects to geometrically nearest, ignoring navigability
        if matches!(path_result.mode, PathCalculationMode::FallbackIndependent) {
            // Fallback was triggered - verify projections exist
            assert!(
                !path_result.projected_positions.is_empty(),
                "Fallback should project all positions despite navigability constraints"
            );
        }
        // Note: If topology-based succeeds, that's also valid - test is about fallback behavior when it triggers
    }

    // ==========================================
    // User Story 7: Debug Path Calculation Tests
    // ==========================================

    // T149: Integration test for debug export of candidate paths
    #[test]
    fn test_debug_export_candidate_paths() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{calculate_train_path, PathConfig};

        // Create a simple network with two possible paths
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        let netrelations = vec![NetRelation::new(
            "NR_AB".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        let gnss_positions = vec![
            GnssPosition::new(50.850, 4.350, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.852, 4.352, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        // Enable debug mode
        let config = PathConfig::builder().debug_mode(true).build().unwrap();

        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);
        assert!(
            result.is_ok(),
            "Path calculation should succeed with debug mode"
        );

        let path_result = result.unwrap();

        // Debug info should be present when debug_mode is enabled (T157 implemented)
        assert!(
            path_result.debug_info.is_some(),
            "Debug info should be populated when debug_mode=true"
        );

        let debug_info = path_result.debug_info.as_ref().unwrap();

        // Verify debug info contains position candidates (T157)
        assert!(
            !debug_info.position_candidates.is_empty(),
            "Debug info should contain position candidates"
        );

        // Verify debug info contains decisions (T157)
        assert!(
            !debug_info.decision_tree.is_empty(),
            "Debug info should contain decision tree entries"
        );

        // Check that we can export to JSON
        let json_result = debug_info.to_json();
        assert!(json_result.is_ok(), "Debug info should serialize to JSON");

        let json = json_result.unwrap();
        assert!(
            json.contains("position_candidates"),
            "JSON should contain position_candidates"
        );
        assert!(
            json.contains("decision_tree"),
            "JSON should contain decision_tree"
        );
    }

    // T150: Integration test for debug export showing track segment candidates per coordinate
    #[test]
    fn test_debug_export_position_candidates() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{
            calculate_train_path, CandidateInfo, PathConfig, PositionCandidates,
        };

        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        let netrelations = vec![NetRelation::new(
            "NR_AB".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        let gnss_positions = vec![
            GnssPosition::new(50.850, 4.350, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.8505, 4.3505, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.851, 4.351, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        let config = PathConfig::builder().debug_mode(true).build().unwrap();

        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);
        assert!(result.is_ok());

        // Test that PositionCandidates struct can be created and used
        let test_candidates = PositionCandidates {
            position_index: 0,
            timestamp: "2025-01-09T12:00:00Z".to_string(),
            coordinates: (50.850, 4.350),
            candidates: vec![CandidateInfo {
                netelement_id: "NE_A".to_string(),
                distance: 5.0,
                heading_difference: Some(2.0),
                distance_probability: 0.9,
                heading_probability: Some(0.8),
                combined_probability: 0.72,
                status: "selected".to_string(),
            }],
            selected_netelement: Some("NE_A".to_string()),
        };

        // Verify serialization works
        let json = serde_json::to_string(&test_candidates).unwrap();
        assert!(json.contains("NE_A"));
        assert!(json.contains("position_index"));
    }

    // T151: Integration test for debug export showing forward/backward probability averaging
    #[test]
    fn test_debug_export_decision_tree() {
        use chrono::Utc;
        use geo::LineString;
        use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
        use tp_lib_core::{
            calculate_train_path, CandidatePath, DebugInfo, PathConfig, PathDecision,
        };

        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            )
            .unwrap(),
        ];

        let netrelations = vec![NetRelation::new(
            "NR_AB".to_string(),
            "NE_A".to_string(),
            "NE_B".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        let gnss_positions = vec![
            GnssPosition::new(50.850, 4.350, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.852, 4.352, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];

        let config = PathConfig::builder().debug_mode(true).build().unwrap();

        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);
        assert!(result.is_ok());

        // Test that PathDecision and CandidatePath structs work correctly
        let test_decision = PathDecision {
            step: 1,
            decision_type: "forward_extend".to_string(),
            current_segment: "NE_A".to_string(),
            options: vec!["NE_B".to_string()],
            option_probabilities: vec![0.85],
            chosen_option: "NE_B".to_string(),
            reason: "Highest probability candidate".to_string(),
        };

        let test_candidate_path = CandidatePath {
            id: "forward_1".to_string(),
            direction: "forward".to_string(),
            segment_ids: vec!["NE_A".to_string(), "NE_B".to_string()],
            probability: 0.85,
            selected: true,
        };

        // Verify DebugInfo can hold all these types
        let mut debug_info = DebugInfo::new();
        debug_info.add_decision(test_decision);
        debug_info.add_candidate_path(test_candidate_path);

        assert!(!debug_info.is_empty());
        assert_eq!(debug_info.decision_tree.len(), 1);
        assert_eq!(debug_info.candidate_paths.len(), 1);

        // Verify JSON export
        let json = debug_info.to_json().unwrap();
        assert!(json.contains("forward_extend"));
        assert!(json.contains("segment_ids"));
    }
}
