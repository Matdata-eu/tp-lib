//! Python bindings for tp-core
//!
//! This module exposes the following features to Python:
//! - Spec 001: GNSS projection (`project_gnss`, `ProjectionConfig`, `ProjectedPosition`)
//! - Spec 002: Train path calculation (`calculate_train_path`, `PathConfig`, `PathResult`,
//!   `TrainPath`, `AssociatedNetElement`)
//! - Spec 004: Train detections (`prepare_detections`, `PreparedDetections`)

use pyo3::exceptions::{PyIOError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use tp_lib_core::{
    // Spec 002: path calculation
    calculate_train_path as core_calculate_train_path,
    crs::transform::CrsTransformer,
    parse_gnss_csv,
    parse_network_geojson,
    prepare_detections as core_prepare_detections,
    project_gnss as core_project_gnss,
    resolve_topology,
    AssociatedNetElement as CoreAssociatedNetElement,
    // Spec 004: detections
    DetectionError,
    DetectionKind,
    DetectionRecord as CoreDetectionRecord,
    DetectionStatus as CoreDetectionStatus,
    DiscardReason as CoreDiscardReason,
    GnssPosition as CoreGnssPosition,
    PathCalculationMode as CorePathCalculationMode,
    PathConfig as CorePathConfig,
    ProjectedPosition as CoreProjectedPosition,
    ProjectionConfig as CoreProjectionConfig,
    ProjectionError,
    RailwayNetwork,
    ResolvedAnchor as CoreResolvedAnchor,
    RetrievalConfig,
    TimestampOrRange as CoreTimestampOrRange,
    TrainPath as CoreTrainPath,
    UreqSparqlClient,
    WorkflowKind,
    DEFAULT_RETRIEVAL_BUFFER_METERS,
    DEFAULT_RINF_ENDPOINT,
};

// ============================================================================
// Error Conversion
// ============================================================================

fn convert_error(error: ProjectionError) -> PyErr {
    match error {
        ProjectionError::InvalidCrs(msg) => PyValueError::new_err(format!("Invalid CRS: {}", msg)),
        ProjectionError::TransformFailed(msg) => {
            PyRuntimeError::new_err(format!("Coordinate transformation failed: {}", msg))
        }
        ProjectionError::InvalidCoordinate(msg) => {
            PyValueError::new_err(format!("Invalid coordinate: {}", msg))
        }
        ProjectionError::MissingTimezone(msg) => {
            PyValueError::new_err(format!("Missing timezone: {}", msg))
        }
        ProjectionError::InvalidTimestamp(msg) => {
            PyValueError::new_err(format!("Invalid timestamp: {}", msg))
        }
        ProjectionError::EmptyNetwork => PyValueError::new_err("Railway network is empty"),
        ProjectionError::InvalidGeometry(msg) => {
            PyValueError::new_err(format!("Invalid geometry: {}", msg))
        }
        ProjectionError::CsvError(err) => PyIOError::new_err(format!("CSV error: {}", err)),
        ProjectionError::GeoJsonError(msg) => PyIOError::new_err(format!("GeoJSON error: {}", msg)),
        ProjectionError::IoError(err) => PyIOError::new_err(format!("IO error: {}", err)),
        ProjectionError::PathCalculationFailed { reason } => {
            PyRuntimeError::new_err(format!("Path calculation failed: {}", reason))
        }
        ProjectionError::NoNavigablePath => {
            PyValueError::new_err("No navigable path found between netelements")
        }
        ProjectionError::InvalidNetRelation(msg) => {
            PyValueError::new_err(format!("Invalid netrelation: {}", msg))
        }
        ProjectionError::InvalidGnssInput(msg) => {
            PyValueError::new_err(format!("Invalid GNSS input: {}", msg))
        }
        ProjectionError::RinfRetrievalFailed(msg) => {
            PyRuntimeError::new_err(format!("RINF retrieval failed: {}", msg))
        }
        ProjectionError::RinfMissingCoverage(msg) => {
            PyValueError::new_err(format!("RINF missing coverage: {}", msg))
        }
        ProjectionError::RinfIncompleteTopology(msg) => {
            PyValueError::new_err(format!("RINF incomplete topology: {}", msg))
        }
    }
}

fn convert_detection_error(error: DetectionError) -> PyErr {
    let is_io = matches!(error, DetectionError::Io(_));
    let msg = error.to_string();
    if is_io {
        PyIOError::new_err(msg)
    } else {
        PyValueError::new_err(msg)
    }
}

/// Resolve topology either from a supplied GeoJSON file or via auto-retrieval
/// from the ERA RINF SPARQL endpoint (feature 006).
fn resolve_python_topology(
    workflow_kind: WorkflowKind,
    network_file: Option<&str>,
    gnss_positions: &[CoreGnssPosition],
    rinf_options: Option<&RinfRetrievalOptions>,
) -> PyResult<(Vec<tp_lib_core::Netelement>, Vec<tp_lib_core::NetRelation>)> {
    if let Some(path) = network_file {
        return parse_network_geojson(path).map_err(convert_error);
    }
    let endpoint = rinf_options
        .map(|o| o.endpoint_url.clone())
        .unwrap_or_else(|| DEFAULT_RINF_ENDPOINT.to_string());
    let buffer = rinf_options
        .map(|o| o.buffer_meters)
        .unwrap_or(DEFAULT_RETRIEVAL_BUFFER_METERS);
    let config = RetrievalConfig::default()
        .with_endpoint(endpoint)
        .with_buffer_meters(buffer);
    let client = UreqSparqlClient::default();
    let (topology, _outcome) =
        resolve_topology(workflow_kind, gnss_positions, None, &config, &client)
            .map_err(convert_error)?;
    Ok((topology.netelements, topology.netrelations))
}

// ============================================================================
// Helper: DetectionRecord → Python dict
// ============================================================================

fn detection_record_to_dict<'py>(
    py: Python<'py>,
    rec: &CoreDetectionRecord,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("source_file", &rec.source_file)?;
    d.set_item("source_row", rec.source_row)?;
    d.set_item(
        "kind",
        match rec.kind {
            DetectionKind::Punctual => "punctual",
            DetectionKind::Linear => "linear",
        },
    )?;

    // Timestamp (single instant or time range)
    match &rec.timestamp {
        CoreTimestampOrRange::Single { timestamp } => {
            d.set_item("timestamp", timestamp.to_rfc3339())?;
        }
        CoreTimestampOrRange::Range { t_from, t_to } => {
            let ts = PyDict::new_bound(py);
            ts.set_item("t_from", t_from.to_rfc3339())?;
            ts.set_item("t_to", t_to.to_rfc3339())?;
            d.set_item("timestamp", ts)?;
        }
    }

    // Status
    match &rec.status {
        CoreDetectionStatus::Applied {
            netelement_id,
            intrinsic,
        } => {
            d.set_item("status", "applied")?;
            d.set_item("netelement_id", netelement_id.as_str())?;
            d.set_item("intrinsic", *intrinsic)?;
        }
        CoreDetectionStatus::Resolved {
            netelement_id,
            distance_m,
        } => {
            d.set_item("status", "resolved")?;
            d.set_item("netelement_id", netelement_id.as_str())?;
            d.set_item("distance_m", *distance_m)?;
        }
        CoreDetectionStatus::Discarded { reason } => {
            d.set_item("status", "discarded")?;
            let reason_str = match reason {
                CoreDiscardReason::OutOfTimeRange {
                    gnss_first,
                    gnss_last,
                } => {
                    d.set_item("gnss_first", gnss_first.to_rfc3339())?;
                    d.set_item("gnss_last", gnss_last.to_rfc3339())?;
                    "out_of_time_range"
                }
                CoreDiscardReason::OutOfReach {
                    nearest_distance_m,
                    cutoff_m,
                } => {
                    d.set_item("nearest_distance_m", *nearest_distance_m)?;
                    d.set_item("cutoff_m", *cutoff_m)?;
                    "out_of_reach"
                }
                CoreDiscardReason::UnknownNetelement { netelement_id } => {
                    d.set_item("netelement_id", netelement_id.as_str())?;
                    "unknown_netelement"
                }
                CoreDiscardReason::IntrinsicOutOfRange { value } => {
                    d.set_item("value", *value)?;
                    "intrinsic_out_of_range"
                }
                CoreDiscardReason::DuplicateOfPriorDetection { kept_index } => {
                    d.set_item("kept_index", *kept_index)?;
                    "duplicate_of_prior_detection"
                }
            };
            d.set_item("discard_reason", reason_str)?;
        }
    }

    if let Some(id) = &rec.id {
        d.set_item("id", id.as_str())?;
    }
    if let Some(source) = &rec.source {
        d.set_item("source", source.as_str())?;
    }
    if !rec.metadata.is_empty() {
        let meta = PyDict::new_bound(py);
        for (k, v) in &rec.metadata {
            meta.set_item(k.as_str(), v.as_str())?;
        }
        d.set_item("metadata", meta)?;
    }

    Ok(d)
}

