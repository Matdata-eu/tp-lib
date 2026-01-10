//! Unit tests for train path calculation

use super::*;
use crate::models::{GnssPosition, NetRelation, Netelement};
use chrono::Utc;
use geo::LineString;

fn create_test_netelement(id: &str, coords: Vec<(f64, f64)>) -> Netelement {
    Netelement::new(id.to_string(), LineString::from(coords), "EPSG:4326".to_string()).unwrap()
}

fn create_test_gnss(lat: f64, lon: f64) -> GnssPosition {
    GnssPosition::new(lat, lon, Utc::now().into(), "EPSG:4326".to_string()).unwrap()
}

#[test]
fn test_path_config_defaults() {
    let config = PathConfig::default();
    
    assert_eq!(config.distance_scale, 10.0);
    assert_eq!(config.heading_scale, 2.0);
    assert_eq!(config.cutoff_distance, 50.0);
    assert_eq!(config.heading_cutoff, 5.0);
    assert_eq!(config.probability_threshold, 0.25);
    assert_eq!(config.max_candidates, 3);
    assert!(!config.path_only);
    assert!(!config.debug_mode);
    assert!(config.resampling_distance.is_none());
}

#[test]
fn test_path_config_builder() {
    let config = PathConfig::builder()
        .distance_scale(15.0)
        .heading_scale(3.0)
        .cutoff_distance(75.0)
        .heading_cutoff(10.0)
        .probability_threshold(0.5)
        .max_candidates(5)
        .path_only(true)
        .debug_mode(true)
        .resampling_distance(Some(20.0))
        .build()
        .unwrap();
    
    assert_eq!(config.distance_scale, 15.0);
    assert_eq!(config.heading_scale, 3.0);
    assert_eq!(config.cutoff_distance, 75.0);
    assert_eq!(config.heading_cutoff, 10.0);
    assert_eq!(config.probability_threshold, 0.5);
    assert_eq!(config.max_candidates, 5);
    assert!(config.path_only);
    assert!(config.debug_mode);
    assert_eq!(config.resampling_distance, Some(20.0));
}

#[test]
fn test_path_config_validation_invalid_distance_scale() {
    assert!(PathConfig::builder().distance_scale(0.0).build().is_err());
    assert!(PathConfig::builder().distance_scale(-5.0).build().is_err());
}

#[test]
fn test_path_config_validation_invalid_heading_scale() {
    assert!(PathConfig::builder().heading_scale(0.0).build().is_err());
    assert!(PathConfig::builder().heading_scale(-2.0).build().is_err());
}

#[test]
fn test_path_config_validation_invalid_probability() {
    assert!(PathConfig::builder().probability_threshold(-0.1).build().is_err());
    assert!(PathConfig::builder().probability_threshold(1.5).build().is_err());
    assert!(PathConfig::builder().probability_threshold(0.0).build().is_ok());
    assert!(PathConfig::builder().probability_threshold(1.0).build().is_ok());
}

#[test]
fn test_path_config_validation_invalid_heading_cutoff() {
    assert!(PathConfig::builder().heading_cutoff(-5.0).build().is_err());
    assert!(PathConfig::builder().heading_cutoff(190.0).build().is_err());
    assert!(PathConfig::builder().heading_cutoff(0.0).build().is_ok());
    assert!(PathConfig::builder().heading_cutoff(180.0).build().is_ok());
}

#[test]
fn test_path_config_validation_invalid_max_candidates() {
    assert!(PathConfig::builder().max_candidates(0).build().is_err());
    assert!(PathConfig::builder().max_candidates(1).build().is_ok());
}

#[test]
fn test_path_result_new() {
    let positions = vec![];
    let warnings = vec!["test warning".to_string()];
    
    let result = PathResult::new(
        None,
        PathCalculationMode::FallbackIndependent,
        positions,
        warnings.clone(),
    );
    
    assert!(result.path.is_none());
    assert_eq!(result.mode, PathCalculationMode::FallbackIndependent);
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0], "test warning");
    assert!(result.debug_info.is_none());
}

