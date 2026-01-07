// Test runner to include unit tests

mod contract;
mod unit;

#[cfg(test)]
mod integration {
    use tp_core::io::{parse_gnss_csv, parse_network_geojson};

    #[test]
    fn test_parse_gnss_csv() {
        let path = "tests/fixtures/test_gnss.csv";
        let result = parse_gnss_csv(path, "EPSG:4326", "latitude", "longitude", "timestamp");

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

        assert!(
            result.is_ok(),
            "Failed to parse GeoJSON: {:?}",
            result.err()
        );
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
        use tp_core::{project_gnss, ProjectionConfig, RailwayNetwork};

        // Load network
        let network_path = "tests/fixtures/test_network.geojson";
        let netelements = parse_network_geojson(network_path).unwrap();
        let network = RailwayNetwork::new(netelements).unwrap();

        // Load GNSS positions
        let gnss_path = "tests/fixtures/test_gnss.csv";
        let gnss_positions =
            parse_gnss_csv(gnss_path, "EPSG:4326", "latitude", "longitude", "timestamp").unwrap();

        // Project
        let config = ProjectionConfig::default();
        let projected = project_gnss(&gnss_positions, &network, &config).unwrap();

        // Verify results
        assert_eq!(projected.len(), 3, "Expected 3 projected positions");

        // Verify all positions have valid netelement IDs
        for pos in &projected {
            assert!(
                !pos.netelement_id.is_empty(),
                "Netelement ID should not be empty"
            );
            assert!(pos.measure_meters >= 0.0, "Measure should be non-negative");
        }

        // Verify temporal ordering preserved
        for i in 1..projected.len() {
            assert!(
                projected[i].original.timestamp >= projected[i - 1].original.timestamp,
                "Temporal ordering should be preserved"
            );
        }
    }

    #[test]
    fn test_write_csv_output() {
        use tp_core::io::write_csv;
        use tp_core::{project_gnss, ProjectionConfig, RailwayNetwork};

        // Load and project
        let network = RailwayNetwork::new(
            parse_network_geojson("tests/fixtures/test_network.geojson").unwrap(),
        )
        .unwrap();
        let gnss_positions = parse_gnss_csv(
            "tests/fixtures/test_gnss.csv",
            "EPSG:4326",
            "latitude",
            "longitude",
            "timestamp",
        )
        .unwrap();
        let projected =
            project_gnss(&gnss_positions, &network, &ProjectionConfig::default()).unwrap();

        // Write to CSV
        let mut output = Vec::new();
        write_csv(&projected, &mut output).unwrap();

        let csv_string = String::from_utf8(output).unwrap();

        // Verify header
        assert!(
            csv_string.contains("original_lat"),
            "CSV should contain header"
        );
        assert!(
            csv_string.contains("netelement_id"),
            "CSV should contain netelement_id column"
        );
        assert!(
            csv_string.contains("measure_meters"),
            "CSV should contain measure_meters column"
        );

        // Verify data rows (3 positions + 1 header = 4 lines)
        let lines: Vec<&str> = csv_string.lines().collect();
        assert_eq!(lines.len(), 4, "Expected header + 3 data rows");
    }

    #[test]
    fn test_write_geojson_output() {
        use tp_core::io::write_geojson;
        use tp_core::{project_gnss, ProjectionConfig, RailwayNetwork};

        // Load and project
        let network = RailwayNetwork::new(
            parse_network_geojson("tests/fixtures/test_network.geojson").unwrap(),
        )
        .unwrap();
        let gnss_positions = parse_gnss_csv(
            "tests/fixtures/test_gnss.csv",
            "EPSG:4326",
            "latitude",
            "longitude",
            "timestamp",
        )
        .unwrap();
        let projected =
            project_gnss(&gnss_positions, &network, &ProjectionConfig::default()).unwrap();

        // Write to GeoJSON
        let mut output = Vec::new();
        write_geojson(&projected, &mut output).unwrap();

        let geojson_string = String::from_utf8(output).unwrap();

        // Verify structure
        assert!(
            geojson_string.contains("\"type\": \"FeatureCollection\""),
            "Should be FeatureCollection"
        );
        assert!(
            geojson_string.contains("\"type\": \"Point\""),
            "Should contain Point geometries"
        );
        assert!(
            geojson_string.contains("\"netelement_id\""),
            "Should have netelement_id property"
        );
        assert!(
            geojson_string.contains("\"measure_meters\""),
            "Should have measure_meters property"
        );

        // Parse to verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&geojson_string).unwrap();
        assert_eq!(parsed["type"], "FeatureCollection");
        assert_eq!(parsed["features"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_csv_to_projection_pipeline() {
        use tp_core::{
            parse_gnss_csv, parse_network_geojson, project_gnss, write_csv, write_geojson,
            ProjectionConfig, RailwayNetwork,
        };

        // Load test network (2 netelements)
        let network_path = "tests/fixtures/test_network.geojson";
        let netelements = parse_network_geojson(network_path).expect("Failed to load network");
        let network = RailwayNetwork::new(netelements).expect("Failed to create network index");

        // Load test GNSS data (3 positions)
        let gnss_path = "tests/fixtures/test_gnss.csv";
        let gnss_positions =
            parse_gnss_csv(gnss_path, "EPSG:4326", "latitude", "longitude", "timestamp")
                .expect("Failed to load GNSS data");

        // Project positions
        let config = ProjectionConfig::default();
        let projected =
            project_gnss(&gnss_positions, &network, &config).expect("Failed to project positions");

        // Verify output count equals input count
        assert_eq!(
            projected.len(),
            gnss_positions.len(),
            "Output count should match input count"
        );
        assert_eq!(projected.len(), 3, "Should have 3 projected positions");

        // Verify all required fields are present
        for pos in &projected {
            assert!(
                !pos.netelement_id.is_empty(),
                "netelement_id should not be empty"
            );
            assert!(
                pos.measure_meters >= 0.0,
                "measure_meters should be non-negative"
            );
            assert!(
                pos.projection_distance_meters >= 0.0,
                "projection_distance should be non-negative"
            );
            assert_eq!(pos.crs, "EPSG:4326", "CRS should be EPSG:4326");
        }

        // Verify CSV output can be written
        let mut csv_buffer = Vec::new();
        write_csv(&projected, &mut csv_buffer).expect("Failed to write CSV");
        let csv_output = String::from_utf8(csv_buffer).expect("Invalid UTF-8");

        // Verify CSV has header and 3 data rows
        let lines: Vec<&str> = csv_output.lines().collect();
        assert_eq!(lines.len(), 4, "CSV should have 1 header + 3 data rows");
        assert!(lines[0].contains("original_lat"), "CSV should have header");

        // Verify GeoJSON output can be written
        let mut json_buffer = Vec::new();
        write_geojson(&projected, &mut json_buffer).expect("Failed to write GeoJSON");
        let json_output = String::from_utf8(json_buffer).expect("Invalid UTF-8");

        // Verify GeoJSON is valid and has 3 features
        assert!(
            json_output.contains("FeatureCollection"),
            "Should be a FeatureCollection"
        );
        let feature_count = json_output.matches("\"type\": \"Feature\"").count();
        assert_eq!(feature_count, 3, "Should have 3 features");
    }
}
