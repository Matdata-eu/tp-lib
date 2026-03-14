//! Debug information export utilities for path calculation (US7)
//!
//! This module provides functions to export intermediate calculation results
//! for troubleshooting and parameter tuning.

use crate::errors::ProjectionError;
use crate::path::DebugInfo;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Export candidate paths to a JSON file (T154)
///
/// Writes all candidate paths evaluated during path construction to a JSON file.
///
/// # Arguments
///
/// * `debug_info` - Debug info containing candidate paths
/// * `output_path` - Path to write the JSON file
///
/// # Returns
///
/// Ok(()) if export succeeds, Err if file writing fails
pub fn export_candidate_paths<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    let json = serde_json::to_string_pretty(&debug_info.candidate_paths).map_err(|e| {
        ProjectionError::InvalidGeometry(format!("Failed to serialize candidate paths: {}", e))
    })?;

    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

/// Export position candidates to a JSON file (T155)
///
/// Writes candidate netelements and probabilities for each GNSS coordinate.
///
/// # Arguments
///
/// * `debug_info` - Debug info containing position candidates
/// * `output_path` - Path to write the JSON file
///
/// # Returns
///
/// Ok(()) if export succeeds, Err if file writing fails
pub fn export_position_candidates<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    let json = serde_json::to_string_pretty(&debug_info.position_candidates).map_err(|e| {
        ProjectionError::InvalidGeometry(format!("Failed to serialize position candidates: {}", e))
    })?;

    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

/// Export decision tree to a JSON file (T156)
///
/// Writes the path selection decision tree showing bidirectional averaging
/// and final path selection reasoning.
///
/// # Arguments
///
/// * `debug_info` - Debug info containing decision tree
/// * `output_path` - Path to write the JSON file
///
/// # Returns
///
/// Ok(()) if export succeeds, Err if file writing fails
pub fn export_decision_tree<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    let json = serde_json::to_string_pretty(&debug_info.decision_tree).map_err(|e| {
        ProjectionError::InvalidGeometry(format!("Failed to serialize decision tree: {}", e))
    })?;

    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;

    Ok(())
}

/// Export all debug information to separate files (T158)
///
/// Convenience function that exports all debug info to a directory:
/// - candidates.json - All candidate paths with probabilities
/// - positions.json - Position candidates per GNSS coordinate  
/// - decisions.json - Decision tree showing path selection
///
/// # Arguments
///
/// * `debug_info` - Debug info to export
/// * `output_dir` - Directory to write files to
///
/// # Returns
///
/// Ok(()) if all exports succeed, Err if any fails
pub fn export_all_debug_info<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_dir: P,
) -> Result<(), ProjectionError> {
    let dir = output_dir.as_ref();

    // Create directory if it doesn't exist
    std::fs::create_dir_all(dir)?;

    // Export each component
    if !debug_info.candidate_paths.is_empty() {
        export_candidate_paths(debug_info, dir.join("candidates.json"))?;
    }

    if !debug_info.position_candidates.is_empty() {
        export_position_candidates(debug_info, dir.join("positions.json"))?;
        export_phase2_geojson(debug_info, dir.join("phase2_candidates.geojson"))?;
    }

    if !debug_info.decision_tree.is_empty() {
        export_decision_tree(debug_info, dir.join("decisions.json"))?;
    }

    if !debug_info.netelement_probabilities.is_empty() {
        export_phase3_geojson(debug_info, dir.join("phase3_netelements.geojson"))?;
        export_phase4_geojson(debug_info, dir.join("phase4_netelement_map.geojson"))?;
    }

    Ok(())
}

