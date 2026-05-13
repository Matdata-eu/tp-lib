# Feature Specification: C#/.NET Bindings (tp-net)

**Feature Branch**: `005-dotnet-bindings`  
**Created**: 2026-05-13  
**Status**: Draft  
**Input**: User description: "we need to allow integration of this library into c#.net applications. So similar to the tp-py project, we also need a tp-net project that exposes all the functions and parameters in a similar way."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - GNSS Projection in C# Application (Priority: P1)

A C# developer working on a railway positioning system wants to project raw GNSS coordinates onto the railway network. They add the tp-net package to their project, load a railway network from a GeoJSON file, load GNSS readings from a CSV file, and call the projection function to get a list of projected positions — each tied to a specific track segment with an intrinsic offset.

**Why this priority**: GNSS projection is the foundational capability of the library. Delivering it first gives C# teams immediate value and represents the smallest viable slice of the overall feature.

**Independent Test**: Can be fully tested by loading `sample_gnss.geojson` and `sample_network.geojson` from the test-data directory, calling the projection function from a C# test project, and verifying that the returned positions contain valid netelement IDs and intrinsic offsets.

**Acceptance Scenarios**:

1. **Given** a loaded railway network and a set of GNSS readings, **When** a C# developer calls the projection function with a valid configuration, **Then** a list of projected positions is returned with one entry per GNSS reading, each containing a netelement ID and an intrinsic offset.
2. **Given** a GNSS reading that falls outside the configured search radius, **When** the projection function is called, **Then** a descriptive exception is thrown indicating no match was found within the allowed distance.
3. **Given** a malformed GeoJSON file, **When** the network is loaded, **Then** an exception with a clear I/O or parse error message is raised.

---

### User Story 2 - Train Path Calculation in C# Application (Priority: P2)

A C# developer needs to reconstruct the path a train followed across the railway network from a sequence of projected GNSS positions. They call the path calculation function with a loaded network, projected positions, and a configuration object, and receive a structured path result describing the ordered sequence of network elements the train traversed.

**Why this priority**: Train path calculation builds directly on GNSS projection and represents the second major capability. It is independently testable once projection works.

**Independent Test**: Can be fully tested by taking projected positions from User Story 1 and passing them to the path calculation function in a C# test project, verifying that the returned path contains an ordered sequence of network elements matching the known route.

**Acceptance Scenarios**:

1. **Given** a valid set of projected positions and a loaded railway network, **When** the path calculation function is called, **Then** a train path result is returned containing an ordered list of traversed network elements with entry/exit offsets.
2. **Given** projected positions spanning a network gap that cannot be bridged, **When** path calculation is called, **Then** an exception is raised indicating that no navigable path was found.
3. **Given** a configuration object specifying a particular calculation mode, **When** path calculation is called, **Then** the result respects the specified mode behavior.

---

### User Story 3 - Train Detection Preparation in C# Application (Priority: P3)

A C# developer wants to correlate train detection events (punctual or linear sensor activations) against a known train path and GNSS projection. They call the detection preparation function with detection records, the projected positions, and the railway network, and receive a set of prepared detections where each detection is either applied to a network element, resolved by proximity, or discarded with a stated reason.

**Why this priority**: Detection preparation depends on the outputs of Stories 1 and 2 and represents the third major capability. Useful independently once the earlier stories are in place.

**Independent Test**: Can be fully tested using sample detection files from the test-data directory, verifying that each detection record in the output has a status (applied, resolved, or discarded) and that discarded records include a reason.

**Acceptance Scenarios**:

1. **Given** a set of detection records and a matching GNSS projection, **When** the detection preparation function is called, **Then** each detection in the result has a status of applied, resolved, or discarded.
2. **Given** a detection record that falls outside the time range of the GNSS data, **When** preparation is called, **Then** the detection is marked as discarded with the reason `out_of_time_range`.
3. **Given** a detection with an unknown netelement ID, **When** preparation is called, **Then** the detection is marked as discarded with the reason `unknown_netelement`.

---

### User Story 5 - Database-Backed Service Processes Tasks Without Disk I/O (Priority: P2)

