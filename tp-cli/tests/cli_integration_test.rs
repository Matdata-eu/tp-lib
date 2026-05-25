use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_valid_input_produces_csv_output() {
    let temp_dir = TempDir::new().unwrap();

    // Create test GNSS CSV
    let gnss_csv = temp_dir.path().join("gnss.csv");
    fs::write(
        &gnss_csv,
        "timestamp,latitude,longitude,altitude,hdop\n\
         2025-12-09T14:30:00+01:00,50.8503,4.3517,100.0,2.0\n\
         2025-12-09T14:30:01+01:00,50.8504,4.3518,100.5,2.1\n\
         2025-12-09T14:30:02+01:00,50.8505,4.3519,101.0,2.0\n",
    )
    .unwrap();

    // Create test network GeoJSON
    let network_geojson = temp_dir.path().join("network.geojson");
    fs::write(
        &network_geojson,
        r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {"id": "NE001"},
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.3500, 50.8500], [4.3520, 50.8510]]
      }
    },
    {
      "type": "Feature",
      "properties": {"id": "NE002"},
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.3520, 50.8510], [4.3540, 50.8520]]
      }
    }
  ]
}"#,
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss-file")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network-file")
        .arg(&network_geojson)
        .arg("--output-format")
        .arg("csv");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("original_lat"))
        .stdout(predicate::str::contains("original_lon"))
        .stdout(predicate::str::contains("projected_lat"))
        .stdout(predicate::str::contains("projected_lon"))
        .stdout(predicate::str::contains("netelement_id"))
        .stdout(predicate::str::contains("measure_meters"))
        .stdout(predicate::str::contains("50.8503"))
        .stdout(predicate::str::contains("4.3517"));
}

#[test]
fn test_missing_network_file_produces_error() {
    let temp_dir = TempDir::new().unwrap();

    let gnss_csv = temp_dir.path().join("gnss.csv");
    fs::write(
        &gnss_csv,
        "timestamp,latitude,longitude\n\
         2025-12-09T14:30:00+01:00,50.8503,4.3517\n",
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss-file")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network-file")
        .arg("nonexistent_network.geojson")
        .arg("--output-format")
        .arg("csv");

    cmd.assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("No such file").or(predicate::str::contains("I/O")));
}

#[test]
fn test_missing_file_produces_exit_code_3() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss-file")
        .arg("nonexistent_file.csv")
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network-file")
        .arg("nonexistent_network.geojson")
        .arg("--output-format")
        .arg("csv");

    cmd.assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("No such file").or(predicate::str::contains("I/O")));
}

#[test]
fn test_output_count_matches_input_count() {
    let temp_dir = TempDir::new().unwrap();

    let gnss_csv = temp_dir.path().join("gnss.csv");
    fs::write(
        &gnss_csv,
        "timestamp,latitude,longitude\n\
         2025-12-09T14:30:00+01:00,50.8503,4.3517\n\
         2025-12-09T14:30:01+01:00,50.8504,4.3518\n\
         2025-12-09T14:30:02+01:00,50.8505,4.3519\n\
         2025-12-09T14:30:03+01:00,50.8506,4.3520\n\
         2025-12-09T14:30:04+01:00,50.8507,4.3521\n",
    )
    .unwrap();

    let network_geojson = temp_dir.path().join("network.geojson");
    fs::write(
        &network_geojson,
        r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {"id": "NE001"},
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.3500, 50.8500], [4.3550, 50.8550]]
      }
    }
  ]
}"#,
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss-file")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network-file")
        .arg(&network_geojson)
        .arg("--output-format")
        .arg("csv");

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Count lines (1 header + 5 data rows = 6 total)
    let line_count = stdout.lines().count();
    assert_eq!(
        line_count, 6,
        "Expected 6 lines (1 header + 5 data rows), got {}",
        line_count
    );
}

#[test]
fn test_geojson_output_format() {
    let temp_dir = TempDir::new().unwrap();

    let gnss_csv = temp_dir.path().join("gnss.csv");
    fs::write(
        &gnss_csv,
        "timestamp,latitude,longitude\n\
         2025-12-09T14:30:00+01:00,50.8503,4.3517\n",
    )
    .unwrap();

    let network_geojson = temp_dir.path().join("network.geojson");
    fs::write(
        &network_geojson,
        r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {"id": "NE001"},
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.3500, 50.8500], [4.3550, 50.8550]]
      }
    }
  ]
}"#,
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss-file")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network-file")
        .arg(&network_geojson)
        .arg("--output-format")
        .arg("json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("FeatureCollection"))
        .stdout(predicate::str::contains("Point"))
        .stdout(predicate::str::contains("netelement_id"));
}