// ============================================================================
// Python Data Classes — Spec 001: GNSS Projection
// ============================================================================

/// Configuration for GNSS projection.
#[pyclass]
#[derive(Clone)]
pub struct ProjectionConfig {
    /// Maximum search radius for nearest-segment lookup (meters).
    #[pyo3(get, set)]
    pub max_search_radius_meters: f64,

    /// Warning threshold for large projection distances (meters).
    #[pyo3(get, set)]
    pub projection_distance_warning_threshold: f64,

    /// Suppress warning messages during projection.
    #[pyo3(get, set)]
    pub suppress_warnings: bool,
}

#[pymethods]
impl ProjectionConfig {
    #[new]
    #[pyo3(signature = (max_search_radius_meters=1000.0, projection_distance_warning_threshold=50.0, suppress_warnings=false))]
    fn new(
        max_search_radius_meters: f64,
        projection_distance_warning_threshold: f64,
        suppress_warnings: bool,
    ) -> Self {
        Self {
            max_search_radius_meters,
            projection_distance_warning_threshold,
            suppress_warnings,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ProjectionConfig(max_search_radius_meters={}, projection_distance_warning_threshold={}, suppress_warnings={})",
            self.max_search_radius_meters,
            self.projection_distance_warning_threshold,
            self.suppress_warnings
        )
    }
}

