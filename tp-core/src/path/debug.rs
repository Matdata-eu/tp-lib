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
    }

    if !debug_info.decision_tree.is_empty() {
        export_decision_tree(debug_info, dir.join("decisions.json"))?;
    }

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
