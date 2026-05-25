//! Feature 006 — RINF topology retrieval integration tests.
//!
//! These tests use the [`MockSparqlClient`] to feed canned JSON responses into
//! [`resolve_topology`], so they run offline and deterministically.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use tp_lib_core::io::rinf::SparqlClient;
use tp_lib_core::workflow::{build_retrieval_area, resolve_topology, RetrievalConfig};
use tp_lib_core::{
    GnssPosition, ProjectionError, RetrievalStatus, TopologySource, TopologyValidationStatus,
    WorkflowKind,
};

/// In-memory SPARQL client used by all feature-006 tests.
///
/// Returns a queued response per call. If the queue is empty, returns an
/// error so tests can verify endpoint-failure paths.
struct MockSparqlClient {
    queue: Mutex<Vec<MockResponse>>,
}

enum MockResponse {
    Ok(Value),
    Err(String),
}

impl MockSparqlClient {
    fn new(responses: Vec<MockResponse>) -> Self {
        Self {
            queue: Mutex::new(responses),
        }
    }
}

impl SparqlClient for MockSparqlClient {
    fn query(&self, _endpoint_url: &str, _sparql: &str) -> Result<Value, ProjectionError> {
        let mut q = self.queue.lock().unwrap();
        if q.is_empty() {
            return Err(ProjectionError::RinfRetrievalFailed(
                "MockSparqlClient queue is empty".to_string(),
            ));
        }
        match q.remove(0) {
            MockResponse::Ok(v) => Ok(v),
            MockResponse::Err(m) => Err(ProjectionError::RinfRetrievalFailed(m)),
        }
    }
}

fn load_fixture(name: &str) -> Value {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(name);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {path:?}: {e}"));
    serde_json::from_str(&raw).expect("fixture must be valid JSON")
}

fn gnss(lat: f64, lon: f64) -> GnssPosition {
    let ts: DateTime<FixedOffset> =
        DateTime::parse_from_rfc3339("2026-05-13T08:00:00+00:00").unwrap();
    GnssPosition {
        latitude: lat,
        longitude: lon,
        timestamp: ts,
        crs: "EPSG:4326".to_string(),
        metadata: HashMap::new(),
        heading: None,
        distance: None,
    }
}

fn empty_results() -> Value {
    serde_json::json!({
        "head": {"vars": []},
        "results": {"bindings": []}
    })
}

#[test]
fn build_retrieval_area_rejects_empty_input() {
    let err = build_retrieval_area(&[], 1000.0).unwrap_err();
    matches!(err, ProjectionError::InvalidGnssInput(_));
}

#[test]
fn build_retrieval_area_expands_by_one_kilometer() {
    let positions = vec![gnss(60.00, 11.50), gnss(60.10, 11.60)];
    let area = build_retrieval_area(&positions, 1000.0).unwrap();
    // Latitude expanded by ~0.00898 deg, longitude by more (cosine factor).
    assert!(area.max_latitude > 60.10);
    assert!(area.min_latitude < 60.00);
    assert!(area.max_longitude > 11.60);
    assert!(area.min_longitude < 11.50);
    assert!(area.polygon_wkt.starts_with("POLYGON(("));
    assert_eq!(area.source_crs, "EPSG:4326");
}

#[test]
fn supplied_topology_short_circuits_retrieval() {
    let positions = vec![gnss(60.00, 11.50)];
    let client = MockSparqlClient::new(vec![]);
    let config = RetrievalConfig::default();
    let (topology, outcome) = resolve_topology(
        WorkflowKind::PathCalculation,
        &positions,
        Some((Vec::new(), Vec::new())),
        &config,
        &client,
    )
    .unwrap();
    assert_eq!(outcome.source_used, TopologySource::SuppliedTopology);
    assert_eq!(outcome.status, RetrievalStatus::Success);
    assert_eq!(
        topology.validation_report.status,
        TopologyValidationStatus::Valid
    );
}