impl From<ProjectionConfig> for CoreProjectionConfig {
    fn from(py_config: ProjectionConfig) -> Self {
        CoreProjectionConfig {
            projection_distance_warning_threshold: py_config.projection_distance_warning_threshold,
            suppress_warnings: py_config.suppress_warnings,
        }
    }
}

/// Options for automatic ERA RINF topology retrieval (feature 006).
#[pyclass]
#[derive(Clone)]
pub struct RinfRetrievalOptions {
    #[pyo3(get, set)]
    pub endpoint_url: String,
    #[pyo3(get, set)]
    pub buffer_meters: f64,
}

#[pymethods]
impl RinfRetrievalOptions {
    #[new]
    #[pyo3(signature = (endpoint_url=None, buffer_meters=None))]
    fn new(endpoint_url: Option<String>, buffer_meters: Option<f64>) -> Self {
        Self {
            endpoint_url: endpoint_url
                .unwrap_or_else(|| tp_lib_core::DEFAULT_RINF_ENDPOINT.to_string()),
            buffer_meters: buffer_meters.unwrap_or(tp_lib_core::DEFAULT_RETRIEVAL_BUFFER_METERS),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "RinfRetrievalOptions(endpoint_url='{}', buffer_meters={})",
            self.endpoint_url, self.buffer_meters
        )
    }
}

/// A single GNSS position projected onto a railway network element.
#[pyclass]
#[derive(Clone)]
pub struct ProjectedPosition {
    /// Original latitude (WGS84).
    #[pyo3(get)]
    pub original_latitude: f64,

    /// Original longitude (WGS84).
    #[pyo3(get)]
    pub original_longitude: f64,

    /// Original timestamp (RFC3339 string).
    #[pyo3(get)]
    pub timestamp: String,

    /// Projected X coordinate in the output CRS.
    #[pyo3(get)]
    pub projected_x: f64,

    /// Projected Y coordinate in the output CRS.
    #[pyo3(get)]
    pub projected_y: f64,

    /// Network element ID.
    #[pyo3(get)]
    pub netelement_id: String,

    /// Linear measure along track (meters).
    #[pyo3(get)]
    pub measure_meters: f64,

    /// Perpendicular distance from track to original GNSS point (meters).
    #[pyo3(get)]
    pub projection_distance_meters: f64,

    /// Coordinate reference system of the projected coordinates.
    #[pyo3(get)]
    pub crs: String,
}

#[pymethods]
impl ProjectedPosition {
    fn __repr__(&self) -> String {
        format!(
            "ProjectedPosition(netelement_id='{}', measure={}m, distance={}m)",
            self.netelement_id, self.measure_meters, self.projection_distance_meters
        )
    }

