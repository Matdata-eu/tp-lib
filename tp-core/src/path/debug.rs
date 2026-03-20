//! Debug information export utilities for path calculation (US7)
//!
//! This module provides functions to export intermediate HMM calculation results
//! for troubleshooting and parameter tuning.
//!
//! Output files are numbered by phase:
//! 1. `01_emission_probabilities.geojson` — Emission probabilities: links from each GNSS
//!    position to its candidate netelements with distance / heading probabilities.
//! 2. `02_transition_probabilities.geojson` — Transition probabilities between every
//!    feasible (non-zero) candidate pair across consecutive GNSS steps.
//! 3. `03_viterbi_trace.geojson` — Viterbi decoding trace: the netelement selected at
//!    each observation step.
//! 4. `04_candidate_netelements.geojson` — All candidate netelements with aggregate
//!    emission probabilities and Viterbi membership flag.
//! 5. `05_path_sanity_decisions.geojson` — Post-Viterbi navigability sanity check
//!    decisions for each consecutive segment pair.
//! 6. `06_filling_gaps.geojson` — Gap-fill decisions: bridge netelements inserted
//!    between disconnected consecutive segments after sanity validation.
//! 7. `07_selected_path.geojson` — Only the netelements that form the final validated
//!    path (including bridge segments).
use crate::errors::ProjectionError;
use crate::path::DebugInfo;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Export all HMM debug information to numbered GeoJSON files (T158)
///
/// Writes seven phase-numbered files to `output_dir`:
/// - `01_emission_probabilities.geojson`
/// - `02_transition_probabilities.geojson`
/// - `03_viterbi_trace.geojson`
/// - `04_candidate_netelements.geojson`
/// - `05_path_sanity_decisions.geojson`
/// - `06_filling_gaps.geojson`
/// - `07_selected_path.geojson`
pub fn export_all_debug_info<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_dir: P,
) -> Result<(), ProjectionError> {
    let dir = output_dir.as_ref();
    std::fs::create_dir_all(dir)?;

    if !debug_info.position_candidates.is_empty() {
        export_hmm_emission_probabilities(
            debug_info,
            dir.join("01_emission_probabilities.geojson"),
        )?;
    }

    if !debug_info.decision_tree.is_empty() {
        export_hmm_viterbi_trace(debug_info, dir.join("03_viterbi_trace.geojson"))?;
    }

    if !debug_info.netelement_probabilities.is_empty() {
        export_hmm_candidate_netelements(
            debug_info,
            dir.join("04_candidate_netelements.geojson"),
        )?;
        export_hmm_selected_path(debug_info, dir.join("07_selected_path.geojson"))?;
    }

    if !debug_info.sanity_decisions.is_empty() {
        export_path_sanity_decisions(
            debug_info,
            dir.join("05_path_sanity_decisions.geojson"),
        )?;
    }

    if !debug_info.gap_fills.is_empty() {
        export_gap_fills(
            debug_info,
            dir.join("06_filling_gaps.geojson"),
        )?;
    }

    if !debug_info.transition_probabilities.is_empty() {
        export_hmm_transition_probabilities(
            debug_info,
            dir.join("02_transition_probabilities.geojson"),
        )?;
    }

    Ok(())
}

