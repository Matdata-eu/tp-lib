//! Python bindings for tp-core
//!
//! This module provides Python FFI via PyO3 for the GNSS track axis projection library.
//!
//! # Example Usage (Python)
//!
//! ```python
//! from tp_lib import project_gnss, ProjectionConfig
//!
//! # Project GNSS positions onto railway network
//! results = project_gnss(
//!     gnss_file="positions.csv",
//!     gnss_crs="EPSG:4326",
//!     network_file="network.geojson",
//!     network_crs="EPSG:4326",
//!     target_crs="EPSG:31370",  # Belgian Lambert 72
//!     config=ProjectionConfig()
//! )
//!
//! for result in results:
//!     print(f"Position at {result.measure_meters}m on {result.netelement_id}")
//! ```

use pyo3::exceptions::{PyIOError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use tp_lib_core::{
    parse_gnss_csv, parse_network_geojson, project_gnss as core_project_gnss,
    ProjectedPosition as CoreProjectedPosition, ProjectionConfig as CoreProjectionConfig,
    ProjectionError, RailwayNetwork,
};

// ============================================================================
// Error Conversion (T058)
// ============================================================================

/// Convert ProjectionError to appropriate Python exception
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
    }
}

// ============================================================================
// Python Data Classes
// ============================================================================

/// Python-exposed projection configuration
#[pyclass]
#[derive(Clone)]
pub struct ProjectionConfig {
    /// Warning threshold for large projection distances
    #[pyo3(get, set)]
    pub projection_distance_warning_threshold: f64,

    /// Suppress warning messages during projection
    #[pyo3(get, set)]
    pub suppress_warnings: bool,
}

#[pymethods]
impl ProjectionConfig {
    #[new]
    #[pyo3(signature = (projection_distance_warning_threshold=50.0, suppress_warnings=false))]
    fn new(
        projection_distance_warning_threshold: f64,
        suppress_warnings: bool,
    ) -> Self {
        Self {
            projection_distance_warning_threshold,
            suppress_warnings,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ProjectionConfig(projection_distance_warning_threshold={}, suppress_warnings={})",
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

/// Python-exposed projected position result
#[pyclass]
#[derive(Clone)]
pub struct ProjectedPosition {
    /// Original latitude (WGS84)
    #[pyo3(get)]
    pub original_latitude: f64,

    /// Original longitude (WGS84)
    #[pyo3(get)]
    pub original_longitude: f64,

    /// Original timestamp (RFC3339 string)
    #[pyo3(get)]
    pub timestamp: String,

    /// Projected X coordinate in target CRS
    #[pyo3(get)]
    pub projected_x: f64,

    /// Projected Y coordinate in target CRS
    #[pyo3(get)]
    pub projected_y: f64,

    /// Network element ID
    #[pyo3(get)]
    pub netelement_id: String,

    /// Linear measure along track in meters
    #[pyo3(get)]
    pub measure_meters: f64,

    /// Perpendicular distance from track in meters
    #[pyo3(get)]
    pub projection_distance_meters: f64,

    /// Coordinate reference system of projected coordinates
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
    fn from(core_result: &CoreProjectedPosition) -> Self {
        ProjectedPosition {
            original_latitude: core_result.original.latitude,
            original_longitude: core_result.original.longitude,
            timestamp: core_result.original.timestamp.to_rfc3339(),
            projected_x: core_result.projected_coords.x(),
            projected_y: core_result.projected_coords.y(),
            netelement_id: core_result.netelement_id.clone(),
            measure_meters: core_result.measure_meters,
            projection_distance_meters: core_result.projection_distance_meters,
            crs: core_result.crs.clone(),
        }
    }
}

// ============================================================================
// Main Python API (T057)
// ============================================================================

/// Project GNSS positions onto railway network elements
///
/// # Arguments
///
/// * `gnss_file` - Path to CSV file containing GNSS positions (columns: latitude, longitude, timestamp)
/// * `gnss_crs` - CRS of input GNSS coordinates (e.g., "EPSG:4326" for WGS84)
/// * `network_file` - Path to GeoJSON file containing network elements with LineString geometries
/// * `network_crs` - CRS of network geometries (e.g., "EPSG:4326")
/// * `target_crs` - CRS for output projected coordinates (e.g., "EPSG:31370" for Belgian Lambert 72)
/// * `config` - Optional projection configuration (defaults provided)
///
/// # Returns
///
/// List of `ProjectedPosition` objects, one per input GNSS position
///
/// # Raises
///
/// * `ValueError` - Invalid CRS, coordinates, or geometry
/// * `IOError` - File reading errors or invalid CSV/GeoJSON format
/// * `RuntimeError` - Coordinate transformation failures
///
/// # Example
///
/// ```python
/// from tp_lib import project_gnss, ProjectionConfig
///
/// results = project_gnss(
///     gnss_file="data/positions.csv",
///     gnss_crs="EPSG:4326",
///     network_file="data/network.geojson",
///     network_crs="EPSG:4326",
///     target_crs="EPSG:31370",
///     config=ProjectionConfig(max_search_radius_meters=500.0)
/// )
///
/// for pos in results:
///     print(f"{pos.netelement_id}: {pos.measure_meters}m")
/// ```
#[pyfunction]
#[pyo3(signature = (gnss_file, gnss_crs, network_file, _network_crs, _target_crs, config=None))]
fn project_gnss(
    gnss_file: &str,
    gnss_crs: &str,
    network_file: &str,
    _network_crs: &str, // Reserved for future use when CRS per file is supported
    _target_crs: &str,  // Reserved for future use when explicit target CRS is supported
    config: Option<ProjectionConfig>,
) -> PyResult<Vec<ProjectedPosition>> {
    // Convert Python config to Rust config
    let core_config: CoreProjectionConfig = config
        .unwrap_or_else(|| ProjectionConfig::new(50.0, false))
        .into();

    // Parse GNSS positions from CSV
    // Note: parse_gnss_csv signature is (path, crs, lat_col, lon_col, time_col)
    let gnss_positions = parse_gnss_csv(gnss_file, gnss_crs, "latitude", "longitude", "timestamp")
        .map_err(convert_error)?;

    // Parse network from GeoJSON
    let netelements = parse_network_geojson(network_file).map_err(convert_error)?;

    // Build spatial index
    let network = RailwayNetwork::new(netelements).map_err(convert_error)?;

    // Project positions
    let results =
        core_project_gnss(&gnss_positions, &network, &core_config).map_err(convert_error)?;

    // Convert to Python objects
    Ok(results.iter().map(ProjectedPosition::from).collect())
}

// ============================================================================
// Python Module Definition
// ============================================================================

/// Python module for train positioning library
#[pymodule]
fn tp_lib(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(project_gnss, m)?)?;
    m.add_class::<ProjectionConfig>()?;
    m.add_class::<ProjectedPosition>()?;
    Ok(())
}
