//! Log-space Viterbi algorithm for HMM-based map matching
//!
//! Implements the Viterbi algorithm (Newson & Krumm, 2009) to decode the most
//! probable sequence of netelements given a sequence of GNSS observations and
//! per-position candidate sets with emission probabilities.
//!
//! The algorithm operates in log-space to avoid numerical underflow on long
//! sequences.

use crate::errors::ProjectionError;
use crate::models::{AssociatedNetElement, Netelement};
use crate::path::candidate::CandidateNetElement;
use crate::path::graph::{
    cached_shortest_path_distance, shortest_path_route, NetelementSide, ShortestPathCache,
};
use crate::path::probability::{calculate_transition_probability, is_near_netelement_edge};
use crate::path::PathConfig;
use geo::{HaversineDistance, Point};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of the Viterbi decoding.
///
/// Contains the decoded path as a single subsequence for continuous GNSS
/// input (no Viterbi breaks — the algorithm uses penalty carry-forward
/// to maintain chain continuity).
#[derive(Debug, Clone)]
pub struct ViterbiResult {
    /// Decoded subsequences.  For continuous GNSS input this will always
    /// contain exactly one entry.
    pub subsequences: Vec<ViterbiSubsequence>,

    /// All feasible (non-zero) transition probabilities computed during the
    /// forward pass, as `(from_t, from_candidate_idx, to_t, to_candidate_idx,
    /// transition_probability_linear)` tuples.
    pub transition_records: Vec<(usize, usize, usize, usize, f64)>,
}

/// A single unbroken Viterbi sub-sequence.
#[derive(Debug, Clone)]
pub struct ViterbiSubsequence {
    /// `(position_index, candidate_index)` for each time-step in this
    /// sub-sequence.  `position_index` is an index into the original
    /// `working_positions` (and `position_candidates`) slice.
    pub states: Vec<(usize, usize)>,

    /// Log-probability of this sub-sequence.
    pub log_probability: f64,
}

/// Decode the most probable netelement sequence using the Viterbi algorithm.
///
/// # Arguments
///
/// * `position_candidates` — Per-position candidate netelements (from Phase 1).
/// * `position_probabilities` — Per-position emission probabilities: `position_probabilities[t][candidate_idx] = emission_prob`.
///   `candidate_idx` is an index into `position_candidates[t]`.
/// * `netelements` — Full netelement set (for geometry lookups).
/// * `netelement_index` — Map from netelement ID → index in `netelements`.
/// * `graph` — Directed topology graph with distance-weighted edges.
/// * `node_map` — Map from `NetelementSide` → `NodeIndex` in `graph`.
/// * `cache` — Mutable shortest-path cache (lazily populated).
/// * `config` — Path configuration (provides `beta`, `edge_zone_distance`).
///
/// # Returns
///
/// A `ViterbiResult` with one or more sub-sequences covering all time-steps
/// that have at least one candidate.
#[allow(clippy::too_many_arguments)]
pub fn viterbi_decode(
    position_candidates: &[Vec<CandidateNetElement>],
    position_probabilities: &[Vec<f64>],
    netelements: &[Netelement],
    netelement_index: &HashMap<String, usize>,
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    cache: &mut ShortestPathCache,
    config: &PathConfig,
) -> Result<ViterbiResult, ProjectionError> {
    let t_count = position_candidates.len();
    if t_count == 0 {
        return Ok(ViterbiResult {
            subsequences: vec![],
            transition_records: vec![],
        });
    }

    // Pre-compute whether each candidate is near a netelement edge.
    // near_edge[t][j] = true ⟹ candidate j at time t is in the edge zone
    let near_edge: Vec<Vec<bool>> = position_candidates
        .iter()
        .map(|cands| {
            cands
                .iter()
                .map(|c| {
                    if let Some(&ne_idx) = netelement_index.get(&c.netelement_id) {
                        is_near_netelement_edge(
                            &c.projected_point,
                            &netelements[ne_idx].geometry,
                            config.edge_zone_distance,
                        )
                    } else {
                        true // unknown NE → conservative: treat as edge
                    }
                })
                .collect()
        })
        .collect();

    // Trellis tables.
    // log_v[t][j] = best log-probability reaching candidate j at time t.
    // backptr[t][j] = (prev_time, prev_candidate_index) for back-tracing.
    let mut log_v: Vec<Vec<f64>> = Vec::with_capacity(t_count);
    let mut backptr: Vec<Vec<Option<usize>>> = Vec::with_capacity(t_count);
    // Parallel to backptr: records the actual predecessor time-step for
    // each (t, j) so the back-trace can skip over empty rows.
    let mut backptr_time: Vec<Vec<Option<usize>>> = Vec::with_capacity(t_count);
    let mut transition_records: Vec<(usize, usize, usize, usize, f64)> = Vec::new();

    // ── Initialise t = 0 ────────────────────────────────────────────────
    {
        let cands = &position_candidates[0];
        let probs = &position_probabilities[0];
        let mut lv = Vec::with_capacity(cands.len());
        let mut bp = Vec::with_capacity(cands.len());
        for (j, _) in cands.iter().enumerate() {
            let emission = probs.get(j).copied().unwrap_or(0.0);
            lv.push(safe_ln(emission));
            bp.push(None);
        }
        log_v.push(lv);
        backptr.push(bp);
        backptr_time.push(vec![None; position_candidates[0].len()]);
    }

    // Penalty applied when no valid transition exists (ln(1e-10) ≈ -23).
    // This is large enough to strongly discourage impossible transitions
    // while keeping the Viterbi chain unbroken for continuous paths.
    const NO_TRANSITION_PENALTY: f64 = -23.0;

    // Minimum emission probability floor.  A single noisy observation
    // (e.g. heading beyond cutoff → emission = 0) must not irrecoverably
    // destroy hundreds of steps of accumulated evidence.  The floor is
    // tiny enough to heavily penalise the step without producing −∞.
    const EMISSION_FLOOR: f64 = 1e-10;

    // ── Recurse t = 1 .. T-1 ───────────────────────────────────────────
    for t in 1..t_count {
        let curr_cands = &position_candidates[t];
        let curr_probs = &position_probabilities[t];

        if curr_cands.is_empty() {
            // No candidates at this time-step — push empty row.
            log_v.push(vec![]);
            backptr.push(vec![]);
            backptr_time.push(vec![]);
            continue;
        }

        // Find the most recent non-empty time-step with finite scores to
        // use as the predecessor.  Normally this is t-1, but if t-1 had
        // no candidates we search further back.
        let mut prev_t = None;
        for pt in (0..t).rev() {
            if !log_v[pt].is_empty() && log_v[pt].iter().any(|&v| v != f64::NEG_INFINITY) {
                prev_t = Some(pt);
                break;
            }
        }

        if prev_t.is_none() {
            // No usable predecessor at all — initialise from emission only.
            // This only happens when *every* earlier time-step was empty.
            let mut lv = Vec::with_capacity(curr_cands.len());
            let mut bp = Vec::with_capacity(curr_cands.len());
            for (j, _) in curr_cands.iter().enumerate() {
                let emission = curr_probs.get(j).copied().unwrap_or(0.0);
                lv.push(safe_ln(emission));
                bp.push(None);
            }
            log_v.push(lv);
            backptr.push(bp);
            backptr_time.push(vec![None; curr_cands.len()]);
            continue;
        }
        let prev_t = prev_t.unwrap();
        let prev_cands = &position_candidates[prev_t];
        let prev_lv = &log_v[prev_t];

        let mut lv = vec![f64::NEG_INFINITY; curr_cands.len()];
        let mut bp: Vec<Option<(usize, usize)>> = vec![None; curr_cands.len()];

        for (j, cand_j) in curr_cands.iter().enumerate() {
            let emission_j = curr_probs.get(j).copied().unwrap_or(0.0);
            let ln_emission_j = safe_ln(emission_j.max(EMISSION_FLOOR));

            for (i, cand_i) in prev_cands.iter().enumerate() {
                if prev_lv[i] == f64::NEG_INFINITY {
                    continue;
                }

                let ln_trans = compute_log_transition(
                    cand_i,
                    cand_j,
                    i,
                    j,
                    prev_t,
                    t,
                    &near_edge,
                    netelements,
                    netelement_index,
                    graph,
                    node_map,
                    cache,
                    config,
                );

                if ln_trans == f64::NEG_INFINITY {
                    continue;
                }

                transition_records.push((prev_t, i, t, j, ln_trans.exp()));

                let score = prev_lv[i] + ln_trans + ln_emission_j;
                if score > lv[j] {
                    lv[j] = score;
                    bp[j] = Some((prev_t, i));
                }
            }
        }

        // If all lv[j] are -∞ after the inner loop, no valid transition
        // was possible.  Instead of creating a Viterbi break, carry
        // forward the best previous state with a penalty so the chain
        // remains continuous (the GNSS input is one unbroken drive).
        if lv.iter().all(|&v| v == f64::NEG_INFINITY) {
            // Find the best previous candidate.
            let best_prev_i = prev_lv
                .iter()
                .enumerate()
                .filter(|(_, &v)| v != f64::NEG_INFINITY)
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i);

            if let Some(best_i) = best_prev_i {
                let carry_score = prev_lv[best_i] + NO_TRANSITION_PENALTY;
                for (j, _) in curr_cands.iter().enumerate() {
                    let emission = curr_probs.get(j).copied().unwrap_or(0.0);
                    let ln_em = safe_ln(emission.max(EMISSION_FLOOR));
                    lv[j] = carry_score + ln_em;
                    bp[j] = Some((prev_t, best_i));
                }
            }
        }

        log_v.push(lv);
        let flat_bp: Vec<Option<usize>> = bp.iter().map(|opt| opt.map(|(_, i)| i)).collect();
        let time_bp: Vec<Option<usize>> = bp.iter().map(|opt| opt.map(|(pt, _)| pt)).collect();
        backptr.push(flat_bp);
        backptr_time.push(time_bp);
    }

    // ── Back-trace ──────────────────────────────────────────────────────
    // With the no-break approach we always produce a single subsequence.
    let subsequences = backtrace_continuous(&log_v, &backptr, &backptr_time, t_count);

    Ok(ViterbiResult {
        subsequences,
        transition_records,
    })
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Compute the log-transition probability between candidate `i` at time
/// `t_prev` and candidate `j` at time `t_curr`.
#[allow(clippy::too_many_arguments)]
fn compute_log_transition(
    cand_i: &CandidateNetElement,
    cand_j: &CandidateNetElement,
    i: usize,
    j: usize,
    t_prev: usize,
    t_curr: usize,
    near_edge: &[Vec<bool>],
    netelements: &[Netelement],
    netelement_index: &HashMap<String, usize>,
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    cache: &mut ShortestPathCache,
    config: &PathConfig,
) -> f64 {
    let same_ne = cand_i.netelement_id == cand_j.netelement_id;

    // Same-netelement shortcut: transition is free.
    if same_ne {
        return 0.0; // ln(1.0)
    }

    // Edge-zone skip: both candidates interior → impossible transition.
    let i_near = near_edge[t_prev][i];
    let j_near = near_edge[t_curr][j];
    if !i_near && !j_near {
        return f64::NEG_INFINITY; // ln(0.0)
    }

    // Need Dijkstra route distance.  Try all (from_side → to_side) combos
    // (there are at most 4: from_0→to_0, from_0→to_1, from_1→to_0, from_1→to_1)
    // and pick the shortest.
    let Some(&ne_i_idx) = netelement_index.get(&cand_i.netelement_id) else {
        return f64::NEG_INFINITY;
    };
    let Some(&ne_j_idx) = netelement_index.get(&cand_j.netelement_id) else {
        return f64::NEG_INFINITY;
    };

    // Which sides of the from-netelement could the candidate be near?
    // If near_edge is true we try both sides; shortest wins.
    let from_sides = candidate_sides(&cand_i.netelement_id);
    let to_sides = candidate_sides(&cand_j.netelement_id);

    // Great-circle distance between the two projected points.
    let gc_distance = cand_i
        .projected_point
        .haversine_distance(&cand_j.projected_point);

    // Evaluate all (from_side, to_side) combinations and pick the one with
    // the highest combined transition probability (route distance + turn angle).
    let ne_i_geom = &netelements[ne_i_idx].geometry;
    let ne_j_geom = &netelements[ne_j_idx].geometry;

    let mut best_ln_trans = f64::NEG_INFINITY;
    for from_side in &from_sides {
        for to_side in &to_sides {
            if let Some(d) =
                cached_shortest_path_distance(cache, graph, node_map, from_side, to_side)
            {
                let from_partial =
                    partial_netelement_distance(cand_i, from_side.position, &netelements[ne_i_idx]);
                let to_partial =
                    partial_netelement_distance(cand_j, to_side.position, &netelements[ne_j_idx]);
                let route_distance = from_partial + d + to_partial;

                let base_trans =
                    calculate_transition_probability(route_distance, gc_distance, config.beta);

                // Turn-angle penalty: compare the exit heading from the
                // from-NE with the entry heading into the to-NE.
                let turn_factor = netelement_connection_turn_factor(
                    ne_i_geom,
                    from_side.position,
                    ne_j_geom,
                    to_side.position,
                    config.turn_scale,
                );

                let combined = base_trans * turn_factor;
                let ln_combined = safe_ln(combined);
                if ln_combined > best_ln_trans {
                    best_ln_trans = ln_combined;
                }
            }
        }
    }

    best_ln_trans
}

