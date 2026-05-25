# Feature Specification: ERA RINF Network Download

**Feature Branch**: `006-download-rinf-topology`  
**Created**: 2026-05-13  
**Status**: Draft  
**Input**: User description: "now we need to implement a new feature to download the network topology (netelements and netrelations) from the ERA RINF knowledge graph https://rinf.data.era.europa.eu/ instead of relying on the user to supply the network topology. We will talk about the technical implementation in the plan step where we detail the SPARQL queries to be made to retrieve the data. The RINF data is not necesary available at the place where the GNSS positions are located. So we'll need to provide proper feedback to the user when that's not the case. This feature needs to become available also through the python and .net bindings."

## Clarifications

### Session 2026-05-13

- Q: When both supplied topology and ERA RINF retrieval are available, which source should take precedence? → A: Use ERA RINF retrieval only when no topology is supplied.
- Q: What should happen when only part of the GNSS dataset falls inside sufficient ERA RINF coverage? → A: Fail the workflow and report which GNSS records or area are uncovered.
- Q: Which workflows should support automatic ERA RINF retrieval in the first release? → A: All existing topology-dependent workflows use automatic retrieval when no topology is supplied.
- Q: How should the retrieval area be defined for GNSS datasets, including ones that span a larger area? → A: Use one bounding box around the GNSS dataset, expand it by 1 km, and download any netelement that lies partly inside that box.
- Q: What should happen when the GNSS dataset is empty or has no usable coordinates? → A: Fail validation before retrieval and report invalid or empty GNSS input.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Retrieve Network Topology Automatically (Priority: P1)

A user who wants to project GNSS positions onto the railway network submits GNSS data without separately providing a network topology file. The system determines the relevant geographic area, retrieves the matching netelements and netrelations from the ERA RINF dataset, and uses that retrieved topology for the rest of the workflow.

**Why this priority**: This is the core value of the feature. It removes the current prerequisite that users must source and prepare network topology themselves.

**Independent Test**: Can be fully tested by running a topology-dependent workflow with GNSS data in an area covered by ERA RINF and verifying that the workflow succeeds without any user-supplied topology file.

**Acceptance Scenarios**:

1. **Given** GNSS data located in an area covered by ERA RINF, **When** the user starts a topology-dependent workflow without providing a network file, **Then** the system retrieves the required netelements and netrelations automatically and continues the workflow successfully.
2. **Given** GNSS data that spans a continuous route segment, **When** the system retrieves ERA RINF topology for that area, **Then** the retrieved network contains the relationships needed to represent the traversable route for subsequent processing.

---

### User Story 2 - Receive Clear Coverage Feedback (Priority: P2)

A user submits GNSS data for a location where ERA RINF does not provide sufficient topology coverage. Instead of failing silently or producing an unclear downstream error, the system explains that topology for the relevant area could not be found or was insufficient for processing.

**Why this priority**: Automatic retrieval is only trustworthy if the user can immediately distinguish between successful retrieval and missing or incomplete source coverage.

**Independent Test**: Can be fully tested by running the same workflow with GNSS data in an uncovered or partially covered area and verifying that the user receives a clear, actionable message about the missing topology coverage.

**Acceptance Scenarios**:

1. **Given** GNSS data for an area with no matching ERA RINF topology, **When** the user starts a topology-dependent workflow, **Then** the system stops before downstream processing and reports that topology is unavailable for the submitted area.
2. **Given** GNSS data for an area with only partial ERA RINF coverage, **When** the retrieved topology is insufficient to support the requested workflow, **Then** the system reports that coverage is incomplete and identifies the affected input area or records.
3. **Given** a temporary retrieval failure from the external source, **When** automatic topology retrieval cannot complete, **Then** the system reports that retrieval failed and distinguishes this from a genuine lack of source coverage.

---

### User Story 3 - Use Retrieval Through Language Bindings (Priority: P3)

A Python or .NET developer integrates tp-lib into an application and expects the same automatic topology retrieval capability that is available through the core library and CLI workflows. They invoke the relevant API without separately supplying topology data and receive either a successful result or the same coverage feedback available in the main workflow.

**Why this priority**: The new capability needs to be available across the supported integration surfaces, not only in the primary Rust-facing workflow.

**Independent Test**: Can be fully tested by invoking the feature through the Python and .NET bindings with covered and uncovered GNSS datasets and verifying equivalent outcomes and feedback in both bindings.

**Acceptance Scenarios**:

1. **Given** a Python application using tp-lib bindings, **When** it runs a topology-dependent workflow without supplying topology input, **Then** it can request automatic ERA RINF retrieval and receive the resulting topology-backed workflow output.
2. **Given** a .NET application using tp-lib bindings, **When** it runs a topology-dependent workflow without supplying topology input, **Then** it can request automatic ERA RINF retrieval and receive the resulting topology-backed workflow output.
3. **Given** either binding is used in an area without sufficient ERA RINF coverage, **When** the workflow is executed, **Then** the binding surfaces a clear failure result consistent with the core library behavior.
4. **Given** any existing topology-dependent workflow is invoked without supplied topology, **When** the workflow requires railway network data, **Then** it uses automatic ERA RINF retrieval instead of requiring manual topology input.

### Edge Cases

