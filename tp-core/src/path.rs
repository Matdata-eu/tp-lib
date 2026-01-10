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
//! )?;
//!
//! if result.path.is_some() {
//!     println!("Path calculated successfully");
//! }
//! # Ok(())
//! # }
//! ```

use crate::errors::ProjectionError;
use crate::models::TrainPath;
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
    pub projected_positions: Vec<crate::models::ProjectedPosition>,

    /// Warnings encountered during calculation
    pub warnings: Vec<String>,

    /// Debug information collected during calculation (only populated when debug_mode=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_info: Option<DebugInfo>,
}

impl PathResult {
    /// Create a new path result
    pub fn new(
        path: Option<TrainPath>,
        mode: PathCalculationMode,
        projected_positions: Vec<crate::models::ProjectedPosition>,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            path,
            mode,
            projected_positions,
            warnings,
            debug_info: None,
        }
    }

    /// Create a new path result with debug info
    pub fn with_debug_info(
        path: Option<TrainPath>,
        mode: PathCalculationMode,
        projected_positions: Vec<crate::models::ProjectedPosition>,
        warnings: Vec<String>,
        debug_info: DebugInfo,
    ) -> Self {
        Self {
            path,
            mode,
            projected_positions,
            warnings,
            debug_info: Some(debug_info),
        }
    }

    /// Check if debug info is available
    pub fn has_debug_info(&self) -> bool {
        self.debug_info.is_some()
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

/// Debug information for path calculation (US7: T153)
///
/// Contains intermediate results for troubleshooting and parameter tuning.
/// Collected when `PathConfig::debug_mode` is enabled.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebugInfo {
    /// Candidate paths evaluated during path construction
    pub candidate_paths: Vec<CandidatePath>,

    /// Candidates considered for each GNSS position
    pub position_candidates: Vec<PositionCandidates>,

    /// Decision tree showing path selection process
    pub decision_tree: Vec<PathDecision>,
}

/// Information about a candidate path evaluated during calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidatePath {
    /// Unique identifier for this candidate path
    pub id: String,

    /// Direction of path construction (forward/backward)
    pub direction: String,

    /// Netelement IDs in this candidate path
    pub segment_ids: Vec<String>,

    /// Overall probability score for this path
    pub probability: f64,

    /// Whether this path was selected as the best path
    pub selected: bool,
}

/// Candidates considered for a single GNSS position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionCandidates {
    /// Index of the GNSS position (0-based)
    pub position_index: usize,

    /// Timestamp of the position (ISO 8601)
    pub timestamp: String,

    /// GNSS coordinates (lat, lon)
    pub coordinates: (f64, f64),

    /// Candidate netelements with their probabilities
    pub candidates: Vec<CandidateInfo>,

    /// ID of the netelement selected for this position
    pub selected_netelement: Option<String>,
}

/// Information about a single candidate netelement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateInfo {
    /// Netelement ID
    pub netelement_id: String,

    /// Distance from GNSS position to netelement (meters)
    pub distance: f64,

    /// Heading difference (degrees)
    pub heading_difference: Option<f64>,

    /// Distance-based probability component
    pub distance_probability: f64,

    /// Heading-based probability component
    pub heading_probability: Option<f64>,

    /// Combined probability
    pub combined_probability: f64,

    /// Why this candidate was included or excluded
    pub status: String,
}

/// A decision point in the path selection process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathDecision {
    /// Step number in the decision process
    pub step: usize,

    /// Type of decision (e.g., "forward_extend", "backward_extend", "path_selection")
    pub decision_type: String,

    /// Current netelement ID
    pub current_segment: String,

    /// Available options at this decision point
    pub options: Vec<String>,

    /// Probabilities for each option
    pub option_probabilities: Vec<f64>,

    /// Which option was chosen
    pub chosen_option: String,

    /// Reason for the choice
    pub reason: String,
}

impl DebugInfo {
    /// Create a new empty DebugInfo
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a candidate path to the debug info
    pub fn add_candidate_path(&mut self, path: CandidatePath) {
        self.candidate_paths.push(path);
    }

    /// Add position candidates info
    pub fn add_position_candidates(&mut self, candidates: PositionCandidates) {
        self.position_candidates.push(candidates);
    }