/// Compute the turn-angle penalty factor at a netelement connection.
///
/// Compares the exit heading from `from_geom` at `from_side` with the entry
/// heading into `to_geom` at `to_side`. Returns a factor in (0, 1] where
/// 1.0 means straight through and values closer to 0 penalise sharper turns.
fn netelement_connection_turn_factor(
    from_geom: &geo::LineString<f64>,
    from_side: u8,
    to_geom: &geo::LineString<f64>,
    to_side: u8,
    turn_scale: f64,
) -> f64 {
    use crate::path::candidate::{directional_heading_difference, haversine_bearing};

    let from_pts: Vec<Point<f64>> = from_geom.points().collect();
    let to_pts: Vec<Point<f64>> = to_geom.points().collect();
    if from_pts.len() < 2 || to_pts.len() < 2 {
        return 1.0; // degenerate geometry — no penalty
    }

    // Exit heading: direction the train is moving as it leaves from_geom.
    let exit_heading = if from_side == 0 {
        // Exiting through start → was traveling from end toward start.
        haversine_bearing(&from_pts[1], &from_pts[0])
    } else {
        // Exiting through end → was traveling from start toward end.
        let n = from_pts.len();
        haversine_bearing(&from_pts[n - 2], &from_pts[n - 1])
    };

    // Entry heading: direction the train moves after entering to_geom.
    let entry_heading = if to_side == 0 {
        // Entering at start → heading toward end.
        haversine_bearing(&to_pts[0], &to_pts[1])
    } else {
        // Entering at end → heading toward start.
        let n = to_pts.len();
        haversine_bearing(&to_pts[n - 1], &to_pts[n - 2])
    };

    let turn_angle = directional_heading_difference(exit_heading, entry_heading);
    (-turn_angle / turn_scale).exp()
}

/// Return both sides of a netelement as `NetelementSide` values.
fn candidate_sides(netelement_id: &str) -> [NetelementSide; 2] {
    [
        NetelementSide {
            netelement_id: netelement_id.to_string(),
            position: 0,
        },
        NetelementSide {
            netelement_id: netelement_id.to_string(),
            position: 1,
        },
    ]
}