- What happens when GNSS points are spread across multiple distant areas? The system still uses a single retrieval area defined as the GNSS dataset bounding box expanded by 1 km, and downloads any netelement that lies partly inside that box.
- How does the system behave when ERA RINF returns netelements for the area but omits the netrelations needed to connect them into a usable topology?
- What happens when only a subset of GNSS records falls inside available ERA RINF coverage? The workflow fails and identifies the uncovered records or area instead of returning partial processing results.
- How does the system respond when the submitted GNSS dataset is empty or contains no usable coordinates for identifying a retrieval area? The workflow fails validation before any ERA RINF request and reports invalid or empty GNSS input.
- How does the system distinguish between missing source coverage, invalid input coordinates, and temporary source unavailability?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST allow users to run topology-dependent workflows without supplying a network topology file when ERA RINF data is intended to be used as the topology source.
- **FR-001A**: All existing topology-dependent workflows MUST support this automatic retrieval path when no topology is supplied by the caller.
- **FR-002**: The system MUST derive a single retrieval area from the submitted GNSS positions by taking the GNSS dataset bounding box and expanding it by 1 km before requesting the relevant ERA RINF netelements and netrelations.
- **FR-003**: The system MUST assemble the retrieved ERA RINF netelements and netrelations into the same logical network representation expected by downstream topology-dependent workflows.
- **FR-003A**: The system MUST include any ERA RINF netelement that lies at least partly inside the retrieval area.
- **FR-004**: The system MUST verify that the retrieved topology is sufficient for the requested workflow before continuing with downstream processing.
- **FR-005**: The system MUST stop the workflow with a clear user-facing message when no ERA RINF topology is available for the relevant GNSS area.
- **FR-006**: The system MUST stop the workflow with a clear user-facing message when ERA RINF topology is only partially available and that partial coverage is insufficient for the requested workflow.
- **FR-007**: The system MUST report retrieval failures from the ERA RINF source separately from missing data coverage so users can distinguish source availability problems from geographic coverage gaps.
- **FR-008**: The system MUST preserve the existing ability for workflows to operate on user-supplied topology where that capability already exists, and MUST use the supplied topology as the authoritative source whenever the caller provides it.
- **FR-009**: The Python bindings MUST expose the automatic ERA RINF topology retrieval capability for the same topology-dependent workflows supported by the core library.
- **FR-010**: The .NET bindings MUST expose the automatic ERA RINF topology retrieval capability for the same topology-dependent workflows supported by the core library.
- **FR-011**: The Python and .NET bindings MUST surface missing-coverage and retrieval-failure feedback with equivalent meaning to the core library behavior.
- **FR-012**: The system MUST record enough retrieval outcome detail for users or calling applications to identify whether processing used downloaded topology, failed because coverage was missing, or failed because retrieval could not be completed.
- **FR-013**: The system MUST attempt automatic ERA RINF retrieval only when a topology-dependent workflow is invoked without caller-supplied topology.
- **FR-014**: The system MUST fail the workflow rather than return partial results when any required portion of the submitted GNSS dataset lacks sufficient ERA RINF topology coverage.
- **FR-015**: When failing due to incomplete coverage, the system MUST report which GNSS records, route segment, or geographic area could not be covered well enough for the requested workflow.
- **FR-016**: The system MUST validate that the submitted GNSS dataset contains usable coordinates before making any ERA RINF retrieval request.
- **FR-017**: The system MUST fail with an invalid-input outcome, rather than a coverage-related outcome, when the GNSS dataset is empty or contains no usable coordinates for defining the retrieval area.

### Key Entities *(include if feature involves data)*

- **Retrieval Area**: A single bounding box derived from the submitted GNSS positions and expanded by 1 km in every direction to determine which ERA RINF topology data must be requested.
- **Retrieved Topology**: The set of netelements and netrelations obtained from ERA RINF for a retrieval area and prepared for downstream workflow use.
- **Coverage Assessment**: The outcome of checking whether the retrieved topology sufficiently covers the GNSS area and supports the requested workflow.
- **Retrieval Outcome**: The result returned to the user or caller indicating success, missing coverage, partial coverage, invalid input, or external retrieval failure.

## Assumptions

- ERA RINF is the authoritative external source for this feature and provides the topology data needed for the supported workflows when coverage exists.
- Existing workflows that accept user-supplied topology remain supported; this feature adds an automatic retrieval path rather than removing manual input, and manual topology input remains authoritative when provided.
- Retrieval scope is determined from the submitted GNSS positions rather than from a separate user-entered area selection, using one bounding box expanded by 1 km.
- If coverage is insufficient for reliable downstream processing, the workflow fails clearly and reports the uncovered portion rather than returning best-effort partial results.
- The first release targets all workflows that already depend on topology rather than a subset limited to projection or path calculation.
- Python and .NET bindings are expected to expose this feature at the same functional level as the core library, even if their API shapes differ.
- Invalid or empty GNSS input is treated as a caller input problem and is reported before any attempt to contact ERA RINF.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a representative covered-area test set, users can complete a topology-dependent workflow without providing a topology file in at least 95% of runs.
- **SC-002**: In a representative uncovered-area test set, 100% of failed runs report a user-visible outcome that explicitly identifies missing or insufficient topology coverage.
- **SC-003**: In a representative source-failure test set, 100% of failed runs report a user-visible outcome that distinguishes external retrieval failure from missing geographic coverage.
- **SC-004**: The Python and .NET bindings both support at least one end-to-end topology-dependent workflow using automatic ERA RINF retrieval with outcomes equivalent to the core library for covered and uncovered areas.
- **SC-005**: Users or calling applications can determine from every automatic-retrieval attempt whether downloaded topology was used successfully, coverage was insufficient, or retrieval failed.
- **SC-006**: Every existing topology-dependent workflow can be executed without caller-supplied topology in covered areas by using automatic ERA RINF retrieval.
