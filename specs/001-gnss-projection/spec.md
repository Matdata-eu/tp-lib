# Feature Specification: GNSS Track Axis Projection

**Feature Branch**: `001-gnss-projection`  
**Created**: 2025-12-09  
**Status**: Draft  
**Input**: User description: "Process GNSS positions by projecting them on track axis for improved accuracy"

## Clarifications

### Session 2025-12-09

- Q: When a GNSS position is too far from any netelement (e.g., positioning error, tunnel, signal loss), how should the library handle it? → A: Force projection to nearest netelement regardless of distance
- Q: What distance threshold should trigger diagnostic warnings for projections far from track axis? → A: Configurable parameter (user-defined at runtime) with 50m as default
- Q: When a GNSS position is equidistant from multiple parallel netelements, what criteria should guide selection? → A: Pure spatial proximity (always pick geometrically nearest, ignore history)
- Q: What input file format should the library support for railway network data? → A: GeoJSON (netelements as features with properties and geometry)
- Q: How should users specify CRS for GNSS CSV input? → A: CLI parameter required: `--crs EPSG:4326`, but only for CSV; specifying this parameter for GeoJSON input must throw an error

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Basic GNSS Projection (Priority: P1) 🎯 MVP

An infrastructure manager has a CSV file containing raw GNSS positions from a single train journey (one direction) and needs to determine which netelement (track segment) the train was on at each point and the train's accurate position along that netelement's centerline.

**Why this priority**: This is the foundational capability that delivers immediate value—transforming noisy GNSS data into accurate track-aligned positions with topology context. Without this, no further analysis is possible.

**Independent Test**: Can be fully tested by providing a simple GNSS CSV file (lat/lon/timestamp) and a railway network GeoJSON file with a few netelement features, then verifying the output contains projected positions, netelement IDs, and measures for each input point.

**Acceptance Scenarios**:

1. **Given** a CSV file with GNSS positions (latitude, longitude, timestamp) from a train journey AND a railway network GeoJSON file with netelement features (LineString geometries), **When** the library processes the GNSS data, **Then** it produces output records with: original GNSS data, corrected position on track axis, netelement ID, and measure along netelement in meters

2. **Given** GNSS positions that closely follow a single straight netelement, **When** projecting to track axis, **Then** each position is mapped to the nearest point on that netelement's linestring with measure increasing monotonically along the journey direction

3. **Given** GNSS positions near a junction where multiple netelements intersect, **When** projecting positions, **Then** the library selects the geometrically nearest netelement based on pure spatial proximity

4. **Given** output from the projection process, **When** examining the results, **Then** every input GNSS record has exactly one corresponding output record preserving record count and order

---

### Edge Cases

- GNSS positions equidistant from multiple parallel netelements are assigned to the geometrically nearest netelement (pure spatial proximity without journey continuity analysis)
- GNSS positions far from any netelement (e.g., positioning errors, tunnels, signal loss) are projected to the nearest netelement regardless of distance, with diagnostic warnings when distance exceeds configurable threshold (default 50m)
- What happens when GNSS timestamp ordering doesn't match spatial progression along the track?
- How are netelements with complex geometry (sharp curves, switchbacks) handled during projection?
- What happens when the railway network has gaps or discontinuities?
- How does the system handle GNSS positions at the boundary between two consecutive netelements?
- What happens when input GNSS data has duplicate timestamps?
- How are positions handled when they fall near the start or end point of a netelement?

## Requirements *(mandatory)*

### Functional Requirements

#### Input Data