#[test]
fn covered_area_retrieves_and_validates_topology() {
    let positions = vec![gnss(60.00, 11.50), gnss(60.02, 11.54)];
    let netelements = load_fixture("rinf_smoke_netelements.json");
    let netrelations = load_fixture("rinf_smoke_netrelations.json");
    let client = MockSparqlClient::new(vec![
        MockResponse::Ok(netelements),
        MockResponse::Ok(netrelations),
    ]);
    let config = RetrievalConfig::default();
    let (topology, outcome) = resolve_topology(
        WorkflowKind::PathCalculation,
        &positions,
        None,
        &config,
        &client,
    )
    .unwrap();
    assert_eq!(outcome.source_used, TopologySource::EraRinf);
    assert_eq!(outcome.status, RetrievalStatus::Success);
    assert_eq!(topology.netelements.len(), 2);
    assert_eq!(topology.netrelations.len(), 1);
    assert_eq!(topology.netrelations[0].from_netelement_id, "SMOKE-A");
    assert_eq!(topology.netrelations[0].to_netelement_id, "SMOKE-B");
    assert!(topology.netrelations[0].navigable_forward);
    assert!(topology.netrelations[0].navigable_backward);
    assert_eq!(
        topology.validation_report.status,
        TopologyValidationStatus::Valid
    );
}

#[test]
fn missing_coverage_yields_missing_coverage_outcome() {
    let positions = vec![gnss(45.0, -30.0)];
    let client = MockSparqlClient::new(vec![MockResponse::Ok(empty_results())]);
    let (_topology, outcome) = resolve_topology(
        WorkflowKind::PathCalculation,
        &positions,
        None,
        &RetrievalConfig::default(),
        &client,
    )
    .unwrap();
    assert_eq!(outcome.status, RetrievalStatus::MissingCoverage);
    assert_eq!(outcome.source_used, TopologySource::EraRinf);
    assert!(outcome.diagnostic_area_wkt.is_some());
}

#[test]
fn netelements_with_no_netrelations_yields_incomplete_topology() {
    let positions = vec![gnss(60.0, 11.5)];
    let netelements = load_fixture("rinf_smoke_netelements.json");
    let client = MockSparqlClient::new(vec![
        MockResponse::Ok(netelements),
        MockResponse::Ok(empty_results()),
    ]);
    let (_topology, outcome) = resolve_topology(
        WorkflowKind::PathCalculation,
        &positions,
        None,
        &RetrievalConfig::default(),
        &client,
    )
    .unwrap();
    assert_eq!(outcome.status, RetrievalStatus::IncompleteTopology);
}

#[test]
fn coarse_geometry_yields_incomplete_topology() {
    // 1 km-long element with only 2 points -> should be flagged coarse.
    let positions = vec![gnss(60.0, 11.5)];
    let coarse = serde_json::json!({
        "head": {"vars": ["netelement", "netelement_wkt"]},
        "results": {"bindings": [{
            "netelement": {"type": "uri", "value": "http://example/coarse/COARSE-X"},
            "netelement_wkt": {"type": "literal", "value": "LINESTRING(11.50 60.00, 11.52 60.00)"}
        }]}
    });
    let client = MockSparqlClient::new(vec![
        MockResponse::Ok(coarse),
        MockResponse::Ok(empty_results()),
    ]);
    let (_topology, outcome) = resolve_topology(
        WorkflowKind::PathCalculation,
        &positions,
        None,
        &RetrievalConfig::default(),
        &client,
    )
    .unwrap();
    assert_eq!(outcome.status, RetrievalStatus::IncompleteTopology);
    assert!(outcome.detail_message.to_lowercase().contains("coarse"));
}

#[test]
fn endpoint_failure_propagates_as_retrieval_error() {
    let positions = vec![gnss(60.0, 11.5)];
    let client = MockSparqlClient::new(vec![MockResponse::Err("503 Service Unavailable".into())]);
    let err = resolve_topology(
        WorkflowKind::PathCalculation,
        &positions,
        None,
        &RetrievalConfig::default(),
        &client,
    )
    .unwrap_err();
    matches!(err, ProjectionError::RinfRetrievalFailed(_));
}

#[test]
fn empty_gnss_yields_invalid_input_error() {
    let client = MockSparqlClient::new(vec![]);
    let err = resolve_topology(
        WorkflowKind::PathCalculation,
        &[],
        None,
        &RetrievalConfig::default(),
        &client,
    )
    .unwrap_err();
    matches!(err, ProjectionError::InvalidGnssInput(_));
}