/// Approximate distance from a candidate's projected point to a netelement endpoint.
///
/// `side` is 0 (start) or 1 (end).  Uses intrinsic coordinate × netelement
/// length as a quick proxy.
fn partial_netelement_distance(
    cand: &CandidateNetElement,
    side: u8,
    netelement: &Netelement,
) -> f64 {
    use geo::HaversineLength;
    let length = netelement.geometry.haversine_length();
    if side == 0 {
        cand.intrinsic_coordinate * length
    } else {
        (1.0 - cand.intrinsic_coordinate) * length
    }
}

/// `ln` that maps 0.0 → -∞ and avoids NaN.
fn safe_ln(x: f64) -> f64 {
    if x <= 0.0 {
        f64::NEG_INFINITY
    } else {
        x.ln()
    }
}

/// Back-trace through the trellis to extract one or more optimal sub-sequences.
#[allow(dead_code)]
fn backtrace(
    log_v: &[Vec<f64>],
    backptr: &[Vec<Option<usize>>],
    subseq_starts: &[usize],
    t_count: usize,
) -> Vec<ViterbiSubsequence> {
    let mut result: Vec<ViterbiSubsequence> = Vec::new();

    // For each sub-sequence delimited by `subseq_starts`, find the best
    // terminal state and trace backwards.
    for (seg_idx, &start_t) in subseq_starts.iter().enumerate() {
        let end_t = if seg_idx + 1 < subseq_starts.len() {
            subseq_starts[seg_idx + 1]
        } else {
            t_count
        };

        // Find the time-step with the last non-empty row in [start_t, end_t).
        let mut last_valid_t = None;
        for t in (start_t..end_t).rev() {
            if !log_v[t].is_empty() && log_v[t].iter().any(|&v| v != f64::NEG_INFINITY) {
                last_valid_t = Some(t);
                break;
            }
        }

        let Some(term_t) = last_valid_t else {
            continue; // entirely empty sub-sequence
        };

        // Best candidate at terminal time-step.
        let (best_j, best_log) = log_v[term_t]
            .iter()
            .enumerate()
            .filter(|(_, &v)| v != f64::NEG_INFINITY)
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap(); // safe: we know at least one is finite

        // Trace back.
        let mut states = Vec::with_capacity(term_t - start_t + 1);
        let mut j = best_j;
        states.push((term_t, j));
        let mut t = term_t;
        while t > start_t {
            if let Some(prev_j) = backptr[t][j] {
                t -= 1;
                j = prev_j;
                states.push((t, j));
            } else {
                break;
            }
        }
        states.reverse();

        result.push(ViterbiSubsequence {
            states,
            log_probability: *best_log,
        });
    }

    result
}

/// Back-trace through the trellis for the continuous (no-break) Viterbi.
///
/// Produces a single `ViterbiSubsequence` covering all time-steps with
/// candidates.  The `backptr_time` table records, for each (t, j), the
/// actual predecessor time-step (which may differ from t-1 if intermediate
/// rows were empty).
fn backtrace_continuous(
    log_v: &[Vec<f64>],
    backptr: &[Vec<Option<usize>>],
    backptr_time: &[Vec<Option<usize>>],
    t_count: usize,
) -> Vec<ViterbiSubsequence> {
    // Find the last time-step with non-empty, finite scores.
    let mut last_valid_t = None;
    for t in (0..t_count).rev() {
        if !log_v[t].is_empty() && log_v[t].iter().any(|&v| v != f64::NEG_INFINITY) {
            last_valid_t = Some(t);
            break;
        }
    }

    let Some(term_t) = last_valid_t else {
        return vec![]; // no valid states at all
    };

    // Best candidate at terminal time-step.
    let (best_j, best_log) = log_v[term_t]
        .iter()
        .enumerate()
        .filter(|(_, &v)| v != f64::NEG_INFINITY)
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();

    // Trace back using backptr_time to jump over empty rows.
    let mut states = Vec::with_capacity(term_t + 1);
    let mut j = best_j;
    let mut t = term_t;
    states.push((t, j));

    loop {
        let prev_j = backptr[t][j];
        let prev_t = backptr_time[t][j];
        match (prev_t, prev_j) {
            (Some(pt), Some(pj)) => {
                t = pt;
                j = pj;
                states.push((t, j));
            }
            _ => break,
        }
    }
    states.reverse();

    vec![ViterbiSubsequence {
        states,
        log_probability: *best_log,
    }]
}

// ─── Post-processing: build AssociatedNetElement path ───────────────────────

/// Convert Viterbi output into a sequence of `AssociatedNetElement` segments.
///
/// 1. Deduplicates consecutive same-netelement entries.
/// 2. Inserts bridge netelements between non-adjacent observed NEs
///    (recovered from cached Dijkstra paths if available; otherwise left
///    as a direct jump, which downstream code can flag).
/// 3. Computes intrinsic ranges and GNSS index ranges per segment.
pub fn build_path_from_viterbi(
    viterbi: &ViterbiResult,
    position_candidates: &[Vec<CandidateNetElement>],
    netelements: &[Netelement],
    netelement_index: &HashMap<String, usize>,
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    cache: &mut ShortestPathCache,
) -> Result<Vec<AssociatedNetElement>, ProjectionError> {
    let mut segments: Vec<AssociatedNetElement> = Vec::new();

    for subseq in &viterbi.subsequences {
        if subseq.states.is_empty() {
            continue;
        }

        // Group consecutive states by netelement.
        let mut groups: Vec<NetelementGroup> = Vec::new();
        for &(pos_idx, cand_idx) in &subseq.states {
            let cand = &position_candidates[pos_idx][cand_idx];
            if let Some(last) = groups.last_mut() {
                if last.netelement_id == cand.netelement_id {
                    // Extend existing group.
                    last.update(pos_idx, cand);
                    continue;
                }
            }
            groups.push(NetelementGroup::new(pos_idx, cand));
        }

        // Emit segments, inserting bridges between non-adjacent groups.
        for (g_idx, group) in groups.iter().enumerate() {
            if g_idx > 0 {
                let prev = &groups[g_idx - 1];
                insert_bridges(
                    prev,
                    group,
                    &mut segments,
                    netelements,
                    netelement_index,
                    graph,
                    node_map,
                    cache,
                )?;
            }

            segments.push(group.to_associated_net_element()?);
        }
    }

    Ok(segments)
}

/// Decision record for a single consecutive-segment pair during sanity validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanityDecision {
    /// Index of this pair (0 = first consecutive pair)
    pub pair_index: usize,
    /// Netelement ID of the source segment
    pub from_netelement_id: String,
    /// Netelement ID of the target segment
    pub to_netelement_id: String,
    /// Whether the target was reachable from the source
    pub reachable: bool,
    /// Action taken: "kept", "removed", or "rerouted"
    pub action: String,
    /// Netelement IDs inserted by Dijkstra re-routing (empty if not rerouted)
    pub rerouted_via: Vec<String>,
    /// Warning message (empty if reachable)
    pub warning: String,
}

/// Record of a gap-fill action between two consecutive segments after sanity validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapFill {
    /// Index of this consecutive pair (0-based)
    pub pair_index: usize,
    /// Netelement ID of the segment before the gap
    pub from_netelement_id: String,
    /// Netelement ID of the segment after the gap
    pub to_netelement_id: String,
    /// Whether a Dijkstra route was found between the two segments
    pub route_found: bool,
    /// Netelement IDs inserted to bridge the gap (empty if no route)
    pub inserted_netelements: Vec<String>,
    /// Warning message (empty if directly connected or successfully filled)
    pub warning: String,
}