/// Export Phase 2 (GNSS-level probability) debug data as GeoJSON
///
/// Produces a FeatureCollection with:
/// - Point features for each GNSS position
/// - LineString features from each GNSS position to its candidate projections
pub fn export_phase2_geojson<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::{Map, Value as JsonValue};

    let mut features = Vec::new();

    for pos in &debug_info.position_candidates {
        // Point feature for the GNSS position
        let point_geom = Geometry::new(Value::Point(vec![pos.coordinates.1, pos.coordinates.0]));
        let mut props = Map::new();
        props.insert("feature_type".to_string(), JsonValue::from("gnss_position"));
        props.insert(
            "position_index".to_string(),
            JsonValue::from(pos.position_index as i64),
        );
        props.insert(
            "timestamp".to_string(),
            JsonValue::from(pos.timestamp.clone()),
        );
        if let Some(ref sel) = pos.selected_netelement {
            props.insert(
                "selected_netelement".to_string(),
                JsonValue::from(sel.clone()),
            );
        }
        features.push(Feature {
            bbox: None,
            geometry: Some(point_geom),
            id: None,
            properties: Some(props),
            foreign_members: None,
        });

        // LineString features from GNSS to each candidate projection
        for candidate in &pos.candidates {
            let line_geom = Geometry::new(Value::LineString(vec![
                vec![pos.coordinates.1, pos.coordinates.0],
                vec![candidate.projected_lon, candidate.projected_lat],
            ]));
            let mut line_props = Map::new();
            line_props.insert(
                "feature_type".to_string(),
                JsonValue::from("projection_line"),
            );
            line_props.insert(
                "position_index".to_string(),
                JsonValue::from(pos.position_index as i64),
            );
            line_props.insert(
                "netelement_id".to_string(),
                JsonValue::from(candidate.netelement_id.clone()),
            );
            line_props.insert(
                "distance".to_string(),
                JsonValue::from(candidate.distance),
            );
            if let Some(hd) = candidate.heading_difference {
                line_props.insert("heading_difference".to_string(), JsonValue::from(hd));
            }
            line_props.insert(
                "distance_probability".to_string(),
                JsonValue::from(candidate.distance_probability),
            );
            if let Some(hp) = candidate.heading_probability {
                line_props.insert("heading_probability".to_string(), JsonValue::from(hp));
            }
            line_props.insert(
                "combined_probability".to_string(),
                JsonValue::from(candidate.combined_probability),
            );
            line_props.insert(
                "status".to_string(),
                JsonValue::from(candidate.status.clone()),
            );
            features.push(Feature {
                bbox: None,
                geometry: Some(line_geom),
                id: None,
                properties: Some(line_props),
                foreign_members: None,
            });
        }
    }

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };
    let json = serde_json::to_string_pretty(&fc).map_err(|e| {
        ProjectionError::InvalidGeometry(format!("Failed to serialize phase2 GeoJSON: {}", e))
    })?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Export Phase 3 (netelement-level probability) debug data as GeoJSON
///
/// Produces a FeatureCollection with LineString features for every netelement
/// that had a non-zero probability, with aggregated probability properties.
pub fn export_phase3_geojson<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::{Map, Value as JsonValue};

    let mut features = Vec::new();

    for ne in &debug_info.netelement_probabilities {
        if ne.geometry_coords.len() < 2 {
            continue;
        }
        let geom = Geometry::new(Value::LineString(ne.geometry_coords.clone()));
        let mut props = Map::new();
        props.insert(
            "netelement_id".to_string(),
            JsonValue::from(ne.netelement_id.clone()),
        );
        props.insert(
            "coverage_probability".to_string(),
            JsonValue::from(ne.coverage_probability),
        );
        props.insert(
            "avg_probability".to_string(),
            JsonValue::from(ne.avg_probability),
        );
        props.insert(
            "position_count".to_string(),
            JsonValue::from(ne.position_count as i64),
        );
        props.insert(
            "in_netelement_map".to_string(),
            JsonValue::from(ne.in_netelement_map),
        );
        features.push(Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(props),
            foreign_members: None,
        });
    }

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };
    let json = serde_json::to_string_pretty(&fc).map_err(|e| {
        ProjectionError::InvalidGeometry(format!("Failed to serialize phase3 GeoJSON: {}", e))
    })?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Export Phase 4 (netelement_map) debug data as GeoJSON