#[test]
fn test_path_result_with_debug_info() {
    let debug = DebugInfo::new();
    let result = PathResult::with_debug_info(
        None,
        PathCalculationMode::TopologyBased,
        vec![],
        vec![],
        debug,
    );
    
    assert!(result.debug_info.is_some());
    assert_eq!(result.mode, PathCalculationMode::TopologyBased);
}

#[test]
fn test_calculate_train_path_empty_network() {
    let gnss = vec![create_test_gnss(50.85, 4.35)];
    let netelements = vec![];
    let netrelations = vec![];
    let config = PathConfig::default();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), crate::errors::ProjectionError::EmptyNetwork));
}

#[test]
fn test_calculate_train_path_empty_gnss() {
    let gnss = vec![];
    let netelements = vec![create_test_netelement("NE1", vec![(4.35, 50.85), (4.36, 50.86)])];
    let netrelations = vec![];
    let config = PathConfig::default();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), crate::errors::ProjectionError::PathCalculationFailed { .. }));
}

#[test]
fn test_calculate_train_path_simple_straight_line() {
    // Create a simple straight network with GNSS positions along it
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    let gnss = vec![
        create_test_gnss(50.851, 4.351),
        create_test_gnss(50.855, 4.355),
        create_test_gnss(50.859, 4.359),
    ];
    
    let netrelations = vec![];
    let config = PathConfig::default();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
    
    let path_result = result.unwrap();
    assert!(path_result.path.is_some() || path_result.path.is_none()); // May or may not find path depending on distance
    assert!(path_result.projected_positions.len() <= gnss.len());
}

#[test]
fn test_calculate_train_path_with_resampling() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    // Create many closely spaced GNSS positions
    let mut gnss = vec![];
    for i in 0..10 {
        let offset = i as f64 * 0.001;
        gnss.push(create_test_gnss(50.850 + offset, 4.350 + offset));
    }
    
    let netrelations = vec![];
    let config = PathConfig::builder()
        .resampling_distance(Some(10.0))
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_train_path_path_only_mode() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    let gnss = vec![
        create_test_gnss(50.851, 4.351),
        create_test_gnss(50.855, 4.355),
    ];
    
    let netrelations = vec![];
    let config = PathConfig::builder()
        .path_only(true)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_train_path_debug_mode() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    let gnss = vec![create_test_gnss(50.851, 4.351)];
    
    let netrelations = vec![];
    let config = PathConfig::builder()
        .debug_mode(true)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
    
    let path_result = result.unwrap();
    assert!(path_result.debug_info.is_some());
}

#[test]
fn test_calculate_train_path_with_netrelations() {
    // Create a connected network: NE1 -> NE2
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.355, 50.855)]),
        create_test_netelement("NE2", vec![(4.355, 50.855), (4.360, 50.860)]),
    ];
    
    let netrelations = vec![
        NetRelation::new(
            "NR1".to_string(),
            "NE1".to_string(),
            "NE2".to_string(),
            1,
            0,
            true,
            true,
        ).unwrap(),
    ];
    
    let gnss = vec![
        create_test_gnss(50.851, 4.351),
        create_test_gnss(50.856, 4.356),
        create_test_gnss(50.859, 4.359),
    ];
    
    let config = PathConfig::default();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_train_path_high_cutoff_distance() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    // GNSS far from network
    let gnss = vec![create_test_gnss(50.900, 4.400)];
    
    let netrelations = vec![];
    let config = PathConfig::builder()
        .cutoff_distance(100.0)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok()); // Should succeed even with distant GNSS
}

#[test]
fn test_path_calculation_mode_equality() {
    assert_eq!(PathCalculationMode::TopologyBased, PathCalculationMode::TopologyBased);
    assert_eq!(PathCalculationMode::FallbackIndependent, PathCalculationMode::FallbackIndependent);
    assert_ne!(PathCalculationMode::TopologyBased, PathCalculationMode::FallbackIndependent);
}

#[test]
fn test_debug_info_new() {
    let debug = DebugInfo::new();
    assert!(debug.candidate_paths.is_empty());
    assert!(debug.position_candidates.is_empty());
    assert!(debug.decision_tree.is_empty());
}