    fn to_dict(&self) -> PyResult<std::collections::HashMap<String, String>> {
        let mut dict = std::collections::HashMap::new();
        dict.insert(
            "original_latitude".to_string(),
            self.original_latitude.to_string(),
        );
        dict.insert(
            "original_longitude".to_string(),
            self.original_longitude.to_string(),
        );
        dict.insert("timestamp".to_string(), self.timestamp.clone());
        dict.insert("projected_x".to_string(), self.projected_x.to_string());
        dict.insert("projected_y".to_string(), self.projected_y.to_string());
        dict.insert("netelement_id".to_string(), self.netelement_id.clone());
        dict.insert(
            "measure_meters".to_string(),
            self.measure_meters.to_string(),
        );
        dict.insert(
            "projection_distance_meters".to_string(),
            self.projection_distance_meters.to_string(),
        );
        dict.insert("crs".to_string(), self.crs.clone());
        Ok(dict)
    }
}

impl From<&CoreProjectedPosition> for ProjectedPosition {
    fn from(core: &CoreProjectedPosition) -> Self {
        ProjectedPosition {
            original_latitude: core.original.latitude,
            original_longitude: core.original.longitude,
            timestamp: core.original.timestamp.to_rfc3339(),
            projected_x: core.projected_coords.x(),
            projected_y: core.projected_coords.y(),
            netelement_id: core.netelement_id.clone(),
            measure_meters: core.measure_meters,
            projection_distance_meters: core.projection_distance_meters,
            crs: core.crs.clone(),
        }
    }
}

// ============================================================================
// Python Data Classes — Spec 002: Path Calculation
// ============================================================================

/// A single network segment in a calculated train path.
#[pyclass]
#[derive(Clone)]
pub struct AssociatedNetElement {
    /// Network element identifier.
    #[pyo3(get)]
    pub netelement_id: String,

    /// Probability that the train traversed this segment.
    #[pyo3(get)]
    pub probability: f64,

    /// Start intrinsic coordinate (0.0–1.0) along the netelement.
    #[pyo3(get)]
    pub start_intrinsic: f64,

    /// End intrinsic coordinate (0.0–1.0) along the netelement.
    #[pyo3(get)]
    pub end_intrinsic: f64,

    /// Index of the first GNSS position associated with this segment.
    #[pyo3(get)]
    pub gnss_start_index: usize,

    /// Index of the last GNSS position associated with this segment.
    #[pyo3(get)]
    pub gnss_end_index: usize,
}

#[pymethods]
impl AssociatedNetElement {
    fn __repr__(&self) -> String {
        format!(
            "AssociatedNetElement(netelement_id='{}', probability={:.4}, start={:.6}, end={:.6})",
            self.netelement_id, self.probability, self.start_intrinsic, self.end_intrinsic
        )
    }

    fn to_dict(&self) -> std::collections::HashMap<String, String> {
        let mut d = std::collections::HashMap::new();
        d.insert("netelement_id".to_string(), self.netelement_id.clone());
        d.insert("probability".to_string(), self.probability.to_string());
        d.insert(
            "start_intrinsic".to_string(),
            self.start_intrinsic.to_string(),
        );
        d.insert("end_intrinsic".to_string(), self.end_intrinsic.to_string());
        d.insert(
            "gnss_start_index".to_string(),
            self.gnss_start_index.to_string(),
        );
        d.insert(
            "gnss_end_index".to_string(),
            self.gnss_end_index.to_string(),
        );
        d
    }
}

impl From<&CoreAssociatedNetElement> for AssociatedNetElement {
    fn from(core: &CoreAssociatedNetElement) -> Self {
        AssociatedNetElement {
            netelement_id: core.netelement_id.clone(),
            probability: core.probability,
            start_intrinsic: core.start_intrinsic,
            end_intrinsic: core.end_intrinsic,
            gnss_start_index: core.gnss_start_index,
            gnss_end_index: core.gnss_end_index,
        }
    }
}

/// The calculated train path through the railway network.
#[pyclass]
#[derive(Clone)]
pub struct TrainPath {
    /// Ordered list of network segments traversed.
    #[pyo3(get)]
    pub segments: Vec<AssociatedNetElement>,

