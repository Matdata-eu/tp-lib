//! Debug test: in-process equivalent of the CLI command
//!
//! Mirrors:
//!   tp-cli simple-projection \
//!     --gnss tp-core/tests/fixtures/log_28876_L36-B.csv \
//!     --crs EPSG:4326 \
//!     --network tp-core/tests/fixtures/test_network_airport.geojson \
//!     --output <...>/log_28876_L36-B-processed.geojson
//!
//! Because this test runs in the `tp-core` process (not a subprocess),
//! breakpoints in `tp-core/src/projection/geom.rs` are hit by the debugger.
//!
//! Run with:
//!   cargo test -p tp-lib-core simple_projection_real_fixture -- --nocapture

use tp_lib_core::io::{parse_gnss_csv, parse_network_geojson};
use tp_lib_core::{project_gnss, ProjectionConfig, RailwayNetwork};

#[test]
fn simple_projection_real_fixture() {
    let gnss_path = "tests/fixtures/log_28876_L36-B.csv";
    let network_path = "tests/fixtures/test_network_airport.geojson";

    // --- load network ---
    let (netelements, _netrelations) =
        parse_network_geojson(network_path).expect("Failed to load network");
    let network = RailwayNetwork::new(netelements).expect("Failed to build network index");

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

    // --- project ---
    let config = ProjectionConfig::default();
    let projected =
        project_gnss(&gnss_positions, &network, &config).expect("Projection failed");

    eprintln!("Projected {} positions", projected.len());

    // Basic sanity checks
    assert_eq!(
        projected.len(),
        gnss_positions.len(),
        "Output count must match input count"
    );

    for (i, pos) in projected.iter().enumerate() {
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

    // Print first few results for quick inspection when run with --nocapture
    for pos in projected.iter().take(3) {
        eprintln!(
            "  netelement={} measure={:.2}m distance={:.2}m coords={:?}",
            pos.netelement_id,
            pos.measure_meters,
            pos.projection_distance_meters,
            pos.projected_coords,
        );
    }
}
