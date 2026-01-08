# Feature Specification: Continuous Train Path Calculation with Network Topology

**Feature Branch**: `002-train-path-calculation`  
**Created**: January 8, 2026  
**Status**: Draft  
**Input**: User description: "Implement continuous train path calculation with network topology and probabilistic routing"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Calculate Train Path from GNSS Data (Priority: P1)

A railway engineer processes GNSS coordinate data collected during a continuous train journey. The system analyzes the network topology and calculates the most probable continuous path the train took through the rail network, considering navigability constraints between track segments. The engineer receives a validated train path that accurately represents the journey.

**Why this priority**: This is the core functionality that enables accurate positioning on the rail network. Without a continuous path, coordinate projections may jump between disconnected track segments, making the data unusable for operational analysis.

**Independent Test**: Can be fully tested by providing GNSS coordinates and a network with netrelations, then verifying that the output path is continuous and all connections are navigable.

**Acceptance Scenarios**:

1. **Given** GNSS coordinates from a continuous train journey and a rail network with netrelations, **When** the user requests path calculation, **Then** the system returns an ordered list of connected track segments that form a continuous, navigable path
2. **Given** GNSS coordinates with heading information, **When** the system calculates the path, **Then** track segments whose heading differs by more than the configured cutoff from GNSS heading are excluded
3. **Given** multiple possible paths through the network, **When** path calculation completes, **Then** the system selects the path with the highest probability based on distance and heading alignment

---

### User Story 2 - Project Coordinates on Calculated Path (Priority: P2)

After the train path is calculated, a railway analyst projects the GNSS coordinates onto this specific path. Each coordinate is assigned to a precise location on the continuous path with intrinsic coordinates, rather than being projected to the nearest arbitrary track segment. This provides accurate linear referencing along the actual journey route.

**Why this priority**: Accurate projection on the known path is essential for detailed analysis, but requires the path calculation (P1) to be completed first. This delivers the final usable positioning data.

**Independent Test**: Can be tested by providing a pre-calculated train path and GNSS coordinates, then verifying each coordinate is projected onto the correct segment within that path with accurate intrinsic coordinates.

**Acceptance Scenarios**:

1. **Given** a calculated train path and GNSS coordinates, **When** projection is performed, **Then** each coordinate is projected onto one of the track segments in the path with intrinsic coordinates between 0 and 1
2. **Given** coordinates that fall between track segments in the path, **When** projection occurs, **Then** the coordinate is assigned to the nearest segment within the path
3. **Given** a pre-supplied train path file, **When** the user provides it as input, **Then** the system skips path calculation and directly projects coordinates onto the supplied path

---

### User Story 3 - Export Train Path Only (Priority: P3)

A railway operations team needs to visualize and validate the calculated train path without processing all coordinate projections. They export just the train path in a structured format (CSV or GeoJSON) showing the ordered sequence of track segments with their connection points. This allows quick validation of the routing logic before processing large datasets.

**Why this priority**: Enables debugging and validation of path calculation independently from coordinate projection. Useful for tuning parameters and understanding system behavior, but not required for basic functionality.

**Independent Test**: Can be tested by requesting path-only export mode and verifying the output contains the ordered track segment sequence without coordinate projection data.

**Acceptance Scenarios**:

1. **Given** GNSS data and a request for path-only export, **When** processing completes, **Then** the system outputs the train path in the requested format (CSV or GeoJSON) without projecting individual coordinates
2. **Given** an exported train path file, **When** inspected, **Then** it shows the ordered sequence of track segments with their intrinsic coordinate ranges and connection information
3. **Given** a failed path calculation, **When** path-only export is requested, **Then** the system reports that no path could be calculated and provides diagnostic information

---

### User Story 4 - Enhanced GNSS Data with Heading and Distance (Priority: P2)

A railway engineer provides GNSS data that includes heading (direction relative to north in degrees) and distance measurements from wheel sensors. The system uses this additional information to improve path calculation accuracy by comparing GNSS heading with track segment orientation and using precise distance measurements for path probability calculations.

**Why this priority**: Significantly improves path calculation accuracy when this data is available, but the system should still work with basic coordinate-only data. This is a quality enhancement rather than a fundamental requirement.

**Independent Test**: Can be tested by providing GNSS data with heading and distance columns, then comparing path calculation results against the same coordinates without this information to verify improved accuracy.