A .NET background service polls a PostgreSQL database for pending train path tasks. For each task, it fetches the relevant railway network, GNSS readings, and detection records from the database as in-memory objects (strings, collections), passes them directly to the tp-net API, and writes the resulting projected positions and path back to the database — without creating any temporary files on disk.

**Why this priority**: This is the primary production deployment pattern for tp-net. The library must work correctly when all input data originates from a database, not from files. Validating this path early avoids late-stage integration surprises.

**Independent Test**: Can be tested by constructing `NetworkInput.FromRecords(IEnumerable<NetworkSegment>)`, `GnssInput.FromRecords(IEnumerable<GnssRecord>)`, and `IEnumerable<DetectionRecord>` in-memory (no File I/O), calling the projection and path calculation functions, and asserting that results are returned correctly and no temporary files are created in any temp directory.

**Acceptance Scenarios**:

1. **Given** a .NET service that fetches GeoJSON strings and detection rows from a PostgreSQL database, **When** it calls the tp-net projection and path calculation functions with those strings and collections, **Then** valid results are returned without any disk I/O occurring in the tp-net library.
2. **Given** a railway network stored in the database as a GeoJSON text column, **When** the calling service wraps the fetched string with `NetworkInput.FromGeoJson()` and passes it to any tp-net function, **Then** the function processes it identically to a GeoJSON string loaded from a file. **Given** a railway network stored as structured rows in a `network_segments` table, **When** the calling service maps those rows to `NetworkSegment` objects and passes them via `NetworkInput.FromRecords()`, **Then** the network is processed correctly with no GeoJSON serialization required on the caller's side.
3. **Given** GNSS readings stored in the database as structured rows (latitude, longitude, timestamp), **When** the calling service maps those rows to `GnssRecord` objects and passes them via `GnssInput.FromRecords()`, **Then** projected positions are returned correctly with no serialization required on the caller's side.
4. **Given** detection events stored in the database as structured rows, **When** the calling service maps those rows to `DetectionRecord` objects and passes them as `IEnumerable<DetectionRecord>`, **Then** prepared detections are returned correctly with no serialization required on the caller's side.

---

### User Story 4 - NuGet Package Distribution (Priority: P4)

A C# developer discovers tp-net on NuGet, adds it to their project with a single package reference, and can immediately start calling the projection, path calculation, and detection APIs without any manual build or native dependency steps.

**Why this priority**: Discoverability and ease of installation are prerequisites for adoption. However, the core API can be developed and tested before publishing; therefore this is lower priority.

**Independent Test**: Can be fully tested by creating a blank .NET class library project, adding the NuGet package reference, and confirming that all three core functions are callable without additional setup.

**Acceptance Scenarios**:

1. **Given** a C# project targeting a supported .NET version, **When** the tp-net NuGet package is added, **Then** all public API types and functions are available without additional native DLL installation.
2. **Given** a developer on Windows, macOS, or Linux, **When** they install the package, **Then** the correct native binaries for their platform are loaded automatically.

---

### Edge Cases

- What happens when an empty GNSS dataset is passed to the projection function?
- How does the library behave when the railway network GeoJSON contains elements with missing or null geometry?
- What happens if the caller passes a `null` configuration object?
- How are detections handled when the detection timestamp exactly equals the boundary of the GNSS time range?
- What happens when the path calculation receives fewer than two projected positions?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The library MUST expose a GNSS projection function that accepts a railway network, a collection of GNSS readings, and a configuration object, and returns a collection of projected positions.
- **FR-002**: The library MUST expose a train path calculation function that accepts projected positions, a railway network, and a configuration object, and returns a structured path result.
- **FR-003**: The library MUST expose a train detection preparation function that accepts detection records, projected positions, and a railway network, and returns a collection of prepared detections with status and reason information.
- **FR-004**: The library MUST provide C#-idiomatic types for all inputs and outputs: `ProjectionConfig`, `ProjectedPosition`, `PathConfig`, `TrainPath`, `AssociatedNetElement`, `PreparedDetections`, and all status/reason enumerations.
- **FR-005**: The library MUST support loading a railway network from a GeoJSON **string (in-memory content), file path, or stream**.
- **FR-006**: The library MUST support loading GNSS readings from an **in-memory collection of structured records, a GeoJSON string, a CSV file path, or a stream**.
- **FR-007**: All error conditions MUST be surfaced as typed C# exceptions with descriptive messages matching the specificity of the equivalent Python bindings.
- **FR-008**: The library MUST be distributed as a NuGet package named `TpLib` (or equivalent naming convention consistent with the project).
- **FR-009**: The NuGet package MUST include pre-built native binaries for Windows (x64), Linux (x64), and macOS (x64/arm64), eliminating the need for consumers to install a Rust toolchain.
- **FR-010**: All public API members MUST have XML documentation comments sufficient for IntelliSense support in Visual Studio and Rider.
- **FR-011**: The library MUST support .NET 8 or later as the minimum target framework.
- **FR-012**: The library MUST NOT create any temporary files on disk during normal operation. All input and output data MUST be transferable as in-memory objects (strings, collections, structs) without requiring file system access.

