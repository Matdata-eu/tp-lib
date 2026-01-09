# Library API Contract: Path Calculation

**Feature**: 002-train-path-calculation  
**Date**: January 9, 2026  
**Phase**: 1 - Design & Contracts

## Overview

This document defines the public Rust API contract for train path calculation functionality in tp-lib-core. All functions, types, and behaviors specified here are guaranteed stable across minor version releases.

---

## Module: `tp_lib_core::path`

### Public Exports

```rust
// Main path calculation function
pub use path::calculate_train_path;
pub use path::project_onto_path;

// Configuration
pub use path::PathConfig;
pub use path::PathConfigBuilder;

// Results
pub use path::PathResult;
pub use path::PathCalculationMode;
```

---

## Core Types

### PathConfig

Configuration parameters for path calculation algorithm.

```rust
/// Configuration for train path calculation algorithm
#[derive(Debug, Clone)]
pub struct PathConfig {
    /// Distance scale for exponential decay (default: 10.0 meters)
    pub distance_scale: f64,
    
    /// Heading scale for exponential decay (default: 2.0 degrees)
    pub heading_scale: f64,
    
    /// Maximum distance for candidate selection (default: 50.0 meters)
    pub cutoff_distance: f64,
    
    /// Maximum heading difference before rejection (default: 5.0 degrees)
    pub heading_cutoff: f64,
    
    /// Minimum probability threshold for path inclusion (default: 0.25)
    pub probability_threshold: f64,
    
    /// Resampling distance for high-frequency GNSS data (default: None)
    /// When Some(distance), GNSS positions are resampled at the specified interval for path calculation
    /// All original positions are still projected in the final output
    pub resampling_distance: Option<f64>,
    
    /// Maximum number of candidate netelements per GNSS position (default: 3)
    pub max_candidates: usize,
}

impl Default for PathConfig {
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
```

### PathConfigBuilder

Builder pattern for PathConfig with validation.

```rust
/// Builder for PathConfig with validation
#[derive(Debug, Default)]
pub struct PathConfigBuilder {
    distance_scale: Option<f64>,
    heading_scale: Option<f64>,
    cutoff_distance: Option<f64>,
    heading_cutoff: Option<f64>,
    probability_threshold: Option<f64>,
    resampling_distance: Option<f64>,
    max_candidates: Option<usize>,
}

impl PathConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn distance_scale(mut self, value: f64) -> Self {
        self.distance_scale = Some(value);
        self
    }
    
    pub fn heading_scale(mut self, value: f64) -> Self {
        self.heading_scale = Some(value);
        self
    }
    
    pub fn cutoff_distance(mut self, value: f64) -> Self {
        self.cutoff_distance = Some(value);
        self
    }
    
    pub fn heading_cutoff(mut self, value: f64) -> Self {
        self.heading_cutoff = Some(value);
        self
    }
    
    pub fn probability_threshold(mut self, value: f64) -> Self {
        self.probability_threshold = Some(value);
        self
    }
    
    pub fn resampling_distance(mut self, value: Option<f64>) -> Self {
        self.resampling_distance = value;
        self
    }
    
    pub fn max_candidates(mut self, value: usize) -> Self {
        self.max_candidates = Some(value);
        self
    }
    
    /// Build PathConfig with validation
    pub fn build(self) -> Result<PathConfig, ProjectionError> {
        let config = PathConfig {
            distance_scale: self.distance_scale.unwrap_or(10.0),
            heading_scale: self.heading_scale.unwrap_or(2.0),
            cutoff_distance: self.cutoff_distance.unwrap_or(50.0),
            heading_cutoff: self.heading_cutoff.unwrap_or(5.0),
            probability_threshold: self.probability_threshold.unwrap_or(0.25),
            resampling_distance: self.resampling_distance.flatten(),
            max_candidates: self.max_candidates.unwrap_or(3),
        };
        
        config.validate()?;
        Ok(config)
    }
}
```

### PathCalculationMode

Enum indicating how the path was calculated.

```rust
/// Mode used for path calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathCalculationMode {
    /// Full path calculation with topology constraints
    TopologyBased,
    
    /// Fallback mode: independent projection without topology
    /// Used when path calculation fails
    FallbackIndependent,
}
```

### PathResult

Result of path calculation including the path and diagnostic information.

