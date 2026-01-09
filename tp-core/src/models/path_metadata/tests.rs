//! Unit tests for path metadata

use super::*;
use crate::models::AssociatedNetElement;

#[test]
fn test_path_metadata_creation() {
    let metadata = PathMetadata {
        distance_scale: 50.0,
        heading_scale: 45.0,
        cutoff_distance: 100.0,
        heading_cutoff: 90.0,
        probability_threshold: 0.001,
        resampling_distance: Some(10.0),
        fallback_mode: false,
        candidate_paths_evaluated: 5,
        bidirectional_path: true,
        diagnostic_info: None,
    };

    assert_eq!(metadata.distance_scale, 50.0);
    assert_eq!(metadata.heading_scale, 45.0);
    assert_eq!(metadata.cutoff_distance, 100.0);
    assert_eq!(metadata.heading_cutoff, 90.0);
    assert_eq!(metadata.probability_threshold, 0.001);
    assert_eq!(metadata.resampling_distance, Some(10.0));
    assert!(!metadata.fallback_mode);
    assert_eq!(metadata.candidate_paths_evaluated, 5);
    assert!(metadata.bidirectional_path);
    assert!(metadata.diagnostic_info.is_none());
}

#[test]
fn test_path_metadata_with_fallback() {
    let metadata = PathMetadata {
        distance_scale: 50.0,
        heading_scale: 45.0,
        cutoff_distance: 100.0,
        heading_cutoff: 90.0,
        probability_threshold: 0.001,
        resampling_distance: None,
        fallback_mode: true,
        candidate_paths_evaluated: 0,
        bidirectional_path: false,
        diagnostic_info: None,
    };

    assert!(metadata.fallback_mode);
    assert!(!metadata.bidirectional_path);
    assert!(metadata.resampling_distance.is_none());
}

#[test]
fn test_segment_diagnostic_creation() {
    let diagnostic = SegmentDiagnostic {
        netelement_id: "NE001".to_string(),
        probability: 0.95,
        start_intrinsic: 0.0,
        end_intrinsic: 150.0,
        gnss_start_index: 0,
        gnss_end_index: 10,
    };

    assert_eq!(diagnostic.netelement_id, "NE001");
    assert_eq!(diagnostic.probability, 0.95);
    assert_eq!(diagnostic.start_intrinsic, 0.0);
    assert_eq!(diagnostic.end_intrinsic, 150.0);
    assert_eq!(diagnostic.gnss_start_index, 0);
    assert_eq!(diagnostic.gnss_end_index, 10);
}

#[test]
fn test_path_diagnostic_info_from_segments() {
    let segments = vec![
        AssociatedNetElement {
            netelement_id: "NE001".to_string(),
            start_intrinsic: 0.0,
            end_intrinsic: 100.0,
            probability: 0.9,
            gnss_start_index: 0,
            gnss_end_index: 5,
        },
        AssociatedNetElement {
            netelement_id: "NE002".to_string(),
            start_intrinsic: 0.0,
            end_intrinsic: 200.0,
            probability: 0.85,
            gnss_start_index: 6,
            gnss_end_index: 15,
        },
    ];

    let diagnostic_info = PathDiagnosticInfo::from_segments(&segments);

    assert_eq!(diagnostic_info.segments.len(), 2);
    assert_eq!(diagnostic_info.segments[0].netelement_id, "NE001");
    assert_eq!(diagnostic_info.segments[0].probability, 0.9);
    assert_eq!(diagnostic_info.segments[1].netelement_id, "NE002");
    assert_eq!(diagnostic_info.segments[1].probability, 0.85);
}

#[test]
fn test_path_diagnostic_info_empty_segments() {
    let segments = vec![];
    let diagnostic_info = PathDiagnosticInfo::from_segments(&segments);
    assert_eq!(diagnostic_info.segments.len(), 0);
}

#[test]
fn test_path_diagnostic_info_single_segment() {
    let segments = vec![AssociatedNetElement {
        netelement_id: "NE001".to_string(),
        start_intrinsic: 50.0,
        end_intrinsic: 150.0,
        probability: 0.75,
        gnss_start_index: 0,
        gnss_end_index: 20,
    }];

    let diagnostic_info = PathDiagnosticInfo::from_segments(&segments);

    assert_eq!(diagnostic_info.segments.len(), 1);
    assert_eq!(diagnostic_info.segments[0].start_intrinsic, 50.0);
    assert_eq!(diagnostic_info.segments[0].end_intrinsic, 150.0);
    assert_eq!(diagnostic_info.segments[0].gnss_start_index, 0);
    assert_eq!(diagnostic_info.segments[0].gnss_end_index, 20);
}

#[test]
fn test_path_metadata_serialization() {
    let metadata = PathMetadata {
        distance_scale: 50.0,
        heading_scale: 45.0,
        cutoff_distance: 100.0,
        heading_cutoff: 90.0,
        probability_threshold: 0.001,
        resampling_distance: Some(10.0),
        fallback_mode: false,
        candidate_paths_evaluated: 3,
        bidirectional_path: true,
        diagnostic_info: None,
    };

    let json = serde_json::to_string(&metadata).expect("Failed to serialize");
    assert!(json.contains("distance_scale"));
    assert!(json.contains("50"));

    let deserialized: PathMetadata =
        serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.distance_scale, 50.0);
    assert_eq!(deserialized.candidate_paths_evaluated, 3);
}

#[test]
fn test_path_metadata_with_diagnostic_info() {
    let segments = vec![AssociatedNetElement {
        netelement_id: "NE001".to_string(),
        start_intrinsic: 0.0,
        end_intrinsic: 100.0,
        probability: 0.9,
        gnss_start_index: 0,
        gnss_end_index: 10,
    }];

    let diagnostic_info = PathDiagnosticInfo::from_segments(&segments);

    let metadata = PathMetadata {
        distance_scale: 50.0,
        heading_scale: 45.0,
        cutoff_distance: 100.0,
        heading_cutoff: 90.0,
        probability_threshold: 0.001,
        resampling_distance: None,
        fallback_mode: false,
        candidate_paths_evaluated: 1,
        bidirectional_path: true,
        diagnostic_info: Some(diagnostic_info),
    };

    assert!(metadata.diagnostic_info.is_some());
    let diag = metadata.diagnostic_info.unwrap();
    assert_eq!(diag.segments.len(), 1);
}

#[test]
fn test_segment_diagnostic_preserves_indices() {
    let segments = vec![
        AssociatedNetElement {
            netelement_id: "NE001".to_string(),
            start_intrinsic: 0.0,
            end_intrinsic: 100.0,
            probability: 0.9,
            gnss_start_index: 5,
            gnss_end_index: 15,
        },
        AssociatedNetElement {
            netelement_id: "NE002".to_string(),
            start_intrinsic: 0.0,
            end_intrinsic: 200.0,
            probability: 0.85,
            gnss_start_index: 16,
            gnss_end_index: 30,
        },
    ];

    let diagnostic_info = PathDiagnosticInfo::from_segments(&segments);

    assert_eq!(diagnostic_info.segments[0].gnss_start_index, 5);
    assert_eq!(diagnostic_info.segments[0].gnss_end_index, 15);
    assert_eq!(diagnostic_info.segments[1].gnss_start_index, 16);
    assert_eq!(diagnostic_info.segments[1].gnss_end_index, 30);
}