/// Export Phase 1 â€“ HMM emission probabilities as GeoJSON
///
/// Produces a FeatureCollection with one LineString per GNSS-position Ã— candidate
/// netelement pair, recording the emission probability components so that the HMM
/// observation model can be inspected spatially.
///
/// Properties per feature:
/// - `step`                   â€“ GNSS position index (0-based)
/// - `netelement_id`          â€“ candidate netelement
/// - `emission_probability`   â€“ combined (distance Ã— heading) emission probability
/// - `distance_probability`   â€“ distance component
/// - `distance_m`             â€“ absolute distance in metres
/// - `heading_probability`    â€“ heading component (omitted when unavailable)
/// - `heading_difference_deg` â€“ absolute heading difference in degrees (omitted when unavailable)
/// - `status`                 â€“ `"selected"`, `"candidate"`, or `"rejected"`
pub fn export_hmm_emission_probabilities<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::{Map, Value as JsonValue};

    let mut features = Vec::new();

    for pos in &debug_info.position_candidates {
        for candidate in &pos.candidates {
            let line_geom = Geometry::new(Value::LineString(vec![
                vec![pos.coordinates.1, pos.coordinates.0],
                vec![candidate.projected_lon, candidate.projected_lat],
            ]));
            let mut props = Map::new();
            props.insert(
                "step".to_string(),
                JsonValue::from(pos.position_index as i64),
            );
            props.insert(
                "netelement_id".to_string(),
                JsonValue::from(candidate.netelement_id.clone()),
            );
            props.insert(
                "emission_probability".to_string(),
                JsonValue::from(candidate.combined_probability),
            );
            props.insert(
                "distance_probability".to_string(),
                JsonValue::from(candidate.distance_probability),
            );
            props.insert("distance_m".to_string(), JsonValue::from(candidate.distance));
            if let Some(hp) = candidate.heading_probability {
                props.insert("heading_probability".to_string(), JsonValue::from(hp));
            }
            if let Some(hd) = candidate.heading_difference {
                props.insert(
                    "heading_difference_deg".to_string(),
                    JsonValue::from(hd),
                );
            }
            props.insert(
                "status".to_string(),
                JsonValue::from(candidate.status.clone()),
            );
            features.push(Feature {
                bbox: None,
                geometry: Some(line_geom),
                id: None,
                properties: Some(props),
                foreign_members: None,
            });
        }
    }

    let mut fc_members = serde_json::Map::new();
    fc_members.insert("phase".to_string(), JsonValue::from(1i64));
    fc_members.insert(
        "description".to_string(),
        JsonValue::from(
            "HMM emission probabilities: links from each GNSS position to its candidate netelements",
        ),
    );

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: Some(fc_members),
    };
    let json = serde_json::to_string_pretty(&fc).map_err(|e| {
        ProjectionError::InvalidGeometry(format!(
            "Failed to serialize emission probabilities GeoJSON: {}",
            e
        ))
    })?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Export Phase 3 â€“ Viterbi decoding trace as GeoJSON
///
/// Produces a FeatureCollection with one LineString feature per HMM decoding step
/// (one per GNSS observation), linking the raw GNSS point to the projected point on
/// the netelement chosen by the Viterbi algorithm at that step.  Features with no
/// matching candidate are emitted with `null` geometry so they still appear in
/// attribute tables.
///
/// Properties per feature:
/// - `step`                 â€" observation index (0-based)
/// - `netelement_id`        â€" the netelement chosen at this step
/// - `decision_type`        â€" type of Viterbi event (`"viterbi_init"` or `"viterbi_transition"`)
/// - `selected_probability` â€" emission probability of the chosen candidate (when available)
/// - `alternatives_count`   â€“ number of alternatives considered
/// - `reason`               â€“ human-readable selection rationale
pub fn export_hmm_viterbi_trace<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::{Map, Value as JsonValue};

    let pos_lookup: std::collections::HashMap<usize, &crate::path::PositionCandidates> =
        debug_info
            .position_candidates
            .iter()
            .map(|pc| (pc.position_index, pc))
            .collect();

    let mut features = Vec::new();

    for decision in &debug_info.decision_tree {
        let (geometry, selected_probability) = match pos_lookup.get(&decision.step) {
            Some(pos) => {
                match pos
                    .candidates
                    .iter()
                    .find(|c| c.netelement_id == decision.chosen_option)
                {
                    Some(c) => {
                        let geom = Geometry::new(Value::LineString(vec![
                            vec![pos.coordinates.1, pos.coordinates.0],
                            vec![c.projected_lon, c.projected_lat],
                        ]));
                        (Some(geom), Some(c.combined_probability))
                    }
                    None => (None, None),
                }
            }
            None => (None, None),
        };

        let mut props = Map::new();
        props.insert("step".to_string(), JsonValue::from(decision.step as i64));
        props.insert(
            "netelement_id".to_string(),
            JsonValue::from(decision.chosen_option.clone()),
        );
        props.insert(
            "decision_type".to_string(),
            JsonValue::from(decision.decision_type.clone()),
        );
        if let Some(prob) = selected_probability {
            props.insert("selected_probability".to_string(), JsonValue::from(prob));
        }
        props.insert(
            "alternatives_count".to_string(),
            JsonValue::from(decision.options.len() as i64),
        );
        props.insert(
            "reason".to_string(),
            JsonValue::from(decision.reason.clone()),
        );

        features.push(Feature {
            bbox: None,
            geometry,
            id: None,
            properties: Some(props),
            foreign_members: None,
        });
    }

    let mut fc_members = serde_json::Map::new();
    fc_members.insert("phase".to_string(), JsonValue::from(3i64));
    fc_members.insert(
        "description".to_string(),
        JsonValue::from(
            "HMM Viterbi decoding trace: links from each GNSS position to the chosen netelement",
        ),
    );

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: Some(fc_members),
    };
    let json = serde_json::to_string_pretty(&fc).map_err(|e| {
        ProjectionError::InvalidGeometry(format!(
            "Failed to serialize Viterbi trace GeoJSON: {}",
            e
        ))
    })?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Export Phase 4 â€“ All candidate netelements with aggregate probabilities as GeoJSON