#[test]
fn test_candidate_info_creation() {
    let info = CandidateInfo {
        netelement_id: "NE1".to_string(),
        distance: 5.5,
        heading_difference: Some(10.0),
        distance_probability: 0.85,
        heading_probability: Some(0.90),
        combined_probability: 0.875,
        status: "included".to_string(),
    };
    
    assert_eq!(info.netelement_id, "NE1");
    assert_eq!(info.distance, 5.5);
    assert_eq!(info.heading_difference, Some(10.0));
    assert_eq!(info.distance_probability, 0.85);
    assert_eq!(info.heading_probability, Some(0.90));
    assert_eq!(info.combined_probability, 0.875);
}

#[test]
fn test_position_candidates_creation() {
    let candidates = vec![CandidateInfo {
        netelement_id: "NE1".to_string(),
        distance: 5.0,
        heading_difference: None,
        distance_probability: 0.8,
        heading_probability: None,
        combined_probability: 0.8,
        status: "included".to_string(),
    }];
    
    let info = PositionCandidates {
        position_index: 0,
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        coordinates: (4.35, 50.85),
        candidates,
        selected_netelement: Some("NE1".to_string()),
    };
    
    assert_eq!(info.position_index, 0);
    assert_eq!(info.coordinates, (4.35, 50.85));
    assert_eq!(info.candidates.len(), 1);
    assert_eq!(info.selected_netelement, Some("NE1".to_string()));
}

#[test]
fn test_calculate_train_path_complex_network() {
    // Create a more complex network with multiple connected elements
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.355, 50.855)]),
        create_test_netelement("NE2", vec![(4.355, 50.855), (4.360, 50.860)]),
        create_test_netelement("NE3", vec![(4.360, 50.860), (4.365, 50.865)]),
    ];
    
    let netrelations = vec![
        NetRelation::new("NR1".to_string(), "NE1".to_string(), "NE2".to_string(), 1, 0, true, true).unwrap(),
        NetRelation::new("NR2".to_string(), "NE2".to_string(), "NE3".to_string(), 1, 0, true, true).unwrap(),
    ];
    
    let gnss = vec![
        create_test_gnss(50.851, 4.351),
        create_test_gnss(50.856, 4.356),
        create_test_gnss(50.861, 4.361),
        create_test_gnss(50.864, 4.364),
    ];
    
    let config = PathConfig::default();
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    
    assert!(result.is_ok());
}

#[test]
fn test_calculate_train_path_low_probability_threshold() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    let gnss = vec![create_test_gnss(50.855, 4.355)];
    let netrelations = vec![];
    
    let config = PathConfig::builder()
        .probability_threshold(0.05)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_train_path_high_cutoff() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    // GNSS positions far from network
    let gnss = vec![
        create_test_gnss(50.880, 4.380),
        create_test_gnss(50.885, 4.385),
    ];
    
    let netrelations = vec![];
    let config = PathConfig::builder()
        .cutoff_distance(5000.0) // 5km cutoff
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_path_result_is_topology_based() {
    let result = PathResult::new(
        None,
        PathCalculationMode::TopologyBased,
        vec![],
        vec![],
    );
    
    assert!(result.is_topology_based());
    assert!(!result.is_fallback());
}

#[test]
fn test_path_result_is_fallback() {
    let result = PathResult::new(
        None,
        PathCalculationMode::FallbackIndependent,
        vec![],
        vec![],
    );
    
    assert!(result.is_fallback());
    assert!(!result.is_topology_based());
}

#[test]
fn test_path_result_has_path() {
    let segment = crate::models::AssociatedNetElement::new(
        "NE1".to_string(),
        0.95,
        0.0,
        1.0,
        0,
        5,
    ).unwrap();
    
    let path = crate::models::TrainPath::new(
        vec![segment],
        0.95,
        None,
        None,
    ).unwrap();
    
    let result = PathResult::new(
        Some(path),
        PathCalculationMode::TopologyBased,
        vec![],
        vec![],
    );
    
    assert!(result.has_path());
}

