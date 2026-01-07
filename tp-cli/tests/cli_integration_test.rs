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
         2025-12-09T14:30:02+01:00,50.8505,4.3519,101.0,2.0\n"
    ).unwrap();
    
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
}"#
    ).unwrap();
    
    let mut cmd = Command::cargo_bin("tp-cli").unwrap();
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
         2025-12-09T14:30:00+01:00,50.8503,4.3517\n"
    ).unwrap();
    
    let mut cmd = Command::cargo_bin("tp-cli").unwrap();
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
    let mut cmd = Command::cargo_bin("tp-cli").unwrap();
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
         2025-12-09T14:30:04+01:00,50.8507,4.3521\n"
    ).unwrap();
    
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
}"#
    ).unwrap();
    
    let mut cmd = Command::cargo_bin("tp-cli").unwrap();
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
    assert_eq!(line_count, 6, "Expected 6 lines (1 header + 5 data rows), got {}", line_count);
}

#[test]
fn test_geojson_output_format() {
    let temp_dir = TempDir::new().unwrap();
    
    let gnss_csv = temp_dir.path().join("gnss.csv");
    fs::write(
        &gnss_csv,
        "timestamp,latitude,longitude\n\
         2025-12-09T14:30:00+01:00,50.8503,4.3517\n"
    ).unwrap();
    
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
}"#
    ).unwrap();
    
    let mut cmd = Command::cargo_bin("tp-cli").unwrap();
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
    let mut cmd = Command::cargo_bin("tp-cli").unwrap();
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--gnss-file"))
        .stdout(predicate::str::contains("--network-file"))
        .stdout(predicate::str::contains("--output-format"));
}

#[test]
fn test_custom_column_names() {
    let temp_dir = TempDir::new().unwrap();
    
    let gnss_csv = temp_dir.path().join("gnss.csv");
    fs::write(
        &gnss_csv,
        "time,lat,lon\n\
         2025-12-09T14:30:00+01:00,50.8503,4.3517\n"
    ).unwrap();
    
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
}"#
    ).unwrap();
    
    let mut cmd = Command::cargo_bin("tp-cli").unwrap();
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