    /// Add a decision to the tree
    pub fn add_decision(&mut self, decision: PathDecision) {
        self.decision_tree.push(decision);
    }

    /// Export debug info to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Check if any debug information was collected
    pub fn is_empty(&self) -> bool {
        self.candidate_paths.is_empty()
            && self.position_candidates.is_empty()
            && self.decision_tree.is_empty()
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

    /// If true, only calculate path without projecting positions (US2: T098)
    /// When true, PathResult.projected_positions will be empty
    pub path_only: bool,

    /// If true, collect and return debug information about path calculation (US7: T152)
    /// Includes candidate paths, position candidates, and decision tree
    pub debug_mode: bool,
}

impl PathConfig {
    /// Create a new PathConfig with validation
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        distance_scale: f64,
        heading_scale: f64,
        cutoff_distance: f64,
        heading_cutoff: f64,
        probability_threshold: f64,
        resampling_distance: Option<f64>,
        max_candidates: usize,
        path_only: bool,
        debug_mode: bool,
    ) -> Result<Self, ProjectionError> {
        let config = Self {
            distance_scale,
            heading_scale,
            cutoff_distance,
            heading_cutoff,
            probability_threshold,
            resampling_distance,
            max_candidates,
            path_only,
            debug_mode,
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
    /// - `path_only`: false (calculate path and project positions)
    /// - `debug_mode`: false (no debug output)
    fn default() -> Self {
        Self {
            distance_scale: 10.0,
            heading_scale: 2.0,
            cutoff_distance: 50.0,
            heading_cutoff: 5.0,
            probability_threshold: 0.25,
            resampling_distance: None,
            max_candidates: 3,
            path_only: false,
            debug_mode: false,
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
    path_only: bool,
    debug_mode: bool,
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
            path_only: defaults.path_only,
            debug_mode: defaults.debug_mode,
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

    /// Set path_only mode (calculate path without projecting positions)
    pub fn path_only(mut self, value: bool) -> Self {
        self.path_only = value;
        self
    }

    /// Set debug mode (collect debug information during path calculation)
    pub fn debug_mode(mut self, value: bool) -> Self {
        self.debug_mode = value;
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
            self.path_only,
            self.debug_mode,
        )
    }
}

pub mod candidate;
pub mod construction;
pub mod debug;
pub mod graph;
pub mod probability;
pub mod selection;
pub mod spacing;

#[cfg(test)]
mod tests;

// Re-exports
pub use candidate::*;
pub use construction::*;
pub use debug::{
    export_all_debug_info, export_candidate_paths, export_decision_tree, export_position_candidates,
};
pub use graph::{build_topology_graph, validate_netrelation_references, NetelementSide};
pub use probability::*;
pub use selection::*;
pub use spacing::{calculate_mean_spacing, select_resampled_subset};

// Re-export configuration types
pub use PathCalculationMode::{FallbackIndependent, TopologyBased};
/// Calculate the most probable continuous train path through the network
///
/// Given GNSS positions, network netelements, netrelations defining connections,
/// and configuration parameters, calculates the most likely continuous path the train
/// traversed through the network.
///
/// # Arguments
///
/// * `gnss_positions` - Ordered sequence of GNSS positions from train journey
/// * `netelements` - Network segments (track) with LineString geometries
/// * `netrelations` - Connections between netelements defining navigable paths
/// * `config` - Path calculation configuration (distance/heading scales, cutoff distances, etc.)
///
/// # Returns
///
/// `Ok(PathResult)` containing:
/// - `path`: The calculated train path as TrainPath (if calculation succeeded)
/// - `mode`: Algorithm mode used (TopologyBased or FallbackIndependent)
/// - `projected_positions`: GNSS positions with projected coordinates
/// - `warnings`: Any non-fatal issues encountered during calculation
///
/// `Err(ProjectionError)` if:
/// - No valid path exists through the network
/// - Input data is invalid
/// - Calculation fails for other reasons
///
/// # Algorithm
///
/// 1. **Candidate Selection**: Find candidate netelements within cutoff_distance for each GNSS position
/// 2. **Probability Calculation**: Calculate probability for each candidate using distance and heading
/// 3. **Netelement Assignment**: Assign each GNSS position to best candidate netelements
/// 4. **Path Construction**: Build continuous path by traversing network using topology constraints
/// 5. **Bidirectional Validation**: Calculate path from both directions and validate consistency
/// 6. **Path Selection**: Return highest probability path from multiple candidates
///
/// # Configuration Impact
///
/// - `distance_scale`: Decay rate for distance probability (default 10.0m)
/// - `heading_scale`: Decay rate for heading probability (default 2.0°)
/// - `cutoff_distance`: Maximum distance for candidate selection (default 50.0m)
/// - `heading_cutoff`: Maximum heading difference, rejects if exceeded (default 5.0°)
/// - `probability_threshold`: Minimum probability for segment inclusion (default 0.25)
/// - `max_candidates`: Maximum candidates to evaluate per GNSS position
///
/// # Example
///
/// ```no_run
/// use tp_lib_core::{calculate_train_path, PathConfig};
/// use tp_lib_core::models::{GnssPosition, Netelement, NetRelation};
/// use geo::LineString;
/// use chrono::Utc;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let gnss_positions = vec![
///     GnssPosition::new(50.8503, 4.3517, Utc::now().into(), "EPSG:4326".to_string())?,
/// ];
///
/// let netelements = vec![
///     Netelement::new(
///         "NE_001".to_string(),
///         LineString::from(vec![(4.3500, 50.8500), (4.3530, 50.8530)]),
///         "EPSG:4326".to_string(),
///     )?,
/// ];
///
/// let netrelations = vec![];
/// let config = PathConfig::default();
///
/// let result = calculate_train_path(&gnss_positions, &netelements, &netrelations, &config)?;
/// println!("Path: {:?}", result.path);
/// # Ok(())
/// # }
/// ```
pub fn calculate_train_path(
    gnss_positions: &[crate::models::GnssPosition],
    netelements: &[crate::models::Netelement],
    netrelations: &[crate::models::NetRelation],
    config: &PathConfig,
) -> Result<PathResult, crate::errors::ProjectionError> {
    use crate::models::AssociatedNetElement;
    use crate::path::candidate::find_candidate_netelements;
    use crate::path::construction::{
        construct_backward_path, construct_forward_path, validate_bidirectional_agreement,
    };
    use crate::path::probability::{
        calculate_combined_probability, calculate_distance_probability,
        calculate_heading_probability, calculate_netelement_probability,
    };
    use crate::path::selection::{average_bidirectional_probability, select_best_path};
    use std::collections::HashMap;

    // Basic input validation
    if netelements.is_empty() {
        return Err(crate::errors::ProjectionError::EmptyNetwork);
    }
    if gnss_positions.is_empty() {
        return Err(crate::errors::ProjectionError::PathCalculationFailed {
            reason: "No GNSS positions provided".to_string(),
        });
    }

    // T157: Initialize debug info collector if debug mode is enabled
    let mut debug_info = if config.debug_mode {
        Some(DebugInfo::new())
    } else {
        None
    };

    // US5 T129-T130: Apply resampling if configured
    let (working_positions, resampling_applied) = if let Some(resample_dist) =
        config.resampling_distance
    {
        let indices = crate::path::spacing::select_resampled_subset(gnss_positions, resample_dist);
        let subset: Vec<_> = indices.iter().map(|&i| &gnss_positions[i]).collect();
        (subset, indices.len() < gnss_positions.len())
    } else {
        // No resampling - use all positions
        (gnss_positions.iter().collect(), false)
    };

    // T098/T108: When path_only is true, skip projection phase
    if config.path_only {
        // Log that path-only mode is enabled (projection will be skipped after path calculation)
        // Continue with path calculation but don't project positions
        // Fall through to the implementation below
    }

    // Phase 1: Candidate Selection (T044-T049)
    // For each GNSS position (potentially resampled), find candidate netelements within cutoff distance
    let mut position_candidates: Vec<Vec<crate::path::candidate::CandidateNetElement>> = Vec::new();

    for gnss in &working_positions {
        let candidates = find_candidate_netelements(
            gnss,
            netelements,
            config.cutoff_distance,
            config.max_candidates,
        )?;
        position_candidates.push(candidates);
    }

    // Phase 2: GNSS-Level Probability (T050-T057)
    // Calculate probability for each GNSS position-netelement pair
    // Build index mapping: netelement_id -> index in netelements array
    let netelement_index: HashMap<String, usize> = netelements
        .iter()
        .enumerate()
        .map(|(idx, ne)| (ne.id.clone(), idx))
        .collect();

    let mut position_probabilities: Vec<HashMap<usize, f64>> = Vec::new(); // Vec<HashMap<netelement_idx, probability>>

    for (pos_idx, candidates) in position_candidates.iter().enumerate() {
        let mut probs = HashMap::new();
        let gnss = working_positions[pos_idx]; // Use working_positions (potentially resampled)

        // T157: Collect debug info for position candidates
        let mut debug_candidates: Vec<CandidateInfo> = Vec::new();

        for candidate in candidates {
            let netelement_idx =
                netelement_index
                    .get(&candidate.netelement_id)
                    .ok_or_else(|| crate::errors::ProjectionError::PathCalculationFailed {
                        reason: format!(
                            "Netelement {} not found in index",
                            candidate.netelement_id
                        ),
                    })?;

            // Distance probability
            let dist_prob =
                calculate_distance_probability(candidate.distance_meters, config.distance_scale);

            // Heading probability (if available)
            let heading_diff_value = if let Some(gnss_heading) = gnss.heading {
                // Calculate netelement heading at projection point
                use crate::path::candidate::{calculate_heading_at_point, heading_difference};
                let netelement = &netelements[*netelement_idx];
                let netelement_heading =
                    calculate_heading_at_point(&candidate.projected_point, &netelement.geometry)?;
                Some(heading_difference(gnss_heading, netelement_heading))
            } else {
                None
            };

            let heading_prob = if let Some(heading_diff) = heading_diff_value {
                calculate_heading_probability(
                    heading_diff,
                    config.heading_scale,
                    config.heading_cutoff,
                )
            } else {
                1.0 // No heading data, assume heading match
            };

            // Combined probability
            let combined = calculate_combined_probability(dist_prob, heading_prob);
            probs.insert(*netelement_idx, combined);

            // T157: Collect candidate info for debug output
            if config.debug_mode {
                debug_candidates.push(CandidateInfo {
                    netelement_id: candidate.netelement_id.clone(),
                    distance: candidate.distance_meters,
                    heading_difference: heading_diff_value,
                    distance_probability: dist_prob,
                    heading_probability: if heading_diff_value.is_some() {
                        Some(heading_prob)
                    } else {
                        None
                    },
                    combined_probability: combined,
                    status: if combined >= config.probability_threshold {
                        "accepted".to_string()
                    } else {
                        "below_threshold".to_string()
                    },
                });
            }
        }

        // T157: Add position candidates to debug info
        if let Some(ref mut debug) = debug_info {
            let selected = probs
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(&idx, _)| netelements[idx].id.clone());

            debug.add_position_candidates(PositionCandidates {
                position_index: pos_idx,
                timestamp: gnss.timestamp.to_rfc3339(),
                coordinates: (gnss.latitude, gnss.longitude),
                candidates: debug_candidates,
                selected_netelement: selected,
            });
        }

        position_probabilities.push(probs);
    }

    // Phase 3: Netelement-Level Probability (T058-T064)
    // Aggregate probabilities for each netelement across all GNSS positions
    let mut netelement_probabilities: HashMap<usize, f64> = HashMap::new();

    for prob_map in &position_probabilities {
        for (&netelement_idx, &prob) in prob_map {
            netelement_probabilities
                .entry(netelement_idx)
                .and_modify(|total| *total += prob)
                .or_insert(prob);
        }
    }

    // Average probabilities by number of positions assigned
    let mut position_counts: HashMap<usize, usize> = HashMap::new();
    for prob_map in &position_probabilities {
        for &netelement_idx in prob_map.keys() {
            *position_counts.entry(netelement_idx).or_insert(0) += 1;
        }
    }

    for (netelement_idx, total_prob) in netelement_probabilities.iter_mut() {
        let count = position_counts.get(netelement_idx).unwrap_or(&1);
        *total_prob = calculate_netelement_probability(&vec![*total_prob / *count as f64; *count]);
    }

    // Phase 4: Path Construction (T065-T074)
    // Build netelement map for path construction
    let mut netelement_map: HashMap<String, (f64, AssociatedNetElement)> = HashMap::new();

    for (&netelement_idx, &prob) in &netelement_probabilities {
        if prob >= config.probability_threshold || netelement_map.is_empty() {
            let netelement = &netelements[netelement_idx];
            // Create basic AssociatedNetElement (simplified for now)
            let segment = AssociatedNetElement::new(
                netelement.id.clone(),
                prob,
                0.0,
                1.0, // Full intrinsic range
                0,
                working_positions.len() - 1, // GNSS range based on working set
            )?;
            netelement_map.insert(netelement.id.clone(), (prob, segment));
        }
    }

    // Find highest probability netelements at start and end
    let start_netelement = position_probabilities
        .first()
        .and_then(|probs| probs.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()))
        .map(|(&idx, _)| &netelements[idx].id);

    let end_netelement = position_probabilities
        .last()
        .and_then(|probs| probs.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()))
        .map(|(&idx, _)| &netelements[idx].id);