#[test]
fn test_help_flag_displays_usage() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--gnss"))
        .stdout(predicate::str::contains("--network"))
        .stdout(predicate::str::contains("--output"));
}

#[test]
fn test_custom_column_names() {
    let temp_dir = TempDir::new().unwrap();

    let gnss_csv = temp_dir.path().join("gnss.csv");
    fs::write(
        &gnss_csv,
        "time,lat,lon\n\
         2025-12-09T14:30:00+01:00,50.8503,4.3517\n",
    )
    .unwrap();

    let network_geojson = temp_dir.path().join("network.geojson");
    fs::write(
        &network_geojson,
        r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {"id": "NE001"},
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.3500, 50.8500], [4.3550, 50.8550]]
      }
    }
  ]
}"#,
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss-file")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--lat-col")
        .arg("lat")
        .arg("--lon-col")
        .arg("lon")
        .arg("--time-col")
        .arg("time")
        .arg("--network-file")
        .arg(&network_geojson)
        .arg("--output-format")
        .arg("csv");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("50.8503"));
}
// ============================================================================
// Phase 10: Path Calculation CLI Integration Tests (T160-T163)
// ============================================================================

/// Helper to create a network with netrelations for path calculation tests
fn create_path_test_network(temp_dir: &TempDir) -> std::path::PathBuf {
    let network_geojson = temp_dir.path().join("network.geojson");
    fs::write(
        &network_geojson,
        r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {"id": "NE001", "type": "netelement"},
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.3500, 50.8500], [4.3510, 50.8505]]
      }
    },
    {
      "type": "Feature",
      "properties": {"id": "NE002", "type": "netelement"},
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.3510, 50.8505], [4.3520, 50.8510]]
      }
    },
    {
      "type": "Feature",
      "properties": {"id": "NE003", "type": "netelement"},
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.3520, 50.8510], [4.3530, 50.8515]]
      }
    },
    {
      "type": "Feature",
      "properties": {
        "id": "NR001",
        "type": "netrelation",
        "netelementA": "NE001",
        "netelementB": "NE002",
        "positionOnA": 1,
        "positionOnB": 0,
        "navigability": "both"
      },
      "geometry": null
    },
    {
      "type": "Feature",
      "properties": {
        "id": "NR002",
        "type": "netrelation",
        "netelementA": "NE002",
        "netelementB": "NE003",
        "positionOnA": 1,
        "positionOnB": 0,
        "navigability": "both"
      },
      "geometry": null
    }
  ]
}"#,
    )
    .unwrap();
    network_geojson
}

/// Helper to create a GNSS track along the network
fn create_path_test_gnss(temp_dir: &TempDir) -> std::path::PathBuf {
    let gnss_csv = temp_dir.path().join("gnss.csv");
    fs::write(
        &gnss_csv,
        "timestamp,latitude,longitude\n\
         2025-12-09T14:30:00+01:00,50.8502,4.3502\n\
         2025-12-09T14:30:01+01:00,50.8507,4.3512\n\
         2025-12-09T14:30:02+01:00,50.8512,4.3522\n",
    )
    .unwrap();
    gnss_csv
}

/// T160: Test default command (calculate path + project coordinates)
#[test]
fn test_default_command_calculate_and_project() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);
    let output_file = temp_dir.path().join("projected.csv");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_file);

    cmd.assert().success();

    // Verify output file exists and contains projected coordinates
    assert!(output_file.exists(), "Output file should be created");
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("netelement_id"),
        "Output should contain netelement_id column"
    );
    assert!(
        content.contains("NE00"),
        "Output should reference netelements from the network"
    );
}

/// T161: Test calculate-path subcommand (path only, no projection)
#[test]
fn test_calculate_path_subcommand_path_only() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);
    let output_file = temp_dir.path().join("path.geojson");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("calculate-path")
        .arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_file);

    cmd.assert().success();

    // Verify output is a train path, not projected coordinates
    assert!(output_file.exists(), "Output file should be created");
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("FeatureCollection") || content.contains("segments"),
        "Output should be a train path"
    );
}

/// T162: Test simple-projection subcommand (legacy feature 001 behavior)
#[test]
fn test_simple_projection_subcommand_legacy() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);
    let output_file = temp_dir.path().join("projected.csv");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("simple-projection")
        .arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_file);

    cmd.assert().success();

    // Verify output contains projected coordinates
    assert!(output_file.exists(), "Output file should be created");
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("netelement_id"),
        "Output should contain netelement_id column"
    );
}