- **FR-001**: Library MUST accept GNSS position data in CSV or GeoJSON format containing at minimum: latitude (decimal degrees), longitude (decimal degrees), and timestamp (with timezone information per Constitution Principle VI)
- **FR-002**: Library MUST accept railway network data in GeoJSON format as a FeatureCollection where each Feature represents a netelement with properties containing unique identifier and geometry as LineString
- **FR-003**: For GNSS CSV input, library MUST require CLI parameter `--crs` specifying CRS (e.g., EPSG:4326); for GNSS GeoJSON input, library MUST use WGS84 per GeoJSON RFC 7946 standard and MUST reject `--crs` parameter with error message
- **FR-004**: Library MUST extract CRS information from GeoJSON railway network (WGS84 per RFC 7946, or via crs property if present) or accept a parameter defining the network CRSS
- **FR-005**: For GNSS CSV input, library MUST support configurable column mappings to identify latitude, longitude, and timestamp fields

#### Processing

#### Processing

- **FR-006**: Library MUST project each GNSS position onto the nearest point on a netelement's track axis linestring
- **FR-007**: Library MUST calculate the measure (distance in meters from linestring start point) for each projected position along the netelement
- **FR-008**: Library MUST perform CRS transformations when GNSS data and railway network use different coordinate reference systems
- **FR-009**: Library MUST select the netelement with minimum spatial distance when a GNSS position is near multiple netelements (pure geometric proximity, no journey context or heading analysis)
- **FR-010**: Library MUST preserve temporal ordering of GNSS positions in the output
- **FR-011**: Library MUST validate GNSS timestamps include timezone information and handle timezone conversions if needed

#### Output Data

- **FR-012**: Library MUST produce output with the same number of records as the input GNSS data
- **FR-013**: Each output record MUST contain: all original GNSS input data (latitude, longitude, timestamp), projected position on track axis (coordinates), netelement identifier, and measure along netelement in meters
- **FR-014**: Library MUST output projected positions with explicit CRS information
- **FR-015**: Library MUST support output formats compatible with downstream analysis tools (JSON and CSV at minimum per Constitution Principle II)

#### Error Handling & Data Quality

- **FR-016**: Library MUST validate input data and fail fast with actionable error messages for: missing required fields, invalid coordinate values, malformed geometries, missing/invalid CRS specification (including --crs parameter misuse with GeoJSON), invalid timestamps
- **FR-017**: Library MUST project every GNSS position to the nearest netelement regardless of distance, and MUST provide diagnostic warnings when projection distance exceeds a configurable threshold (default: 50 meters from track axis)
- **FR-018**: Library MUST log all data transformations including CRS conversions, projection calculations, and netelement assignments for audit trail (per Constitution Principle IX)
- **FR-019**: Library MUST provide diagnostic information about projection quality (e.g., distance between original GNSS position and projected position)

#### CLI Interface

- **FR-020**: Library MUST expose projection functionality via command-line interface accepting file paths for GNSS data and railway network, with required `--crs` parameter for CSV input (rejected with error for GeoJSON input)
- **FR-021**: CLI MUST emit processing results to stdout in JSON or CSV format (per Constitution Principle II)
- **FR-022**: CLI MUST emit errors and warnings to stderr with appropriate exit codes (0 for success, non-zero for failures)
- **FR-023**: CLI MUST support --help flag documenting all parameters and usage examples

### Key Entities

- **GNSS Position**: Represents a single positioning measurement from a train's GNSS receiver. Key attributes: latitude, longitude, timestamp with timezone, original CRS (specified via CLI parameter for CSV input, or WGS84 for GeoJSON input per RFC 7946). May include additional metadata (accuracy, satellite count, etc.) which should be preserved through processing.

- **Netelement**: Represents a topological track segment in the railway network—an elementary section between switches or between a switch and buffer stop. Represented as a GeoJSON Feature with properties containing unique identifier and geometry as LineString, with CRS information from GeoJSON metadata. The term "netelement" is used consistently to avoid ambiguity of the term "track."

- **Projected Position**: Represents the corrected position after projecting a GNSS measurement onto a netelement's track axis. Key attributes: original GNSS data (preserved), projected coordinates on track axis, distance from original GNSS position, netelement ID, measure along netelement (meters from linestring start), CRS of projected coordinates.