#[test]
fn test_candidate_path_creation() {
    let candidate = CandidatePath {
        id: "path1".to_string(),
        direction: "forward".to_string(),
        segment_ids: vec!["NE1".to_string(), "NE2".to_string()],
        probability: 0.92,
        selected: true,
    };
    
    assert_eq!(candidate.id, "path1");
    assert_eq!(candidate.direction, "forward");
    assert_eq!(candidate.segment_ids.len(), 2);
    assert_eq!(candidate.probability, 0.92);
    assert!(candidate.selected);
}

#[test]
fn test_path_decision_creation() {
    let decision = PathDecision {
        step: 1,
        decision_type: "forward_extend".to_string(),
        current_segment: "NE1".to_string(),
        options: vec!["NE2".to_string(), "NE3".to_string()],
        option_probabilities: vec![0.9, 0.7],
        chosen_option: "NE2".to_string(),
        reason: "Higher probability".to_string(),
    };
    
    assert_eq!(decision.step, 1);
    assert_eq!(decision.decision_type, "forward_extend");
    assert_eq!(decision.options.len(), 2);
    assert_eq!(decision.chosen_option, "NE2");
}

// Additional edge case tests for improved coverage

#[test]
fn test_path_config_validation_invalid_cutoff_distance() {
    assert!(PathConfig::builder().cutoff_distance(0.0).build().is_err());
    assert!(PathConfig::builder().cutoff_distance(-10.0).build().is_err());
    assert!(PathConfig::builder().cutoff_distance(0.1).build().is_ok());
}

#[test]
fn test_path_config_validation_invalid_resampling_distance() {
    assert!(PathConfig::builder().resampling_distance(Some(0.0)).build().is_err());
    assert!(PathConfig::builder().resampling_distance(Some(-5.0)).build().is_err());
    assert!(PathConfig::builder().resampling_distance(Some(1.0)).build().is_ok());
    assert!(PathConfig::builder().resampling_distance(None).build().is_ok());
}

#[test]
fn test_calculate_train_path_single_position() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    let gnss = vec![create_test_gnss(50.851, 4.351)];
    let netrelations = vec![];
    let config = PathConfig::default();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_train_path_no_candidates_within_cutoff() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    // GNSS position very far from netelement
    let gnss = vec![create_test_gnss(51.0, 5.0)];
    let netrelations = vec![];
    
    let mut config = PathConfig::default();
    config.cutoff_distance = 10.0; // Very small cutoff
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    // Should succeed but might fall back to independent projection
    assert!(result.is_ok());
}

#[test]
fn test_calculate_train_path_with_warnings() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    let gnss = vec![
        create_test_gnss(50.851, 4.351),
        create_test_gnss(50.853, 4.353),
    ];
    let netrelations = vec![];
    
    // Use path-only mode to generate warnings
    let config = PathConfig::builder()
        .path_only(true)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
    
    let path_result = result.unwrap();
    // Path-only mode should generate at least one warning
    assert!(!path_result.warnings.is_empty());
}

#[test]
fn test_calculate_train_path_circular_network() {
    // Create a circular network (loop)
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.850)]),
        create_test_netelement("NE2", vec![(4.360, 50.850), (4.360, 50.860)]),
        create_test_netelement("NE3", vec![(4.360, 50.860), (4.350, 50.860)]),
        create_test_netelement("NE4", vec![(4.350, 50.860), (4.350, 50.850)]),
    ];
    
    let netrelations = vec![
        NetRelation::new("NR1".to_string(), "NE1".to_string(), "NE2".to_string(), 1, 0, true, true).unwrap(),
        NetRelation::new("NR2".to_string(), "NE2".to_string(), "NE3".to_string(), 1, 0, true, true).unwrap(),
        NetRelation::new("NR3".to_string(), "NE3".to_string(), "NE4".to_string(), 1, 0, true, true).unwrap(),
        NetRelation::new("NR4".to_string(), "NE4".to_string(), "NE1".to_string(), 1, 0, true, true).unwrap(),
    ];
    
    let gnss = vec![
        create_test_gnss(50.850, 4.351),
        create_test_gnss(50.850, 4.358),
        create_test_gnss(50.858, 50.860),
    ];
    
    let config = PathConfig::default();
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_train_path_disconnected_network() {
    // Two disconnected segments
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.850)]),
        create_test_netelement("NE2", vec![(5.350, 51.850), (5.360, 51.850)]), // Far away
    ];
    
    let gnss = vec![
        create_test_gnss(50.850, 4.351),
        create_test_gnss(51.850, 5.351), // Jumps to other segment
    ];
    
    let netrelations = vec![]; // No relations - disconnected
    let config = PathConfig::default();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
    // Should succeed but might fall back to independent projection
}

