# Train Path Calculation Algorithm

**Feature**: Continuous Train Path Calculation with Network Topology  
**Document Version**: 2.0  
**Last Updated**: June 2025

- [Train Path Calculation Algorithm](#train-path-calculation-algorithm)
  - [Overview](#overview)
  - [Algorithm Phases](#algorithm-phases)
  - [Phase 1: Candidate Selection](#phase-1-candidate-selection)
    - [Objective](#objective)
    - [Process](#process)
    - [Output](#output)
  - [Phase 2: Emission Probability](#phase-2-emission-probability)
    - [Objective](#objective-1)
    - [Formula](#formula)
    - [Distance Component](#distance-component)
    - [Heading Estimation from Adjacent Positions](#heading-estimation-from-adjacent-positions)
    - [Heading Component](#heading-component)
    - [Output](#output-1)
  - [Phase 3: Viterbi Decoding and Path Reconstruction](#phase-3-viterbi-decoding-and-path-reconstruction)
    - [Objective](#objective-2)
    - [Topology Graph Construction](#topology-graph-construction)
    - [Transition Probability](#transition-probability)
    - [Edge-Zone Optimization](#edge-zone-optimization)
    - [Log-Space Viterbi Algorithm](#log-space-viterbi-algorithm)
    - [Penalty Carry-Forward (No Viterbi Breaks)](#penalty-carry-forward-no-viterbi-breaks)
    - [Backtrace](#backtrace)
    - [Bridge Netelement Insertion](#bridge-netelement-insertion)
    - [Path Probability Calculation](#path-probability-calculation)
    - [Output](#output-2)
  - [Fallback Behavior](#fallback-behavior)
    - [Conditions for Fallback](#conditions-for-fallback)
    - [Fallback Strategy](#fallback-strategy)
  - [Performance Optimization: Resampling](#performance-optimization-resampling)
    - [Objective](#objective-3)
    - [Process](#process-1)
  - [Configuration Parameters](#configuration-parameters)
  - [Algorithm Properties](#algorithm-properties)
    - [Strengths](#strengths)
    - [Limitations](#limitations)
    - [Complexity](#complexity)
  - [References](#references)


## Overview

This document describes the HMM-based map matching algorithm for calculating a continuous train path through a rail network based on GNSS coordinate data. The algorithm uses a Hidden Markov Model (HMM) with Viterbi decoding (Newson & Krumm, 2009) to determine the **globally optimal** sequence of track segments (netelements) that the train traversed, considering network topology constraints, spatial proximity, directional alignment, and route plausibility.

## Algorithm Phases

The path calculation consists of three main phases:

1. **Candidate Selection** — Identify potential track segments for each GNSS coordinate
2. **Emission Probability** — Calculate the likelihood that each GNSS position was on each candidate segment (HMM emission model)
3. **Viterbi Decoding & Path Reconstruction** — Decode the globally optimal netelement sequence using a log-space Viterbi algorithm with transition probabilities derived from shortest-path routing, then insert bridge netelements to produce the final continuous path

---

## Phase 1: Candidate Selection

### Objective
For each GNSS coordinate, identify the track segments that could plausibly contain that position.

### Process

**For each GNSS coordinate:**

1. Find all netelements within the configured **cutoff distance** (default: 50 meters)
2. Sort candidates by distance (nearest first)
3. Select at most **N nearest netelements** (default: N = 3)
4. Project the GNSS coordinate onto each candidate to determine the exact location on the linear geometry
5. **Reject edge projections**: Remove candidates where the projection falls at the very start or end of the netelement (intrinsic coordinate < 1×10⁻⁶ or > 1 − 1×10⁻⁶). Projections at the geometric endpoints indicate the GNSS point is more likely located on an adjacent netelement; including them risks linking positions to already-passed or not-yet-reached tracks.

   **Fallback**: If *all* candidates for a position are edge projections (no interior candidate exists), none are removed. This prevents the position from having zero candidates when the GNSS point sits exactly at a netelement connection boundary.

### Output
A mapping of each GNSS coordinate to its candidate netelements with projection points.

---

## Phase 2: Emission Probability

### Objective
Calculate the emission probability that each GNSS coordinate was located on each of its candidate netelements. In HMM terms, this models how likely the observed GNSS measurement is given the hidden state (true netelement).

### Formula

**For each GNSS coordinate and its candidate netelement:**

The emission probability is:

```
P_emission(position on netelement) = P_distance × P_heading
```

### Distance Component

```
P_distance = e^(-distance / distance_scale)
```

Where:
- `distance` = perpendicular distance from GNSS coordinate to netelement (in meters)
- `distance_scale` = tunable parameter controlling decay rate

**Properties:**
- Nearer segments receive higher probability (exponential decay)
- Distance = 0 → P_distance = 1.0 (maximum)
- Distance increases → P_distance approaches 0

### Heading Estimation from Adjacent Positions

When supplied heading data is not available for a GNSS position, the system estimates heading from the geometry of adjacent positions. For position `x` (0-indexed), the estimated heading is the azimuth of the line from position `x-1` to position `x+1`.

**Formula:**
```
estimated_heading(x) = haversine_bearing(position[x-1], position[x+1])
```

**Guard conditions** — all three must pass for the estimated heading to be used:

1. **Not an endpoint**: `x` is not the first or last position in the sequence. Endpoints have no symmetric neighbors, so heading is left as `None`.
2. **Distance symmetry**: The distances from `x-1 → x` and `x → x+1` must be approximately equal:
   ```
   |dist(x-1, x) − dist(x, x+1)| / max(dist(x-1, x), dist(x, x+1)) < 0.20
   ```
   A ratio difference ≥ 20% indicates the position may be at a curve apex or the spacing is irregular, making the two-neighbor azimuth unreliable.
3. **Heading continuity**: The change between consecutive estimated headings must be < 5°. If the heading change between `estimated_heading(x)` and `estimated_heading(x-1)` exceeds 5°, discard `estimated_heading(x)` (set to `None`). This rejects implausible heading jumps that a train cannot physically produce between consecutive positions.
4. **Bearing deviation guard**: The forward bearing (from `x-1 → x`) and backward bearing (from `x → x+1`) must not diverge by more than 5°. Specifically:
   ```
   |haversine_bearing(x-1, x) − haversine_bearing(x, x+1)| ≤ 5°
   ```
   When the two half-bearings diverge (≥ 5°), the midpoint azimuth is unreliable (e.g. at a curve apex), and the estimated heading is discarded (set to `None`).

Estimated headings are computed once for the entire working position set before Phase 2 probability calculation begins.

### Heading Component

When heading data is available (either supplied or estimated from neighbors):

```
P_heading = e^(-heading_difference / heading_scale)  [if heading_difference ≤ cutoff]
P_heading = 0                                          [if heading_difference > cutoff]
```

Where:
- `heading_difference` = angular difference between GNSS heading and netelement heading at the projection point, with bidirectional track equivalence (a heading and its opposite are both considered aligned), range [0, 90]
- `heading_scale` = tunable parameter controlling decay rate
- `cutoff` = configurable threshold in [0, 90] (default: 10 degrees)

**Special Cases:**
- No heading data available (neither supplied nor estimable from neighbors) → P_heading = 1.0 (no heading constraint)
- Heading difference > cutoff → P_heading = 0 (hard reject)

### Output
For each GNSS position-netelement pair: a probability score between 0 and 1.

---

## Phase 3: Viterbi Decoding and Path Reconstruction

### Objective
Decode the globally optimal sequence of netelements using the Viterbi algorithm on a Hidden Markov Model. 

### Topology Graph Construction

Before Viterbi decoding, a directed graph is built from the rail network:

**Graph Structure:**
- **Nodes** = netelement sides. Each netelement has two sides: start (intrinsic = 0) and end (intrinsic = 1).
- **Internal edges**: Connect start → end and end → start of the same netelement, weighted with the **haversine length** of the netelement geometry (in meters).
- **Connection edges**: Derived from netrelations, connecting the appropriate sides of adjacent netelements based on `positionOnA`/`positionOnB` and navigability. These edges have weight **0.0** (zero cost to cross a netelement connection).

This graph representation enables Dijkstra shortest-path queries between any two netelement sides.

**Implementation**: `DiGraph<NetelementSide, f64>` from the petgraph crate. A `node_map: HashMap<NetelementSide, NodeIndex>` provides O(1) lookup.

### Transition Probability

The transition probability models how plausible it is for the train to move from one candidate netelement to another between consecutive GNSS observations. It follows the formulation of Newson & Krumm (2009):

```
P_transition(i → j) = exp(-|d_route - d_gc| / β)  ×  exp(-turn_angle / turn_scale)
```

Where:
- `d_route` = shortest-path distance through the topology graph from the projected point on candidate `i` to the projected point on candidate `j` (computed via Dijkstra)
- `d_gc` = great-circle (haversine) distance between the two projected points
- `β` = scale parameter (default: 50.0 meters); higher values tolerate larger detours
- `turn_angle` = directional heading difference (0–180°) between the exit heading from candidate `i`'s netelement and the entry heading into candidate `j`'s netelement at the connection point
- `turn_scale` = turn-angle penalty scale (default: 30.0 degrees); smaller values penalise sharper turns more aggressively

**Route direction**: For each transition, all four (from_side, to_side) combinations are evaluated, and the combination with the highest combined probability (route distance + turn angle) is kept. The exit heading is derived from the last segment of the from-netelement in the direction of travel, and the entry heading from the first segment of the to-netelement in the direction of travel.

**Properties:**
- When `d_route ≈ d_gc` (direct route) and the connection is straight-through, P_transition ≈ 1.0
- When `d_route ≫ d_gc` (large detour), P_transition → 0
- Same netelement → P_transition = 1.0 (no route needed)
- Sharp turn at a connection (high `turn_angle`) → P_transition reduced by the turn-angle factor

**Shortest-path caching**: Results are cached in a `ShortestPathCache: HashMap<(String, u8, String, u8), Option<f64>>` keyed by `(from_ne_id, from_side, to_ne_id, to_side)`. This avoids redundant Dijkstra runs for recurring origin-destination pairs.

### Edge-Zone Optimization

To reduce unnecessary Dijkstra computations on long netelements where the train is clearly in the interior (far from any netelement connection), an edge-zone check is applied:

A candidate is **near a netelement edge** if the haversine distance from its projected point to the nearest endpoint of the netelement geometry is ≤ `edge_zone_distance` (default: 50.0 meters).

**Optimization rules:**
- If both candidates `i` and `j` are on the **same netelement** → P_transition = 1.0 (skip Dijkstra)
- If both candidates are on **different netelements** and both are in the **interior** (not near an edge) → P_transition = 0.0 (impossible to transition without passing through a netelement connection)
- Otherwise → compute P_transition via Dijkstra normally

### Log-Space Viterbi Algorithm

The Viterbi algorithm operates in **log-space** to prevent numerical underflow on long GNSS sequences. All probabilities are stored as natural logarithms.

**Trellis construction:**

For each time-step `t` and each candidate `j`:

```
log_V[t][j] = max_i { log_V[t-1][i] + ln(P_transition(i → j)) + ln(P_emission(t, j)) }
backptr[t][j] = argmax_i { ... }
```

**Initialization (t = 0):**
```
log_V[0][j] = ln(P_emission(0, j))
backptr[0][j] = None
```

The algorithm processes each time-step sequentially, computing the best predecessor for each current candidate based on the sum of (1) the previous best log-probability, (2) the log-transition probability, and (3) the log-emission probability.

### Penalty Carry-Forward (No Viterbi Breaks)

When **all** transition scores at a time-step `t` are `-∞` (no feasible transition from any previous state to any current candidate), the algorithm does **not** create a Viterbi break. Instead, it uses **penalty carry-forward** to maintain a single continuous chain:

1. Find the best previous candidate `i*` (highest `log_V[t-1][i*]`)
2. Compute a carry-forward score: `carry_score = log_V[t-1][i*] + NO_TRANSITION_PENALTY` where `NO_TRANSITION_PENALTY = ln(1×10⁻¹⁰) ≈ −23`
3. For each current candidate `j` with non-zero emission: `log_V[t][j] = carry_score + ln(P_emission(t, j))`
4. Set `backptr[t][j] = i*` so the backtrace follows the best previous state

This produces a **single unbroken subsequence** for all GNSS input (the GNSS data represents one continuous drive). The heavy penalty ensures that carry-forward transitions are strongly disfavoured relative to genuine topological transitions, but the chain is never severed.

**Important**: Because carry-forward preserves chain continuity, the backtrace always yields exactly one subsequence covering the entire GNSS timeline.

### Backtrace

After the forward pass, the single subsequence is decoded via standard backtrace:

1. Find the candidate with the highest `log_V[t_end][j]` in the final time-step
2. Follow `backptr[t][j]` backwards to the start of the sequence
3. Collect `(position_index, candidate_index)` pairs for each time-step

The result is a `ViterbiResult` containing one `ViterbiSubsequence`.

### Bridge Netelement Insertion

Viterbi states represent only the netelements that had GNSS candidates. When consecutive Viterbi states are on **non-adjacent** netelements, the intervening netelements (bridges) must be recovered:

**Process:**
For each pair of consecutive Viterbi states `(NE_A, NE_B)`:
1. If `NE_A == NE_B` → no bridge needed
2. If `NE_A` and `NE_B` are directly connected via a netrelation → no bridge needed
3. Otherwise, use the Dijkstra predecessor map to trace the intermediate netelements along the shortest path from `NE_A` to `NE_B`
4. Insert the bridge netelements between the two states in the final path

Bridge netelements are **not** hidden Viterbi states — they are purely a post-processing step to ensure path continuity.

### Path Probability Calculation

The overall path probability is derived from the Viterbi log-probabilities:

```
avg_log_prob = (Σ log_probability of all subsequences) / (total number of Viterbi states)
path_probability = min(exp(avg_log_prob), 1.0)
```

This represents the geometric mean of per-state probabilities, clamped to [0, 1].

### Output
A single optimal `TrainPath` consisting of an ordered list of `AssociatedNetElement` segments with intrinsic coordinate ranges, plus an overall probability score.

---

## Fallback Behavior

### Conditions for Fallback

Path calculation may fail to produce a valid path when:
- No continuous navigable path exists through the network
- Network topology is incomplete or disconnected
- GNSS data quality is insufficient (e.g., large gaps, all positions beyond cutoff distance)
- All candidate paths have probability = 0

### Fallback Strategy

When path calculation fails:
1. **Notify user** that path calculation failed
2. **Fall back to simple projection:** Project each GNSS coordinate independently to its nearest netelement
3. **Ignore navigability:** No topology constraints applied
4. **Output generated:** Results clearly indicate fallback mode was used

---

## Performance Optimization: Resampling

### Objective
Reduce computational cost when processing high-frequency GNSS data without significantly impacting accuracy.

### Process

1. **Calculate Mean Spacing:**
   - Sample random pairs of consecutive GNSS coordinates
   - Compute distance between each pair (using `distance` column if available)
   - Calculate mean distance between neighboring coordinates

2. **Determine Resampling Factor:**
   ```
   resampling_factor = resampling_distance / mean_spacing
   ```
   
   Example: If mean_spacing = 1m and resampling_distance = 10m → use every 10th coordinate

3. **Apply to Path Calculation Only:**
   - Use resampled subset for Phases 1-5 (candidate selection through path selection)
   - Use **all** GNSS coordinates for final projection onto selected path
   - Ensures path calculation efficiency while maintaining projection accuracy

---

## Configuration Parameters

The algorithm exposes several tunable parameters:

| Parameter | Default | Purpose |
|-----------|---------|---------|
| Max nearest netelements | 3 | Limits candidate segments per GNSS position |
| Distance cutoff | 50 meters | Maximum distance to consider a segment candidate |
| Heading difference cutoff | 10 degrees | Hard threshold for heading alignment |
| Minimum probability threshold | 2% | Minimum emission probability for segment inclusion |
| Resampling distance | 10 meters | Target spacing for performance optimization |
| Distance scale | 10.0 meters | Controls distance probability decay rate (exponential decay) |
| Heading scale | 2.0 degrees | Controls heading probability decay rate (exponential decay) |
| Beta (β) | 50.0 meters | Transition probability scale (Newson & Krumm). Controls tolerance for mismatch between route distance and great-circle distance. Higher values are more forgiving of detours. |
| Edge-zone distance | 50.0 meters | Distance threshold from projected point to nearest netelement endpoint. Candidates farther than this from any endpoint are considered interior and cannot transition to a different netelement (transition probability = 0). |
| Turn-angle penalty scale | 30.0 degrees | Controls how aggressively sharp turns at netelement connections are penalised. `exp(-turn_angle / turn_scale)`: smaller values yield stronger penalty for the same angle. |

---

## Algorithm Properties

### Strengths

- **Globally Optimal:** Viterbi decoding finds the most probable netelement sequence across the entire journey, avoiding locally greedy decisions at individual netelement connections
- **Topology-Aware:** Transition probabilities incorporate actual route distances through the rail network graph
- **Robust to Noise:** The HMM formulation naturally smooths noisy GNSS data by combining emission and transition evidence
- **Handles Gaps:** Penalty carry-forward keeps the Viterbi chain continuous even through disconnected network regions; bridge insertion recovers intervening netelements
- **Scalable:** Edge-zone optimization and shortest-path caching prevent redundant Dijkstra runs, keeping performance practical on large networks
- **Graceful Degradation:** Fallback mode ensures output even when optimal path cannot be determined

### Limitations

- **Assumes Single Traversal:** Cannot handle loops where the same segment is traversed multiple times
- **Offline Only:** Not designed for real-time streaming processing
- **Requires Quality Topology:** Network data must be accurate and complete
- **Parameter Sensitivity:** The β parameter and edge-zone distance require tuning for different network geometries

### Complexity

- **Time:** O(N × M² × D) where:
  - N = number of GNSS positions (after resampling)
  - M = average number of candidate netelements per position (typically 3)
  - D = average cost of a Dijkstra shortest-path query (amortised by caching)
- **Space:** O(N × M) for the Viterbi trellis, plus O(E) for the topology graph where E = edges in the network

---

## References

- Newson, P. & Krumm, J. (2009). "Hidden Markov Map Matching Through Noise and Sparseness." *ACM SIGSPATIAL GIS 2009*. [PDF](https://www.microsoft.com/en-us/research/publication/hidden-markov-map-matching-through-noise-and-sparseness/)
- Feature Specification: [spec.md](spec.md)
- Functional Requirements: FR-013 through FR-032
- Configuration Parameters: See spec.md Configuration Parameters section
