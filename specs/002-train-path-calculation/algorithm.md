# Train Path Calculation Algorithm

**Feature**: Continuous Train Path Calculation with Network Topology  
**Document Version**: 1.0  
**Last Updated**: January 8, 2026

- [Train Path Calculation Algorithm](#train-path-calculation-algorithm)
  - [Overview](#overview)
  - [Algorithm Phases](#algorithm-phases)
  - [Phase 1: Candidate Selection](#phase-1-candidate-selection)
    - [Objective](#objective)
    - [Process](#process)
    - [Output](#output)
  - [Phase 2: Individual Position Probability](#phase-2-individual-position-probability)
    - [Objective](#objective-1)
    - [Formula](#formula)
    - [Distance Component](#distance-component)
    - [Heading Component](#heading-component)
    - [Output](#output-1)
  - [Phase 3: Aggregate Netelement Probability](#phase-3-aggregate-netelement-probability)
    - [Objective](#objective-2)
    - [Formula](#formula-1)
    - [Average Position Probability (P\_avg)](#average-position-probability-p_avg)
    - [Distance Coverage Correction (C\_distance)](#distance-coverage-correction-c_distance)
    - [Output](#output-2)
  - [Phase 4: Path Construction](#phase-4-path-construction)
    - [Objective](#objective-3)
    - [Forward Path Construction](#forward-path-construction)
    - [Backward Path Construction](#backward-path-construction)
    - [Path Validation](#path-validation)
    - [Output](#output-3)
  - [Phase 5: Path Selection](#phase-5-path-selection)
    - [Objective](#objective-4)
    - [Path Probability Calculation](#path-probability-calculation)
    - [Selection](#selection)
  - [Fallback Behavior](#fallback-behavior)
    - [Conditions for Fallback](#conditions-for-fallback)
    - [Fallback Strategy](#fallback-strategy)
  - [Performance Optimization: Resampling](#performance-optimization-resampling)
    - [Objective](#objective-5)
    - [Process](#process-1)
  - [Configuration Parameters](#configuration-parameters)
  - [Algorithm Properties](#algorithm-properties)
    - [Strengths](#strengths)
    - [Limitations](#limitations)
    - [Complexity](#complexity)
  - [References](#references)


## Overview

This document describes the probabilistic algorithm for calculating a continuous train path through a rail network based on GNSS coordinate data. The algorithm determines the most likely sequence of track segments (netelements) that the train traversed, considering network topology constraints, spatial proximity, and directional alignment.

## Algorithm Phases

The path calculation consists of five main phases:

1. **Candidate Selection** - Identify potential track segments for each GNSS coordinate
2. **Individual Position Probability** - Calculate likelihood that each GNSS position was on each candidate segment
3. **Aggregate Netelement Probability** - Calculate overall probability for each track segment across all positions
4. **Path Construction** - Build candidate paths following navigability rules
5. **Path Selection** - Choose the optimal path based on probability scores

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

### Output
A mapping of each GNSS coordinate to its candidate netelements with projection points.

---

## Phase 2: Individual Position Probability

### Objective
Calculate the probability that each GNSS coordinate was located on each of its candidate netelements.

### Formula

**For each GNSS coordinate and its candidate netelement:**

The probability that the GNSS position was on the netelement is calculated as:

```
P(position on netelement) = P_distance × P_heading
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

### Heading Component

When heading data is available:

```
P_heading = e^(-heading_difference / heading_scale)  [if heading_difference ≤ cutoff]
P_heading = 0                                          [if heading_difference > cutoff]
```

Where:
- `heading_difference` = minimum angular difference between GNSS heading and netelement heading at the projection point
- Accounts for 180° equivalence (opposite orientation, same path)
- `heading_scale` = tunable parameter controlling decay rate
- `cutoff` = configurable threshold (default: 5 degrees)

**Special Cases:**
- No heading data available → P_heading = 1.0 (no heading constraint)
- Heading difference > cutoff → P_heading = 0 (hard reject)

### Output
For each GNSS position-netelement pair: a probability score between 0 and 1.

---

## Phase 3: Aggregate Netelement Probability

### Objective
Calculate the overall probability that each netelement was part of the actual train path by aggregating evidence from all associated GNSS positions.

### Formula

**For each netelement, two probability values are computed with different roles:**

```
P_avg     = (Σ P(position_i on netelement)) / N      ← used for threshold insertion
coverage_prob = C_coverage × P_avg                   ← used for junction selection / BFS
```

- `P_avg` (raw average quality) is used to decide whether the netelement is **inserted** into the candidate map at all. Using the raw average preserves the original threshold behaviour and avoids unfairly excluding short segments that have only a few GNSS associations.
- `coverage_prob` (coverage-adjusted quality) is used when **comparing** competing netelements at junctions and during BFS candidate ranking. It rewards segments with extensive absolute GNSS coverage.

### Average Position Probability (P_avg)

```
P_avg = (Σ P(position_i on netelement)) / N
```

Where N = number of GNSS positions associated with this netelement.

This represents the mean individual probability across all positions that considered this netelement as a candidate.

### Coverage Factor (C_coverage)

```
covered_meters = (max_intrinsic − min_intrinsic) × netelement_length
C_coverage     = max(covered_meters / R, N / T).min(1.0)
```

Where:
- `max_intrinsic`, `min_intrinsic` = maximum and minimum intrinsic coordinates of all GNSS projections onto the netelement (range over [0, 1])
- `netelement_length` = Haversine length of the netelement geometry in metres
- `R` = 500 m (reference coverage length — a netelement covered by 500 m of GNSS track receives full score)
- `N` = number of GNSS positions associated with this netelement
- `T` = total number of GNSS positions in the working set

**Two-term max ensures fair scoring in both regimes:**
- `covered_meters / R` rewards netelements with large absolute GNSS footprint (e.g., 880 m on a 1 024 m segment → 1.0)
- `N / T` provides a proportional fallback for isolated single-point associations where the intrinsic range is zero

**Examples:**

| Scenario | NE Length | GNSS count (N) | Total GNSS (T) | covered_meters | C_coverage | Reasoning |
|----------|-----------|----------------|----------------|---------------|------------|-----------|
| Full-length coverage | 1000m | 50 | 50 | 950m | 1.00 | Exceeds 500m reference |
| Good coverage | 1000m | 20 | 40 | 600m | 1.00 | Exceeds 500m reference |
| Partial coverage | 1000m | 10 | 40 | 300m | max(0.60, 0.25) = 0.60 | covered_meters term dominates |
| Short netelement | 135m | 2 | 6 | 130m | max(0.26, 0.33) = 0.33 | count term dominates |
| Single position | 100m | 1 | 6 | 0m | max(0.00, 0.17) = 0.17 | count term prevents zero |

**Purpose:**
- Rewards netelements with large absolute GNSS coverage over those that were only briefly observed
- The `N / T` fallback ensures isolated points are not silently dropped when comparing junction branches
- Decoupling from netelement length means a short 135 m segment and a long 1 024 m segment are evaluated on the same absolute scale

### Output
For each netelement: `P_avg` (for threshold check in Phase 4) and `coverage_prob` (for junction selection in Phases 4–5).

---

## Phase 4: Path Construction

### Objective
Build candidate paths through the network that satisfy continuity and navigability constraints.

### Forward Path Construction

**Starting Point:**
- Begin with the netelement having the highest probability among candidates for the **first GNSS position**

**Iterative Process:**

For each position along the candidate path:

1. **Current Segment:** Track the current netelement and its orientation
2. **Find Connections:** Query netrelations where:
   - `elementA` or `elementB` matches the current netelement
   - `positionOnA` or `positionOnB` matches the current netelement's end position (forward traversal)
3. **Filter by Navigability:**
   - Check if the netrelation allows traversal in the direction of travel
   - Navigability values: `"both"`, `"AB"`, `"BA"`, `"none"`
4. **Filter by Probability:**
   - Check if the connected netelement has `P_avg` ≥ minimum threshold (default: 25%)
   - **Exception:** If it's the only navigable connection, include it regardless of probability
5. **Bridge BFS for topology gaps:**
   - If a direct neighbour is **not** in the probability map (no GNSS evidence), perform a bounded BFS (up to 10 hops) through topology-only netelements to find the nearest mapped netelement
   - Bridge netelements (no GNSS evidence) are inserted with probability 1.0 (topologically certain) to avoid artificially reducing overall path probability
   - At junctions where multiple mapped candidates exist at the same BFS hop distance, the one with the highest `coverage_prob` is selected
6. **Handle Branching:**
   - If multiple valid next segments exist → select the one with the highest `coverage_prob`
7. **Termination:**
   - Path continues until reaching coverage of the last GNSS position
   - Path assigned probability = 0 if it terminates prematurely

### Backward Path Construction

**Starting Point:**
- Begin with the netelement having the highest probability among candidates for the **last GNSS position**

**Process:**
- Identical to forward construction but traverse in reverse direction
- Follow netrelations in the opposite navigability direction
- Construct path from last to first GNSS position

**Reversal for Comparison:**
Before comparing with the forward path, the backward path must be reversed:
1. **Reverse segment order**: [D, C, B, A] → [A, B, C, D]
2. **Swap intrinsic coordinates** for each AssociatedNetElement:
   - Original: `start_intrinsic=0.2, end_intrinsic=0.8`
   - Reversed: `start_intrinsic=0.8, end_intrinsic=0.2`

This ensures the backward path represents the same physical traversal as the forward path for proper comparison.

### Path Validation

A path is considered **valid** if:
1. It exists in at least one direction (forward or backward construction)
2. All connections respect navigability constraints
3. The path spans from first to last GNSS position

**Note:** Paths existing in only one direction are still valid but will have reduced probability (see Phase 5).

### Output
A set of valid candidate paths, each with forward and/or backward probability scores.

---

## Phase 5: Path Selection

### Objective
Select the single optimal path from all valid candidates based on probability scores.

### Path Probability Calculation

For each valid candidate path:

```
P(path) = [ P_forward(path) + P_backward(path) ] / 2
```

Where:

```
P_forward(path) = Σ(P(netelement_i) × length_i) / Σ(length_i)
P_backward(path) = Σ(P(netelement_i) × length_i) / Σ(length_i)
```

- Length-weighted average of constituent netelement probabilities
- Longer segments have more influence on path probability
- Computed separately for forward and backward constructions
- **If path exists in only one direction, use 0 for the missing direction**
  - Example: Path only exists forward → P(path) = P_forward(path) / 2
  - This is equivalent to having the path in both directions where one direction has 0 probability
- Final path probability is always the average of both directions (existing and/or 0)

### Selection

The path with the **highest final probability** is selected as the train path.

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
| Heading difference cutoff | 5 degrees | Hard threshold for heading alignment |
| Minimum probability threshold | 25% | Minimum probability to include segment in path |
| Resampling distance | 10 meters | Target spacing for performance optimization |
| Distance scale | 10.0 meters | Controls distance probability decay rate (exponential decay) |
| Heading scale | 2.0 degrees | Controls heading probability decay rate (exponential decay) |

---

## Algorithm Properties

### Strengths

- **Topology-Aware:** Respects rail network structure and navigability rules
- **Robust:** Handles noisy GNSS data through probabilistic smoothing
- **Bidirectional:** Forward/backward validation ensures path consistency
- **Coverage-Sensitive:** Distance correction favors segments with continuous GNSS coverage
- **Graceful Degradation:** Fallback mode ensures output even when optimal path cannot be determined

### Limitations

- **Assumes Single Traversal:** Cannot handle loops where the same segment is traversed multiple times
- **Offline Only:** Not designed for real-time streaming processing
- **Requires Quality Topology:** Network data must be accurate and complete
- **Parameter Sensitivity:** Performance depends on appropriate configuration for operational context

### Complexity

- **Time:** O(N × M × B) where:
  - N = number of GNSS positions
  - M = average number of candidate netelements per position
  - B = average branching factor in path construction
- **Space:** O(P × L) where:
  - P = number of candidate paths
  - L = average path length in segments

---

## References

- Feature Specification: [spec.md](spec.md)
- Functional Requirements: FR-013 through FR-032
- Configuration Parameters: See spec.md Configuration Parameters section