    /// Overall path probability.
    #[pyo3(get)]
    pub overall_probability: f64,

    /// Timestamp when the path was calculated (RFC3339), if available.
    #[pyo3(get)]
    pub calculated_at: Option<String>,
}

#[pymethods]
impl TrainPath {
    fn __repr__(&self) -> String {
        format!(
            "TrainPath(segments={}, overall_probability={:.4})",
            self.segments.len(),
            self.overall_probability
        )
    }
}

impl From<&CoreTrainPath> for TrainPath {
    fn from(core: &CoreTrainPath) -> Self {
        TrainPath {
            segments: core
                .segments
                .iter()
                .map(AssociatedNetElement::from)
                .collect(),
            overall_probability: core.overall_probability,
            calculated_at: core.calculated_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Configuration for train path calculation.
///
/// All parameters have sensible defaults; only override what you need.
#[pyclass]
#[derive(Clone)]
pub struct PathConfig {
    /// Emission probability distance scale (meters). Default: 10.0
    #[pyo3(get, set)]
    pub distance_scale: f64,

    /// Emission probability heading scale (degrees). Default: 2.0
    #[pyo3(get, set)]
    pub heading_scale: f64,

    /// Maximum candidate distance from GNSS position (meters). Default: 500.0
    #[pyo3(get, set)]
    pub cutoff_distance: f64,

    /// Maximum heading difference for candidates (degrees). Default: 10.0
    #[pyo3(get, set)]
    pub heading_cutoff: f64,

    /// Minimum probability threshold for candidates (0.0–1.0). Default: 0.02
    #[pyo3(get, set)]
    pub probability_threshold: f64,

    /// Resampling distance between GNSS positions (meters). `None` disables resampling.
    #[pyo3(get, set)]
    pub resampling_distance: Option<f64>,

    /// Maximum candidate netelements per GNSS position. Default: 3
    #[pyo3(get, set)]
    pub max_candidates: usize,

    /// If `true`, skip projecting positions onto the calculated path. Default: `false`
    #[pyo3(get, set)]
    pub path_only: bool,

    /// Transition probability scale β in meters (Newson & Krumm). Default: 50.0
    #[pyo3(get, set)]
    pub beta: f64,

    /// Distance threshold for edge-zone handling (meters). Default: 50.0
    #[pyo3(get, set)]
    pub edge_zone_distance: f64,

    /// Turn-angle scale (degrees). Default: 30.0
    #[pyo3(get, set)]
    pub turn_scale: f64,

    /// Max distance for resolving coordinate-only detections (meters). Default: 2.5
    #[pyo3(get, set)]
    pub detection_cutoff_distance: f64,
}

impl Default for PathConfig {
    fn default() -> Self {
        PathConfig {
            distance_scale: 10.0,
            heading_scale: 2.0,
            cutoff_distance: 500.0,
            heading_cutoff: 10.0,
            probability_threshold: 0.02,
            resampling_distance: None,
            max_candidates: 3,
            path_only: false,
            beta: 50.0,
            edge_zone_distance: 50.0,
            turn_scale: 30.0,
            detection_cutoff_distance: 2.5,
        }
    }
}

#[pymethods]
impl PathConfig {
    #[new]
    #[pyo3(signature = (
        distance_scale=10.0,
        heading_scale=2.0,
        cutoff_distance=500.0,
        heading_cutoff=10.0,
        probability_threshold=0.02,
        resampling_distance=None,
        max_candidates=3,
        path_only=false,
        beta=50.0,
        edge_zone_distance=50.0,
        turn_scale=30.0,
        detection_cutoff_distance=2.5
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        distance_scale: f64,
        heading_scale: f64,
        cutoff_distance: f64,
        heading_cutoff: f64,
        probability_threshold: f64,
        resampling_distance: Option<f64>,
        max_candidates: usize,
        path_only: bool,
        beta: f64,
        edge_zone_distance: f64,
        turn_scale: f64,
        detection_cutoff_distance: f64,
    ) -> Self {
        Self {
            distance_scale,
            heading_scale,
            cutoff_distance,
            heading_cutoff,
            probability_threshold,
            resampling_distance,
            max_candidates,
            path_only,
            beta,
            edge_zone_distance,
            turn_scale,
            detection_cutoff_distance,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PathConfig(distance_scale={}, heading_scale={}, cutoff_distance={}, max_candidates={})",
            self.distance_scale, self.heading_scale, self.cutoff_distance, self.max_candidates
        )
    }
}

/// Result of `calculate_train_path`.
#[pyclass]
pub struct PathResult {
    inner_path: Option<TrainPath>,
    inner_mode: String,
    inner_projected_positions: Vec<ProjectedPosition>,
    inner_warnings: Vec<String>,
    inner_detection_provenance: Vec<CoreDetectionRecord>,
}

#[pymethods]
impl PathResult {
    /// The calculated train path, or `None` if no path was found.
    #[getter]
    fn path(&self) -> Option<TrainPath> {
        self.inner_path.clone()
    }

    /// Calculation mode: `"topology_based"` or `"fallback_independent"`.
    #[getter]
    fn mode(&self) -> &str {
        &self.inner_mode
    }

    /// GNSS positions projected onto the calculated path.
    #[getter]
    fn projected_positions(&self) -> Vec<ProjectedPosition> {
        self.inner_projected_positions.clone()
    }

    /// Non-fatal warnings emitted during path calculation.
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner_warnings.clone()
    }

    /// Per-detection provenance records.
    ///
    /// Returns a list of dicts with keys: `source_file`, `source_row`, `kind`,
    /// `timestamp`, `status` (`"applied"` / `"resolved"` / `"discarded"`),
    /// and optional detail fields (`netelement_id`, `intrinsic`, `distance_m`,
    /// `discard_reason`, `kept_index`, `id`, `source`, `metadata`).
    fn detection_provenance<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        self.inner_detection_provenance
            .iter()
            .map(|rec| detection_record_to_dict(py, rec))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "PathResult(mode='{}', has_path={}, positions={}, warnings={})",
            self.inner_mode,
            self.inner_path.is_some(),
            self.inner_projected_positions.len(),
            self.inner_warnings.len()
        )
    }
}

