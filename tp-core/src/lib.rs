//! TP-Core: Train Positioning Library - Core Engine
//!
//! This library provides geospatial projection of GNSS positions onto railway track netelements.
//!
//! # Overview
//!
//! TP-Core enables projection of GNSS (GPS) coordinates onto railway track centerlines (netelements),
//! calculating precise measures along the track and assigning positions to specific track segments.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use tp_lib_core::{parse_gnss_csv, parse_network_geojson, RailwayNetwork, project_gnss, ProjectionConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load railway network from GeoJSON
//! let netelements = parse_network_geojson("network.geojson")?;
//! let network = RailwayNetwork::new(netelements)?;
//!
//! // Load GNSS positions from CSV
//! let positions = parse_gnss_csv("gnss.csv", "EPSG:4326", "latitude", "longitude", "timestamp")?;
//!
//! // Project onto network with default configuration
//! let config = ProjectionConfig::default();
//! let projected = project_gnss(&positions, &network, &config)?;
//!
//! // Use projected results
//! for pos in projected {
//!     println!("Position at measure {} on netelement {}", pos.measure_meters, pos.netelement_id);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - **Spatial Indexing**: R-tree based spatial indexing for efficient nearest-netelement search
//! - **CRS Support**: Explicit coordinate reference system handling with optional transformations
//! - **Timezone Awareness**: RFC3339 timestamps with explicit timezone offsets
//! - **Multiple Formats**: CSV and GeoJSON input/output support

pub mod crs;
pub mod errors;
pub mod io;
pub mod models;
pub mod path;
pub mod projection;
pub mod temporal;

// Re-export main types for convenience
pub use errors::ProjectionError;
pub use io::{parse_gnss_csv, parse_gnss_geojson, parse_network_geojson, write_csv, write_geojson};
pub use models::{GnssPosition, Netelement, ProjectedPosition};

/// Result type alias using ProjectionError
pub type Result<T> = std::result::Result<T, ProjectionError>;

use geo::Point;
use projection::geom::project_gnss_position;
use projection::spatial::{find_nearest_netelement, NetworkIndex};

/// Configuration for GNSS projection operations
///
/// # Fields
///
/// * `projection_distance_warning_threshold` - Distance in meters above which warnings are emitted
/// * `suppress_warnings` - If true, suppresses console warnings during projection
///
/// # Examples
///
/// ```
/// use tp_lib_core::ProjectionConfig;
///
/// // Use default configuration (50m warning threshold)
/// let config = ProjectionConfig::default();
///
/// // Custom configuration with higher threshold
/// let config = ProjectionConfig {
///     projection_distance_warning_threshold: 100.0,
///     suppress_warnings: false,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ProjectionConfig {
    /// Threshold distance in meters for emitting warnings about large projection distances
    pub projection_distance_warning_threshold: f64,
    /// Whether to suppress console warnings (useful for benchmarking)
    pub suppress_warnings: bool,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            projection_distance_warning_threshold: 50.0,
            suppress_warnings: false,
        }
    }
}

/// Railway network with spatial indexing for efficient projection
///
/// The `RailwayNetwork` wraps netelements with an R-tree spatial index for O(log n)
/// nearest-neighbor searches, enabling efficient projection of large GNSS datasets.
///
/// # Examples
///
/// ```rust,no_run
/// use tp_lib_core::{parse_network_geojson, RailwayNetwork};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load netelements from GeoJSON
/// let netelements = parse_network_geojson("network.geojson")?;
///
/// // Build spatial index
/// let network = RailwayNetwork::new(netelements)?;
///
/// // Query netelements
/// println!("Network has {} netelements", network.netelements().len());
/// # Ok(())
/// # }
/// ```
pub struct RailwayNetwork {
    index: NetworkIndex,
}

impl RailwayNetwork {
    /// Create a new railway network from netelements
    ///
    /// Builds an R-tree spatial index for efficient nearest-neighbor queries.
    ///
    /// # Arguments
    ///
    /// * `netelements` - Vector of railway track segments with LineString geometries
    ///
    /// # Returns
    ///
    /// * `Ok(RailwayNetwork)` - Successfully indexed network
    /// * `Err(ProjectionError)` - If netelements are empty or geometries are invalid
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use tp_lib_core::{Netelement, RailwayNetwork};
    /// use geo::LineString;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let netelements = vec![
    ///     Netelement {
    ///         id: "NE001".to_string(),
    ///         geometry: LineString::from(vec![(4.35, 50.85), (4.36, 50.86)]),
    ///         crs: "EPSG:4326".to_string(),
    ///     },
    /// ];
    ///
    /// let network = RailwayNetwork::new(netelements)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(netelements: Vec<Netelement>) -> Result<Self> {
        let index = NetworkIndex::new(netelements)?;
        Ok(Self { index })
    }

    /// Find the nearest netelement to a given point
    ///
    /// Uses R-tree spatial index for efficient O(log n) lookup.
    ///
    /// # Arguments
    ///
    /// * `point` - Geographic point in (longitude, latitude) coordinates
    ///
    /// # Returns
    ///
    /// Index of the nearest netelement in the network
    pub fn find_nearest(&self, point: &Point<f64>) -> Result<usize> {
        find_nearest_netelement(point, &self.index)
    }

    /// Get netelement by index
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based index of the netelement
    ///
    /// # Returns
    ///
    /// * `Some(&Netelement)` - If index is valid
    /// * `None` - If index is out of bounds
    pub fn get_by_index(&self, index: usize) -> Option<&Netelement> {
        self.index.netelements().get(index)
    }

    /// Get all netelements
    ///
    /// Returns a slice containing all netelements in the network.
    pub fn netelements(&self) -> &[Netelement] {
        self.index.netelements()
    }

    /// Get the number of netelements in the network
    ///
    /// Returns the total count of railway track segments indexed in this network.
    pub fn netelement_count(&self) -> usize {
        self.index.netelements().len()
    }
}