///
/// Produces a FeatureCollection with LineString features for every netelement that was
/// considered as an HMM candidate state, annotated with aggregate emission probabilities
/// and a flag indicating Viterbi path membership.
///
/// Properties per feature:
/// - `netelement_id`            â€“ netelement identifier
/// - `avg_emission_probability` â€“ average emission probability across matched positions
/// - `position_count`           â€“ number of GNSS positions for which this was a candidate
/// - `in_viterbi_path`          â€“ whether this netelement is part of the decoded path
/// - `is_bridge`                â€“ whether this segment was inserted as a topological bridge
pub fn export_hmm_candidate_netelements<P: AsRef<Path>>(
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
            "avg_emission_probability".to_string(),
            JsonValue::from(ne.avg_emission_probability),
        );
        props.insert(
            "position_count".to_string(),
            JsonValue::from(ne.position_count as i64),
        );
        props.insert(
            "in_viterbi_path".to_string(),
            JsonValue::from(ne.in_viterbi_path),
        );
        props.insert("is_bridge".to_string(), JsonValue::from(ne.is_bridge));
        features.push(Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(props),
            foreign_members: None,
        });
    }

    let mut fc_members = serde_json::Map::new();
    fc_members.insert("phase".to_string(), JsonValue::from(4i64));
    fc_members.insert(
        "description".to_string(),
        JsonValue::from(
            "HMM candidate netelements: all states considered during Viterbi decoding",
        ),
    );

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: Some(fc_members),
    };
    let json = serde_json::to_string_pretty(&fc).map_err(|e| {
        ProjectionError::InvalidGeometry(format!(
            "Failed to serialize candidate netelements GeoJSON: {}",
            e
        ))
    })?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Export Phase 5 â€“ Selected Viterbi path netelements as GeoJSON
///
/// Produces a FeatureCollection with LineString features only for the netelements
/// that appear in the final Viterbi path (including topological bridge segments).
///
/// Properties per feature:
/// - `netelement_id`            â€“ netelement identifier
/// - `avg_emission_probability` â€“ average emission probability (0 for bridges)
/// - `position_count`           â€“ number of GNSS positions associated (0 for bridges)
/// - `is_bridge`                â€“ whether this segment is a topological bridge
pub fn export_hmm_selected_path<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::{Map, Value as JsonValue};

    let mut features = Vec::new();

    for ne in &debug_info.netelement_probabilities {
        if !ne.in_viterbi_path {
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
            "avg_emission_probability".to_string(),
            JsonValue::from(ne.avg_emission_probability),
        );
        props.insert(
            "position_count".to_string(),
            JsonValue::from(ne.position_count as i64),
        );
        props.insert("is_bridge".to_string(), JsonValue::from(ne.is_bridge));
        features.push(Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(props),
            foreign_members: None,
        });
    }

    let mut fc_members = serde_json::Map::new();
    fc_members.insert("phase".to_string(), JsonValue::from(6i64));
    fc_members.insert(
        "description".to_string(),
        JsonValue::from(
            "HMM selected path: netelements in the final validated path",
        ),
    );

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: Some(fc_members),
    };
    let json = serde_json::to_string_pretty(&fc).map_err(|e| {
        ProjectionError::InvalidGeometry(format!(
            "Failed to serialize selected path GeoJSON: {}",
            e
        ))
    })?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Export Phase 2 â€" HMM transition probabilities as GeoJSON