**Acceptance Scenarios**:

1. **Given** GNSS coordinates with heading values, **When** path calculation runs, **Then** the system compares GNSS heading against track segment heading and reduces probability for segments with misaligned orientation
2. **Given** GNSS coordinates with distance values, **When** calculating traveled distance between points, **Then** the system uses the provided distance values instead of computing geometric distance
3. **Given** GNSS data without heading or distance columns, **When** processing occurs, **Then** the system proceeds with path calculation using only coordinate positions

---

### User Story 5 - Performance-Optimized Processing (Priority: P3)

When processing dense GNSS data (coordinates recorded every meter or less), a railway analyst configures the resampling distance parameter to reduce computational load. The system intelligently samples coordinates at the specified interval for path calculation, maintaining accuracy while processing large datasets efficiently.

**Why this priority**: Enables practical processing of high-frequency GNSS data, but the core functionality works without this optimization. Important for production use with real-world data volumes.

**Independent Test**: Can be tested by processing the same dataset with different resampling values and measuring execution time and path calculation accuracy.

**Acceptance Scenarios**:

1. **Given** GNSS coordinates approximately 1 meter apart and a resampling parameter of 10 meters, **When** path calculation starts, **Then** the system uses approximately every 10th coordinate for path calculation
2. **Given** GNSS data with varying spacing, **When** resampling is configured, **Then** the system calculates mean distance between neighboring coordinates and resamples accordingly
3. **Given** GNSS data with distance column values, **When** determining coordinate spacing, **Then** the system uses distance values to calculate mean spacing between points

---

### User Story 6 - Fallback to Simple Projection (Priority: P2)

When processing challenging data where no continuous path can be calculated (e.g., network topology errors, disconnected track segments, or data quality issues), the system automatically falls back to projecting each coordinate onto its nearest track segment. The user is notified of the fallback and receives the best possible output despite the path calculation failure.

**Why this priority**: Ensures the system remains useful even when ideal conditions aren't met. Provides graceful degradation rather than complete failure, maintaining some utility of the output.

**Independent Test**: Can be tested by providing data that cannot form a continuous path (disconnected network segments) and verifying the system produces simple projection results with appropriate warnings.

**Acceptance Scenarios**:

1. **Given** GNSS coordinates and a network where no continuous navigable path exists, **When** path calculation fails, **Then** the system notifies the user and falls back to simple nearest-segment projection
2. **Given** a fallback projection result, **When** the user examines the output, **Then** it clearly indicates that path calculation failed and simple projection was used instead
3. **Given** network topology with navigation restrictions that prevent path calculation, **When** processing completes in fallback mode, **Then** each coordinate is still projected to its geometrically nearest segment regardless of navigability

---

### User Story 7 - Debug Path Calculation with Intermediate Results (Priority: P4)

A developer troubleshooting path calculation issues exports intermediate results showing all candidate paths with their probability scores, track segment candidates for each coordinate, and the decision tree used to select the final path. This diagnostic output helps tune configuration parameters and understand why certain paths were chosen or rejected.

**Why this priority**: Essential for development and troubleshooting but not needed for normal operations. This is a developer/support tool rather than an end-user feature.

**Independent Test**: Can be tested by enabling debug export mode and verifying that intermediate files contain expected probability calculations, candidate paths, and decision criteria.

**Acceptance Scenarios**:

1. **Given** debug export mode is enabled, **When** path calculation runs, **Then** the system exports files showing all candidate paths with their probability scores
2. **Given** intermediate result files, **When** examined, **Then** they show for each GNSS coordinate which track segments were considered and their calculated probabilities
3. **Given** debug output of path candidates, **When** reviewed, **Then** it shows both forward and backward path calculations with the probability averaging that led to final path selection

---

## Clarifications

### Session 2026-01-08