```rust
/// Result of train path calculation
#[derive(Debug, Clone)]
pub struct PathResult {
    /// The calculated train path (None if fallback and path-only mode)
    pub path: Option<TrainPath>,
    
    /// Mode used for calculation
    pub mode: PathCalculationMode,
    
    /// Projected GNSS positions onto the path (empty if path-only mode)
    pub projected_positions: Vec<ProjectedPosition>,
    
    /// Diagnostic warnings (e.g., "No navigable path found, using fallback")
    pub warnings: Vec<String>,
}

impl PathResult {
    /// Check if calculation succeeded with topology-based path
    pub fn is_topology_based(&self) -> bool {
        self.mode == PathCalculationMode::TopologyBased
    }
    
    /// Check if fallback mode was used
    pub fn is_fallback(&self) -> bool {
        self.mode == PathCalculationMode::FallbackIndependent
    }
    
    /// Get path or return error if fallback was used
    pub fn path_or_err(&self) -> Result<&TrainPath, ProjectionError> {
        match self.mode {
            PathCalculationMode::TopologyBased => {
                self.path.as_ref().ok_or_else(|| {
                    ProjectionError::PathCalculationFailed {
                        reason: "Path was None despite topology-based mode".to_string(),
                    }
                })
            }
            PathCalculationMode::FallbackIndependent => {
                Err(ProjectionError::NoNavigablePath {
                    start: "unknown".to_string(),
                    end: "unknown".to_string(),
                })
            }
        }
    }
}
```

---

## Primary Functions

### calculate_train_path

Calculate the most probable continuous path through the network based on GNSS data and topology.

```rust
/// Calculate train path from GNSS coordinates and network topology
///
/// # Arguments
///
/// * `gnss_positions` - Ordered sequence of GNSS coordinates with optional heading/distance
/// * `netelements` - Track segments in the rail network
/// * `netrelations` - Navigability connections between track segments
/// * `config` - Algorithm configuration parameters
/// * `path_only` - If true, only calculate path without projecting coordinates
///
/// # Returns
///
/// * `Ok(PathResult)` - Successful calculation with path and/or projected positions
/// * `Err(ProjectionError)` - Validation failure or unrecoverable error
///
/// # Behavior
///
/// - If path calculation succeeds: Returns topology-based path with probability score
/// - If path calculation fails: Falls back to independent projection mode
/// - If `path_only = true`: Only calculates path, `projected_positions` will be empty
/// - If `path_only = false`: Calculates path AND projects all GNSS positions onto it
///
/// # Errors
///
/// - `EmptyNetwork` if netelements or netrelations are empty
/// - `NoNetRelations` if netrelations vector is empty
/// - `InvalidGeometry` if GNSS positions or netelements fail validation
///
/// # Examples
///
/// ```no_run
/// use tp_lib_core::{calculate_train_path, PathConfig, GnssPosition, Netelement, NetRelation};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let gnss_positions: Vec<GnssPosition> = load_gnss_data()?;
/// let (netelements, netrelations) = load_network()?;
/// let config = PathConfig::default();
///
/// let result = calculate_train_path(
///     &gnss_positions,
///     &netelements,
///     &netrelations,
///     &config,
///     false, // Project coordinates onto path
/// )?;
///
/// if result.is_topology_based() {
///     let path = result.path.unwrap();
///     println!("Path probability: {}", path.overall_probability);
///     println!("Segments: {:?}", path.netelement_ids());
/// } else {
///     println!("Fallback mode used: {}", result.warnings.join("; "));
/// }
/// # Ok(())
/// # }
/// # fn load_gnss_data() -> Result<Vec<GnssPosition>, Box<dyn std::error::Error>> { unimplemented!() }
/// # fn load_network() -> Result<(Vec<Netelement>, Vec<NetRelation>), Box<dyn std::error::Error>> { unimplemented!() }
/// ```
pub fn calculate_train_path(
    gnss_positions: &[GnssPosition],
    netelements: &[Netelement],
    netrelations: &[NetRelation],
    config: &PathConfig,
    path_only: bool,
) -> Result<PathResult, ProjectionError>
```

**Guarantees:**
- Always returns a result (path or fallback mode)
- GNSS positions remain in original order
- Fallback mode preserves existing projection behavior (feature 001)
- Thread-safe (no internal mutable state)

**Performance:**
- Time complexity: O(N × M × log K) where N = GNSS positions, M = candidates per position, K = network size
- Space complexity: O(N × M + P × L) where P = candidate paths, L = path length
- Resampling reduces N proportionally to resampling distance

---

### project_onto_path

Project GNSS coordinates onto a pre-calculated train path.

```rust
/// Project GNSS coordinates onto a pre-calculated train path
///
/// # Arguments
///
/// * `gnss_positions` - GNSS coordinates to project
/// * `train_path` - Pre-calculated train path
/// * `netelements` - Track segments (must include all segments in path)
///
/// # Returns
///
/// * `Ok(Vec<ProjectedPosition>)` - Projected coordinates in original order
/// * `Err(ProjectionError)` - Missing netelement or projection failure
///
/// # Behavior
///
/// - Projects each GNSS position onto the nearest segment within the path
/// - Maintains original GNSS position order
/// - Reuses existing `project_point_onto_linestring()` function
/// - Assigns intrinsic coordinates relative to path segment
///
/// # Errors
///
/// - `InvalidGeometry` if netelement from path not found in netelements vector
/// - `InvalidGeometry` if projection fails (should not occur with valid inputs)
///
/// # Examples
///
/// ```no_run
/// use tp_lib_core::{project_onto_path, GnssPosition, TrainPath, Netelement};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let gnss_positions: Vec<GnssPosition> = load_gnss_data()?;
/// let train_path: TrainPath = load_train_path()?;
/// let netelements: Vec<Netelement> = load_netelements()?;
///
/// let projected = project_onto_path(
///     &gnss_positions,
///     &train_path,
///     &netelements,
/// )?;
///
/// for (gnss, proj) in gnss_positions.iter().zip(projected.iter()) {
///     println!(
///         "Position {} projected to netelement {} at intrinsic {}",
///         gnss.timestamp,
///         proj.netelement_id,
///         proj.intrinsic_coord,
///     );
/// }
/// # Ok(())
/// # }
/// # fn load_gnss_data() -> Result<Vec<GnssPosition>, Box<dyn std::error::Error>> { unimplemented!() }
/// # fn load_train_path() -> Result<TrainPath, Box<dyn std::error::Error>> { unimplemented!() }
/// # fn load_netelements() -> Result<Vec<Netelement>, Box<dyn std::error::Error>> { unimplemented!() }
/// ```
pub fn project_onto_path(
    gnss_positions: &[GnssPosition],
    train_path: &TrainPath,
    netelements: &[Netelement],
) -> Result<Vec<ProjectedPosition>, ProjectionError>
```

**Guarantees:**
- Output length equals input length (1:1 mapping)
- Order preserved (projected[i] corresponds to gnss_positions[i])
- All segments in path must exist in netelements
- Thread-safe

---

## Error Handling

### New ProjectionError Variants

```rust
#[derive(Debug, Error)]
pub enum ProjectionError {
    // ... existing variants ...
    