// ============================================================================
// Python Data Classes — Spec 004: Detections
// ============================================================================

/// Output of `prepare_detections`: resolved anchors and per-detection provenance.
///
/// Pass this to `calculate_train_path` via the `detections` parameter to anchor
/// the HMM path calculation to known train locations.
#[pyclass]
pub struct PreparedDetections {
    inner_records: Vec<CoreDetectionRecord>,
    inner_warnings: Vec<String>,
    /// Resolved anchors — consumed internally by `calculate_train_path`.
    pub(crate) anchors: Vec<CoreResolvedAnchor>,
}

#[pymethods]
impl PreparedDetections {
    /// Warnings emitted during detection preparation.
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner_warnings.clone()
    }

    /// Number of resolved anchors ready for path calculation.
    #[getter]
    fn anchor_count(&self) -> usize {
        self.anchors.len()
    }

    /// Per-detection provenance records.
    ///
    /// Returns a list of dicts with keys: `source_file`, `source_row`, `kind`,
    /// `timestamp`, `status` (`"applied"` / `"resolved"` / `"discarded"`),
    /// and optional detail fields.
    fn records<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        self.inner_records
            .iter()
            .map(|rec| detection_record_to_dict(py, rec))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "PreparedDetections(records={}, anchors={}, warnings={})",
            self.inner_records.len(),
            self.anchors.len(),
            self.inner_warnings.len()
        )
    }
}

// ============================================================================
// Functions — Spec 001: GNSS Projection
// ============================================================================

