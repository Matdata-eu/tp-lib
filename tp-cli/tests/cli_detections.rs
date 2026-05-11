//! T022 — CLI contract tests for `--punctual-detections` flag and stderr summary.
//!
//! Covers:
//! - Flag absent: normal run, no detection summary line emitted.
//! - Flag with a valid in-window CSV: stderr contains
//!   `"detections: 1 applied, 0 discarded"`.
//! - Flag with an out-of-window detection: stderr contains
//!   `"detections: 0 applied, 1 discarded"` plus an `out_of_time_range` breakdown.
//!
//! Spec references: SC-008, FR-019, FR-020.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

const GNSS_CSV: &str = "timestamp,latitude,longitude,altitude,hdop\n\
     2025-12-09T14:30:00+01:00,50.8503,4.3517,100.0,2.0\n\
     2025-12-09T14:30:01+01:00,50.8504,4.3518,100.5,2.1\n\
     2025-12-09T14:30:02+01:00,50.8505,4.3519,101.0,2.0\n";

const NETWORK_GEOJSON: &str = r#"{
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
}"#;

fn write_inputs(tmp: &TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
    let gnss = tmp.path().join("gnss.csv");
    let network = tmp.path().join("network.geojson");
    fs::write(&gnss, GNSS_CSV).unwrap();
    fs::write(&network, NETWORK_GEOJSON).unwrap();
    (gnss, network)
}

#[test]
fn no_detections_flag_omits_summary_line() {
    let tmp = TempDir::new().unwrap();
    let (gnss, network) = write_inputs(&tmp);
    let output = tmp.path().join("out.csv");

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss")
        .arg(&gnss)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network)
        .arg("--output")
        .arg(&output)
        .arg("--format")
        .arg("csv");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("detections:").not());
}

#[test]
fn punctual_detection_in_window_applies_anchor() {
    let tmp = TempDir::new().unwrap();
    let (gnss, network) = write_inputs(&tmp);

    let detections_csv = tmp.path().join("detections.csv");
    fs::write(
        &detections_csv,
        "timestamp,netelement_id,id,source\n\
         2025-12-09T14:30:01+01:00,NE001,beacon-1,BTM-A1\n",
    )
    .unwrap();

    let output = tmp.path().join("out.csv");
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss")
        .arg(&gnss)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network)
        .arg("--output")
        .arg(&output)
        .arg("--format")
        .arg("csv")
        .arg("--punctual-detections")
        .arg(&detections_csv);

    cmd.assert().success().stderr(predicate::str::contains(
        "detections: 1 applied, 0 discarded",
    ));
}

#[test]
fn punctual_detection_out_of_window_is_discarded() {
    let tmp = TempDir::new().unwrap();
    let (gnss, network) = write_inputs(&tmp);

    let detections_csv = tmp.path().join("detections.csv");
    // Timestamp far before the GNSS window → OutOfTimeRange.
    fs::write(
        &detections_csv,
        "timestamp,netelement_id,id,source\n\
         2025-12-09T13:00:00+01:00,NE001,beacon-old,BTM-A1\n",
    )
    .unwrap();

    let output = tmp.path().join("out.csv");
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss")
        .arg(&gnss)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network)
        .arg("--output")
        .arg(&output)
        .arg("--format")
        .arg("csv")
        .arg("--punctual-detections")
        .arg(&detections_csv);

    cmd.assert().success().stderr(
        predicate::str::contains("detections: 0 applied, 1 discarded")
            .and(predicate::str::contains("out_of_time_range")),
    );
}

#[test]
fn linear_detection_in_window_applies_anchor() {
    let tmp = TempDir::new().unwrap();
    let (gnss, network) = write_inputs(&tmp);
    let output = tmp.path().join("out.csv");
    let detections_csv = tmp.path().join("linear.csv");
    fs::write(
        &detections_csv,
        "t_from,t_to,netelement_id,start_intrinsic,end_intrinsic,id,source\n\
         2025-12-09T14:30:00+01:00,2025-12-09T14:30:02+01:00,NE001,0.0,1.0,seg-1,track-circuit\n",
    )
    .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("tp-cli"));
    cmd.arg("--gnss")
        .arg(&gnss)
        .arg("--crs")
        .arg("EPSG:4326")
        .arg("--network")
        .arg(&network)
        .arg("--output")
        .arg(&output)
        .arg("--format")
        .arg("csv")
        .arg("--linear-detections")
        .arg(&detections_csv);

    cmd.assert().success().stderr(predicate::str::contains(
        "detections: 1 applied, 0 discarded",
    ));
}