/// Project GNSS positions onto railway network
///
/// Projects each GNSS position onto the nearest railway netelement, calculating
/// the measure (distance along track) and perpendicular projection distance.
///
/// # Algorithm
///
/// 1. Find nearest netelement using R-tree spatial index
/// 2. Project GNSS point onto netelement LineString geometry
/// 3. Calculate measure from start of netelement
/// 4. Calculate perpendicular distance from point to line
/// 5. Emit warning if projection distance exceeds threshold
///
/// # Arguments
///
/// * `positions` - Slice of GNSS positions with coordinates and timestamps
/// * `network` - Railway network with spatial index
/// * `config` - Projection configuration (warning threshold, CRS settings)
///
/// # Returns
///
/// * `Ok(Vec<ProjectedPosition>)` - Successfully projected positions
/// * `Err(ProjectionError)` - If projection fails (invalid geometry, CRS mismatch, etc.)
///
/// # Examples
///
/// ```rust,no_run
/// use tp_lib_core::{parse_gnss_csv, parse_network_geojson, RailwayNetwork};
/// use tp_lib_core::{project_gnss, ProjectionConfig};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load data
/// let netelements = parse_network_geojson("network.geojson")?;
/// let network = RailwayNetwork::new(netelements)?;
/// let positions = parse_gnss_csv("gnss.csv", "EPSG:4326", "latitude", "longitude", "timestamp")?;
///
/// // Project with custom warning threshold
/// let config = ProjectionConfig {
///     projection_distance_warning_threshold: 100.0,
///     suppress_warnings: false,
/// };
/// let projected = project_gnss(&positions, &network, &config)?;
///
/// // Check projection quality
/// for pos in &projected {
///     if pos.projection_distance_meters > 50.0 {
///         println!("Warning: large projection distance for {}", pos.netelement_id);
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Performance
///
/// - O(n log m) where n = GNSS positions, m = netelements
/// - Spatial indexing enables efficient nearest-neighbor search
/// - Target: <10 seconds for 1000 positions × 50 netelements
#[tracing::instrument(skip(positions, network), fields(position_count = positions.len(), netelement_count = network.netelement_count()))]
pub fn project_gnss(
    positions: &[GnssPosition],
    network: &RailwayNetwork,
    config: &ProjectionConfig,
) -> Result<Vec<ProjectedPosition>> {
    tracing::info!(
        "Starting projection of {} GNSS positions onto {} netelements",
        positions.len(),
        network.netelement_count()
    );

    let mut results = Vec::with_capacity(positions.len());

    for (idx, gnss) in positions.iter().enumerate() {
        // Create point from GNSS position
        let gnss_point = Point::new(gnss.longitude, gnss.latitude);

        tracing::debug!(
            position_idx = idx,
            latitude = gnss.latitude,
            longitude = gnss.longitude,
            timestamp = %gnss.timestamp,
            crs = %gnss.crs,
            "Processing GNSS position"
        );

        // Find nearest netelement
        let netelement_idx = network.find_nearest(&gnss_point)?;
        let netelement = network.get_by_index(netelement_idx).ok_or_else(|| {
            ProjectionError::InvalidGeometry(format!(
                "Netelement index {} out of bounds",
                netelement_idx
            ))
        })?;

        tracing::debug!(
            position_idx = idx,
            netelement_id = %netelement.id,
            netelement_idx = netelement_idx,
            "Assigned to nearest netelement"
        );

        // Project onto netelement
        let projected = project_gnss_position(
            gnss,
            netelement.id.clone(),
            &netelement.geometry,
            netelement.crs.clone(),
        )?;

        tracing::debug!(
            position_idx = idx,
            netelement_id = %netelement.id,
            measure_meters = projected.measure_meters,
            projection_distance_meters = projected.projection_distance_meters,
            "Projection completed"
        );

        // Emit warning if projection distance exceeds threshold
        if !config.suppress_warnings
            && projected.projection_distance_meters > config.projection_distance_warning_threshold
        {
            tracing::warn!(
                position_idx = idx,
                projection_distance_meters = projected.projection_distance_meters,
                threshold = config.projection_distance_warning_threshold,
                timestamp = %gnss.timestamp,
                netelement_id = %netelement.id,
                "Large projection distance exceeds threshold"
            );

            eprintln!(
                "WARNING: Large projection distance ({:.2}m > {:.2}m threshold) for position at {:?}",
                projected.projection_distance_meters,
                config.projection_distance_warning_threshold,
                gnss.timestamp
            );
        }

        results.push(projected);
    }

    tracing::info!(
        projected_count = results.len(),
        "Projection completed successfully"
    );

    Ok(results)
}