/// Project GNSS positions onto railway network elements.
///
/// # Arguments
///
/// * `gnss_file` - Path to CSV file (columns: `latitude`, `longitude`, `timestamp`)
/// * `gnss_crs` - CRS of GNSS coordinates (e.g. `"EPSG:4326"` for WGS84)
/// * `network_file` - Path to GeoJSON file with network elements
/// * `network_crs` - CRS of network geometries (e.g. `"EPSG:4326"`)
/// * `target_crs` - CRS for output projected coordinates (e.g. `"EPSG:31370"`)
/// * `config` - Optional projection configuration
///
/// # Returns
///
/// List of `ProjectedPosition`, one per input GNSS point.
#[pyfunction]
#[pyo3(signature = (gnss_file, gnss_crs, network_file=None, network_crs=None, target_crs=None, config=None, rinf_options=None))]
fn project_gnss(
    gnss_file: &str,
    gnss_crs: &str,
    network_file: Option<&str>,
    network_crs: Option<&str>,
    target_crs: Option<&str>,
    config: Option<ProjectionConfig>,
    rinf_options: Option<RinfRetrievalOptions>,
) -> PyResult<Vec<ProjectedPosition>> {
    // Validate all CRS strings upfront
    CrsTransformer::new(gnss_crs.to_string(), gnss_crs.to_string()).map_err(convert_error)?;
    if let Some(nc) = network_crs {
        CrsTransformer::new(nc.to_string(), nc.to_string()).map_err(convert_error)?;
    }
    let resolved_target_crs = target_crs.unwrap_or(gnss_crs).to_string();
    CrsTransformer::new(resolved_target_crs.clone(), resolved_target_crs.clone())
        .map_err(convert_error)?;

    let core_config: CoreProjectionConfig = config
        .unwrap_or_else(|| ProjectionConfig::new(1000.0, 50.0, false))
        .into();

    let gnss_positions = parse_gnss_csv(gnss_file, gnss_crs, "latitude", "longitude", "timestamp")
        .map_err(convert_error)?;

    let (netelements, _netrelations) = resolve_python_topology(
        WorkflowKind::Projection,
        network_file,
        &gnss_positions,
        rinf_options.as_ref(),
    )?;

    let network = RailwayNetwork::new(netelements).map_err(convert_error)?;

    let core_results =
        core_project_gnss(&gnss_positions, &network, &core_config).map_err(convert_error)?;

    let mut py_results = Vec::with_capacity(core_results.len());
    for core_result in &core_results {
        let mut pos = ProjectedPosition::from(core_result);
        if pos.crs != resolved_target_crs {
            let transformer = CrsTransformer::new(pos.crs.clone(), resolved_target_crs.clone())
                .map_err(convert_error)?;
            let point = geo::Point::new(pos.projected_x, pos.projected_y);
            let transformed = transformer.transform(point).map_err(convert_error)?;
            pos.projected_x = transformed.x();
            pos.projected_y = transformed.y();
            pos.crs = resolved_target_crs.clone();
        }
        py_results.push(pos);
    }
    Ok(py_results)
}

// ============================================================================
// Functions — Spec 002: Path Calculation
// ============================================================================

/// Calculate the most probable train path through the railway network.
///
/// Uses a Hidden Markov Model (Viterbi algorithm) to find the most likely
/// sequence of network elements given the GNSS trace.
///
/// # Arguments
///
/// * `gnss_file` - Path to CSV file (columns: `latitude`, `longitude`, `timestamp`)
/// * `gnss_crs` - CRS of GNSS coordinates (e.g. `"EPSG:4326"`)
/// * `network_file` - Path to GeoJSON file with netelements and netrelations
/// * `config` - Optional `PathConfig` (uses defaults if `None`)
/// * `detections` - Optional `PreparedDetections` to anchor the path calculation
///
/// # Returns
///
/// `PathResult` with `path`, `mode`, `projected_positions`, `warnings`,
/// and `detection_provenance()`.
#[pyfunction]
#[pyo3(signature = (gnss_file, gnss_crs, network_file=None, config=None, detections=None, rinf_options=None))]
fn calculate_train_path(
    gnss_file: &str,
    gnss_crs: &str,
    network_file: Option<&str>,
    config: Option<PathConfig>,
    detections: Option<&PreparedDetections>,
    rinf_options: Option<RinfRetrievalOptions>,
) -> PyResult<PathResult> {
    let gnss_positions = parse_gnss_csv(gnss_file, gnss_crs, "latitude", "longitude", "timestamp")
        .map_err(convert_error)?;

    let (netelements, netrelations) = resolve_python_topology(
        WorkflowKind::PathCalculation,
        network_file,
        &gnss_positions,
        rinf_options.as_ref(),
    )?;

    let cfg = config.unwrap_or_default();

    let mut builder = CorePathConfig::builder()
        .distance_scale(cfg.distance_scale)
        .heading_scale(cfg.heading_scale)
        .cutoff_distance(cfg.cutoff_distance)
        .heading_cutoff(cfg.heading_cutoff)
        .probability_threshold(cfg.probability_threshold)
        .resampling_distance(cfg.resampling_distance)
        .max_candidates(cfg.max_candidates)
        .path_only(cfg.path_only)
        .beta(cfg.beta)
        .edge_zone_distance(cfg.edge_zone_distance)
        .turn_scale(cfg.turn_scale)
        .detection_cutoff_distance(cfg.detection_cutoff_distance);

    if let Some(det) = detections {
        builder = builder.anchors(det.anchors.clone());
    }

    let core_config = builder.build().map_err(convert_error)?;

    let core_result =
        core_calculate_train_path(&gnss_positions, &netelements, &netrelations, &core_config)
            .map_err(convert_error)?;

    let py_path = core_result.path.as_ref().map(TrainPath::from);
    let py_mode = match core_result.mode {
        CorePathCalculationMode::TopologyBased => "topology_based".to_string(),
        CorePathCalculationMode::FallbackIndependent => "fallback_independent".to_string(),
    };
    let py_positions: Vec<ProjectedPosition> = core_result
        .projected_positions
        .iter()
        .map(ProjectedPosition::from)
        .collect();

    Ok(PathResult {
        inner_path: py_path,
        inner_mode: py_mode,
        inner_projected_positions: py_positions,
        inner_warnings: core_result.warnings.clone(),
        inner_detection_provenance: core_result.detection_provenance.clone(),
    })
}

