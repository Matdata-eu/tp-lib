//! Train path calculation module
//!
//! This module provides probabilistic train path calculation through rail networks
//! using GNSS data and network topology (netrelations).
//!
//! # Overview
//!
//! The path calculation algorithm:
//! 1. Identifies candidate netelements for each GNSS position
//! 2. Calculates probabilities based on distance and heading alignment
//! 3. Constructs paths bidirectionally (forward and backward)
//! 4. Validates path continuity and navigability
//! 5. Selects the highest probability path
//!
//! # Examples
//!
//! ```no_run
//! use tp_lib_core::{calculate_train_path, PathConfig, GnssPosition, Netelement, NetRelation};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let gnss_positions: Vec<GnssPosition> = vec![]; // Load GNSS data
//! let netelements: Vec<Netelement> = vec![]; // Load network
//! let netrelations: Vec<NetRelation> = vec![]; // Load topology
//! let config = PathConfig::default();
//!
//! let result = calculate_train_path(
//!     &gnss_positions,
//!     &netelements,
//!     &netrelations,
//!     &config,
//!     false, // Project coordinates onto path
//! )?;
//!
//! if result.is_topology_based() {
//!     println!("Path calculated successfully");
//! }
//! # Ok(())
//! # }
//! ```

use crate::errors::ProjectionError;
use crate::models::{GnssPosition, TrainPath};
use serde::{Deserialize, Serialize};

/// Path calculation mode indicating which algorithm was used
///
/// Determines whether the path was calculated using network topology
/// or fell back to independent position-by-position projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathCalculationMode {
    /// Path calculated using network topology and navigability rules
    TopologyBased,

    /// Fallback mode: positions projected independently without topology
    FallbackIndependent,
}

/// Result of train path calculation
///
/// Contains the calculated path (if any), mode used, projected positions,
/// and any warnings encountered during calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    /// The calculated path, None if calculation failed
    pub path: Option<TrainPath>,

    /// Mode used for calculation
    pub mode: PathCalculationMode,

    /// Projected GNSS positions onto the path
    pub projected_positions: Vec<GnssPosition>,

    /// Warnings encountered during calculation
    pub warnings: Vec<String>,
}

impl PathResult {
    /// Create a new path result
    pub fn new(
        path: Option<TrainPath>,
        mode: PathCalculationMode,
        projected_positions: Vec<GnssPosition>,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            path,
            mode,
            projected_positions,
            warnings,
        }
    }

    /// Check if topology-based calculation was used
    pub fn is_topology_based(&self) -> bool {
        self.mode == PathCalculationMode::TopologyBased
    }

    /// Check if fallback mode was used
    pub fn is_fallback(&self) -> bool {
        self.mode == PathCalculationMode::FallbackIndependent
    }

    /// Check if path calculation succeeded
    pub fn has_path(&self) -> bool {
        self.path.is_some()
    }
}

/// Configuration for train path calculation algorithm
///
/// Controls probability thresholds, distance cutoffs, and other parameters
/// that affect path selection and candidate filtering.
///
/// # Examples
///
/// ```
/// use tp_lib_core::PathConfig;
///
/// // Use default configuration
/// let config = PathConfig::default();
/// assert_eq!(config.distance_scale, 10.0);
///
/// // Create custom configuration
/// let config = PathConfig::builder()
///     .distance_scale(15.0)
///     .heading_scale(3.0)
///     .cutoff_distance(75.0)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    /// Distance scale parameter for exponential decay (meters)
    /// Controls how quickly probability decreases with distance
    pub distance_scale: f64,

    /// Heading scale parameter for exponential decay (degrees)
    /// Controls how quickly probability decreases with heading difference
    pub heading_scale: f64,

    /// Maximum distance from GNSS position to consider netelement as candidate (meters)
    pub cutoff_distance: f64,

    /// Maximum heading difference to consider netelement as candidate (degrees)
    /// Positions with larger heading differences are filtered out
    pub heading_cutoff: f64,

    /// Minimum probability threshold for including segment in path (0.0 to 1.0)
    pub probability_threshold: f64,

    /// Distance between resampled positions (meters), None to disable resampling
    pub resampling_distance: Option<f64>,

    /// Maximum number of candidate netelements per GNSS position
    pub max_candidates: usize,
}