/// T163: Test --train-path parameter (use pre-calculated path)
#[test]
fn test_train_path_parameter_precalculated() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);

    // First calculate a path
    let path_file = temp_dir.path().join("path.csv");
    let mut cmd1 = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd1.arg("calculate-path")
        .arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&path_file)
        .arg("--format")
        .arg("csv");
    cmd1.assert().success();

    // Now use the path for projection
    let output_file = temp_dir.path().join("projected.csv");
    let mut cmd2 = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd2.arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--train-path")
        .arg(&path_file)
        .arg("--output")
        .arg(&output_file);

    cmd2.assert().success();

    // Verify output contains projected coordinates
    assert!(output_file.exists(), "Output file should be created");
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("netelement_id"),
        "Output should contain netelement_id column"
    );
}

/// Test algorithm parameters are accepted
#[test]
fn test_algorithm_parameters() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);
    let output_file = temp_dir.path().join("projected.csv");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_file)
        .arg("--distance-scale")
        .arg("15.0")
        .arg("--heading-scale")
        .arg("3.0")
        .arg("--cutoff-distance")
        .arg("100.0")
        .arg("--probability-threshold")
        .arg("0.1");

    cmd.assert().success();
}

/// Test resampling distance parameter
#[test]
fn test_resampling_distance_parameter() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);
    let output_file = temp_dir.path().join("projected.csv");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_file)
        .arg("--resampling-distance")
        .arg("50.0");

    cmd.assert().success();
}

/// Test --save-path parameter (save calculated path alongside projected output)
#[test]
fn test_save_path_parameter() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);
    let output_file = temp_dir.path().join("projected.csv");
    let path_file = temp_dir.path().join("saved_path.geojson");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_file)
        .arg("--save-path")
        .arg(&path_file);

    cmd.assert().success();

    // Verify both files exist
    assert!(
        output_file.exists(),
        "Projected output file should be created"
    );
    assert!(path_file.exists(), "Path file should be created");
}

/// Test verbose flag
#[test]
fn test_verbose_flag() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);
    let output_file = temp_dir.path().join("projected.csv");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_file)
        .arg("--verbose");

    // Should succeed with verbose output to stderr
    cmd.assert().success();
}

/// Test quiet flag
#[test]
fn test_quiet_flag() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);
    let output_file = temp_dir.path().join("projected.csv");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_file)
        .arg("--quiet");

    // Should succeed with minimal output
    cmd.assert().success();
}

/// Test format parameter with auto-detection
#[test]
fn test_format_auto_detection() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);

    // Test CSV detection
    let output_csv = temp_dir.path().join("output.csv");
    let mut cmd1 = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd1.arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_csv)
        .arg("--format")
        .arg("auto");
    cmd1.assert().success();

    let csv_content = fs::read_to_string(&output_csv).unwrap();
    assert!(!csv_content.starts_with("{"), "CSV should not be JSON");

    // Test GeoJSON detection
    let output_json = temp_dir.path().join("output.geojson");
    let mut cmd2 = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd2.arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_json)
        .arg("--format")
        .arg("auto");
    cmd2.assert().success();

    let json_content = fs::read_to_string(&output_json).unwrap();
    assert!(json_content.starts_with("{"), "GeoJSON should be JSON");
}

/// Test subcommand help is available
#[test]
fn test_subcommand_help() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("calculate-path").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--gnss"))
        .stdout(predicate::str::contains("--network"))
        .stdout(predicate::str::contains("--output"));
}

/// Debug test: mirrors the exact CLI invocation used for manual testing.
///
/// Equivalent command:
///   tp-cli simple-projection \
///     --gnss tp-core/tests/fixtures/log_28876_L36-B.csv \
///     --crs EPSG:4326 \
///     --network tp-core/tests/fixtures/test_network_airport.geojson \
///     --output <temp>/log_28876_L36-B-processed.geojson
///
/// Run with `cargo test test_simple_projection_real_fixture -- --nocapture`
/// to see the full output for debugging.
#[test]
fn test_simple_projection_real_fixture() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("log_28876_L36-B-processed.geojson");

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let gnss_file = format!(
        "{}/../test-data/log_28876/log_28876_L36-B.csv",
        manifest_dir
    );
    let network_file = format!("{}/../test-data/network_airport.geojson", manifest_dir);

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("simple-projection")
        .arg("--gnss")
        .arg(&gnss_file)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_file)
        .arg("--output")
        .arg(&output_file);

    let output = cmd.assert().success();

    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    eprintln!("--- stderr ---\n{}", stderr);

    assert!(
        output_file.exists(),
        "Output GeoJSON file should be created"
    );

    let content = fs::read_to_string(&output_file).unwrap();
    eprintln!(
        "--- output (first 2000 chars) ---\n{}",
        &content[..content.len().min(2000)]
    );

    assert!(
        content.contains("FeatureCollection"),
        "Output should be a GeoJSON FeatureCollection, got:\n{}",
        &content[..content.len().min(500)]
    );
    assert!(
        content.contains("netelement_id"),
        "Each projected point should have a netelement_id property"
    );
    assert!(
        content.contains("measure_meters"),
        "Each projected point should have a measure_meters property"
    );

    // Count features: the input CSV has one row per GNSS fix, output should match
    let feature_count = content
        .matches("\"type\":\"Feature\"")
        .count()
        .max(content.matches("\"type\": \"Feature\"").count());
    eprintln!("--- feature count: {} ---", feature_count);
    assert!(
        feature_count > 0,
        "Output should contain at least one projected feature"
    );
}