/// Post-Viterbi navigability validation.
///
/// Walks the assembled path segments sequentially, checking each consecutive
/// pair for topological reachability via the directed topology graph.
/// When a segment is unreachable from its predecessor:
///   1. The unreachable segment is removed.
///   2. A Dijkstra re-route is attempted from the last valid segment to the
///      *next* remaining segment. If a route exists, bridge NEs are inserted.
///   3. A warning is recorded.
///
/// Returns the validated path segments, a list of warnings, and structured
/// sanity decisions for debug output.
#[allow(clippy::too_many_arguments)]
pub fn validate_path_navigability(
    segments: Vec<AssociatedNetElement>,
    _netelements: &[Netelement],
    netelement_index: &HashMap<String, usize>,
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    cache: &mut ShortestPathCache,
) -> (Vec<AssociatedNetElement>, Vec<String>, Vec<SanityDecision>) {
    if segments.len() < 2 {
        return (segments, vec![], vec![]);
    }

    let mut validated: Vec<AssociatedNetElement> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut decisions: Vec<SanityDecision> = Vec::new();
    let mut pair_index: usize = 0;

    // Start with the first segment (always kept).
    validated.push(segments[0].clone());

    let mut i = 1;
    while i < segments.len() {
        // Clone data from last valid segment to avoid borrowing `validated` across mutations.
        let last_ne_id = validated.last().unwrap().netelement_id.clone();
        let last_gnss_end = validated.last().unwrap().gnss_end_index;
        let candidate = &segments[i];

        // Same netelement → always valid.
        if last_ne_id == candidate.netelement_id {
            decisions.push(SanityDecision {
                pair_index,
                from_netelement_id: last_ne_id,
                to_netelement_id: candidate.netelement_id.clone(),
                reachable: true,
                action: "kept".to_string(),
                rerouted_via: vec![],
                warning: String::new(),
            });
            validated.push(candidate.clone());
            pair_index += 1;
            i += 1;
            continue;
        }

        // Check reachability across all side combinations.
        let from_sides = candidate_sides(&last_ne_id);
        let to_sides = candidate_sides(&candidate.netelement_id);
        let mut reachable = false;
        for from in &from_sides {
            for to in &to_sides {
                if cached_shortest_path_distance(cache, graph, node_map, from, to).is_some() {
                    reachable = true;
                    break;
                }
            }
            if reachable {
                break;
            }
        }

        if reachable {
            decisions.push(SanityDecision {
                pair_index,
                from_netelement_id: last_ne_id,
                to_netelement_id: candidate.netelement_id.clone(),
                reachable: true,
                action: "kept".to_string(),
                rerouted_via: vec![],
                warning: String::new(),
            });
            validated.push(candidate.clone());
            pair_index += 1;
            i += 1;
            continue;
        }

        // Unreachable — remove this segment and attempt re-route to the next one.
        let warn_msg = format!(
            "Removed unreachable segment {} (no navigable path from {})",
            candidate.netelement_id, last_ne_id
        );
        warnings.push(warn_msg.clone());

        // Look ahead: try to find the next segment reachable from last valid.
        let mut rerouted = false;
        let mut rerouted_via: Vec<String> = Vec::new();

        if i + 1 < segments.len() {
            let next = &segments[i + 1];
            // Try Dijkstra from last valid to next.
            let fwd_from_sides = candidate_sides(&last_ne_id);
            let fwd_to_sides = candidate_sides(&next.netelement_id);

            let mut best_route_cost: Option<f64> = None;
            let mut best_from_side = 0u8;
            let mut best_to_side = 0u8;
            for from in &fwd_from_sides {
                for to in &fwd_to_sides {
                    if let Some(d) = cached_shortest_path_distance(cache, graph, node_map, from, to)
                    {
                        if best_route_cost.is_none() || d < best_route_cost.unwrap() {
                            best_route_cost = Some(d);
                            best_from_side = from.position;
                            best_to_side = to.position;
                        }
                    }
                }
            }

            if let Some(cost) = best_route_cost {
                rerouted = true;
                // Insert bridge NEs if needed (cost > 0 means non-adjacent).
                if cost > 1e-9 {
                    let from_side = NetelementSide {
                        netelement_id: last_ne_id.clone(),
                        position: best_from_side,
                    };
                    let to_side = NetelementSide {
                        netelement_id: next.netelement_id.clone(),
                        position: best_to_side,
                    };
                    let bridge_ne_ids = trace_intermediate_netelements(
                        graph,
                        node_map,
                        &from_side,
                        &to_side,
                        &last_ne_id,
                        &next.netelement_id,
                    );
                    let gnss_idx = last_gnss_end;
                    for ne_id in &bridge_ne_ids {
                        if netelement_index.contains_key(ne_id) {
                            if let Ok(bridge) = AssociatedNetElement::new(
                                ne_id.clone(),
                                1.0,
                                0.0,
                                1.0,
                                gnss_idx,
                                gnss_idx,
                            ) {
                                validated.push(bridge);
                            }
                        }
                    }
                    rerouted_via = bridge_ne_ids;
                }
            }
        }

        let action = if rerouted { "rerouted" } else { "removed" };
        decisions.push(SanityDecision {
            pair_index,
            from_netelement_id: last_ne_id,
            to_netelement_id: candidate.netelement_id.clone(),
            reachable: false,
            action: action.to_string(),
            rerouted_via,
            warning: warn_msg,
        });

        pair_index += 1;
        i += 1;
    }

    // Second pass: detect and collapse oscillation patterns (A→…→A).
    let validated = remove_oscillations(validated, &mut warnings, &mut decisions, &mut pair_index);

    // Third pass: remove segments that violate directional consistency.
    // A triple (A, B, C) is consistent if the entry side of B from A and the
    // exit side of B towards C use opposite sides (i.e. the train passes through
    // B without reversing direction).
    let validated = remove_direction_violations(
        validated,
        graph,
        node_map,
        &mut warnings,
        &mut decisions,
        &mut pair_index,
    );

    (validated, warnings, decisions)
}

/// Detect and collapse oscillation patterns in the path.
///
/// An oscillation is when the same netelement appears more than once with a short
/// intermediate detour—for example `A→B→C→A` where B and C cover fewer GNSS positions
/// than the first A occurrence.  The collapse merges the two A-segments and removes
/// the intermediate segments.
fn remove_oscillations(
    segments: Vec<AssociatedNetElement>,
    warnings: &mut Vec<String>,
    decisions: &mut Vec<SanityDecision>,
    pair_index: &mut usize,
) -> Vec<AssociatedNetElement> {
    if segments.len() < 3 {
        return segments;
    }

    let mut result = segments;
    let mut changed = true;

    while changed {
        changed = false;
        let mut i = 0;
        while i < result.len() {
            let ne_id = result[i].netelement_id.clone();

            // Look for the next occurrence of the same netelement.
            let mut j_opt: Option<usize> = None;
            for (k, step) in result.iter().enumerate().skip(i + 1) {
                if step.netelement_id == ne_id {
                    j_opt = Some(k);
                    break;
                }
            }

            let j = match j_opt {
                Some(v) => v,
                None => {
                    i += 1;
                    continue;
                }
            };

            // Check whether the intermediate segments are short enough to be
            // oscillation noise.  We compare the total GNSS coverage of the
            // intermediates against the first occurrence's GNSS coverage.
            let first_gnss_count = result[i]
                .gnss_end_index
                .saturating_sub(result[i].gnss_start_index);
            let intermediate_gnss_count = if j > i + 1 {
                result[j - 1]
                    .gnss_end_index
                    .saturating_sub(result[i + 1].gnss_start_index)
            } else {
                0
            };

            // Collapse when the intermediate is shorter than the first segment
            // or very short in absolute terms (< 10 GNSS positions), AND the
            // number of distinct intermediate NEs is small.  Long chains of
            // distinct NEs represent genuine path segments even when GNSS
            // coverage is low (e.g. poor satellite reception).
            let intermediate_ne_count = j - i - 1;
            let is_oscillation = intermediate_ne_count <= MAX_OSCILLATION_INTERMEDIATE_NES
                && (intermediate_gnss_count <= first_gnss_count || intermediate_gnss_count < 10);

            if !is_oscillation {
                i += 1;
                continue;
            }

            // Record the collapsed oscillation.
            let removed_ne_ids: Vec<String> = result[(i + 1)..j]
                .iter()
                .map(|s| s.netelement_id.clone())
                .collect();

            let warn_msg = format!(
                "Collapsed oscillation: {} revisited after [{}] (intermediate covered {} GNSS positions)",
                ne_id,
                removed_ne_ids.join(", "),
                intermediate_gnss_count,
            );
            warnings.push(warn_msg.clone());

            decisions.push(SanityDecision {
                pair_index: *pair_index,
                from_netelement_id: ne_id.clone(),
                to_netelement_id: ne_id.clone(),
                reachable: true,
                action: "collapsed-oscillation".to_string(),
                rerouted_via: removed_ne_ids,
                warning: warn_msg,
            });
            *pair_index += 1;

            // Merge: extend segment[i] to cover segment[j]'s GNSS range.
            result[i].gnss_end_index = result[j].gnss_end_index;

            // Update end_intrinsic from the second occurrence, unless it is a
            // bridge segment (gnss_start == gnss_end) whose intrinsic is just a
            // placeholder.
            if result[j].gnss_start_index != result[j].gnss_end_index {
                result[i].end_intrinsic = result[j].end_intrinsic;
            }

            // Remove segments (i+1)..=j  (the intermediates plus the duplicate).
            result.drain((i + 1)..=j);
            changed = true;
            // Don't increment i — the merged segment may trigger another collapse.
        }
    }

    result
}