// ============================================================================
// Functions — Spec 004: Detections
// ============================================================================

/// Load, validate, time-filter, and resolve train detections from a file.
///
/// Produces `PreparedDetections` containing resolved anchors ready to inject
/// into `calculate_train_path`, plus per-detection provenance records.
///
/// # Arguments
///
/// * `detections_file` - Path to CSV or GeoJSON detections file
/// * `kind` - Detection type: `"punctual"` or `"linear"`
/// * `gnss_file` - Path to GNSS CSV file (columns: `latitude`, `longitude`, `timestamp`)
/// * `gnss_crs` - CRS of GNSS coordinates (e.g. `"EPSG:4326"`)
/// * `network_file` - Path to GeoJSON file with netelements
/// * `cutoff_distance` - Max distance (meters) for coordinate-only resolution. Default: 2.5
///
/// # Returns
///
/// `PreparedDetections` with resolved anchors and provenance records.
#[pyfunction]
#[pyo3(signature = (detections_file, kind, gnss_file, gnss_crs, network_file, cutoff_distance=2.5))]
fn prepare_detections(
    detections_file: &str,
    kind: &str,
    gnss_file: &str,
    gnss_crs: &str,
    network_file: &str,
    cutoff_distance: f64,
) -> PyResult<PreparedDetections> {
    let det_kind = match kind.to_lowercase().as_str() {
        "punctual" => DetectionKind::Punctual,
        "linear" => DetectionKind::Linear,
        other => {
            return Err(PyValueError::new_err(format!(
                "Invalid detection kind '{}': expected 'punctual' or 'linear'",
                other
            )))
        }
    };

    let gnss_positions = parse_gnss_csv(gnss_file, gnss_crs, "latitude", "longitude", "timestamp")
        .map_err(convert_error)?;

    let (netelements, _netrelations) =
        parse_network_geojson(network_file).map_err(convert_error)?;

    let result = core_prepare_detections(
        std::path::Path::new(detections_file),
        det_kind,
        &gnss_positions,
        &netelements,
        cutoff_distance,
    )
    .map_err(convert_detection_error)?;

    Ok(PreparedDetections {
        inner_records: result.records,
        inner_warnings: result.warnings,
        anchors: result.anchors,
    })
}

// ============================================================================
// Module Registration
// ============================================================================

/// Python module for the train positioning library.
#[pymodule]
fn tp_lib(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Spec 001: GNSS projection
    m.add_function(wrap_pyfunction!(project_gnss, m)?)?;
    m.add_class::<ProjectionConfig>()?;
    m.add_class::<ProjectedPosition>()?;

    // Spec 002: path calculation
    m.add_function(wrap_pyfunction!(calculate_train_path, m)?)?;
    m.add_class::<PathConfig>()?;
    m.add_class::<TrainPath>()?;
    m.add_class::<AssociatedNetElement>()?;
    m.add_class::<PathResult>()?;

    // Spec 004: detections
    m.add_function(wrap_pyfunction!(prepare_detections, m)?)?;
    m.add_class::<PreparedDetections>()?;

    // Spec 006: ERA RINF retrieval options
    m.add_class::<RinfRetrievalOptions>()?;

    Ok(())
}