    // Construct paths bidirectionally
    let forward_path = if let Some(start_id) = start_netelement {
        construct_forward_path(
            start_id,
            &netelement_map,
            netrelations,
            config.probability_threshold,
        )
        .ok()
    } else {
        None
    };

    let backward_path = if let Some(end_id) = end_netelement {
        construct_backward_path(
            end_id,
            &netelement_map,
            netrelations,
            config.probability_threshold,
        )
        .ok()
    } else {
        None
    };

    // T157: Record candidate paths in debug info
    if let Some(ref mut debug) = debug_info {
        let mut step = 1;

        // Record forward path as candidate
        if let Some(ref fwd) = forward_path {
            debug.add_candidate_path(CandidatePath {
                id: "forward".to_string(),
                direction: "forward".to_string(),
                segment_ids: fwd
                    .segments
                    .iter()
                    .map(|s| s.netelement_id.clone())
                    .collect(),
                probability: fwd.probability,
                selected: false, // Will update after selection
            });

            debug.add_decision(PathDecision {
                step,
                decision_type: "forward_construction".to_string(),
                current_segment: start_netelement.cloned().unwrap_or_default(),
                options: fwd
                    .segments
                    .iter()
                    .map(|s| s.netelement_id.clone())
                    .collect(),
                option_probabilities: fwd.segments.iter().map(|s| s.probability).collect(),
                chosen_option: fwd
                    .segments
                    .last()
                    .map(|s| s.netelement_id.clone())
                    .unwrap_or_default(),
                reason: format!(
                    "Forward path constructed with {} segments, probability {:.4}",
                    fwd.segments.len(),
                    fwd.probability
                ),
            });
            step += 1;
        } else {
            debug.add_decision(PathDecision {
                step,
                decision_type: "forward_construction_failed".to_string(),
                current_segment: start_netelement.cloned().unwrap_or_default(),
                options: vec![],
                option_probabilities: vec![],
                chosen_option: String::new(),
                reason: "No forward path could be constructed".to_string(),
            });
            step += 1;
        }

        // Record backward path as candidate
        if let Some(ref bwd) = backward_path {
            debug.add_candidate_path(CandidatePath {
                id: "backward".to_string(),
                direction: "backward".to_string(),
                segment_ids: bwd
                    .segments
                    .iter()
                    .map(|s| s.netelement_id.clone())
                    .collect(),
                probability: bwd.probability,
                selected: false, // Will update after selection
            });

            debug.add_decision(PathDecision {
                step,
                decision_type: "backward_construction".to_string(),
                current_segment: end_netelement.cloned().unwrap_or_default(),
                options: bwd
                    .segments
                    .iter()
                    .map(|s| s.netelement_id.clone())
                    .collect(),
                option_probabilities: bwd.segments.iter().map(|s| s.probability).collect(),
                chosen_option: bwd
                    .segments
                    .first()
                    .map(|s| s.netelement_id.clone())
                    .unwrap_or_default(),
                reason: format!(
                    "Backward path constructed with {} segments, probability {:.4}",
                    bwd.segments.len(),
                    bwd.probability
                ),
            });
            // step is intentionally not incremented here (end of construction phase)
        } else {
            debug.add_decision(PathDecision {
                step,
                decision_type: "backward_construction_failed".to_string(),
                current_segment: end_netelement.cloned().unwrap_or_default(),
                options: vec![],
                option_probabilities: vec![],
                chosen_option: String::new(),
                reason: "No backward path could be constructed".to_string(),
            });
        }
    }