/// Check whether a zero-weight (external) edge exists from `from` to `to` in the
/// topology graph, indicating a direct netrelation connection.
fn has_direct_connection(
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    from: &NetelementSide,
    to: &NetelementSide,
) -> bool {
    let Some(&from_idx) = node_map.get(from) else {
        return false;
    };
    let Some(&to_idx) = node_map.get(to) else {
        return false;
    };

    graph
        .edges_directed(from_idx, petgraph::Direction::Outgoing)
        .any(|e| e.target() == to_idx && *e.weight() < 1e-9)
}

/// Check if the triple of consecutive netelements A→B→C is directionally
/// consistent.  For at least one way the train can enter B from A, the
/// opposite (exit) side of B must have a direct connection to some side of C.
fn triple_is_consistent(
    ne_a: &str,
    ne_b: &str,
    ne_c: &str,
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
) -> bool {
    // Same-NE transitions are always consistent.
    if ne_a == ne_b || ne_b == ne_c {
        return true;
    }

    for a_exit in [0u8, 1] {
        let a_side = NetelementSide {
            netelement_id: ne_a.to_string(),
            position: a_exit,
        };
        for b_entry in [0u8, 1] {
            let b_entry_side = NetelementSide {
                netelement_id: ne_b.to_string(),
                position: b_entry,
            };

            if !has_direct_connection(graph, node_map, &a_side, &b_entry_side) {
                continue;
            }

            // Train enters B from b_entry, exits from opposite side.
            let b_exit_side = NetelementSide {
                netelement_id: ne_b.to_string(),
                position: 1 - b_entry,
            };

            for c_entry in [0u8, 1] {
                let c_entry_side = NetelementSide {
                    netelement_id: ne_c.to_string(),
                    position: c_entry,
                };
                if has_direct_connection(graph, node_map, &b_exit_side, &c_entry_side) {
                    return true;
                }
            }
        }
    }

    false
}

/// Minimum number of GNSS positions a real segment must span to be
/// protected from automatic removal.  Segments below this threshold are
/// considered dead-end artefacts eligible for removal; segments at or above
/// it are kept with a warning.
const DIRECTION_REMOVAL_GNSS_THRESHOLD: usize = 100;

/// Maximum number of distinct intermediate netelements that can be collapsed
/// as an oscillation.  Sequences with more intermediates are treated as
/// genuine path segments (the train actually traversed them) even when their
/// total GNSS coverage is small relative to the repeated netelement.
const MAX_OSCILLATION_INTERMEDIATE_NES: usize = 3;

/// Maximum number of neighbour removals a single netelement can cause during
/// direction-violation processing before it is force-removed as the likely
/// source of path corruption (wrong GNSS assignment).
const MAX_DIRECTION_CASCADE_REMOVALS: usize = 3;