- Q: When GNSS coordinates fall outside the configured cutoff distance (default 50m) from all track segments during the projection phase (after path is calculated), how should the system handle these outliers? → A: Exclude outlier coordinates from output entirely (omit from results file). Future feature will address better handling.
- Q: How should the distance between a GNSS coordinate and a candidate netelement be factored into the probability calculation? → A: Inverse exponential decay based on both distance (e.g., e^(-distance/scale)) and heading difference (e.g., e^(-heading_diff/scale))
- Q: When multiple candidate paths have identical probability scores (after forward/backward averaging), which path should be selected? → A: Select the first path found during calculation (arbitrary but deterministic)
- Q: When a pre-calculated train path is provided as input (FR-041), what format should the system expect? → A: Same format as path-only export: CSV or GeoJSON with ordered AssociatedNetElements
- Q: When the system encounters invalid netrelations (e.g., elementA equals elementB, or references to non-existent netelement IDs), how should it proceed? → A: Skip invalid netrelations, log warnings, and continue processing with remaining valid topology
- Q: How are netelements and netrelations structured in the network GeoJSON file? → A: Single feature collection where features have a "type" property with value "netelement" or "netrelation"
- Q: How should netelement probability be calculated considering GNSS position coverage? → A: Average probability of associated GNSS positions multiplied by distance correction factor: (sum of distances between consecutive associated positions) / (total netelement length). Non-consecutive positions contribute 0 to distance sum.
- Q: What is the assumed sensor accuracy for heading data? → A: Less than 2° typical error (not 5°)
- Q: What constitutes good continuous coverage of a netelement? → A: Coverage above 90% (C_distance > 0.90) is considered good quality

---

### Edge Cases

- GNSS coordinates more than the configured cutoff distance (default 50m) from any track segment are excluded from output (omitted from results)
- NetRelations where elementA equals elementB (self-referencing) are skipped with warnings logged
- NetRelations referencing non-existent netelement IDs are skipped with warnings logged; segments with only invalid netrelations are treated as isolated
- What happens when a track segment has no netrelations connecting it to other segments (isolated segment)?
- How does the system behave when GNSS heading values are invalid (outside 0-360 degrees range)?
- What happens when distance values in GNSS data are inconsistent or decreasing (suggesting data errors)?
- How does the system handle very short track segments where start and end positions are nearly identical?
- When multiple paths have identical probability scores, the first path found during calculation is selected
- How does the system behave when all track segments within cutoff distance have probability 0 (all below threshold or heading misalignment)?
- What happens when netrelation navigability is "none" for all connections from a track segment in the middle of a calculated path?
- How does the system handle GNSS data files with missing coordinate values or malformed data?
- What happens when the resampling distance parameter is larger than the total journey distance?

## Requirements *(mandatory)*

### Functional Requirements

#### Network Topology

- **FR-001**: System MUST accept network GeoJSON input containing a single feature collection where features are distinguished by a "type" property with value "netelement" or "netrelation"
- **FR-002**: Each netrelation MUST have point coordinates representing the connection location between two track segments
- **FR-003**: Each netrelation MUST have properties: navigability, positionOnA, positionOnB, elementA, and elementB
- **FR-004**: Navigability property MUST support values: "both" (navigable in both directions), "none" (not navigable), "AB" (navigable from A to B only), "BA" (navigable from B to A only)
- **FR-005**: positionOnA and positionOnB MUST be either 0 (geometric start of track segment) or 1 (geometric end of track segment), never decimal values
- **FR-006**: elementA MUST reference a valid netelement ID and MUST be distinct from elementB
- **FR-006a**: System MUST skip invalid netrelations (elementA equals elementB, or references to non-existent netelement IDs), log warnings, and continue processing with remaining valid topology
- **FR-007**: elementB MUST reference a valid netelement ID and MUST be distinct from elementA

#### GNSS Input Enhancement

- **FR-008**: System MUST accept optional "heading" column/property in GNSS coordinate input containing decimal degree values (0-360)
- **FR-009**: System MUST accept optional "distance" column/property in GNSS coordinate input containing decimal distance values
- **FR-010**: Heading values MUST represent direction relative to north where 0° = north, 90° = east, 180° = south, 270° = west
- **FR-011**: Distance values MUST be used relatively to calculate traveled distance between consecutive GNSS positions when present
- **FR-012**: System MUST continue to support GNSS input without heading or distance columns (backward compatibility)

#### Path Calculation

