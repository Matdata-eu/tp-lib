# Train Path Calculation Algorithm

**Feature**: Continuous Train Path Calculation with Network Topology  
**Document Version**: 1.0  
**Last Updated**: January 8, 2026

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

**For each netelement, considering all associated GNSS positions:**

```
P(netelement in path) = P_avg × C_distance
```

### Average Position Probability (P_avg)

```
P_avg = (Σ P(position_i on netelement)) / N
```

Where N = number of GNSS positions associated with this netelement.

This represents the mean individual probability across all positions that considered this netelement as a candidate.

### Distance Coverage Correction (C_distance)

```
C_distance = (Σ consecutive_distances) / total_netelement_length
```

**Calculation:**
1. Identify groups of **consecutive** GNSS positions associated with the netelement
2. For each consecutive pair, calculate the distance traveled between them:
   - Use `distance` column values if available (wheel sensor data)
   - Otherwise, compute geometric distance between coordinates
3. Sum all consecutive distances
4. Divide by the total geometric length of the netelement

**Examples:**

| Scenario | Netelement Length | GNSS Associations | Consecutive Distances | C_distance | Reasoning |
|----------|-------------------|-------------------|----------------------|------------|-----------|
| Excellent coverage | 100m | 20 positions covering 95m | 95m | 0.95 | Good quality |
| Continuous coverage | 100m | Positions 5,6,7,8 (10m+10m+10m) | 30m | 0.30 | Partial coverage |
| Partial coverage | 100m | Positions 3,4 and 8,9 (10m + 10m) | 20m | 0.20 | Two separate consecutive groups |
| Sparse coverage | 100m | Positions 5,10 (both isolated) | 0m | 0.00 | Non-consecutive = no path continuity |
| Single position | 100m | Position 5 only | 0m | 0.00 | No consecutive pairs |

**Purpose:**
- Ensures netelements are only probable if GNSS data shows **continuous travel** along them
- Prevents selection of segments where the train merely passed nearby without traversing
- Acts as a coverage quality metric

### Output
For each netelement: an aggregate probability score accounting for both position alignment and coverage continuity.

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
   - Check if the connected netelement has probability ≥ minimum threshold (default: 25%)
   - **Exception:** If it's the only navigable connection, include it regardless of probability
5. **Handle Branching:**
   - If multiple valid next segments exist → create separate path branches
   - Track each branch independently
6. **Termination:**
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
| Distance scale | TBD | Controls distance probability decay rate |
| Heading scale | TBD | Controls heading probability decay rate |

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
