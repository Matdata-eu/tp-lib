use tp_lib_core::io::{parse_gnss_csv, parse_network_geojson, write_csv, write_geojson};
use tp_lib_core::{project_gnss, ProjectionConfig, RailwayNetwork};

#[test]
fn test_csv_to_projection_pipeline() {
    // Load test network (2 netelements)
    let network_path = "tests/fixtures/test_network.geojson";
    let (netelements, _netrelations) =
        parse_network_geojson(network_path).expect("Failed to load network");
    let network = RailwayNetwork::new(netelements).expect("Failed to create network index");

    // Load test GNSS data (3 positions)
    let gnss_path = "tests/fixtures/test_gnss.csv";
    let gnss_positions =
        parse_gnss_csv(gnss_path, "EPSG:4326", "latitude", "longitude", "timestamp")
            .expect("Failed to load GNSS data");

    // Project positions
    let config = ProjectionConfig::default();
    let projected = project_gnss(&gnss_positions, &network, &config).expect("Failed to project");

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