impl PathConfig {
    /// Create a new PathConfig with validation
    pub fn new(
        distance_scale: f64,
        heading_scale: f64,
        cutoff_distance: f64,
        heading_cutoff: f64,
        probability_threshold: f64,
        resampling_distance: Option<f64>,
        max_candidates: usize,
    ) -> Result<Self, ProjectionError> {
        let config = Self {
            distance_scale,
            heading_scale,
            cutoff_distance,
            heading_cutoff,
            probability_threshold,
            resampling_distance,
            max_candidates,
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration parameters
    fn validate(&self) -> Result<(), ProjectionError> {
        if self.distance_scale <= 0.0 {
            return Err(ProjectionError::InvalidGeometry(
                "distance_scale must be positive".to_string(),
            ));
        }

        if self.heading_scale <= 0.0 {
            return Err(ProjectionError::InvalidGeometry(
                "heading_scale must be positive".to_string(),
            ));
        }

        if self.cutoff_distance <= 0.0 {
            return Err(ProjectionError::InvalidGeometry(
                "cutoff_distance must be positive".to_string(),
            ));
        }

        if !(0.0..=180.0).contains(&self.heading_cutoff) {
            return Err(ProjectionError::InvalidGeometry(
                "heading_cutoff must be in [0, 180]".to_string(),
            ));
        }

        if !(0.0..=1.0).contains(&self.probability_threshold) {
            return Err(ProjectionError::InvalidGeometry(
                "probability_threshold must be in [0, 1]".to_string(),
            ));
        }

        if let Some(resampling) = self.resampling_distance {
            if resampling <= 0.0 {
                return Err(ProjectionError::InvalidGeometry(
                    "resampling_distance must be positive".to_string(),
                ));
            }
        }

        if self.max_candidates == 0 {
            return Err(ProjectionError::InvalidGeometry(
                "max_candidates must be at least 1".to_string(),
            ));
        }

        Ok(())
    }

    /// Create a builder for PathConfig
    pub fn builder() -> PathConfigBuilder {
        PathConfigBuilder::default()
    }
}

impl Default for PathConfig {
    /// Create default configuration with documented parameter values
    ///
    /// Default values:
    /// - `distance_scale`: 10.0 meters (exponential decay)
    /// - `heading_scale`: 2.0 degrees (exponential decay)
    /// - `cutoff_distance`: 50.0 meters
    /// - `heading_cutoff`: 5.0 degrees
    /// - `probability_threshold`: 0.25 (25%)
    /// - `resampling_distance`: None (disabled)
    /// - `max_candidates`: 3 netelements per position
    fn default() -> Self {
        Self {
            distance_scale: 10.0,
            heading_scale: 2.0,
            cutoff_distance: 50.0,
            heading_cutoff: 5.0,
            probability_threshold: 0.25,
            resampling_distance: None,
            max_candidates: 3,
        }
    }
}

/// Builder for PathConfig with fluent API and validation
///
/// # Examples
///
/// ```
/// use tp_lib_core::PathConfig;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = PathConfig::builder()
///     .distance_scale(15.0)
///     .heading_scale(3.0)
///     .cutoff_distance(75.0)
///     .heading_cutoff(10.0)
///     .probability_threshold(0.3)
///     .resampling_distance(Some(10.0))
///     .max_candidates(5)
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct PathConfigBuilder {
    distance_scale: f64,
    heading_scale: f64,
    cutoff_distance: f64,
    heading_cutoff: f64,
    probability_threshold: f64,
    resampling_distance: Option<f64>,
    max_candidates: usize,
}

impl Default for PathConfigBuilder {
    fn default() -> Self {
        let defaults = PathConfig::default();
        Self {
            distance_scale: defaults.distance_scale,
            heading_scale: defaults.heading_scale,
            cutoff_distance: defaults.cutoff_distance,
            heading_cutoff: defaults.heading_cutoff,
            probability_threshold: defaults.probability_threshold,
            resampling_distance: defaults.resampling_distance,
            max_candidates: defaults.max_candidates,
        }
    }
}

impl PathConfigBuilder {
    /// Set distance scale parameter
    pub fn distance_scale(mut self, value: f64) -> Self {
        self.distance_scale = value;
        self
    }

    /// Set heading scale parameter
    pub fn heading_scale(mut self, value: f64) -> Self {
        self.heading_scale = value;
        self
    }

    /// Set cutoff distance
    pub fn cutoff_distance(mut self, value: f64) -> Self {
        self.cutoff_distance = value;
        self
    }

    /// Set heading cutoff
    pub fn heading_cutoff(mut self, value: f64) -> Self {
        self.heading_cutoff = value;
        self
    }

    /// Set probability threshold
    pub fn probability_threshold(mut self, value: f64) -> Self {
        self.probability_threshold = value;
        self
    }

    /// Set resampling distance
    pub fn resampling_distance(mut self, value: Option<f64>) -> Self {
        self.resampling_distance = value;
        self
    }

    /// Set maximum candidates
    pub fn max_candidates(mut self, value: usize) -> Self {
        self.max_candidates = value;
        self
    }

    /// Build and validate the PathConfig
    pub fn build(self) -> Result<PathConfig, ProjectionError> {
        PathConfig::new(
            self.distance_scale,
            self.heading_scale,
            self.cutoff_distance,
            self.heading_cutoff,
            self.probability_threshold,
            self.resampling_distance,
            self.max_candidates,
        )
    }
}

pub mod candidate;
pub mod construction;
pub mod graph;
pub mod probability;
pub mod selection;

// Re-exports
pub use candidate::*;
pub use construction::*;
pub use graph::{build_topology_graph, validate_netrelation_references, NetelementSide};
pub use probability::*;
pub use selection::*;

// Re-export configuration types
pub use PathCalculationMode::{FallbackIndependent, TopologyBased};
