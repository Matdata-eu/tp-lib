//! Retrieval domain types for automatic ERA RINF topology download (feature 006).
//!
//! See `specs/006-download-rinf-topology/data-model.md` for the canonical
//! definitions. Types here describe spatial search regions, request envelopes,
//! typed SPARQL rows, the assembled topology bundle, its validation report,
//! and the caller-visible outcome.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{NetRelation, Netelement};

/// Default expansion (meters) around the GNSS envelope.
pub const DEFAULT_RETRIEVAL_BUFFER_METERS: f64 = 1000.0;

/// Default RINF SPARQL endpoint.
pub const DEFAULT_RINF_ENDPOINT: &str = "https://graph.data.era.europa.eu/repositories/rinf-plus";

/// Coarse-geometry threshold (meters) above which more than two points are required.
pub const COARSE_GEOMETRY_LENGTH_THRESHOLD_METERS: f64 = 250.0;

/// Spatial search region sent to the RINF SPARQL endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalArea {
    pub min_longitude: f64,
    pub max_longitude: f64,
    pub min_latitude: f64,
    pub max_latitude: f64,
    pub expansion_meters: f64,
    pub polygon_wkt: String,
    pub source_crs: String,
}

/// Which workflow triggered the automatic-retrieval decision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowKind {
    Projection,
    PathCalculation,
    DetectionPreparation,
    PathReview,
}

/// Workflow invocation that may require automatic topology retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTopologyRequest {
    pub workflow_kind: WorkflowKind,
    pub supplied_topology_present: bool,
    pub rinf_endpoint_url: String,
    pub retrieval_area: Option<RetrievalArea>,
    pub requested_at: DateTime<Utc>,
}

/// One parsed netelement row from the SPARQL response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RinfNetelementRow {
    pub netelement_iri: String,
    pub netelement_id: String,
    pub wkt: String,
    pub geometry_point_count: usize,
    pub length_meters: f64,
}

/// Navigability classification as encoded by ERA.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum RinfNavigability {
    Both,
    /// Navigable only from element A to element B.
    AB,
    /// Navigable only from element B to element A.
    BA,
    None,
}

/// One parsed netrelation row from the SPARQL response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RinfNetrelationRow {
    pub netrelation_iri: String,
    pub element_a_id: String,
    pub element_b_id: String,
    pub is_on_origin_of_element_a: bool,
    pub is_on_origin_of_element_b: bool,
    pub navigability: RinfNavigability,
    pub valid_on_date: NaiveDate,
}

/// Validation status for a retrieved topology bundle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TopologyValidationStatus {
    Valid,
    MissingCoverage,
    IncompleteTopology,
    InvalidInput,
    EndpointFailure,
}

/// Explains whether a downloaded topology is usable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyValidationReport {
    pub status: TopologyValidationStatus,
    pub netelement_count: usize,
    pub netrelation_count: usize,
    pub coarse_geometry_ids: Vec<String>,
    pub uncovered_gnss_indices: Vec<usize>,
    pub message: String,
}

/// Normalized topology bundle ready for downstream workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedTopology {
    pub netelements: Vec<Netelement>,
    pub netrelations: Vec<NetRelation>,
    pub retrieval_area: RetrievalArea,
    pub endpoint_url: String,
    pub retrieved_at: DateTime<Utc>,
    pub validation_report: TopologyValidationReport,
}

/// Which source supplied the topology used by a workflow.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TopologySource {
    SuppliedTopology,
    EraRinf,
}

/// Outcome status surfaced to callers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalStatus {
    Success,
    InvalidInput,
    MissingCoverage,
    IncompleteTopology,
    EndpointFailure,
}

/// Caller-visible outcome of source selection and validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalOutcome {
    pub source_used: TopologySource,
    pub status: RetrievalStatus,
    pub detail_message: String,
    pub diagnostic_area_wkt: Option<String>,
    pub affected_gnss_indices: Vec<usize>,
}

impl RetrievalOutcome {
    pub fn supplied_success() -> Self {
        Self {
            source_used: TopologySource::SuppliedTopology,
            status: RetrievalStatus::Success,
            detail_message: "Using supplied topology".to_string(),
            diagnostic_area_wkt: None,
            affected_gnss_indices: Vec::new(),
        }
    }
}