- **FR-013**: System MUST calculate a train path represented as an ordered list of AssociatedNetElements
- **FR-014**: Each AssociatedNetElement MUST reference one netelement and include begin and end intrinsic coordinates (0-1 range)
- **FR-015**: The calculated train path MUST be continuous (each segment connects to the next via a netrelation)
- **FR-016**: All netrelations between consecutive segments in the path MUST have navigability in the direction of travel (not "none" or opposing direction)
- **FR-017**: System MUST find at most N nearest netelements for each GNSS coordinate (where N is configurable, default 3)
- **FR-018**: System MUST only consider netelements within a configurable cutoff distance (default 50 meters) from each GNSS coordinate
- **FR-018a**: System MUST exclude from output any GNSS coordinates that are more than the cutoff distance from all track segments in the calculated path
- **FR-019**: System MUST calculate probability for each candidate netelement using inverse exponential decay for both distance (e.g., e^(-distance/distance_scale)) and heading alignment (e.g., e^(-heading_difference/heading_scale)), with the overall probability being the product of distance and heading probability factors
- **FR-020**: System MUST set probability to 0 when heading difference between GNSS coordinate and netelement exceeds configurable cutoff (default 5 degrees), overriding exponential decay calculation
- **FR-021**: When calculating heading for a netelement at a projection point, system MUST consider the heading at that specific location on the linear geometry
- **FR-022**: System MUST recognize that netelement heading and GNSS heading can be 180° apart and still be aligned (opposite orientation but same path)
- **FR-023**: System MUST calculate total probability for each netelement as: (average of probabilities from all associated GNSS positions) × (distance correction factor), where distance correction factor = (sum of distances between consecutive associated GNSS positions) / (total length of netelement). Non-consecutive GNSS positions contribute 0 to the distance sum. If no consecutive positions exist, the correction factor is 0.
- **FR-024**: System MUST perform path calculation in forward direction starting from the most probable netelement at the beginning
- **FR-025**: System MUST perform path calculation in backward direction starting from the most probable netelement at the end
- **FR-026**: System MUST only include netelements in a path that are both navigable (per netrelations) and above minimum probability threshold (default 25%)
- **FR-027**: System MUST allow navigable netelements below probability threshold only when it is the only navigable option
- **FR-028**: System MUST assign probability 0 to any path that terminates before reaching the end of the GNSS coordinate sequence
- **FR-029**: System MUST calculate path probability as the length-weighted average of constituent netelement probabilities
- **FR-030**: System MUST calculate path probability as the average of forward and backward path probabilities: P(path) = [P_forward(path) + P_backward(path)] / 2
- **FR-031**: For paths that exist in only one direction, system MUST use 0 as the probability for the missing direction when calculating the average (e.g., if only forward path exists: P(path) = P_forward(path) / 2)
- **FR-032**: System MUST select the path with the highest final probability as the train path; if multiple paths have identical probability, the first path found during calculation is selected

#### Performance Optimization

- **FR-033**: System MUST support a configurable resampling distance parameter (default 10 meters) to reduce GNSS coordinates used in path calculation
- **FR-034**: When resampling is configured, system MUST calculate mean distance between neighboring GNSS coordinates
- **FR-035**: System MUST use distance column values to calculate spacing between coordinates when available
- **FR-036**: System MUST select subset of GNSS coordinates for path calculation based on resampling distance divided by mean coordinate spacing
- **FR-037**: System MUST use all GNSS coordinates for final projection regardless of resampling (resampling applies only to path calculation)

#### Input/Output Options

- **FR-038**: System MUST support command-line argument to request train path calculation and export without coordinate projection
- **FR-039**: System MUST support exporting train path only in CSV format
- **FR-040**: System MUST support exporting train path only in GeoJSON format
- **FR-041**: System MUST accept command-line argument providing a pre-calculated train path file as input in CSV or GeoJSON format (same format as path-only exports)
- **FR-042**: When train path file is provided as input, system MUST skip path calculation and project coordinates directly onto the supplied path
- **FR-043**: System MUST include the calculated train path in the complete output (when full projection is performed)

#### Fallback Behavior

- **FR-044**: When no continuous train path can be calculated, system MUST fall back to simple projection of each GNSS coordinate onto its nearest netelement
- **FR-045**: System MUST notify the user when fallback to simple projection occurs
- **FR-046**: Fallback projection MUST ignore navigability constraints and project each coordinate independently

#### Debugging and Diagnostics

- **FR-047**: System MUST support exporting intermediate path calculation results when debug mode is enabled
- **FR-048**: Intermediate results MUST include list of all candidate paths with their probability scores
- **FR-049**: Intermediate results MUST include for each GNSS coordinate the candidate netelements and their calculated probabilities
- **FR-050**: Intermediate results MUST include the decision tree showing path selection criteria and outcomes

