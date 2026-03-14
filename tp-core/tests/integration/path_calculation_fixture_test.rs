//! Debug test: in-process equivalent of the CLI command
//!
//! Mirrors:
//!   tp-cli calculate-path \
//!     --gnss test-data/log_28554/log_28554_L36-A_to_L36C-A.csv \
//!     --crs EPSG:4326 \
//!     --network test-data/network_airport.geojson \
//!     --output <...>/log_28554_L36-A_to_L36C-A-path-calculation.geojson
//!
//! Because this test runs in the `tp-core` process (not a subprocess),
//! breakpoints in `tp-core/src/path/` are hit by the debugger.
//!
//! Run with:
//!   cargo test -p tp-lib-core path_calculation_real_fixture -- --nocapture

use tp_lib_core::io::{parse_gnss_csv, parse_network_geojson};
use tp_lib_core::{calculate_train_path, PathCalculationMode, PathConfig};

#[test]
fn path_calculation_real_fixture() {
    let gnss_path = "../test-data/log_28554/log_28554_L36-A_to_L36C-A.csv";
    let network_path = "../test-data/network_airport.geojson";

    // --- load network ---
    let (netelements, netrelations) =
        parse_network_geojson(network_path).expect("Failed to load network");

    // --- load GNSS (real column names from the CSV) ---
    let gnss_positions = parse_gnss_csv(
        gnss_path,
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp",
    )
    .expect("Failed to load GNSS CSV");

    eprintln!("Loaded {} GNSS positions", gnss_positions.len());
    eprintln!(
        "Loaded {} netelements, {} netrelations",
        netelements.len(),
        netrelations.len()
    );

    // --- calculate path ---
    let config = PathConfig::default();
    let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config)
        .expect("Path calculation failed");

    eprintln!("Calculation mode: {:?}", result.mode);
    eprintln!("Warnings: {:?}", result.warnings);

    // Basic sanity checks
    assert_eq!(
        result.mode,
        PathCalculationMode::TopologyBased,
        "Expected topology-based calculation for a connected network"
    );

    assert!(result.path.is_some(), "A path should be calculated");
    let path = result.path.as_ref().unwrap();

    assert!(
        !path.segments.is_empty(),
        "Path must contain at least one segment"
    );
    assert!(
        path.overall_probability > 0.0,
        "Overall path probability must be positive"
    );

    for (i, pos) in result.projected_positions.iter().enumerate() {
        assert!(
            !pos.netelement_id.is_empty(),
            "Position {i}: netelement_id must not be empty"
        );
        assert!(
            pos.measure_meters >= 0.0,
            "Position {i}: measure_meters must be non-negative"
        );
        assert!(
            pos.projection_distance_meters >= 0.0,
            "Position {i}: projection_distance_meters must be non-negative"
        );
    }

    // Print summary for quick inspection when run with --nocapture
    eprintln!(
        "Path has {} segments, overall probability = {:.4}",
        path.segments.len(),
        path.overall_probability
    );
    eprintln!(
        "Projected {} / {} GNSS positions onto path",
        result.projected_positions.len(),
        gnss_positions.len()
    );
    for seg in path.segments.iter().take(5) {
        eprintln!("  segment: {}", seg.netelement_id);
    }
    if path.segments.len() > 5 {
        eprintln!("  ... ({} more)", path.segments.len() - 5);
    }

    for pos in result.projected_positions.iter().take(3) {
        eprintln!(
            "  netelement={} measure={:.2}m distance={:.2}m coords={:?}",
            pos.netelement_id,
            pos.measure_meters,
            pos.projection_distance_meters,
            pos.projected_coords,
        );
    }
}