    // Phase 5: Path Selection (T075-T088)
    let (final_path, selected_direction) = match (&forward_path, &backward_path) {
        (Some(fwd), Some(bwd)) => {
            // Validate bidirectional agreement
            let agreement = validate_bidirectional_agreement(fwd, bwd);
            if agreement {
                // Average probabilities
                let avg_prob =
                    average_bidirectional_probability(Some(fwd.probability), Some(bwd.probability));
                (Some((fwd.segments.clone(), avg_prob)), Some("both"))
            } else {
                // Select best path by probability
                let probabilities = vec![fwd.probability, bwd.probability];
                let reaches_end = vec![true, true]; // Simplified: assume both reach end
                let best_idx = select_best_path(&probabilities, &reaches_end)?;
                if best_idx == 0 {
                    (
                        Some((fwd.segments.clone(), fwd.probability)),
                        Some("forward"),
                    )
                } else {
                    (
                        Some((bwd.segments.clone(), bwd.probability)),
                        Some("backward"),
                    )
                }
            }
        }
        (Some(fwd), None) => (
            Some((fwd.segments.clone(), fwd.probability)),
            Some("forward"),
        ),
        (None, Some(bwd)) => (
            Some((bwd.segments.clone(), bwd.probability)),
            Some("backward"),
        ),
        (None, None) => (None, None),
    };