///
/// Produces a FeatureCollection with LineString features only for netelements
/// that passed the probability threshold and were included in the netelement_map.
pub fn export_phase4_geojson<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::{Map, Value as JsonValue};

    let mut features = Vec::new();

    for ne in &debug_info.netelement_probabilities {
        if !ne.in_netelement_map {
            continue;
        }
        if ne.geometry_coords.len() < 2 {
            continue;
        }
        let geom = Geometry::new(Value::LineString(ne.geometry_coords.clone()));
        let mut props = Map::new();
        props.insert(
            "netelement_id".to_string(),
            JsonValue::from(ne.netelement_id.clone()),
        );
        props.insert(
            "coverage_probability".to_string(),
            JsonValue::from(ne.coverage_probability),
        );
        props.insert(
            "avg_probability".to_string(),
            JsonValue::from(ne.avg_probability),
        );
        props.insert(
            "position_count".to_string(),
            JsonValue::from(ne.position_count as i64),
        );
        features.push(Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(props),
            foreign_members: None,
        });
    }

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };
    let json = serde_json::to_string_pretty(&fc).map_err(|e| {
        ProjectionError::InvalidGeometry(format!("Failed to serialize phase4 GeoJSON: {}", e))
    })?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::{CandidateInfo, CandidatePath, PathDecision, PositionCandidates};
    use std::io::Read;

    #[test]
    fn test_export_candidate_paths() {
        let mut debug_info = DebugInfo::new();
        debug_info.add_candidate_path(CandidatePath {
            id: "test_path".to_string(),
            direction: "forward".to_string(),
            segment_ids: vec!["NE_A".to_string(), "NE_B".to_string()],
            probability: 0.85,
            selected: true,
        });

        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_candidates.json");

        let result = export_candidate_paths(&debug_info, &output_path);
        assert!(result.is_ok());

        // Verify file contents
        let mut file = File::open(&output_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("test_path"));
        assert!(contents.contains("NE_A"));

        // Cleanup
        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_position_candidates() {
        let mut debug_info = DebugInfo::new();
        debug_info.add_position_candidates(PositionCandidates {
            position_index: 0,
            timestamp: "2025-01-09T12:00:00Z".to_string(),
            coordinates: (50.85, 4.35),
            candidates: vec![CandidateInfo {
                netelement_id: "NE_A".to_string(),
                distance: 5.0,
                heading_difference: Some(2.0),
                distance_probability: 0.9,
                heading_probability: Some(0.8),
                combined_probability: 0.72,
                status: "selected".to_string(),
                projected_lat: 50.851,
                projected_lon: 4.351,
            }],
            selected_netelement: Some("NE_A".to_string()),
        });

        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_positions.json");

        let result = export_position_candidates(&debug_info, &output_path);
        assert!(result.is_ok());

        // Verify file contents
        let mut file = File::open(&output_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("position_index"));
        assert!(contents.contains("NE_A"));

        // Cleanup
        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_decision_tree() {
        let mut debug_info = DebugInfo::new();
        debug_info.add_decision(PathDecision {
            step: 1,
            decision_type: "forward_extend".to_string(),
            current_segment: "NE_A".to_string(),
            options: vec!["NE_B".to_string()],
            option_probabilities: vec![0.85],
            chosen_option: "NE_B".to_string(),
            reason: "Highest probability".to_string(),
        });

        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_decisions.json");

        let result = export_decision_tree(&debug_info, &output_path);
        assert!(result.is_ok());

        // Verify file contents
        let mut file = File::open(&output_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("forward_extend"));
        assert!(contents.contains("Highest probability"));

        // Cleanup
        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_all_debug_info() {
        let mut debug_info = DebugInfo::new();
        debug_info.add_candidate_path(CandidatePath {
            id: "path_1".to_string(),
            direction: "forward".to_string(),
            segment_ids: vec!["NE_A".to_string()],
            probability: 0.9,
            selected: true,
        });
        debug_info.add_decision(PathDecision {
            step: 1,
            decision_type: "select".to_string(),
            current_segment: "NE_A".to_string(),
            options: vec!["NE_A".to_string()],
            option_probabilities: vec![0.9],
            chosen_option: "NE_A".to_string(),
            reason: "Only option".to_string(),
        });

        let temp_dir = std::env::temp_dir().join("tp_debug_test");

        let result = export_all_debug_info(&debug_info, &temp_dir);
        assert!(result.is_ok());

        // Verify files created
        assert!(temp_dir.join("candidates.json").exists());
        assert!(temp_dir.join("decisions.json").exists());

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
