//! Integration tests for path calculation
//! 
//! End-to-end tests that validate complete workflows from GNSS data
//! and network topology to calculated paths and projected positions.

#[cfg(test)]
mod tests {
    use tp_lib_core::*;

    // US1: Path Calculation Tests (T039-T042)
    
    // T039: Simple linear path without junctions
    #[test]
    #[ignore] // Remove ignore after implementing calculate_train_path()
    fn test_successful_path_calculation_linear() {
        use tp_lib_core::models::{GnssPosition, Netelement, NetRelation};
        use tp_lib_core::{PathConfig, calculate_train_path};
        use geo::LineString;
        use chrono::Utc;
        
        // Create simple linear network: NE_A -> NE_B -> NE_C
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
            Netelement::new(
                "NE_C".to_string(),
                LineString::from(vec![(4.352, 50.852), (4.353, 50.853)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
        ];
        
        // Create netrelations connecting segments
        let netrelations = vec![
            NetRelation::new(
                "NR_AB".to_string(),
                "NE_A".to_string(),
                "NE_B".to_string(),
                1, 0, true, true,
            ).unwrap(),
            NetRelation::new(
                "NR_BC".to_string(),
                "NE_B".to_string(),
                "NE_C".to_string(),
                1, 0, true, true,
            ).unwrap(),
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
    #[ignore] // Remove ignore after implementing calculate_train_path()
    fn test_path_calculation_with_junction() {
        use tp_lib_core::models::{GnssPosition, Netelement, NetRelation};
        use tp_lib_core::{PathConfig, calculate_train_path};
        use geo::LineString;
        use chrono::Utc;
        
        // Create junction network: NE_A connects to NE_B1, NE_B2, NE_B3
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
            Netelement::new(
                "NE_B1".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
            Netelement::new(
                "NE_B2".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.350, 50.852)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
            Netelement::new(
                "NE_B3".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.351, 50.853)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
        ];
        
        // Create netrelations for junction
        let netrelations = vec![
            NetRelation::new("NR_AB1".to_string(), "NE_A".to_string(), "NE_B1".to_string(), 1, 0, true, true).unwrap(),
            NetRelation::new("NR_AB2".to_string(), "NE_A".to_string(), "NE_B2".to_string(), 1, 0, true, true).unwrap(),
            NetRelation::new("NR_AB3".to_string(), "NE_A".to_string(), "NE_B3".to_string(), 1, 0, true, true).unwrap(),
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
        assert!(path.segments.iter().any(|s| s.netelement_id == "NE_B1"), "Should select NE_B1 branch");
    }

    // T041: Heading filtering
    #[test]
    #[ignore] // Remove ignore after implementing calculate_train_path()
    fn test_heading_filtering() {
        use tp_lib_core::models::{GnssPosition, Netelement, NetRelation};
        use tp_lib_core::{PathConfig, calculate_train_path};
        use geo::LineString;
        use chrono::Utc;
        
        // Create network with segment at wrong heading
        let netelements = vec![
            Netelement::new(
                "NE_forward".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
            Netelement::new(
                "NE_backward".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.350, 50.850)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
        ];
        
        let netrelations = vec![
            NetRelation::new("NR_conn".to_string(), "NE_forward".to_string(), "NE_backward".to_string(), 1, 0, true, true).unwrap(),
        ];
        
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
            let forward_prob = path.segments.iter().find(|s| s.netelement_id == "NE_forward").map(|s| s.probability);
            let backward_prob = path.segments.iter().find(|s| s.netelement_id == "NE_backward").map(|s| s.probability);
            
            if let (Some(fw), Some(bw)) = (forward_prob, backward_prob) {
                assert!(fw > bw, "Forward segment should have higher probability than backward");
            }
        }
    }

    // T042: Select highest probability path
    #[test]
    #[ignore] // Remove ignore after implementing calculate_train_path()
    fn test_highest_probability_path_selection() {
        use tp_lib_core::models::{GnssPosition, Netelement, NetRelation};
        use tp_lib_core::{PathConfig, calculate_train_path};
        use geo::LineString;
        use chrono::Utc;
        
        // Create network with two possible paths
        let netelements = vec![
            Netelement::new(
                "NE_A".to_string(),
                LineString::from(vec![(4.350, 50.850), (4.351, 50.851)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
            // Path 1: close to GNSS positions
            Netelement::new(
                "NE_B_close".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.352, 50.852)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
            // Path 2: far from GNSS positions
            Netelement::new(
                "NE_B_far".to_string(),
                LineString::from(vec![(4.351, 50.851), (4.360, 50.860)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
        ];
        
        let netrelations = vec![
            NetRelation::new("NR_A_close".to_string(), "NE_A".to_string(), "NE_B_close".to_string(), 1, 0, true, true).unwrap(),
            NetRelation::new("NR_A_far".to_string(), "NE_A".to_string(), "NE_B_far".to_string(), 1, 0, true, true).unwrap(),
        ];
        
        // GNSS positions near the "close" path
        let gnss_positions = vec![
            GnssPosition::new(50.8503, 4.3503, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
            GnssPosition::new(50.8515, 4.3515, Utc::now().into(), "EPSG:4326".to_string()).unwrap(),
        ];
        
        let config = PathConfig::default();
        let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);
        
        assert!(result.is_ok());
        let path_result = result.unwrap();
        let path = path_result.path.unwrap();
        
        // Should select the closer path (NE_B_close) with higher probability
        assert!(path.segments.iter().any(|s| s.netelement_id == "NE_B_close"), "Should select closer path");
        assert!(path.overall_probability > 0.5, "Path should have reasonable probability");
    }

    // US2: Projection Tests (T089-T091)
    
    #[test]
    #[ignore] // Remove ignore after T089 implementation
    fn test_project_coordinates_on_path() {
        // T089: Project GNSS coordinates onto calculated path
    }

    // Additional integration tests to be added per user stories
    
    // T031: NetRelation GeoJSON parsing test
    #[test]
    fn test_netrelation_geojson_parsing() {
        use std::collections::HashMap;
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
        assert_eq!(netrelations.len(), 2, "Should parse 2 netrelations, skipping netelement");
        
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
        use tp_lib_core::{TrainPath, AssociatedNetElement, Netelement};
        use tp_lib_core::io::{write_trainpath_csv, parse_trainpath_csv, write_trainpath_geojson};
        use geo::LineString;
        use std::collections::HashMap;
        use chrono::Utc;
        
        // Create test TrainPath
        let segments = vec![
            AssociatedNetElement::new(
                "NE_A".to_string(),
                0.87,
                0.0,
                1.0,
                0,
                10,
            ).unwrap(),
            AssociatedNetElement::new(
                "NE_B".to_string(),
                0.92,
                0.0,
                1.0,
                11,
                18,
            ).unwrap(),
        ];
        
        let original_path = TrainPath::new(
            segments,
            0.89,
            Some(Utc::now()),
            None,
        ).unwrap();
        
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
            ).unwrap(),
        );
        netelements_map.insert(
            "NE_B".to_string(),
            Netelement::new(
                "NE_B".to_string(),
                LineString::from(vec![(4.36, 50.86), (4.37, 50.87)]),
                "EPSG:4326".to_string(),
            ).unwrap(),
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
}