- **Railway Network**: Collection of netelements representing the track topology. Forms a connected graph structure where netelements connect at switches and stops.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Infrastructure managers can process a typical train journey (1000 GNSS positions, 50 netelements) and receive accurate projected results in under 10 seconds
- **SC-002**: Projected positions are within 2 meters of the actual track centerline for 95% of GNSS measurements under normal operating conditions (clear sky, no tunnels)
- **SC-003**: The library correctly identifies the netelement for 98% of GNSS positions when the train follows a clear path without ambiguous parallel track situations
- **SC-004**: Every input GNSS position produces exactly one output record, maintaining 100% record correspondence between input and output
- **SC-005**: Users can successfully process GNSS data without requiring deep knowledge of geospatial concepts—providing only basic inputs (GNSS CSV, network file, CRS parameters) produces valid results
- **SC-006**: Processing completes successfully for datasets containing 10,000+ GNSS positions without memory exhaustion or performance degradation
- **SC-007**: Diagnostic information enables users to identify and troubleshoot poor-quality GNSS data (positions exceeding configurable distance threshold from track axis are flagged, default 50m)

## Assumptions & Constraints

### Assumptions

- **A-001**: Input GNSS data represents a single continuous train journey in one direction (no reversals or multiple trips in the same dataset)
- **A-002**: The railway network dataset is complete and accurate for the geographic area covered by the GNSS journey
- **A-003**: GNSS positions are provided in chronological order by timestamp (though library should validate this)
- **A-004**: Netelement LineString geometries have consistent direction (start to end) matching the intended measurement direction
- **A-005**: Infrastructure managers have access to railway network data in a format that can be converted to GeoJSON FeatureCollection structure

### Constraints

- **C-001**: Initial implementation focuses on single-train, single-journey processing (batch processing of multiple journeys is out of scope)
- **C-002**: Advanced map-matching algorithms considering train dynamics, track topology, and historical patterns are deferred to future enhancements
- **C-003**: Real-time streaming processing is out of scope—this is a post-processing batch library
- **C-004**: The library assumes GNSS data quality issues (tunnels, urban canyons, multipath) are handled through projection rejection rather than sophisticated error modeling

## Dependencies

### External Systems

- None—library operates on files provided by the user

### External Libraries (per Constitution Principle I)

Infrastructure managers should leverage quality external dependencies rather than reimplementing geospatial operations:
- Geospatial library for CRS transformations and coordinate operations
- Geometry library for linestring operations (distance calculations, point projection)
- Spatial indexing library for efficient nearest-netelement queries
- Data parsing libraries for CSV/JSON handling

## Out of Scope

The following are explicitly **not** included in this feature specification:

- **Multi-journey processing**: Processing multiple train trips in a single operation
- **Sensor fusion**: Integration with odometry, punctual registrations, or other positioning sources beyond GNSS
- **Advanced map-matching**: Probabilistic algorithms, Kalman filtering, or trajectory optimization
- **Real-time processing**: Streaming or live position processing
- **Railway network editing**: Tools to create, modify, or validate netelement datasets
- **Visualization**: Graphical display of GNSS tracks and projections
- **Database integration**: Direct reading from or writing to databases
- **Authentication/authorization**: Security features for multi-user scenarios
- **Network topology validation**: Verification that netelements form valid connected networks

## Future Enhancements

Potential extensions beyond this MVP specification:

- Journey continuity analysis for netelement selection (considering previous netelement assignments and topology connectivity)
- Heading-based disambiguation (comparing GNSS bearing with netelement orientation)
- Integration with additional positioning sources (odometry, axle counters, balises)
- Advanced map-matching using Hidden Markov Models or particle filters
- Batch processing of multiple train journeys
- Confidence scoring and uncertainty quantification for projected positions
- Railway network quality validation and gap detection
- Support for bidirectional journeys and reversals
- Performance optimization for very large datasets (millions of points)