### Key Entities

- **NetRelation**: Represents a connection point between two track segments (netelements). Contains:
  - Point coordinates of the connection location
  - Navigability: direction(s) in which trains can traverse this connection (both, none, AB, BA)
  - positionOnA: whether connection is at start (0) or end (1) of elementA
  - positionOnB: whether connection is at start (0) or end (1) of elementB  
  - elementA: reference to first netelement ID
  - elementB: reference to second netelement ID
  - Defines the rail network topology and valid traversal rules

- **AssociatedNetElement**: A segment of a train path. Contains:
  - Reference to a netelement ID (track segment)
  - Begin intrinsic coordinate (0-1 decimal value)
  - End intrinsic coordinate (0-1 decimal value)
  - Represents a portion of a track segment that the train traversed
  - Multiple AssociatedNetElements ordered sequentially form a complete train path

- **Train Path**: Ordered collection of AssociatedNetElements. Represents:
  - Complete continuous route taken by train through the rail network
  - Must respect navigability constraints defined in netrelations
  - Each segment connects to next segment via a valid netrelation
  - Can be exported independently or used as input for coordinate projection

- **Enhanced GNSS Coordinate**: Position measurement with optional additional properties:
  - Latitude/Longitude (or other coordinate system)
  - Heading: direction of travel in degrees relative to north (0-360)
  - Distance: cumulative or incremental distance value from sensors
  - Standard timestamp and other existing properties
  - Enhanced properties improve path calculation accuracy when available

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Train path calculation successfully produces a continuous, navigable path for at least 95% of input datasets with complete network topology
- **SC-002**: Coordinate projection accuracy improves by at least 30% when using calculated train path compared to simple nearest-segment projection (measured by average distance between projected points and actual track geometry)
- **SC-003**: Path calculation with GNSS heading data reduces incorrect path selection by at least 50% compared to distance-only calculation (measured on test datasets with known ground truth)
- **SC-004**: Processing time for path calculation on datasets with 10,000 GNSS coordinates completes within 2 minutes on standard hardware
- **SC-005**: Resampling configuration reduces path calculation time by at least 60% when processing dense GNSS data (1-meter spacing) compared to processing all coordinates
- **SC-006**: System successfully falls back to simple projection when path calculation fails, providing usable output in 100% of failure cases
- **SC-007**: Exported train paths can be successfully re-imported and used for coordinate projection in 100% of cases, producing identical results to original processing
- **SC-008**: Debug output provides sufficient information for developers to understand path selection decisions within 10 minutes of review (measured through usability testing)

## Assumptions

- GNSS coordinates represent a single continuous journey without reversals in travel direction (no front/back direction changes)
- Network topology provided via netrelations is accurate and complete for the geographic area covered by GNSS coordinates
- When heading data is provided, it represents the actual direction of train travel with reasonable sensor accuracy (less than 2 degrees typical error)
- Distance values, when provided, are monotonically increasing or can be processed to calculate incremental distances
- Track segments (netelements) are modeled as linear geometries where heading can be calculated at any point along the segment
- The rail network does not contain loops where a train could traverse the same physical track segment multiple times in a single journey
- Configuration parameters (cutoff distances, probability thresholds, resampling distance) can be tuned based on operational requirements and data characteristics
- Processing occurs offline or in batch mode; real-time streaming processing is not required
- The probability model (distance-based with heading alignment) provides sufficient accuracy for operational needs without requiring machine learning or more complex algorithms

## Configuration Parameters

The following configuration parameters are referenced in the requirements with default values:

- **Max nearest netelements**: Default 3 - maximum number of candidate track segments considered for each GNSS coordinate
- **Distance cutoff**: Default 50 meters - maximum distance from GNSS coordinate to consider a track segment as candidate
- **Heading difference cutoff**: Default 5 degrees - maximum heading misalignment before probability is set to 0
- **Minimum probability threshold**: Default 25% - minimum probability for including a netelement in path (unless it's the only navigable option)
- **Resampling distance**: Default 10 meters - target spacing between GNSS coordinates used for path calculation

These parameters should be exposed through configuration or command-line arguments to allow tuning for different operational scenarios.
