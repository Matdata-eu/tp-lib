# Research: Continuous Train Path Calculation

**Feature**: 002-train-path-calculation  
**Date**: January 9, 2026  
**Phase**: 0 - Research & Outline

- [Research: Continuous Train Path Calculation](#research-continuous-train-path-calculation)
  - [Overview](#overview)
  - [1. Network Topology Graph Representation](#1-network-topology-graph-representation)
    - [Decision](#decision)
    - [Rationale](#rationale)
    - [Alternatives Considered](#alternatives-considered)
    - [Integration Pattern](#integration-pattern)
  - [2. Exponential Decay Probability Formula](#2-exponential-decay-probability-formula)
    - [Decision](#decision-1)
    - [Rationale](#rationale-1)
    - [Probability Examples](#probability-examples)
    - [Properties Validation](#properties-validation)
    - [Alternatives Considered](#alternatives-considered-1)
  - [3. Path Construction Algorithm: Bidirectional Validation](#3-path-construction-algorithm-bidirectional-validation)
    - [Decision](#decision-2)
    - [Rationale](#rationale-2)
    - [Alternatives Considered](#alternatives-considered-2)
    - [Graph Traversal Implementation](#graph-traversal-implementation)
  - [4. Distance Coverage Correction Factor](#4-distance-coverage-correction-factor)
    - [Decision](#decision-3)
    - [Rationale](#rationale-3)
    - [Example Scenario](#example-scenario)
    - [Alternatives Considered](#alternatives-considered-3)
  - [5. Reusing Existing Codebase Functions](#5-reusing-existing-codebase-functions)
    - [Functions to Reuse](#functions-to-reuse)
      - [Spatial Indexing (projection/spatial.rs)](#spatial-indexing-projectionspatialrs)
      - [Geometry Operations (projection/geom.rs)](#geometry-operations-projectiongeomrs)
      - [Data Models](#data-models)
      - [I/O Functions (io/geojson.rs, io/csv.rs)](#io-functions-iogeojsonrs-iocsvrs)
    - [Code Reuse Strategy](#code-reuse-strategy)
  - [6. Performance Optimization: Resampling Strategy](#6-performance-optimization-resampling-strategy)
    - [Decision](#decision-4)
    - [Rationale](#rationale-4)
    - [Performance Impact](#performance-impact)
    - [Alternatives Considered](#alternatives-considered-4)
  - [7. Error Handling Strategy](#7-error-handling-strategy)
    - [Decision](#decision-5)
    - [Fallback Behavior](#fallback-behavior)
    - [Rationale](#rationale-5)
  - [8. Testing Strategy](#8-testing-strategy)
    - [Test Coverage Plan](#test-coverage-plan)
      - [Unit Tests (`tests/unit/`)](#unit-tests-testsunit)
      - [Integration Tests (`tests/integration/`)](#integration-tests-testsintegration)
      - [Contract Tests (`tests/contract/`)](#contract-tests-testscontract)
      - [Property-Based Tests (using `proptest` crate)](#property-based-tests-using-proptest-crate)
      - [Performance Benchmarks (`benches/`)](#performance-benchmarks-benches)
    - [TDD Workflow Enforcement](#tdd-workflow-enforcement)
  - [9. CLI Interface Design](#9-cli-interface-design)
    - [Decision](#decision-6)
    - [Rationale](#rationale-6)
  - [10. Dependencies and Licenses](#10-dependencies-and-licenses)
    - [New Dependencies](#new-dependencies)
    - [License Verification](#license-verification)
  - [Summary of Research Outcomes](#summary-of-research-outcomes)
    - [Technical Decisions Finalized](#technical-decisions-finalized)
    - [Unknowns Resolved](#unknowns-resolved)
    - [Ready for Phase 1](#ready-for-phase-1)


## Overview

This document consolidates research findings for implementing probabilistic train path calculation through a rail network. It resolves all technical unknowns from the Technical Context and provides rationale for key design decisions.

---

## 1. Network Topology Graph Representation

### Decision
Use **petgraph** crate (MIT OR Apache-2.0 license) with a directed graph where:
- **Nodes = netelement sides** (each netelement has two sides: start with intrinsic=0 and end with intrinsic=1)
- **Edges = two types:**
  1. **Internal edges**: Connect start side to end side of the same netelement (representing traversal along the track segment)
  2. **Connection edges**: Netrelations connecting specific sides of different netelements (via positionOnA/positionOnB)
- Edge weights = unused (navigability is binary: allowed or not)

### Rationale
- **Accurate topology**: NetRelations connect to specific ends of netelements (positionOnA/positionOnB = 0 or 1), so nodes must represent these connection points
- **Proper directionality**: A train traverses a netelement from one side to the other, and can enter/exit at either end
- **Enables complex junctions**: Multiple netelements can connect to the same side (e.g., three tracks converging at a switch)
- **Established library**: petgraph is the de facto standard for graph algorithms in Rust (>7M downloads)
- **Rich API**: Provides traversal algorithms (DFS, BFS), topological operations, and path finding
- **Zero-copy construction**: Can build graph from indices without duplicating netelement data
- **License compatible**: MIT OR Apache-2.0 matches project requirements
- **Performance**: Optimized graph representation with O(1) edge lookup via adjacency lists

### Alternatives Considered
1. **Nodes = netelements** (original approach): Simpler but loses information about which end of the track the connection occurs; doesn't properly model positionOnA/positionOnB
2. **Custom graph implementation**: More control but significant development time; petgraph provides all needed algorithms
3. **graphlib crate**: Less mature, fewer features, smaller ecosystem
4. **No explicit graph**: Traverse via linear search through netrelations; O(N) per connection lookup vs O(1) with graph

### Integration Pattern
```rust
use petgraph::graph::{DiGraph, NodeIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct NetelementSide {
    netelement_id: String,
    position: u8, // 0 = start, 1 = end
}

// Build graph once from network data
let mut graph = DiGraph::<NetelementSide, ()>::new();
let node_map: HashMap<NetelementSide, NodeIndex> = HashMap::new();

// For each netelement: create two nodes and internal edge
for netelement in netelements {
    let start_side = NetelementSide { netelement_id: netelement.id.clone(), position: 0 };
    let end_side = NetelementSide { netelement_id: netelement.id.clone(), position: 1 };
    
    let start_node = graph.add_node(start_side);
    let end_node = graph.add_node(end_side);
    
    node_map.insert(start_side, start_node);
    node_map.insert(end_side, end_node);
    
    // Internal edges: bidirectional traversal within netelement
    graph.add_edge(start_node, end_node, ()); // forward
    graph.add_edge(end_node, start_node, ()); // backward
}

// For each netrelation: add edge between specified sides
for relation in netrelations {
    let side_a = NetelementSide { 
        netelement_id: relation.from_netelement_id.clone(), 
        position: relation.position_on_a 
    };
    let side_b = NetelementSide { 
        netelement_id: relation.to_netelement_id.clone(), 
        position: relation.position_on_b 
    };
    
    let node_a = node_map[&side_a];
    let node_b = node_map[&side_b];
    
    // Add edges based on navigability
    if relation.navigable_forward {
        graph.add_edge(node_a, node_b, ());
    }
    if relation.navigable_backward {
        graph.add_edge(node_b, node_a, ());
    }
}
```

---

## 2. Exponential Decay Probability Formula

### Decision
Use **negative exponential decay** for both distance and heading components:
```rust
P_distance = (-distance / distance_scale).exp()
P_heading = (-heading_diff / heading_scale).exp()
P_combined = P_distance * P_heading
```

Default scale parameters:
- `distance_scale` = 10.0 meters (tunable)
- `heading_scale` = 2.0 degrees (tunable)

### Rationale
- **Smooth decay**: Exponential function provides gradual probability reduction rather than hard cutoffs
- **Interpretable parameters**: Scale parameter controls how quickly probability drops (larger scale = slower decay)
- **Multiplicative combination**: Independent probability assumptions; both conditions must hold
- **Standard in positioning**: Widely used in GPS/GNSS error modeling and map matching algorithms

### Probability Examples

**Distance component** (with distance_scale = 10.0 m):

| Distance | Formula | P_distance | Interpretation |
|----------|---------|-----------|----------------|
| 0 m | exp(-0/10) | 1.000 (100%) | Perfect spatial match |
| 2.5 m | exp(-2.5/10) | 0.779 (78%) | Very close, high confidence |
| 5 m | exp(-5/10) | 0.607 (61%) | Close, good candidate |
| 10 m | exp(-10/10) | 0.368 (37%) | At scale distance, moderate confidence |
| 15 m | exp(-15/10) | 0.223 (22%) | Distant, low confidence |
| 20 m | exp(-20/10) | 0.135 (14%) | Very distant, very low confidence |
| 30 m | exp(-30/10) | 0.050 (5%) | Near cutoff threshold |

**Heading component** (with heading_scale = 2.0°):

| Heading Diff | Formula | P_heading | Interpretation |
|--------------|---------|----------|----------------|
| 0° | exp(-0/2) | 1.000 (100%) | Perfect directional match |
| 1° | exp(-1/2) | 0.606 (61%) | Very aligned |
| 2° | exp(-2/2) | 0.368 (37%) | At scale difference, aligned |
| 3° | exp(-3/2) | 0.223 (22%) | Moderately aligned |
| 5° | exp(-5/2) | 0.082 (8%) | Poorly aligned, near cutoff |
| 10° | exp(-10/2) | 0.007 (0.7%) | Severely misaligned, rejected |

**Combined probability** (P_combined = P_distance × P_heading):

| Scenario | Distance | Heading | P_distance | P_heading | P_combined | Assessment |
|----------|----------|---------|-----------|-----------|-----------|------------|
| Perfect match | 0 m | 0° | 1.000 | 1.000 | **1.000 (100%)** | Ideal candidate |
| Very good | 2.5 m | 1° | 0.779 | 0.606 | **0.472 (47%)** | Strong candidate |
| Good | 5 m | 1° | 0.607 | 0.606 | **0.368 (37%)** | Solid candidate |
| Moderate | 5 m | 2° | 0.607 | 0.368 | **0.223 (22%)** | Acceptable |
| Acceptable | 10 m | 2° | 0.368 | 0.368 | **0.135 (14%)** | Marginal |
| Poor | 15 m | 3° | 0.223 | 0.223 | **0.050 (5%)** | Near threshold |
| Very poor | 20 m | 5° | 0.135 | 0.082 | **0.011 (1%)** | Likely rejected |

**Key insights:**
- At scale parameters (10m, 2°): probability ≈ 37% for single component, ≈ 14% combined
- Default probability threshold (25%) filters out candidates worse than ~7.5m with 2° or ~10m with 1°
- Heading misalignment penalizes more aggressively than distance (smaller scale = steeper decay)
- Perfect spatial match (0m) can tolerate up to ~3° heading difference to stay above 25% threshold

### Properties Validation
- Distance = 0 → P = 1.0 (exact match)
- Distance = distance_scale → P = e^(-1) ≈ 0.37 (37% probability)
- Distance = 3 × distance_scale → P ≈ 0.05 (5% probability, near cutoff)
- Heading perfectly aligned → P = 1.0
- Heading 180° opposite → P = 0.0 (after accounting for bidirectionality)

### Alternatives Considered
1. **Linear decay**: P = 1 - (distance / max_distance); less realistic for sensor noise
2. **Gaussian**: P = exp(-distance^2 / (2 * σ^2)); more complex, similar results for typical parameters
3. **Inverse distance**: P = 1 / (1 + distance); no clear physical interpretation

---

## 3. Path Construction Algorithm: Bidirectional Validation

### Decision
Implement **forward and backward path construction** with consistency validation:

**Forward Construction:**
1. Start from netelement with highest probability at first GNSS position
2. For each subsequent position, select next netelement that is:
   - Navigable (connected via netrelations graph)
   - Above probability threshold (default 25%)
   - Spatially progressing (avoid backtracking)
3. Build ordered path: [A, B, C, D]

**Backward Construction:**
1. Start from netelement with highest probability at last GNSS position
2. For each previous position, select previous netelement (reverse navigation)
3. Build path in backward direction: [D, C, B, A]
4. **Reverse the path** for comparison:
   - Reverse segment order: [D, C, B, A] → [A, B, C, D]
   - Swap start/end intrinsics for each segment: if segment had start=0.2, end=0.8 → becomes start=0.8, end=0.2

**Validation:**
- Compare reversed backward path with forward path
- Accept path if forward == backward (same segment sequence and intrinsic ranges)
- If forward != backward: path probability = (P_forward + 0) / 2 (unidirectional path)
- If path terminates early (no navigable connection): probability = 0

### Rationale
- **Bidirectional validation**: Ensures path consistency; reduces false positives from noisy data
- **Handles ambiguity**: When multiple paths exist, bidirectional agreement increases confidence
- **Graceful degradation**: Unidirectional paths still valid but penalized (50% probability reduction)
- **Prevents dead ends**: Early termination detection avoids incomplete paths

### Alternatives Considered
1. **Forward only**: Faster but less robust; may select wrong path at ambiguous junctions
2. **All-paths search**: Exponential complexity; impractical for large networks

### Graph Traversal Implementation
Use petgraph's `neighbors()` for O(1) edge lookup:
```rust
// Forward: find netelements navigable FROM current segment
let candidates = graph.neighbors(current_node)
    .filter(|&next| meets_probability_threshold(next, position));

// Backward: find netelements navigable TO current segment (reverse edges)
let candidates = graph.neighbors_directed(current_node, Direction::Incoming)
    .filter(|&prev| meets_probability_threshold(prev, position));
```

---

## 4. Distance Coverage Correction Factor

### Decision
Calculate **consecutive position coverage** for each netelement:

```rust
// For netelement with associated GNSS positions [P1, P3, P4, P7]
// Identify consecutive pairs: (P1→P3 gap), (P3→P4 consecutive), (P4→P7 gap)

let consecutive_distance = sum of distances between consecutive positions only;
let total_distance = distance from first to last associated GNSS position;
let coverage_factor = consecutive_distance / total_distance;

// Final netelement probability
P(netelement) = P_avg × coverage_factor
```

**Consecutive definition**: Positions are consecutive if they appear sequentially in the original GNSS coordinate list (no other positions between them).

### Rationale
- **Prevents false positives**: Segments that pass several times near the ground truth segment can also contain hits but not consecutive
- **Distance-weighted**: Longer coverage (more consecutive positions) increases confidence
- **Normalized**: Division by total distance ensures correction factor ∈ [0, 1]

### Example Scenario
```
Netelement A (100m long):
- Associated positions: P1 (0m), P10 (50m), P11 (60m), P30 (100m)
- Consecutive pairs: P10→P11 (10m)
- Total span: P1→P30 (100m)
- Coverage factor: 10/100 = 0.1
- Even with high individual probabilities, low coverage reduces final score
```

### Alternatives Considered
1. **Simple average**: P = mean(all position probabilities); ignores coverage gaps
2. **Count-based**: P = P_avg × (consecutive_count / total_count); doesn't weight by distance
3. **No correction**: Proximity alone insufficient for path determination

---

## 5. Reusing Existing Codebase Functions

### Functions to Reuse

#### Spatial Indexing (projection/spatial.rs)
**Reuse**: `NetworkIndex` with R-tree for candidate selection
```rust
pub struct NetworkIndex {
    tree: RTree<NetelementIndexEntry>,
    netelements: Vec<Netelement>,
}

impl NetworkIndex {
    pub fn find_nearest_within_distance(&self, point: Point, max_distance: f64, limit: usize) 
        -> Vec<(usize, f64)>
}
```
**Why**: Already implements efficient O(log N) spatial queries; no need to rewrite.

#### Geometry Operations (projection/geom.rs)
**Reuse**: `project_point_onto_linestring()` for finding projection points
```rust
pub fn project_point_onto_linestring(
    point: &Point<f64>,
    linestring: &LineString<f64>
) -> (Point<f64>, f64) // Returns (projected_point, intrinsic_coordinate)
```
**Why**: Accurate projection with intrinsic coordinates already implemented and tested.

**Reuse**: `calculate_measure_along_linestring()` for distance calculations
```rust
pub fn calculate_measure_along_linestring(
    point: &Point<f64>,
    linestring: &LineString<f64>
) -> f64 // Returns distance along linestring to projection point
```
**Why**: Needed for calculating traversal distances and consecutive position spans.

#### Data Models
**Extend** (not replace):
- `GnssPosition`: Add optional `heading: Option<f64>` and `distance: Option<f64>` fields
- `Netelement`: Already has id, geometry, CRS; use as-is
- `ProjectedPosition`: May extend with path context in future

**New Models Required**:
- `NetRelation`: Topology connection between netelements
- `AssociatedNetElement`: Netelement with probability and projection details in a path
- `TrainPath`: Ordered list of AssociatedNetElements
- `GnssNetElementLink`: Link between a single GNSS position and a candidate netelement

#### I/O Functions (io/geojson.rs, io/csv.rs)
**Extend**: `parse_network_geojson()` to handle netrelations
```rust
// Current: parses netelements only
pub fn parse_network_geojson(path: &Path) -> Result<Vec<Netelement>, ProjectionError>

// Extended: parse both netelements and netrelations from single feature collection
pub fn parse_network_geojson(path: &Path) 
    -> Result<(Vec<Netelement>, Vec<NetRelation>), ProjectionError>
```

**Why**: Specification clarified that network file is a single GeoJSON feature collection with `type` property distinguishing netelements from netrelations.

**New I/O**:
- `parse_train_path_csv()` / `write_train_path_csv()`
- `parse_train_path_geojson()` / `write_train_path_geojson()`

### Code Reuse Strategy
1. **Minimize duplication**: Leverage all existing spatial and geometry functions
2. **Extend, don't replace**: Add fields to existing models where possible
3. **Preserve API compatibility**: New path calculation is additive, doesn't break existing projection API
4. **Follow existing patterns**: Match error handling, testing structure, and documentation style

---

## 6. Performance Optimization: Resampling Strategy

### Decision
Implement **configurable GNSS coordinate resampling** for path calculation only:

```rust
// Configuration
let resampling_distance: Option<f64> = Some(10.0); // meters

// Calculate mean spacing between consecutive GNSS positions
let mean_spacing = calculate_mean_gnss_spacing(&gnss_positions);

// Resampling factor: every Nth position
let resample_every = (resampling_distance / mean_spacing).ceil() as usize;

// Select subset for path calculation
let resampled_positions: Vec<&GnssPosition> = gnss_positions
    .iter()
    .step_by(resample_every)
    .collect();

// Use resampled set for Phases 1-5 (path calculation)
let path = calculate_train_path(&resampled_positions, network)?;

// Use FULL set for final projection onto path
let projected = project_onto_path(&gnss_positions, &path)?;
```

### Rationale
- **Reduces computational load**: High-frequency GNSS (1m spacing) generates excessive candidates; 10m resampling reduces positions by 90%
- **Maintains path accuracy**: Path structure determined by overall trajectory, not every single point
- **Preserves projection precision**: All original coordinates projected onto calculated path
- **Configurable trade-off**: Users can tune resampling distance based on their accuracy vs speed requirements

### Performance Impact
For 10,000 GNSS positions at 1m spacing with 10m resampling:
- Path calculation uses 1,000 positions (10x reduction)
- Candidate selection: 10,000 spatial queries → 1,000 queries
- Probability calculations: 10,000 × 3 candidates → 1,000 × 3 candidates
- Path construction: Fewer positions to traverse
- **Estimated speedup**: 5-10x for path calculation phase

### Alternatives Considered
1. **No resampling**: Simple but slow for high-frequency data
2. **Adaptive resampling**: Variable density based on path complexity; adds implementation complexity (can be considered for a later feature release)
3. **Parallel processing**: More engineering effort; Rust async overhead may exceed gains for CPU-bound work

---

## 7. Error Handling Strategy

### Decision
Extend existing `ProjectionError` enum with path-specific variants:

```rust
#[derive(Debug, Error)]
pub enum ProjectionError {
    // Existing variants
    #[error("Invalid geometry: {0}")]
    InvalidGeometry(String),
    
    #[error("Empty network")]
    EmptyNetwork,
    
    // NEW: Path calculation errors
    #[error("No netrelations found in network data")]
    NoNetRelations,
    
    #[error("Invalid netrelation: {0}")]
    InvalidNetRelation(String),
    
    #[error("No navigable path found from {start} to {end}")]
    NoNavigablePath { start: String, end: String },
    
    #[error("Path calculation failed: {reason}")]
    PathCalculationFailed { reason: String },
    
    #[error("All candidate paths below probability threshold ({threshold})")]
    BelowProbabilityThreshold { threshold: f64 },
}
```

### Fallback Behavior
When path calculation fails (no navigable path found):
1. Log warning with diagnostic information (positions involved, candidate segments, topology constraints)
2. Fall back to independent projection mode (existing behavior from feature 001)
3. Clearly indicate in output that fallback mode was used
4. Return `Ok(result)` (not error) since independent projection succeeds

### Rationale
- **Typed errors**: Each failure mode has specific error type with context
- **Graceful degradation**: Fallback ensures output even when topology incomplete
- **Actionable diagnostics**: Error messages guide users to fix data issues
- **Maintains compatibility**: Fallback uses existing projection code, no duplication

---

## 8. Testing Strategy

### Test Coverage Plan

#### Unit Tests (`tests/unit/`)
- **Probability calculations**: Test exponential decay formulas, edge cases (distance=0, heading=180°)
- **Candidate selection**: Verify spatial queries, distance filtering, limit enforcement
- **Coverage correction**: Test consecutive position detection, distance accumulation
- **Graph construction**: Validate node/edge creation from netrelations
- **Path validation**: Test forward/backward comparison, unidirectional path handling

#### Integration Tests (`tests/integration/`)
- **End-to-end scenarios**:
  1. Simple linear path: 10 GNSS positions, 5 segments, single path
  2. Branch junction: Path splits, correct branch selected based on probability
  3. Missing connection: Navigability gap triggers fallback
  4. Ambiguous path: Multiple paths, highest probability wins
  5. Sparse GNSS data: Large gaps between positions
  6. High-frequency data: Resampling enabled, verify path accuracy maintained
  
#### Contract Tests (`tests/contract/`)
- **API stability**: Verify public function signatures don't change
- **Backward compatibility**: Existing projection API still works
- **Data format stability**: CSV/GeoJSON schemas versioned

#### Property-Based Tests (using `proptest` crate)
- **Probability properties**:
  - P ∈ [0, 1] for all inputs
  - P_distance decreases monotonically with distance
  - P_heading = 1 when heading_diff = 0
- **Path properties**:
  - All segments in path are navigable (edge exists in graph)
  - Path is continuous (no gaps)
  - Forward and backward paths are consistent

#### Performance Benchmarks (`benches/`)
- **Candidate selection**: Time to find N nearest segments for M positions
- **Path calculation**: End-to-end time for various dataset sizes (100, 1k, 10k positions)
- **Resampling impact**: Compare performance with/without resampling

### TDD Workflow Enforcement
Per Constitution Principle IV (NON-NEGOTIABLE):
1. Write test describing desired behavior (test fails)
2. Seek user/stakeholder approval of test
3. Implement minimal code to pass test
4. Refactor while keeping test green
5. Repeat for each function/feature

---

## 9. CLI Interface Design

### Decision
Extend `tp-cli` with three command modes:

1. **Default command** `tp-cli` - Combined workflow (calculate + project)
2. **calculate-path** - Path calculation only
3. **simple-projection** - Legacy independent projection

```bash
# 1. Default: Calculate path and project coordinates (one-step workflow)
tp-cli \
  --gnss gnss_data.csv \
  --network network.geojson \
  --output projected.csv

# With algorithm parameters
tp-cli \
  --gnss gnss_data.csv \
  --network network.geojson \
  --output projected.csv \
  --distance-scale 10.0 \
  --heading-scale 2.0 \
  --cutoff-distance 50.0 \
  --heading-cutoff 5.0 \
  --probability-threshold 0.25 \
  --resampling-distance 10.0 \
  --max-candidates 3

# Save path alongside projection
tp-cli \
  --gnss gnss_data.csv \
  --network network.geojson \
  --output projected.csv \
  --save-path path.json

# 2. Path calculation only (for inspection/editing/reuse)
tp-cli calculate-path \
  --gnss gnss_data.csv \
  --network network.geojson \
  --output path.json

# 3. Use pre-calculated path
tp-cli \
  --gnss gnss_data.csv \
  --network network.geojson \
  --train-path existing_path.json \
  --output projected.csv

# 4. Legacy independent projection (backward compatibility)
tp-cli simple-projection \
  --gnss gnss_data.csv \
  --network network.geojson \
  --output projected.csv
```

### Rationale
- **Default command**: Simplest workflow for most users (calculate + project in one step)
- **calculate-path**: Enables path inspection, editing, and reuse
- **simple-projection**: Provides feature 001 backward compatibility
- **--train-path option**: Allows using pre-calculated paths with default command
- **Configuration via flags**: All algorithm parameters exposed as CLI options with defaults
- **Multiple output formats**: Support CSV and GeoJSON
- **Composable**: Separate commands enable debugging, caching, and flexible workflows

---

## 10. Dependencies and Licenses

### New Dependencies

| Dependency | Version | License | Purpose | Approval |
|------------|---------|---------|---------|----------|
| petgraph | ~1.0 | MIT OR Apache-2.0 | Graph data structure and algorithms | ✅ APPROVED |

### License Verification
- **petgraph**: Dual-licensed MIT OR Apache-2.0 → Compatible with project's Apache-2.0 license
- **No GPL/LGPL/proprietary**: Confirmed clean dependency tree
- **CI license scanning**: Add `cargo-deny` to CI pipeline to enforce license compliance

---

## Summary of Research Outcomes

### Technical Decisions Finalized
1. ✅ Network topology: petgraph directed graph
2. ✅ Probability formula: Negative exponential decay for distance and heading
3. ✅ Path construction: Bidirectional validation with consistency check
4. ✅ Coverage correction: Consecutive position distance weighting
5. ✅ Code reuse: NetworkIndex, projection functions, I/O infrastructure
6. ✅ Performance: Configurable resampling for high-frequency GNSS data
7. ✅ Error handling: Extend ProjectionError, graceful fallback to independent projection
8. ✅ Testing: Unit, integration, contract, property-based, benchmarks
9. ✅ CLI: Three-command architecture (default, calculate-path, simple-projection)
10. ✅ Dependencies: petgraph (Apache-2.0 compatible)

### Unknowns Resolved
- ~~How to represent network topology~~ → petgraph directed graph
- ~~Probability formula details~~ → Exponential decay with scale parameters
- ~~Path validation approach~~ → Bidirectional construction with consistency check
- ~~Coverage correction calculation~~ → Consecutive position distance ratio
- ~~Resampling strategy~~ → Configurable distance-based resampling
- ~~License compatibility~~ → All dependencies Apache-2.0 compatible

### Ready for Phase 1
All technical unknowns resolved. Proceeding to:
- **data-model.md**: Define NetRelation, TrainPath, AssociatedNetElement structs
- **contracts/**: Document API signatures and CLI interface contracts
- **quickstart.md**: Provide usage examples and integration patterns

---

**Phase 0 Complete** | Next: Phase 1 - Design & Contracts
