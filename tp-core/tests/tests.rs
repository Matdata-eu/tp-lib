// Test runner to include unit tests

mod unit;

#[cfg(test)]
mod integration {
    use tp_core::io::{parse_gnss_csv, parse_network_geojson};

    #[test]
    fn test_parse_gnss_csv() {
        let path = "tests/fixtures/test_gnss.csv";
        let result = parse_gnss_csv(
            path,
            "EPSG:4326",
            "latitude",
            "longitude",
            "timestamp",
        );
        
        assert!(result.is_ok(), "Failed to parse CSV: {:?}", result.err());
        let positions = result.unwrap();
        assert_eq!(positions.len(), 3, "Expected 3 positions");
        
        // Check first position
        let first = &positions[0];
        assert!((first.latitude - 50.8503).abs() < 0.0001);
        assert!((first.longitude - 4.3517).abs() < 0.0001);
        assert_eq!(first.crs, "EPSG:4326");
        
        // Check metadata preserved
        assert!(first.metadata.contains_key("altitude"));
        assert!(first.metadata.contains_key("hdop"));
    }

    #[test]
    fn test_parse_network_geojson() {
        let path = "tests/fixtures/test_network.geojson";
        let result = parse_network_geojson(path);
        
        assert!(result.is_ok(), "Failed to parse GeoJSON: {:?}", result.err());
        let netelements = result.unwrap();
        assert_eq!(netelements.len(), 2, "Expected 2 netelements");
        
        // Check first netelement
        let first = &netelements[0];
        assert_eq!(first.id, "NE001");
        assert_eq!(first.crs, "EPSG:4326");
        assert_eq!(first.geometry.0.len(), 2, "Expected 2 points in linestring");
    }

    #[test]
    fn test_csv_invalid_column() {
        let path = "tests/fixtures/test_gnss.csv";
        let result = parse_gnss_csv(
            path,
            "EPSG:4326",
            "invalid_column",
            "longitude",
            "timestamp",
        );
        
        assert!(result.is_err(), "Expected error for invalid column");
    }
    #[test]
    fn test_end_to_end_projection() {
        use tp_core::{RailwayNetwork, project_gnss, ProjectionConfig};
        
        // Load network
        let network_path = "tests/fixtures/test_network.geojson";
        let netelements = parse_network_geojson(network_path).unwrap();
        let network = RailwayNetwork::new(netelements).unwrap();
        
        // Load GNSS positions
        let gnss_path = "tests/fixtures/test_gnss.csv";
        let gnss_positions = parse_gnss_csv(
            gnss_path,
            "EPSG:4326",
            "latitude",
            "longitude",
            "timestamp",
        ).unwrap();
        
        // Project
        let config = ProjectionConfig::default();
        let projected = project_gnss(&gnss_positions, &network, &config).unwrap();
        
        // Verify results
        assert_eq!(projected.len(), 3, "Expected 3 projected positions");
        
        // Verify all positions have valid netelement IDs
        for pos in &projected {
            assert!(!pos.netelement_id.is_empty(), "Netelement ID should not be empty");
            assert!(pos.measure_meters >= 0.0, "Measure should be non-negative");
        }
        
        // Verify temporal ordering preserved
        for i in 1..projected.len() {
            assert!(
                projected[i].original.timestamp >= projected[i-1].original.timestamp,
                "Temporal ordering should be preserved"
            );
        }
    }
}