### Key Entities

- **ProjectionConfig**: Configuration for GNSS projection; includes maximum search radius in meters and optional CRS override.
- **ProjectedPosition**: A single GNSS reading projected onto the railway network; contains netelement ID, intrinsic offset, timestamp, and source coordinate.
- **PathConfig**: Configuration for train path calculation; includes calculation mode and tolerance parameters.
- **TrainPath**: The reconstructed path of a train; contains an ordered list of `AssociatedNetElement` entries.
- **AssociatedNetElement**: A single network element in a train path; contains element ID, entry and exit intrinsic offsets, and direction.
- **DetectionRecord**: An input detection event (punctual or linear); contains kind, timestamp or time range, and optional netelement reference.
- **PreparedDetection**: A detection record enriched with a processing status (applied, resolved, or discarded) and a discard reason when applicable.

## Assumptions

- The tp-net package will be implemented as a Rust-based native library with a C# wrapper layer, following the same architecture as tp-py (pyo3 → Rust core). The exact FFI mechanism (e.g., CsBindgen, uniffi, or manual P/Invoke) is a technical decision outside this specification.
- API surface parity with tp-py is the baseline; tp-net does not need to add capabilities beyond what tp-py currently exposes.
- The package targets developers building server-side or desktop .NET applications; a MAUI/mobile scenario is out of scope for the initial release.
- XML documentation is sufficient; a separate documentation website is not required for the initial release.
- **GNSS data format in database-backed workflows**: GNSS readings stored in a PostgreSQL database are typically held as structured rows (latitude, longitude, timestamp, accuracy). The tp-net API accepts GNSS input via the `GnssInput` wrapper type. The preferred entry point for in-memory or database-sourced data is `GnssInput.FromRecords(IEnumerable<GnssRecord>)` — callers map query results directly to typed `GnssRecord` objects with no serialization step. `GnssInput.FromGeoJson()` and `GnssInput.FromCsv()` remain available for file-based or streaming workflows. All internal serialization across the FFI boundary is handled by the library.
- **Network input from database**: Railway network data may be stored in two ways: as a GeoJSON text/jsonb column (wrap with `NetworkInput.FromGeoJson()`), or as structured rows in a relational `network_segments` table (map rows to `NetworkSegment` objects and wrap with `NetworkInput.FromRecords()`). Both paths require no intermediate file or manual serialization on the caller's side.
- **Detection records from database**: Detection records fetched from a database can be mapped directly to `DetectionRecord` objects and passed as an `IEnumerable<DetectionRecord>` — no intermediate file is required.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All three core capabilities (GNSS projection, path calculation, detection preparation) available in tp-py are available in tp-net with equivalent parameters and return values.
- **SC-002**: A C# developer with no prior tp-lib experience can write a working integration that projects GNSS data in under 30 minutes, using only the package documentation and IntelliSense.
- **SC-003**: The NuGet package installs and runs without errors on Windows x64, Linux x64, and macOS (at least one architecture) without requiring any manual native dependency steps.
- **SC-004**: Every error condition that produces a typed exception in tp-py produces a corresponding typed exception in tp-net with a message of equivalent clarity.
- **SC-005**: 100% of public API types and functions have IntelliSense-visible XML documentation.
