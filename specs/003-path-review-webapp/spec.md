# Feature Specification: Train Path Review Webapp

**Feature Branch**: `003-path-review-webapp`  
**Created**: March 21, 2026  
**Status**: Draft  
**Input**: User description: "Add a companion map webapp to tp-lib that users can run locally from the CLI to visually review and edit a calculated train path before using it for GNSS projection."

## Clarifications

### Session 2026-03-21

- Q: How does the browser signal the CLI process to continue in integrated mode (Confirm action)? → A: HTTP POST to a local endpoint (`POST /confirm`); CLI server blocks the pipeline awaiting that request
- Q: What confidence value should the webapp assign to a manually-added segment for display purposes? → A: Assign a fixed confidence of 1.0 (100%) — the user is certain
- Q: Should the webapp offer an explicit abort action in integrated mode, or is Ctrl+C the only escape? → A: Provide an "Abort" button that calls `POST /abort`; CLI exits with non-zero code and prints a cancellation message
- Q: In standalone mode, does clicking "Save" shut the server down, or should the server stay alive for further editing? → A: Save writes the file and keeps the server running; user can save again or quit via Ctrl+C
- Q: What is the authoritative source of topology for snap insertion order, and must netrelations be present in the network file? → A: Netrelations are required in the network file (same format as feature 002); if no unambiguous insertion position can be determined, the segment is appended at the nearest end of the path with a disconnected marker (no geometry guessing)