///
/// Produces a FeatureCollection with one LineString feature per feasible
/// (non-zero) candidate-pair transition across consecutive GNSS observations.
/// Each feature links the projected point of the preceding candidate to the
/// projected point of the succeeding candidate, so that connectivity gaps
/// and long transitions stand out visually.
///
/// Only transitions with a non-zero probability are included; impossible
/// transitions (disconnected network paths, edge-zone constraints) are
/// omitted.
///
/// Properties per feature:
/// - `from_step`              â€" observation index of the preceding position (0-based)
/// - `to_step`                â€" observation index of the succeeding position (0-based)
/// - `from_netelement_id`     â€" netelement of the preceding candidate
/// - `to_netelement_id`       â€" netelement of the succeeding candidate
/// - `transition_probability` â€" linear-scale transition probability [0, 1]
/// - `is_viterbi_chosen`      â€" whether this pair was chosen by the Viterbi algorithm
pub fn export_hmm_transition_probabilities<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::{Map, Value as JsonValue};

    // Build lookup: (position_index, netelement_id) -> (projected_lon, projected_lat)
    let mut point_lookup: std::collections::HashMap<(usize, &str), (f64, f64)> =
        std::collections::HashMap::new();
    for pos in &debug_info.position_candidates {
        for c in &pos.candidates {
            point_lookup.insert(
                (pos.position_index, c.netelement_id.as_str()),
                (c.projected_lon, c.projected_lat),
            );
        }
    }

    let mut features = Vec::new();

    for entry in &debug_info.transition_probabilities {
        let from_pt = point_lookup.get(&(entry.from_step, entry.from_netelement_id.as_str()));
        let to_pt = point_lookup.get(&(entry.to_step, entry.to_netelement_id.as_str()));
        let geometry = match (from_pt, to_pt) {
            (Some(&(from_lon, from_lat)), Some(&(to_lon, to_lat))) => {
                Some(Geometry::new(Value::LineString(vec![
                    vec![from_lon, from_lat],
                    vec![to_lon, to_lat],
                ])))
            }
            _ => None,
        };

        let mut props = Map::new();
        props.insert(
            "from_step".to_string(),
            JsonValue::from(entry.from_step as i64),
        );
        props.insert(
            "to_step".to_string(),
            JsonValue::from(entry.to_step as i64),
        );
        props.insert(
            "from_netelement_id".to_string(),
            JsonValue::from(entry.from_netelement_id.clone()),
        );
        props.insert(
            "to_netelement_id".to_string(),
            JsonValue::from(entry.to_netelement_id.clone()),
        );
        props.insert(
            "transition_probability".to_string(),
            JsonValue::from(entry.transition_probability),
        );
        props.insert(
            "is_viterbi_chosen".to_string(),
            JsonValue::from(entry.is_viterbi_chosen),
        );

        features.push(Feature {
            bbox: None,
            geometry,
            id: None,
            properties: Some(props),
            foreign_members: None,
        });
    }

    let mut fc_members = serde_json::Map::new();
    fc_members.insert("phase".to_string(), JsonValue::from(2i64));
    fc_members.insert(
        "description".to_string(),
        JsonValue::from(
            "HMM transition probabilities: feasible candidate-pair links across consecutive GNSS steps",
        ),
    );

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: Some(fc_members),
    };
    let json = serde_json::to_string_pretty(&fc).map_err(|e| {
        ProjectionError::InvalidGeometry(format!(
            "Failed to serialize transition probabilities GeoJSON: {}",
            e
        ))
    })?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Export Phase 5 — Post-Viterbi path sanity decisions as GeoJSON