    /// No netrelations found in network data
    #[error("No netrelations found in network data")]
    NoNetRelations,
    
    /// Invalid netrelation (validation failure)
    #[error("Invalid netrelation: {0}")]
    InvalidNetRelation(String),
    
    /// No navigable path found between positions
    #[error("No navigable path found from {start} to {end}")]
    NoNavigablePath {
        start: String,
        end: String,
    },
    
    /// Path calculation failed for other reason
    #[error("Path calculation failed: {reason}")]
    PathCalculationFailed {
        reason: String,
    },
    
    /// All candidate paths below probability threshold
    #[error("All candidate paths below probability threshold ({threshold})")]
    BelowProbabilityThreshold {
        threshold: f64,
    },
}
```

---

## Backward Compatibility

### Existing APIs (Unchanged)

These existing functions remain unchanged and fully compatible:

```rust
// From existing projection module (feature 001)
pub fn project_gnss_to_network(
    gnss_positions: &[GnssPosition],
    netelements: &[Netelement],
) -> Result<Vec<ProjectedPosition>, ProjectionError>

// From projection/spatial.rs
pub fn find_nearest_netelement(
    point: Point<f64>,
    network_index: &NetworkIndex,
) -> Option<(usize, f64)>

// From projection/geom.rs
pub fn project_point_onto_linestring(
    point: &Point<f64>,
    linestring: &LineString<f64>,
) -> (Point<f64>, f64)
```

### Migration Path

Existing code using independent projection (feature 001) continues to work without changes. New path calculation is purely additive.

**Before (feature 001):**
```rust
use tp_lib_core::project_gnss_to_network;

let projected = project_gnss_to_network(&gnss, &network)?;
```

**After (feature 002 - optional upgrade):**
```rust
use tp_lib_core::{calculate_train_path, PathConfig};

let result = calculate_train_path(&gnss, &netelements, &netrelations, &PathConfig::default(), false)?;
let projected = result.projected_positions;
```

---

## Stability Guarantees

| API Element | Stability | Notes |
|-------------|-----------|-------|
| `calculate_train_path()` signature | **Stable** | Will not change in minor versions |
| `project_onto_path()` signature | **Stable** | Will not change in minor versions |
| `PathConfig` fields | **Stable** | New fields may be added (with defaults) |
| `PathResult` structure | **Stable** | New fields may be added (with defaults) |
| `TrainPath` serialization format | **Versioned** | Version field ensures forward/backward compat |
| Default parameter values | **Subject to tuning** | May change in minor versions based on research |

---

## Testing Contract

All public APIs have corresponding tests:

- **Unit tests**: Each function tested independently with mocked inputs
- **Integration tests**: End-to-end scenarios with real-world-like data
- **Contract tests**: Verify API signatures and behavior stability across versions
- **Property tests**: Validate mathematical properties of probability calculations

Test coverage: **Target 100%** (Constitution Principle V)

---

**API Contract Version**: 1.0  
**Feature Version**: 002-train-path-calculation  
**Last Updated**: January 9, 2026