- [Feature Specification: Train Path Review Webapp](#feature-specification-train-path-review-webapp)
  - [User Scenarios \& Testing *(mandatory)*](#user-scenarios--testing-mandatory)
    - [User Story 1 - Standalone Path Review and Export (Priority: P1)](#user-story-1---standalone-path-review-and-export-priority-p1)
    - [User Story 2 - Integrated Review During GNSS Projection Pipeline (Priority: P2)](#user-story-2---integrated-review-during-gnss-projection-pipeline-priority-p2)
    - [User Story 3 - GNSS Position Overlay in Standalone Mode (Priority: P3)](#user-story-3---gnss-position-overlay-in-standalone-mode-priority-p3)
    - [Edge Cases](#edge-cases)
  - [Requirements *(mandatory)*](#requirements-mandatory)
    - [Functional Requirements](#functional-requirements)
      - [Map Display](#map-display)
      - [Path Editing](#path-editing)
      - [Save and Confirm](#save-and-confirm)
      - [CLI Integration](#cli-integration)
    - [Key Entities](#key-entities)
  - [Success Criteria *(mandatory)*](#success-criteria-mandatory)
    - [Measurable Outcomes](#measurable-outcomes)
  - [Assumptions](#assumptions)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Standalone Path Review and Export (Priority: P1)

A railway engineer has already calculated a train path (or obtained one from a colleague) and wants to visually review it on a map before using it for GNSS projection. They launch the webapp directly from the CLI, providing the network file and the pre-calculated path file. The map shows all network segments with the path highlighted. If anything looks wrong, they click segments on the map to add or remove them from the path, then save the edited result to a file. The saved file can be reused immediately with the existing `--train-path` flag.

**Why this priority**: This is the most self-contained and directly useful scenario. It delivers immediate value independently of the projection pipeline, requires no integration with other pipeline steps, and establishes the map interaction model that all other stories depend on.

**Independent Test**: Can be fully tested by running `tp-cli webapp --network network.geojson --train-path path.csv --output modified.csv`, editing the path in the browser, saving, and verifying the output file is a valid path CSV that the existing `--train-path` flag accepts.

**Acceptance Scenarios**:

1. **Given** a network GeoJSON file and a path CSV file, **When** the user runs `tp-cli webapp --network network.geojson --train-path path.csv`, **Then** a local server starts, the browser opens automatically to the map URL, and the CLI prints the local URL to the terminal
2. **Given** the webapp is open in the browser, **When** the map loads, **Then** all network netelements are displayed on the map, each identifiable by its ID; the train path segments are visually highlighted in order; each highlighted segment shows a visual indication of the algorithm's confidence score
3. **Given** the path is displayed on the map, **When** the user clicks a non-highlighted netelement, **Then** it is added to the path, visually distinguished from algorithm-selected segments, and its position in the path snaps to the correct location relative to adjacent netelements
4. **Given** the path is displayed on the map, **When** the user clicks a highlighted netelement to remove it, **Then** it is removed from the path and the display updates immediately
5. **Given** the user has finished editing, **When** they click "Save", **Then** the modified path is written to the output file in the same CSV format as the algorithm-produced path, the webapp shows a confirmation that the file was saved, and the server remains running so the user can continue editing and save again
6. **Given** no `--output` flag is provided, **When** the user saves, **Then** the output is written to a default filename and the CLI confirms the path

---

### User Story 2 - Integrated Review During GNSS Projection Pipeline (Priority: P2)

A railway data analyst runs the standard GNSS projection pipeline but wants to review the automatically calculated path before the projection proceeds. They add `--review` to the standard command. After path calculation, the webapp opens automatically. The analyst reviews the path on the map, makes any needed corrections, then confirms. The projection continues with the reviewed path, and the final output reflects the manually corrected route.

**Why this priority**: This is the primary operational workflow integration point, but it depends on the standalone webapp (P1) being functional first. It delivers high value for production use where path accuracy is critical, but is meaningless without the core map interaction.

**Independent Test**: Can be tested by running the full pipeline with `--review`, verifying the process pauses after path calculation and opens the browser, then confirming that after the user clicks "Confirm" in the webapp, the projection proceeds and the output reflects any path changes made in the review session.

**Acceptance Scenarios**:

1. **Given** a standard pipeline command with `--review` added, **When** path calculation completes, **Then** the webapp server starts automatically, the browser opens to the map, and the CLI displays a waiting message with the local URL
2. **Given** the webapp is open after automatic path calculation, **When** the map loads, **Then** the calculated path is highlighted with confidence indicators, network netelements are all visible, and GNSS positions are shown as markers if `--gnss` data was provided
3. **Given** the user has reviewed and optionally edited the path, **When** they click "Confirm", **Then** the webapp signals the CLI process to continue, the server shuts down, and projection proceeds using the (possibly modified) confirmed path
4. **Given** the user closes the browser without confirming, **When** the CLI detects this, **Then** it waits for reconnection rather than proceeding silently; the CLI must not advance the pipeline without explicit user confirmation
5. **Given** the user decides the path is unrecoverable, **When** they click the "Abort" button in the webapp, **Then** the browser sends `POST /abort` to the local server, the server shuts down, and the CLI exits with a non-zero exit code and prints a clear cancellation message
5. **Given** the projection completes after review, **When** inspecting the output, **Then** the output reflects the path as confirmed in the review session, not the originally calculated path if changes were made

---

### User Story 3 - GNSS Position Overlay in Standalone Mode (Priority: P3)

An analyst uses the standalone webapp and also provides a GNSS positions file to visualize where the raw positions fall relative to the network and the calculated path. This helps them understand which segments were likely chosen by the algorithm and why, making editing decisions more informed.

**Why this priority**: Adds valuable diagnostic context to standalone mode but does not change the core editing workflow. Lower priority because the webapp is fully usable without GNSS overlay.

**Independent Test**: Can be tested by running `tp-cli webapp --network network.geojson --train-path path.csv --gnss positions.csv` and verifying that GNSS position markers appear on the map in addition to the network and path.

**Acceptance Scenarios**:

1. **Given** a `--gnss` file is provided to the standalone command, **When** the map loads, **Then** GNSS position markers are displayed on the map alongside the network segments and highlighted path
2. **Given** GNSS markers are visible on the map, **When** the user zooms or pans, **Then** markers remain associated with their geographic position and do not obscure the ability to interact with network segments
3. **Given** no `--gnss` flag is provided, **When** the map loads, **Then** no GNSS markers are shown and everything else behaves identically

---

### Edge Cases

- What happens when the provided network file is very large (thousands of segments)? The map must still load and remain usable; segments not in the visible area should not block interaction.
- What happens when the user attempts to add a netelement that is not topologically adjacent to any segment in the current path? The segment is still added, but it is visually marked as disconnected from the rest of the path.
- What happens if the user removes all segments from the path and tries to save? The system prompts for confirmation before saving an empty path.
- What happens when the network contains netelements with no geometry? Those segments are omitted from the map but the system does not crash.
- What happens when the CLI server port is already in use? The system tries the next available port and prints the actual URL to the terminal.
- What happens when the browser cannot be opened automatically (headless server, CI)? The CLI prints the URL to the terminal and continues waiting for a connection, falling back gracefully.
- What happens if the user closes the browser tab in integrated mode without confirming? The CLI continues waiting and re-displays the URL so the user can reconnect.

## Requirements *(mandatory)*

### Functional Requirements

#### Map Display

- **FR-001**: The webapp MUST display all netelements from the provided network file as interactive segments on the map, each identifiable by its netelement ID (visible on hover or click)
- **FR-002**: Netelements that are part of the calculated train path MUST be visually distinguished from those that are not (e.g., highlighted color, thickness)
- **FR-003**: For each path segment, the webapp MUST display a visual indication of the confidence score using a continuous scale (e.g., colour gradient from low to high); algorithm-selected segments use the score from the path file; manually-added segments are assigned a confidence of 1.0 (100%)
- **FR-004**: Manually added segments MUST be visually distinguished from those selected by the algorithm, using a distinct style (e.g., different colour or pattern), in addition to showing the 1.0 confidence colour
- **FR-005**: When a GNSS positions file is provided, positions MUST be rendered as markers on the map
- **FR-006**: The map MUST support standard pan and zoom interactions

#### Path Editing

- **FR-007**: Users MUST be able to add a netelement to the path by clicking it on the map
- **FR-008**: Users MUST be able to remove a netelement from the path (e.g., via a click action on a highlighted segment or a UI control)
- **FR-009**: When a netelement is added to the path, its insertion position in the ordered path MUST be determined using the netrelations present in the network file; if no unambiguous topologically correct position exists, the segment MUST be appended at the nearest end of the current path and marked as disconnected; spatial geometry guessing MUST NOT be used as a fallback
- **FR-010**: Path edits MUST be reflected on the map immediately without requiring a page reload

#### Save and Confirm

- **FR-011**: In standalone mode, users MUST be able to save the current (edited) path to a file via a "Save" action in the webapp; saving MUST write the file immediately and display a confirmation in the UI, but the server MUST remain running so the user can continue editing and save again; the server shuts down only when the user terminates the CLI process (e.g., Ctrl+C)
- **FR-012**: In integrated mode, users MUST be able to confirm the reviewed path via a "Confirm" action; the browser sends an HTTP `POST /confirm` request to the local server, which unblocks the CLI pipeline and shuts down the server
- **FR-013**: The saved or confirmed path MUST be in the same CSV format as the path produced by the algorithm, compatible with the existing `--train-path` flag
- **FR-014**: In integrated mode, the pipeline MUST NOT advance past path review without explicit user confirmation from the webapp
- **FR-021**: In integrated mode, the webapp MUST provide an "Abort" button distinct from the "Confirm" button; clicking it sends `POST /abort` to the local server, causing the CLI to shut down the server and exit with a non-zero exit code with a cancellation message printed to the terminal

#### CLI Integration

- **FR-015**: The standalone command MUST be `tp-cli webapp --network <file> --train-path <file> [--gnss <file>] [--output <file>]`; the network file MUST include both netelements and netrelations (same format required by the path calculation pipeline)
- **FR-016**: The integrated review mode MUST be triggered by adding `--review` to the standard pipeline command
- **FR-017**: The CLI MUST start a local-only server (not exposed to the network) when the webapp is launched
- **FR-018**: The CLI MUST attempt to open the default browser automatically when the server starts
- **FR-019**: The CLI MUST print the local URL to the terminal as a fallback, regardless of whether the browser opened successfully
- **FR-020**: When no `--output` is provided in standalone mode, the output MUST be written to a sensible default filename and the actual path printed to the terminal

### Key Entities

- **Network**: The complete set of netelements (track segments) and netrelations (topology connections) loaded from a GeoJSON network file; each netelement has a unique ID and LineString geometry; netrelations encode navigability between netelements and are required for snap insertion ordering
- **Train Path**: An ordered sequence of associated netelements, each carrying a confidence score and an origin flag (algorithm-selected vs. manually added); manually-added netelements carry a confidence score of 1.0
- **Path Edit Session**: The user's in-progress review state, tracking which segments were added or removed relative to the original calculated path
- **GNSS Overlay**: Optional set of position markers loaded from a GNSS file, displayed on the map for reference only (not editable)

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can launch the webapp, review a path of up to 200 netelements, make edits, and save the result within 5 minutes on a standard workstation
- **SC-002**: The map loads and becomes interactive within 10 seconds for a network of up to 5,000 netelements
- **SC-003**: Path edits (add or remove a segment) are reflected on the map within 1 second of the user action
- **SC-004**: The output path file produced by the webapp is accepted without error by the existing `--train-path` flag in 100% of cases
- **SC-005**: In integrated mode, the GNSS projection output is identical to what would have been produced if the confirmed path had been supplied via `--train-path` from the start
- **SC-006**: The local server startup to browser-open sequence completes within 5 seconds on a standard workstation

## Assumptions

- The network file fits in memory on a standard developer workstation (no streaming or tiling required for MVP).
- The webapp runs in a modern desktop browser (Chrome, Firefox, Edge); mobile browser support is out of scope.
- The local server binds to `localhost` only; no authentication or network security is required for this local-only tool.
- The path CSV format (AssociatedNetElement sequence with confidence scores) is stable and already defined by feature 002-train-path-calculation.
- Snap insertion of a newly added segment uses the netrelations from the network file to determine the correct position in the path order; spatial geometry fallback is not used — if no unambiguous position exists, the segment is appended at the nearest end of the path with a disconnected marker.
- The webapp does not need to support undo/redo beyond the session (i.e., refreshing or closing the tab resets to the original path).
- Only one review session runs at a time; concurrent multi-user access is not a requirement.
- The confidence score visualization uses a simple continuous scale (e.g., red → yellow → green); thresholds are not configurable in this phase.