///
/// Produces a FeatureCollection with one Point feature per consecutive
/// segment pair evaluated during navigability validation.  Each feature
/// is placed at the midpoint of the from-netelement's geometry.
///
/// Properties per feature:
/// - `pair_index`          — sequential index of the pair (0-based)
/// - `from_netelement_id`  — source netelement
/// - `to_netelement_id`    — target netelement
/// - `reachable`           — whether the target was reachable
/// - `action`              — "kept", "removed", or "rerouted"
/// - `rerouted_via`        — comma-separated NE IDs of bridge segments (empty if N/A)
/// - `warning`             — warning message (empty if reachable)
pub fn export_path_sanity_decisions<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, Value};
    use serde_json::{Map, Value as JsonValue};
    use std::collections::HashMap;

    // Build a lookup from netelement ID to geometry coords for spatial placement.
    let ne_geom: HashMap<&str, &Vec<Vec<f64>>> = debug_info
        .netelement_probabilities
        .iter()
        .map(|np| (np.netelement_id.as_str(), &np.geometry_coords))
        .collect();

    let mut features = Vec::new();

    for decision in &debug_info.sanity_decisions {
        // Place the point at the midpoint of the from-netelement's geometry.
        let coords = ne_geom
            .get(decision.from_netelement_id.as_str())
            .and_then(|g| {
                if g.is_empty() {
                    return None;
                }
                let mid = g.len() / 2;
                Some(vec![g[mid][0], g[mid][1]])
            })
            .unwrap_or_else(|| vec![0.0, 0.0]);
        let geom = Geometry::new(Value::Point(coords));
        let mut props = Map::new();
        props.insert(
            "pair_index".to_string(),
            JsonValue::from(decision.pair_index as i64),
        );
        props.insert(
            "from_netelement_id".to_string(),
            JsonValue::from(decision.from_netelement_id.clone()),
        );
        props.insert(
            "to_netelement_id".to_string(),
            JsonValue::from(decision.to_netelement_id.clone()),
        );
        props.insert(
            "reachable".to_string(),
            JsonValue::from(decision.reachable),
        );
        props.insert(
            "action".to_string(),
            JsonValue::from(decision.action.clone()),
        );
        props.insert(
            "rerouted_via".to_string(),
            JsonValue::from(decision.rerouted_via.join(",")),
        );
        props.insert(
            "warning".to_string(),
            JsonValue::from(decision.warning.clone()),
        );

        features.push(Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(props),
            foreign_members: None,
        });
    }

    let mut fc_members = serde_json::Map::new();
    fc_members.insert("phase".to_string(), JsonValue::from(5i64));
    fc_members.insert(
        "description".to_string(),
        JsonValue::from(
            "Path sanity decisions: post-Viterbi navigability validation for each consecutive segment pair",
        ),
    );

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: Some(fc_members),
    };
    let json = serde_json::to_string_pretty(&fc).map_err(|e| {
        ProjectionError::InvalidGeometry(format!(
            "Failed to serialize path sanity decisions GeoJSON: {}",
            e
        ))
    })?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Export gap-fill records to a GeoJSON file (phase 6).