/// Remove segments that violate directional consistency (U-turns).
///
/// Walks the path checking each triple (A, B, C).  When the triple is
/// inconsistent the function determines which segment to remove:
///
///   1. **A == C (oscillation remnant)**: remove C (the second occurrence).
///   2. **A→B connected (wrong exit)**: target B.  If B exceeds the GNSS
///      threshold, try C as a fallback.
///   3. **A→B disconnected (orphan)**: prefer bridge, then smaller of {A, B}.
///
/// Segments with fewer than [`DIRECTION_REMOVAL_GNSS_THRESHOLD`] GNSS
/// positions are always removable.  Larger real segments are kept with a
/// warning when no smaller alternative exists.
fn remove_direction_violations(
    segments: Vec<AssociatedNetElement>,
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    warnings: &mut Vec<String>,
    decisions: &mut Vec<SanityDecision>,
    pair_index: &mut usize,
) -> Vec<AssociatedNetElement> {
    if segments.len() < 3 {
        return segments;
    }

    let gnss_span = |seg: &AssociatedNetElement| -> usize {
        seg.gnss_end_index.saturating_sub(seg.gnss_start_index)
    };
    let is_bridge =
        |seg: &AssociatedNetElement| -> bool { seg.gnss_start_index == seg.gnss_end_index };
    let is_removable = |seg: &AssociatedNetElement| -> bool {
        is_bridge(seg) || gnss_span(seg) < DIRECTION_REMOVAL_GNSS_THRESHOLD
    };

    let mut result = segments;
    let mut changed = true;
    let mut warned_triples: std::collections::HashSet<(String, String, String)> =
        std::collections::HashSet::new();
    // Track how many neighbour removals each NE has caused during the
    // direction-violation cascade. Two separate counters avoid conflating
    // unrelated removal patterns:
    //
    // • *anchor*  — incremented for A when B is removed and A→B was
    //   connected (A stays, successive Bs get eaten).
    // • *protected* — incremented for B when C is removed as fallback
    //   because B was too significant (B stays, successive Cs get eaten).
    //
    // When either counter for B reaches the threshold **and** the current
    // C would be removable (i.e. the cascade would continue), B is force-
    // removed as the likely source of path corruption.  The C-removability
    // guard prevents force-removal when the cascade would naturally stop
    // at a significant C (kept-direction-warning).
    let mut cascade_as_anchor: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut cascade_as_protected: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    while changed {
        changed = false;

        let mut i = 1;
        while i < result.len().saturating_sub(1) {
            let ne_a = result[i - 1].netelement_id.clone();
            let ne_b = result[i].netelement_id.clone();
            let ne_c = result[i + 1].netelement_id.clone();

            if triple_is_consistent(&ne_a, &ne_b, &ne_c, graph, node_map) {
                i += 1;
                continue;
            }

            // ── 0. Cascade detection: force-remove B if it caused too
            //       many neighbour removals ───────────────────────────────
            let b_anchor = cascade_as_anchor.get(&ne_b).copied().unwrap_or(0);
            let b_protected = cascade_as_protected.get(&ne_b).copied().unwrap_or(0);
            let c_would_be_removable = is_removable(&result[i + 1]);
            if (b_anchor >= MAX_DIRECTION_CASCADE_REMOVALS
                || b_protected >= MAX_DIRECTION_CASCADE_REMOVALS)
                && c_would_be_removable
            {
                // B has caused too many cascade removals — it is likely a
                // wrong GNSS assignment.  Force-remove it regardless of
                // GNSS coverage.
                if !is_bridge(&result[i]) && i > 0 {
                    let end = result[i].gnss_end_index;
                    if result[i - 1].gnss_end_index < end {
                        result[i - 1].gnss_end_index = end;
                    }
                }

                let warn = format!(
                    "Removed {} (cascade: anchor={} protected={} at triple {}/{}/{})",
                    ne_b, b_anchor, b_protected, ne_a, ne_b, ne_c,
                );
                warnings.push(warn.clone());
                decisions.push(SanityDecision {
                    pair_index: *pair_index,
                    from_netelement_id: ne_a,
                    to_netelement_id: ne_c,
                    reachable: false,
                    action: "removed-direction-cascade".to_string(),
                    rerouted_via: vec![ne_b],
                    warning: warn,
                });
                *pair_index += 1;

                result.remove(i);
                changed = true;
                break;
            }

            // ── 1. Oscillation remnant (A == C): always target C ─────────
            if ne_a == ne_c {
                let target_idx = i + 1;
                let target_ne = result[target_idx].netelement_id.clone();

                if !is_bridge(&result[target_idx]) && target_idx > 0 {
                    let end = result[target_idx].gnss_end_index;
                    if result[target_idx - 1].gnss_end_index < end {
                        result[target_idx - 1].gnss_end_index = end;
                    }
                }

                let warn = format!(
                    "Removed {} (oscillation remnant: triple {}/{}/{})",
                    target_ne, ne_a, ne_b, ne_c,
                );
                warnings.push(warn.clone());
                decisions.push(SanityDecision {
                    pair_index: *pair_index,
                    from_netelement_id: ne_a,
                    to_netelement_id: ne_c,
                    reachable: false,
                    action: "removed-direction-violation".to_string(),
                    rerouted_via: vec![target_ne],
                    warning: warn,
                });
                *pair_index += 1;

                result.remove(target_idx);
                changed = true;
                break;
            }

            // ── 2. Determine primary target ──────────────────────────────
            let a_to_b_connected = (0u8..=1).any(|ap| {
                (0u8..=1).any(|bp| {
                    has_direct_connection(
                        graph,
                        node_map,
                        &NetelementSide {
                            netelement_id: ne_a.clone(),
                            position: ap,
                        },
                        &NetelementSide {
                            netelement_id: ne_b.clone(),
                            position: bp,
                        },
                    )
                })
            });

            let primary_idx = if a_to_b_connected {
                i // B
            } else {
                match (is_bridge(&result[i - 1]), is_bridge(&result[i])) {
                    (_, true) => i,
                    (true, false) => i - 1,
                    _ => {
                        if gnss_span(&result[i - 1]) <= gnss_span(&result[i]) {
                            i - 1
                        } else {
                            i
                        }
                    }
                }
            };

            // ── 3. If primary is too large, try fallback ─────────────────
            //
            // Fallback to C is only allowed when A→B is connected (wrong-
            // exit): C may be forcing B to exit on the wrong side.
            //
            // When A→B is disconnected the triple fails because of the A/B
            // gap itself; C is an innocent bystander (possibly a valid
            // intermediary for B→next) and removing it would make subsequent
            // connectivity worse.
            let target_idx = if is_removable(&result[primary_idx]) {
                primary_idx
            } else if a_to_b_connected {
                // Wrong-exit case: try C (i+1) as fallback — it may be
                // forcing the impossible exit direction on B.
                if is_removable(&result[i + 1]) {
                    i + 1
                } else {
                    // Both B and C too large — warn and skip.
                    let triple_key = (ne_a.clone(), ne_b.clone(), ne_c.clone());
                    if warned_triples.insert(triple_key) {
                        let warn = format!(
                            "Directional violation at {}/{}/{} \
                             (kept: segments too significant to remove automatically)",
                            ne_a, ne_b, ne_c,
                        );
                        warnings.push(warn.clone());
                        decisions.push(SanityDecision {
                            pair_index: *pair_index,
                            from_netelement_id: ne_a,
                            to_netelement_id: ne_c,
                            reachable: false,
                            action: "kept-direction-warning".to_string(),
                            rerouted_via: vec![ne_b],
                            warning: warn,
                        });
                        *pair_index += 1;
                    }
                    i += 1;
                    continue;
                }
            } else {
                // Disconnected case: warn and skip — do NOT eat C.
                let triple_key = (ne_a.clone(), ne_b.clone(), ne_c.clone());
                if warned_triples.insert(triple_key) {
                    let warn = format!(
                        "Directional violation at {}/{}/{} \
                         (kept: segments too significant to remove automatically)",
                        ne_a, ne_b, ne_c,
                    );
                    warnings.push(warn.clone());
                    decisions.push(SanityDecision {
                        pair_index: *pair_index,
                        from_netelement_id: ne_a,
                        to_netelement_id: ne_c,
                        reachable: false,
                        action: "kept-direction-warning".to_string(),
                        rerouted_via: vec![ne_b],
                        warning: warn,
                    });
                    *pair_index += 1;
                }
                i += 1;
                continue;
            };

            let target_ne = result[target_idx].netelement_id.clone();

            // ── 4. Remove target and extend neighbour ────────────────────
            if !is_bridge(&result[target_idx]) {
                if target_idx > 0 {
                    let end = result[target_idx].gnss_end_index;
                    if result[target_idx - 1].gnss_end_index < end {
                        result[target_idx - 1].gnss_end_index = end;
                    }
                } else if target_idx + 1 < result.len() {
                    let start = result[target_idx].gnss_start_index;
                    if result[target_idx + 1].gnss_start_index > start {
                        result[target_idx + 1].gnss_start_index = start;
                    }
                }
            }

            // ── 5. Track cascade causes ──────────────────────────────────
            // When B is removed: A is the anchor that caused the removal.
            // When C is removed (fallback because B was protected): B is
            // the anchor.  A high count for an anchor signals a wrong GNSS
            // assignment that poisons subsequent triples.
            if target_idx == i {
                *cascade_as_anchor.entry(ne_a.clone()).or_insert(0) += 1;
            } else if target_idx == i + 1 {
                *cascade_as_protected.entry(ne_b.clone()).or_insert(0) += 1;
            }

            let warn = format!(
                "Removed {} (directional violation: triple {}/{}/{})",
                target_ne, ne_a, ne_b, ne_c,
            );
            warnings.push(warn.clone());
            decisions.push(SanityDecision {
                pair_index: *pair_index,
                from_netelement_id: ne_a,
                to_netelement_id: ne_c,
                reachable: false,
                action: "removed-direction-violation".to_string(),
                rerouted_via: vec![target_ne],
                warning: warn,
            });
            *pair_index += 1;

            result.remove(target_idx);
            changed = true;
            break;
        }
    }

    result
}

/// Temporary grouping of consecutive Viterbi states on the same netelement.
struct NetelementGroup {
    netelement_id: String,
    min_intrinsic: f64,
    max_intrinsic: f64,
    first_pos_idx: usize,
    last_pos_idx: usize,
    count: usize,
}

impl NetelementGroup {
    fn new(pos_idx: usize, cand: &CandidateNetElement) -> Self {
        Self {
            netelement_id: cand.netelement_id.clone(),
            min_intrinsic: cand.intrinsic_coordinate,
            max_intrinsic: cand.intrinsic_coordinate,
            first_pos_idx: pos_idx,
            last_pos_idx: pos_idx,
            count: 1,
        }
    }

    fn update(&mut self, pos_idx: usize, cand: &CandidateNetElement) {
        self.min_intrinsic = self.min_intrinsic.min(cand.intrinsic_coordinate);
        self.max_intrinsic = self.max_intrinsic.max(cand.intrinsic_coordinate);
        self.last_pos_idx = pos_idx;
        self.count += 1;
    }

    fn to_associated_net_element(&self) -> Result<AssociatedNetElement, ProjectionError> {
        AssociatedNetElement::new(
            self.netelement_id.clone(),
            1.0, // Probability will be recomputed downstream if needed.
            self.min_intrinsic,
            self.max_intrinsic,
            self.first_pos_idx,
            self.last_pos_idx,
        )
    }
}