    // T157: Record final path selection decision
    if let Some(ref mut debug) = debug_info {
        let step = debug.decision_tree.len() + 1;

        match (&forward_path, &backward_path, selected_direction) {
            (Some(fwd), Some(bwd), Some(dir)) => {
                let agreement = validate_bidirectional_agreement(fwd, bwd);
                let avg_prob =
                    average_bidirectional_probability(Some(fwd.probability), Some(bwd.probability));

                debug.add_decision(PathDecision {
                    step,
                    decision_type: "path_selection".to_string(),
                    current_segment: String::new(),
                    options: vec!["forward".to_string(), "backward".to_string()],
                    option_probabilities: vec![fwd.probability, bwd.probability],
                    chosen_option: dir.to_string(),
                    reason: if agreement {
                        format!(
                            "Bidirectional agreement: averaged probability = {:.4}",
                            avg_prob
                        )
                    } else {
                        format!(
                            "No agreement: selected {} path with higher probability",
                            dir
                        )
                    },
                });

                // Mark the selected path(s)
                for candidate in debug.candidate_paths.iter_mut() {
                    candidate.selected = dir == "both" || candidate.direction == dir;
                }
            }
            (Some(_), None, Some(_)) => {
                debug.add_decision(PathDecision {
                    step,
                    decision_type: "path_selection".to_string(),
                    current_segment: String::new(),
                    options: vec!["forward".to_string()],
                    option_probabilities: vec![forward_path.as_ref().unwrap().probability],
                    chosen_option: "forward".to_string(),
                    reason: "Only forward path available".to_string(),
                });
                for candidate in debug.candidate_paths.iter_mut() {
                    candidate.selected = candidate.direction == "forward";
                }
            }
            (None, Some(_), Some(_)) => {
                debug.add_decision(PathDecision {
                    step,
                    decision_type: "path_selection".to_string(),
                    current_segment: String::new(),
                    options: vec!["backward".to_string()],
                    option_probabilities: vec![backward_path.as_ref().unwrap().probability],
                    chosen_option: "backward".to_string(),
                    reason: "Only backward path available".to_string(),
                });
                for candidate in debug.candidate_paths.iter_mut() {
                    candidate.selected = candidate.direction == "backward";
                }
            }
            _ => {
                // No valid paths (None, None case or any unexpected combination)
                debug.add_decision(PathDecision {
                    step,
                    decision_type: "path_selection_failed".to_string(),
                    current_segment: String::new(),
                    options: vec![],
                    option_probabilities: vec![],
                    chosen_option: String::new(),
                    reason: "No valid paths available for selection".to_string(),
                });
            }
        }
    }