///
/// Each gap-fill decision becomes a Point feature at the midpoint of the
/// from-netelement's geometry, with tabular properties
/// describing the pair, whether a route was found, and which bridge NEs were inserted.
pub fn export_gap_fills<P: AsRef<Path>>(
    debug_info: &DebugInfo,
    output_path: P,
) -> Result<(), ProjectionError> {
    use geojson::{Feature, FeatureCollection, Geometry, JsonObject, Value as GeoValue};
    use serde_json::Value as JsonValue;

    // Build a lookup from netelement ID to geometry coords for spatial placement.
    let ne_geom: std::collections::HashMap<&str, &Vec<Vec<f64>>> = debug_info
        .netelement_probabilities
        .iter()
        .map(|np| (np.netelement_id.as_str(), &np.geometry_coords))
        .collect();

    let mut features = Vec::new();

    for gf in &debug_info.gap_fills {
        // Place the point at the midpoint of the from-netelement's geometry.
        let coords = ne_geom
            .get(gf.from_netelement_id.as_str())
            .and_then(|g| {
                if g.is_empty() {
                    return None;
                }
                let mid = g.len() / 2;
                Some(vec![g[mid][0], g[mid][1]])
            })
            .unwrap_or_else(|| vec![0.0, 0.0]);
        let geom = Geometry::new(GeoValue::Point(coords));
        let mut props = JsonObject::new();
        props.insert("pair_index".to_string(), JsonValue::from(gf.pair_index as u64));
        props.insert(
            "from_netelement_id".to_string(),
            JsonValue::from(gf.from_netelement_id.as_str()),
        );
        props.insert(
            "to_netelement_id".to_string(),
            JsonValue::from(gf.to_netelement_id.as_str()),
        );
        props.insert("route_found".to_string(), JsonValue::from(gf.route_found));
        props.insert(
            "inserted_netelements".to_string(),
            JsonValue::from(gf.inserted_netelements.join(", ")),
        );
        props.insert(
            "inserted_count".to_string(),
            JsonValue::from(gf.inserted_netelements.len() as u64),
        );
        props.insert("warning".to_string(), JsonValue::from(gf.warning.as_str()));

        features.push(Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(props),
            foreign_members: None,
        });
    }

    let mut fc_members = JsonObject::new();
    fc_members.insert("phase".to_string(), JsonValue::from(6));
    fc_members.insert(
        "description".to_string(),
        JsonValue::from(
            "Gap filling: bridge netelements inserted between disconnected consecutive segments after sanity validation",
        ),
    );

    let fc = FeatureCollection {
        bbox: None,
        features,
        foreign_members: Some(fc_members),
    };
    let json = serde_json::to_string_pretty(&fc).map_err(|e| {
        ProjectionError::InvalidGeometry(format!(
            "Failed to serialize gap fills GeoJSON: {}",
            e
        ))
    })?;
    let mut file = File::create(output_path.as_ref())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::{
        CandidateInfo, NetelementProbabilityInfo, PathDecision, PositionCandidates,
        TransitionProbabilityEntry,
    };
    use std::io::Read;

    fn make_debug_info() -> DebugInfo {
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
        debug_info.add_decision(PathDecision {
            step: 0,
            decision_type: "viterbi_transition".to_string(),
            current_segment: "NE_A".to_string(),
            options: vec!["NE_A".to_string()],
            option_probabilities: vec![0.72],
            chosen_option: "NE_A".to_string(),
            reason: "Only candidate".to_string(),
        });
        debug_info.netelement_probabilities.push(NetelementProbabilityInfo {
            netelement_id: "NE_A".to_string(),
            avg_emission_probability: 0.72,
            position_count: 1,
            geometry_coords: vec![vec![4.35, 50.85], vec![4.36, 50.86]],
            in_viterbi_path: true,
            is_bridge: false,
        });
        debug_info.transition_probabilities.push(TransitionProbabilityEntry {
            from_step: 0,
            to_step: 1,
            from_netelement_id: "NE_A".to_string(),
            to_netelement_id: "NE_B".to_string(),
            transition_probability: 0.65,
            is_viterbi_chosen: true,
        });
        debug_info
    }

    #[test]
    fn test_export_hmm_emission_probabilities() {
        let debug_info = make_debug_info();
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_hmm_emission.geojson");

        let result = export_hmm_emission_probabilities(&debug_info, &output_path);
        assert!(result.is_ok());

        let mut file = File::open(&output_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("NE_A"));
        assert!(contents.contains("emission_probability"));
        assert!(contents.contains("distance_m"));
        // Should NOT contain raw gnss_position point features
        assert!(!contents.contains("gnss_position"));

        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_hmm_viterbi_trace() {
        let debug_info = make_debug_info();
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_hmm_viterbi_trace.geojson");

        let result = export_hmm_viterbi_trace(&debug_info, &output_path);
        assert!(result.is_ok());

        let mut file = File::open(&output_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("NE_A"));
        assert!(contents.contains("viterbi_transition"));
        assert!(contents.contains("netelement_id"));

        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_hmm_candidate_netelements() {
        let debug_info = make_debug_info();
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_hmm_candidates.geojson");

        let result = export_hmm_candidate_netelements(&debug_info, &output_path);
        assert!(result.is_ok());

        let mut file = File::open(&output_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("NE_A"));
        assert!(contents.contains("in_viterbi_path"));

        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_hmm_selected_path() {
        let debug_info = make_debug_info();
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_hmm_selected_path.geojson");

        let result = export_hmm_selected_path(&debug_info, &output_path);
        assert!(result.is_ok());

        let mut file = File::open(&output_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("NE_A"));
        assert!(contents.contains("is_bridge"));

        std::fs::remove_file(&output_path).ok();
    }

    #[test]
    fn test_export_all_debug_info() {
        let debug_info = make_debug_info();
        let temp_dir = std::env::temp_dir().join("tp_hmm_debug_test");

        let result = export_all_debug_info(&debug_info, &temp_dir);
        assert!(result.is_ok());

        assert!(temp_dir.join("01_emission_probabilities.geojson").exists());
        assert!(temp_dir.join("03_viterbi_trace.geojson").exists());
        assert!(temp_dir.join("04_candidate_netelements.geojson").exists());
        assert!(temp_dir.join("07_selected_path.geojson").exists());
        assert!(temp_dir.join("02_transition_probabilities.geojson").exists());

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_export_hmm_transition_probabilities() {
        let debug_info = make_debug_info();
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("test_hmm_transition_probs.geojson");

        let result = export_hmm_transition_probabilities(&debug_info, &output_path);
        assert!(result.is_ok());

        let mut file = File::open(&output_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("NE_A"));
        assert!(contents.contains("NE_B"));
        assert!(contents.contains("transition_probability"));
        assert!(contents.contains("is_viterbi_chosen"));

        std::fs::remove_file(&output_path).ok();
    }
}