/// Insert bridge netelements between two consecutive observed groups
/// by recovering the Dijkstra shortest path from the cache.
#[allow(clippy::too_many_arguments)]
fn insert_bridges(
    prev: &NetelementGroup,
    next: &NetelementGroup,
    segments: &mut Vec<AssociatedNetElement>,
    _netelements: &[Netelement],
    netelement_index: &HashMap<String, usize>,
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    cache: &mut ShortestPathCache,
) -> Result<(), ProjectionError> {
    // Check direct adjacency first (any side combination connects with cost 0).
    let from_sides = candidate_sides(&prev.netelement_id);
    let to_sides = candidate_sides(&next.netelement_id);

    // Find shortest route to identify bridge NEs along the path.
    let mut best_route: Option<f64> = None;
    let mut best_from_side = 0u8;
    let mut best_to_side = 0u8;
    for from in &from_sides {
        for to in &to_sides {
            if let Some(d) = cached_shortest_path_distance(cache, graph, node_map, from, to) {
                if best_route.is_none() || d < best_route.unwrap() {
                    best_route = Some(d);
                    best_from_side = from.position;
                    best_to_side = to.position;
                }
            }
        }
    }

    if best_route.is_none() {
        // Disconnected — nothing to bridge.
        return Ok(());
    }

    // To recover intermediate netelements we would need the actual Dijkstra
    // path (not just the cost).  Since our current `shortest_path_distance`
    // only returns the cost, we do a lightweight BFS/Dijkstra trace here.
    // For now, if the route cost equals 0 (directly adjacent) we skip.
    // Otherwise we attempt to reconstruct intermediate NEs by running Dijkstra
    // and walking the predecessor map.
    let route_cost = best_route.unwrap();
    if route_cost < 1e-9 {
        // Directly adjacent — no bridges needed.
        return Ok(());
    }

    // Run Dijkstra from the best from_side and trace predecessors to best to_side.
    let from_side = NetelementSide {
        netelement_id: prev.netelement_id.clone(),
        position: best_from_side,
    };
    let to_side = NetelementSide {
        netelement_id: next.netelement_id.clone(),
        position: best_to_side,
    };

    let bridge_ne_ids = trace_intermediate_netelements(
        graph,
        node_map,
        &from_side,
        &to_side,
        &prev.netelement_id,
        &next.netelement_id,
    );

    // Build a lookup from NE id to whether it exists in netelements.
    let gnss_idx = prev.last_pos_idx; // Bridge GNSS range = gap between groups.
    for ne_id in &bridge_ne_ids {
        if netelement_index.contains_key(ne_id) {
            segments.push(AssociatedNetElement::new(
                ne_id.clone(),
                1.0, // Bridge probability
                0.0, // Full intrinsic range
                1.0,
                gnss_idx,
                gnss_idx,
            )?);
        }
    }

    Ok(())
}

/// Fill gaps left in the path after sanity validation.
///
/// Walks the final path segments and checks each consecutive pair for direct
/// topological connectivity.  When a gap is found (no direct edge), a Dijkstra
/// route is attempted and intermediate bridge netelements are inserted.
///
/// Before inserting bridges the function checks whether the route would create
/// a **U-turn** at the target segment (the last bridge NE, the target, and the
/// segment after the target form a directionally inconsistent triple).  When a
/// U-turn is detected the target segment is skipped entirely — its GNSS range
/// is absorbed by the predecessor — and the gap is re-evaluated against the
/// next segment in the path.
///
/// Returns the gap-filled path, any warnings, and structured gap-fill records.
pub fn fill_path_gaps(
    segments: Vec<AssociatedNetElement>,
    netelement_index: &HashMap<String, usize>,
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    cache: &mut ShortestPathCache,
) -> (Vec<AssociatedNetElement>, Vec<String>, Vec<GapFill>) {
    if segments.len() < 2 {
        return (segments, vec![], vec![]);
    }

    let mut result: Vec<AssociatedNetElement> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut gap_fills: Vec<GapFill> = Vec::new();

    result.push(segments[0].clone());

    // Cursor-based loop so we can skip U-turn segments and re-process
    // the gap from the same predecessor.
    let mut cursor = 1usize;
    let mut effective_prev = 0usize; // index in `segments` of the active predecessor

    while cursor < segments.len() {
        let prev = &segments[effective_prev];
        let next = &segments[cursor];

        // Same netelement — always connected.
        if prev.netelement_id == next.netelement_id {
            result.push(next.clone());
            effective_prev = cursor;
            cursor += 1;
            continue;
        }

        // Check if any side combination gives a direct connection.
        let from_sides = candidate_sides(&prev.netelement_id);
        let to_sides = candidate_sides(&next.netelement_id);

        let mut directly_connected = false;
        for from in &from_sides {
            for to in &to_sides {
                if has_direct_connection(graph, node_map, from, to) {
                    directly_connected = true;
                    break;
                }
            }
            if directly_connected {
                break;
            }
        }

        if directly_connected {
            result.push(next.clone());
            effective_prev = cursor;
            cursor += 1;
            continue;
        }

        // Not directly connected — try Dijkstra to find a route.
        let mut best_route_cost: Option<f64> = None;
        let mut best_from_side = 0u8;
        let mut best_to_side = 0u8;
        for from in &from_sides {
            for to in &to_sides {
                if let Some(d) = cached_shortest_path_distance(cache, graph, node_map, from, to) {
                    if best_route_cost.is_none() || d < best_route_cost.unwrap() {
                        best_route_cost = Some(d);
                        best_from_side = from.position;
                        best_to_side = to.position;
                    }
                }
            }
        }

        if let Some(_cost) = best_route_cost {
            let from_side = NetelementSide {
                netelement_id: prev.netelement_id.clone(),
                position: best_from_side,
            };
            let to_side = NetelementSide {
                netelement_id: next.netelement_id.clone(),
                position: best_to_side,
            };
            let bridge_ne_ids = trace_intermediate_netelements(
                graph,
                node_map,
                &from_side,
                &to_side,
                &prev.netelement_id,
                &next.netelement_id,
            );

            // ── U-turn detection ─────────────────────────────────────────
            // If the last bridge NE, the target, and the segment after the
            // target form a directionally inconsistent triple, the gap-fill
            // would force the path to enter `next` and immediately reverse.
            // Skip `next` and re-evaluate the gap from the same predecessor.
            if !bridge_ne_ids.is_empty() && cursor + 1 < segments.len() {
                let last_bridge = bridge_ne_ids.last().unwrap();
                let after_target = &segments[cursor + 1];
                if !triple_is_consistent(
                    last_bridge,
                    &next.netelement_id,
                    &after_target.netelement_id,
                    graph,
                    node_map,
                ) {
                    let warn = format!(
                        "Gap fill: removed {} (U-turn: bridge route via {} \
                         creates direction violation with successor {})",
                        next.netelement_id, last_bridge, after_target.netelement_id,
                    );
                    warnings.push(warn.clone());
                    gap_fills.push(GapFill {
                        pair_index: cursor - 1,
                        from_netelement_id: prev.netelement_id.clone(),
                        to_netelement_id: next.netelement_id.clone(),
                        route_found: false,
                        inserted_netelements: vec![],
                        warning: warn,
                    });
                    // Absorb the skipped segment's GNSS range.
                    if let Some(last) = result.last_mut() {
                        if last.gnss_end_index < next.gnss_end_index {
                            last.gnss_end_index = next.gnss_end_index;
                        }
                    }
                    // Re-process from the same predecessor.
                    cursor += 1;
                    continue;
                }
            }

            if !bridge_ne_ids.is_empty() {
                let gnss_idx = result
                    .last()
                    .map_or(prev.gnss_end_index, |r| r.gnss_end_index);
                let mut inserted = Vec::new();
                for ne_id in &bridge_ne_ids {
                    if netelement_index.contains_key(ne_id) {
                        if let Ok(bridge) = AssociatedNetElement::new(
                            ne_id.clone(),
                            1.0,
                            0.0,
                            1.0,
                            gnss_idx,
                            gnss_idx,
                        ) {
                            result.push(bridge);
                            inserted.push(ne_id.clone());
                        }
                    }
                }
                let warn = format!(
                    "Gap fill: inserted {} bridge NE(s) between {} and {}: [{}]",
                    inserted.len(),
                    prev.netelement_id,
                    next.netelement_id,
                    inserted.join(", "),
                );
                warnings.push(warn.clone());
                gap_fills.push(GapFill {
                    pair_index: cursor - 1,
                    from_netelement_id: prev.netelement_id.clone(),
                    to_netelement_id: next.netelement_id.clone(),
                    route_found: true,
                    inserted_netelements: inserted,
                    warning: warn,
                });
            } else {
                // Dijkstra found a route but no intermediate NEs (adjacent via non-zero-weight edge).
                gap_fills.push(GapFill {
                    pair_index: cursor - 1,
                    from_netelement_id: prev.netelement_id.clone(),
                    to_netelement_id: next.netelement_id.clone(),
                    route_found: true,
                    inserted_netelements: vec![],
                    warning: String::new(),
                });
            }
        } else {
            let warn = format!(
                "Gap fill: no route found between {} and {}",
                prev.netelement_id, next.netelement_id,
            );
            warnings.push(warn.clone());
            gap_fills.push(GapFill {
                pair_index: cursor - 1,
                from_netelement_id: prev.netelement_id.clone(),
                to_netelement_id: next.netelement_id.clone(),
                route_found: false,
                inserted_netelements: vec![],
                warning: warn,
            });
        }

        result.push(next.clone());
        effective_prev = cursor;
        cursor += 1;
    }

    (result, warnings, gap_fills)
}