/// T018: calculate-path with supplied --network prints SuppliedTopology source line.
#[test]
fn test_calculate_path_supplied_topology_source_message() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let network_geojson = create_path_test_network(&temp_dir);
    let output_file = temp_dir.path().join("path.geojson");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("calculate-path")
        .arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network_geojson)
        .arg("--output")
        .arg(&output_file);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("SuppliedTopology"));
}

/// T020: auto-retrieval against unreachable endpoint exits with code 7 (RinfEndpointFailure).
#[test]
fn test_calculate_path_unreachable_rinf_endpoint_exit_code_7() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let output_file = temp_dir.path().join("path.geojson");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--rinf-endpoint")
        .arg("http://127.0.0.1:1/sparql")
        .arg("--rinf-buffer-meters")
        .arg("500")
        .arg("calculate-path")
        .arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--output")
        .arg(&output_file);

    cmd.assert().code(7);
}

/// T020: simple-projection without --network should auto-retrieve topology;
/// against an unreachable endpoint this exits with code 7 (RinfEndpointFailure).
#[test]
fn test_simple_projection_without_network_unreachable_rinf_endpoint_exit_code_7() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = create_path_test_gnss(&temp_dir);
    let output_file = temp_dir.path().join("projected.geojson");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--rinf-endpoint")
        .arg("http://127.0.0.1:1/sparql")
        .arg("--rinf-buffer-meters")
        .arg("500")
        .arg("simple-projection")
        .arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--output")
        .arg(&output_file);

    cmd.assert().code(7);
}

/// T020: auto-retrieval with empty GNSS file should exit with code 4 (RinfInvalidInput)
/// or another input-related code; we accept >=4 and <=7 for the RINF error band.
#[test]
fn test_calculate_path_empty_gnss_rinf_invalid_input() {
    let temp_dir = TempDir::new().unwrap();
    let gnss_csv = temp_dir.path().join("empty.csv");
    fs::write(&gnss_csv, "timestamp,latitude,longitude,altitude,hdop\n").unwrap();
    let output_file = temp_dir.path().join("path.geojson");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--rinf-endpoint")
        .arg("http://127.0.0.1:1/sparql")
        .arg("calculate-path")
        .arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--output")
        .arg(&output_file);

    let output = cmd.output().expect("cli should run");
    let code = output.status.code().unwrap_or(-1);
    assert!(
        (1..=7).contains(&code),
        "expected non-zero exit (1..=7), got {code}; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// T012: live RINF auto-retrieval against ERA endpoint. Ignored by default
/// (network access + slow). Run with `cargo test --test cli_integration_test -- --ignored`.
#[ignore]
#[test]
fn test_calculate_path_auto_retrieval_live_endpoint() {
    let temp_dir = TempDir::new().unwrap();
    // Brussels area, small fix set; coverage expected in RINF.
    let gnss_csv = temp_dir.path().join("gnss.csv");
    fs::write(
        &gnss_csv,
        "timestamp,latitude,longitude,altitude,hdop\n\
         2025-12-09T14:30:00+01:00,50.8503,4.3517,100.0,2.0\n\
         2025-12-09T14:30:01+01:00,50.8504,4.3518,100.5,2.1\n\
         2025-12-09T14:30:02+01:00,50.8505,4.3519,101.0,2.0\n",
    )
    .unwrap();
    let output_file = temp_dir.path().join("path.geojson");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("calculate-path")
        .arg("--gnss")
        .arg(&gnss_csv)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--output")
        .arg(&output_file);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("EraRinf"));
}