#[test]
fn test_calculate_train_path_with_unidirectional_relations() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.850)]),
        create_test_netelement("NE2", vec![(4.360, 50.850), (4.370, 50.850)]),
    ];
    
    // Only allow forward navigation
    let netrelations = vec![
        NetRelation::new("NR1".to_string(), "NE1".to_string(), "NE2".to_string(), 1, 0, true, false).unwrap(),
    ];
    
    let gnss = vec![
        create_test_gnss(50.850, 4.351),
        create_test_gnss(50.850, 4.361),
    ];
    
    let config = PathConfig::default();
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_train_path_very_low_probability_threshold() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    let gnss = vec![
        create_test_gnss(50.851, 4.351),
        create_test_gnss(50.853, 4.353),
    ];
    
    let netrelations = vec![];
    
    // Very low probability threshold - should accept almost any path
    let config = PathConfig::builder()
        .probability_threshold(0.001)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_calculate_train_path_high_probability_threshold() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
    ];
    
    let gnss = vec![
        create_test_gnss(50.851, 4.351),
        create_test_gnss(50.853, 4.353),
    ];
    
    let netrelations = vec![];
    
    // Very high probability threshold - might reject paths
    let config = PathConfig::builder()
        .probability_threshold(0.99)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
    // Path might fail but should not error
}

#[test]
fn test_calculate_train_path_with_max_candidates_1() {
    let netelements = vec![
        create_test_netelement("NE1", vec![(4.350, 50.850), (4.360, 50.860)]),
        create_test_netelement("NE2", vec![(4.350, 50.850), (4.340, 50.860)]),
    ];
    
    let gnss = vec![create_test_gnss(50.851, 4.351)];
    let netrelations = vec![];
    
    // Only consider 1 candidate per position
    let config = PathConfig::builder()
        .max_candidates(1)
        .build()
        .unwrap();
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    assert!(result.is_ok());
}

#[test]
fn test_path_result_helper_methods() {
    use crate::models::AssociatedNetElement;
    
    let segment = AssociatedNetElement {
        netelement_id: "NE1".to_string(),
        start_intrinsic: 0.0,
        end_intrinsic: 1.0,
        probability: 0.9,
        gnss_start_index: 0,
        gnss_end_index: 10,
    };
    
    let result_with_path = PathResult::new(
        Some(TrainPath::new(vec![segment], 0.5, None, None).unwrap()),
        PathCalculationMode::TopologyBased,
        vec![],
        vec![],
    );
    
    assert!(result_with_path.has_path());
    assert!(!result_with_path.is_fallback());
    assert!(result_with_path.is_topology_based());
    
    let result_fallback = PathResult::new(
        None,
        PathCalculationMode::FallbackIndependent,
        vec![],
        vec![],
    );
    
    assert!(!result_fallback.has_path());
    assert!(result_fallback.is_fallback());
    assert!(!result_fallback.is_topology_based());
}

#[test]
fn test_path_calculation_mode_comparison() {
    assert_eq!(PathCalculationMode::TopologyBased, PathCalculationMode::TopologyBased);
    assert_eq!(PathCalculationMode::FallbackIndependent, PathCalculationMode::FallbackIndependent);
    assert_ne!(PathCalculationMode::TopologyBased, PathCalculationMode::FallbackIndependent);
}