    // Create TrainPath if path was found
    let train_path = if let Some((segments, prob)) = final_path {
        use chrono::Utc;
        // Use original gnss_positions for timestamp (not working_positions which might be references)
        let timestamp = gnss_positions
            .first()
            .map(|p| p.timestamp.with_timezone(&Utc));
        Some(crate::models::TrainPath::new(
            segments, prob, timestamp, None, // No metadata for now
        )?)
    } else {
        None
    };

    // Generate warnings if path calculation had issues
    let mut warnings = Vec::new();
    if config.path_only {
        warnings.push("Path-only mode enabled: skipping projection phase".to_string());
    }
    if resampling_applied {
        warnings.push(format!(
            "Resampling applied: used {} of {} positions for path calculation",
            working_positions.len(),
            gnss_positions.len()
        ));
    }

    // US6 T140-T145: Fallback to independent projection when path calculation fails
    if train_path.is_none() {
        warnings.push("No continuous path found using topology-based calculation".to_string());
        if forward_path.is_none() && backward_path.is_none() {
            warnings.push("Both forward and backward path construction failed".to_string());
        }

        // T143: Set mode to FallbackIndependent
        let fallback_positions = if config.path_only {
            // In path-only mode, return empty projected positions
            warnings.push("Path-only mode: skipping fallback projection".to_string());
            Vec::new()
        } else {
            warnings.push("Falling back to independent nearest-segment projection".to_string());

            // T141-T142: Use existing simple projection logic from feature 001
            // Project each GNSS position to nearest netelement independently, ignoring topology/navigability
            use crate::projection::{find_nearest_netelement, NetworkIndex};
            let network_index = NetworkIndex::new(netelements.to_vec())?;

            let mut positions = Vec::new();
            for gnss in gnss_positions {
                // T145: Fallback ignores navigability - projects to geometrically nearest
                use geo::Point;
                let gnss_point = Point::new(gnss.longitude, gnss.latitude);

                if let Ok(netelement_idx) = find_nearest_netelement(&gnss_point, &network_index) {
                    let nearest = &network_index.netelements()[netelement_idx];

                    use crate::projection::project_point_onto_linestring;
                    let projected_point =
                        project_point_onto_linestring(&gnss_point, &nearest.geometry)?;

                    use crate::projection::calculate_measure_along_linestring;
                    let measure =
                        calculate_measure_along_linestring(&nearest.geometry, &projected_point)?;

                    // Calculate projection distance
                    use geo::HaversineDistance;
                    let distance = gnss_point.haversine_distance(&projected_point);

                    let projected = crate::models::ProjectedPosition::new(
                        gnss.clone(),
                        projected_point,
                        nearest.id.clone(),
                        measure,
                        distance,
                        gnss.crs.clone(),
                    );
                    positions.push(projected);
                }
            }
            positions
        };

        // T157: Include debug info in fallback result
        let mut result = PathResult::new(
            None, // No path calculated
            PathCalculationMode::FallbackIndependent,
            fallback_positions,
            warnings,
        );
        result.debug_info = debug_info;
        return Ok(result);
    }