/// Find intermediate netelement IDs along the direction-aware shortest path
/// between two netelement sides (excluding `from_ne_id` and `to_ne_id`).
fn trace_intermediate_netelements(
    graph: &DiGraph<NetelementSide, f64>,
    node_map: &HashMap<NetelementSide, NodeIndex>,
    from: &NetelementSide,
    to: &NetelementSide,
    from_ne_id: &str,
    to_ne_id: &str,
) -> Vec<String> {
    let Some(path_nodes) = shortest_path_route(graph, node_map, from, to) else {
        return vec![];
    };

    // Extract unique netelement IDs, excluding from and to NEs.
    let mut ne_ids: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for nidx in &path_nodes {
        let ne_side = &graph[*nidx];
        if ne_side.netelement_id != from_ne_id
            && ne_side.netelement_id != to_ne_id
            && seen.insert(ne_side.netelement_id.clone())
        {
            ne_ids.push(ne_side.netelement_id.clone());
        }
    }
    ne_ids
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{LineString, Point};

    fn make_ne(id: &str, coords: Vec<(f64, f64)>) -> Netelement {
        Netelement {
            id: id.to_string(),
            geometry: LineString::from(coords),
            crs: "EPSG:4326".to_string(),
        }
    }

    fn make_cand(ne_id: &str, intrinsic: f64, lon: f64, lat: f64) -> CandidateNetElement {
        CandidateNetElement {
            netelement_id: ne_id.to_string(),
            distance_meters: 5.0,
            intrinsic_coordinate: intrinsic,
            projected_point: Point::new(lon, lat),
        }
    }

    /// Simple 3-position × 2-candidate trellis where the optimal path is obvious.
    #[test]
    fn test_viterbi_simple_trellis() {
        // Two netelements: A (3.0,50.0)→(3.001,50.0) and B (3.001,50.0)→(3.002,50.0)
        let netelements = vec![
            make_ne("A", vec![(3.0, 50.0), (3.001, 50.0)]),
            make_ne("B", vec![(3.001, 50.0), (3.002, 50.0)]),
        ];
        let netelement_index: HashMap<String, usize> = [("A".to_string(), 0), ("B".to_string(), 1)]
            .into_iter()
            .collect();

        // Netrelation: A(1) → B(0) forward
        use crate::models::NetRelation;
        let netrelations = vec![NetRelation::new(
            "NR1".to_string(),
            "A".to_string(),
            "B".to_string(),
            1,
            0,
            true,
            true,
        )
        .unwrap()];

        let (graph, node_map) =
            crate::path::graph::build_topology_graph(&netelements, &netrelations).unwrap();
        let mut cache = ShortestPathCache::new();

        let config = PathConfig::default();

        // 3 positions: t0 on A, t1 on A, t2 on B
        let position_candidates = vec![
            vec![make_cand("A", 0.2, 3.0002, 50.0)],
            vec![make_cand("A", 0.8, 3.0008, 50.0)],
            vec![
                make_cand("A", 0.99, 3.00099, 50.0),
                make_cand("B", 0.1, 3.0011, 50.0),
            ],
        ];

        // Emission probabilities (higher for the "correct" candidate)
        let position_probabilities = vec![
            vec![0.9],
            vec![0.85],
            vec![0.3, 0.9], // B is much more likely at t2
        ];

        let result = viterbi_decode(
            &position_candidates,
            &position_probabilities,
            &netelements,
            &netelement_index,
            &graph,
            &node_map,
            &mut cache,
            &config,
        )
        .unwrap();

        assert_eq!(result.subsequences.len(), 1);
        let seq = &result.subsequences[0];
        assert_eq!(seq.states.len(), 3);
        // t0: candidate 0 (A)
        assert_eq!(seq.states[0], (0, 0));
        // t1: candidate 0 (A)
        assert_eq!(seq.states[1], (1, 0));
        // t2: candidate 1 (B) — higher emission probability
        assert_eq!(seq.states[2], (2, 1));
    }

    /// When no transitions are possible between disconnected netelements,
    /// the algorithm carries forward with a penalty instead of breaking,
    /// producing a single continuous subsequence.
    #[test]
    fn test_viterbi_no_break_on_disconnected() {
        // Two disconnected netelements
        let netelements = vec![
            make_ne("A", vec![(3.0, 50.0), (3.001, 50.0)]),
            make_ne("B", vec![(4.0, 51.0), (4.001, 51.0)]),
        ];
        let netelement_index: HashMap<String, usize> = [("A".to_string(), 0), ("B".to_string(), 1)]
            .into_iter()
            .collect();

        let (graph, node_map) =
            crate::path::graph::build_topology_graph(&netelements, &[]).unwrap();
        let mut cache = ShortestPathCache::new();
        let config = PathConfig::default();

        // t0 on A, t1 on B (disconnected → penalty carry-forward, no break)
        let position_candidates = vec![
            vec![make_cand("A", 0.5, 3.0005, 50.0)],
            vec![make_cand("B", 0.5, 4.0005, 51.0)],
        ];
        let position_probabilities = vec![vec![0.9], vec![0.9]];

        let result = viterbi_decode(
            &position_candidates,
            &position_probabilities,
            &netelements,
            &netelement_index,
            &graph,
            &node_map,
            &mut cache,
            &config,
        )
        .unwrap();

        // Should produce one continuous subsequence (no break)
        assert_eq!(result.subsequences.len(), 1);
        assert_eq!(result.subsequences[0].states.len(), 2);
        // t0: A, t1: B (carried forward with penalty)
        assert_eq!(
            position_candidates[result.subsequences[0].states[0].0]
                [result.subsequences[0].states[0].1]
                .netelement_id,
            "A"
        );
        assert_eq!(
            position_candidates[result.subsequences[0].states[1].0]
                [result.subsequences[0].states[1].1]
                .netelement_id,
            "B"
        );
    }

    #[test]
    fn test_viterbi_empty_input() {
        let netelements: Vec<Netelement> = vec![];
        let netelement_index = HashMap::new();
        let graph = DiGraph::new();
        let node_map = HashMap::new();
        let mut cache = ShortestPathCache::new();
        let config = PathConfig::default();

        let result = viterbi_decode(
            &[],
            &[],
            &netelements,
            &netelement_index,
            &graph,
            &node_map,
            &mut cache,
            &config,
        )
        .unwrap();

        assert!(result.subsequences.is_empty());
    }

    #[test]
    fn test_safe_ln() {
        assert_eq!(safe_ln(0.0), f64::NEG_INFINITY);
        assert_eq!(safe_ln(-1.0), f64::NEG_INFINITY);
        assert!((safe_ln(1.0) - 0.0).abs() < 1e-12);
        assert!((safe_ln(std::f64::consts::E) - 1.0).abs() < 1e-12);
    }
}