    // T157: Include debug info in successful result
    let mut result = PathResult::new(
        train_path,
        PathCalculationMode::TopologyBased,
        vec![], // No projected positions yet
        warnings,
    );
    result.debug_info = debug_info;
    Ok(result)
}

/// Project GNSS coordinates onto a calculated train path (US2: T093-T097)
///
/// Projects each GNSS position onto the nearest segment in the provided path,
/// calculating intrinsic coordinates (0-1 range) for each position.
///
/// # Arguments
///
/// * `gnss_positions` - Vector of GNSS positions to project
/// * `path` - Pre-calculated train path (from calculate_train_path or loaded from file)
/// * `netelements` - Railway network elements (needed for geometry)
/// * `config` - Path configuration (not currently used, reserved for future)
///
/// # Returns
///
/// Vector of ProjectedPosition structs, one per GNSS coordinate
///
/// # Errors
///
/// Returns `ProjectionError` if:
/// - GNSS positions or path is empty
/// - Netelement IDs in path don't exist in netelements collection
/// - Projection onto linestring fails
/// - Intrinsic coordinates fall outside valid range [0, 1]
///
/// # Example
///
/// ```no_run
/// use tp_lib_core::{project_onto_path, PathConfig};
/// use tp_lib_core::models::{GnssPosition, TrainPath, Netelement};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load pre-calculated path
/// let path: TrainPath = todo!(); // From file or calculate_train_path
/// let netelements: Vec<Netelement> = todo!();
/// let gnss_positions: Vec<GnssPosition> = todo!();
///
/// let config = PathConfig::default();
/// let projected = project_onto_path(&gnss_positions, &path, &netelements, &config)?;
///
/// // Each projected position has netelement_id and intrinsic coordinate
/// for proj in projected {
///     if let Some(intrinsic) = proj.intrinsic {
///         println!("Projected to {} at intrinsic {:.3}", proj.netelement_id, intrinsic);
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub fn project_onto_path(
    gnss_positions: &[crate::models::GnssPosition],
    path: &crate::models::TrainPath,
    netelements: &[crate::models::Netelement],
    _config: &PathConfig,
) -> Result<Vec<crate::models::ProjectedPosition>, crate::errors::ProjectionError> {
    use crate::projection::geom::{
        calculate_measure_along_linestring, project_point_onto_linestring,
    };
    use geo::{HaversineLength, Point};
    use std::collections::HashMap;

    // Validate inputs (T100)
    if gnss_positions.is_empty() {
        return Err(crate::errors::ProjectionError::PathCalculationFailed {
            reason: "No GNSS positions provided".to_string(),
        });
    }

    if path.segments.is_empty() {
        return Err(crate::errors::ProjectionError::PathCalculationFailed {
            reason: "Path has no segments".to_string(),
        });
    }

    // Build netelement lookup map (T094)
    let netelement_map: HashMap<_, _> = netelements.iter().map(|ne| (ne.id.as_str(), ne)).collect();

    // Validate all path segments exist in netelements
    for segment in &path.segments {
        if !netelement_map.contains_key(segment.netelement_id.as_str()) {
            return Err(crate::errors::ProjectionError::PathCalculationFailed {
                reason: format!(
                    "Netelement {} in path not found in network",
                    segment.netelement_id
                ),
            });
        }
    }

    let mut projected_positions = Vec::with_capacity(gnss_positions.len());

    // Project each GNSS position onto the path
    for gnss in gnss_positions {
        // Find closest segment in path (T094)
        let mut best_distance = f64::MAX;
        let mut best_segment_idx = 0;
        let gnss_point = Point::new(gnss.longitude, gnss.latitude);

        for (idx, segment) in path.segments.iter().enumerate() {
            let netelement = netelement_map[segment.netelement_id.as_str()];

            // Project point onto this segment
            if let Ok(projected_point) =
                project_point_onto_linestring(&gnss_point, &netelement.geometry)
            {
                use geo::HaversineDistance;
                let distance = gnss_point.haversine_distance(&projected_point);

                if distance < best_distance {
                    best_distance = distance;
                    best_segment_idx = idx;
                }
            }
        }

        // Project onto best segment (T095, T096)
        let best_segment = &path.segments[best_segment_idx];
        let best_netelement = netelement_map[best_segment.netelement_id.as_str()];

        let projected_point =
            project_point_onto_linestring(&gnss_point, &best_netelement.geometry)?;

        // Calculate intrinsic coordinate (0-1 range) (T096)
        let distance_along =
            calculate_measure_along_linestring(&best_netelement.geometry, &projected_point)?;
        let total_length = best_netelement.geometry.haversine_length();

        let intrinsic = if total_length > 0.0 {
            distance_along / total_length
        } else {
            0.0
        };

        // Validate intrinsic coordinate (T100)
        if !(0.0..=1.0).contains(&intrinsic) {
            return Err(crate::errors::ProjectionError::PathCalculationFailed {
                reason: format!(
                    "Intrinsic coordinate {} outside valid range [0, 1]",
                    intrinsic
                ),
            });
        }

        // Create ProjectedPosition with intrinsic coordinate (T096)
        projected_positions.push(crate::models::ProjectedPosition::with_intrinsic(
            gnss.clone(),
            projected_point,
            best_netelement.id.clone(),
            distance_along,
            best_distance,
            gnss.crs.clone(),
            intrinsic,
        ));
    }

    Ok(projected_positions)
}
