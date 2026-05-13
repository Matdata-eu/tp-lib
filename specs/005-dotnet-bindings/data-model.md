# Data Model: C#/.NET Bindings (tp-net)

**Phase**: Phase 1 — Design & Contracts  
**Date**: 2026-05-13  
**Feature**: `005-dotnet-bindings`

---

## Overview

The tp-net data model mirrors tp-py's public surface, mapping Rust core types to idiomatic C# records and classes. All types live in the `TpLib` namespace. Serialization across the native FFI boundary uses JSON (`System.Text.Json`); public C# types are the consumer-facing API.

---

## Entity Map

### Input Entities

#### `ProjectionConfig`
Configuration for GNSS projection.

| C# Property | Rust source | Type | Description |
|---|---|---|---|
| `MaxSearchRadiusMeters` | `max_search_radius_meters` | `double` | Maximum search radius for nearest-segment lookup (m). Default: 1000.0 |
| `ProjectionDistanceWarningThreshold` | `projection_distance_warning_threshold` | `double` | Warning threshold for large projection distances (m). Default: 50.0 |
| `SuppressWarnings` | `suppress_warnings` | `bool` | Suppress warning messages. Default: false |

Validation:
- `MaxSearchRadiusMeters > 0`
- `ProjectionDistanceWarningThreshold >= 0`

---

#### `PathConfig`
Configuration for train path calculation.

| C# Property | Rust source | Type | Description |
|---|---|---|---|
| `DistanceScale` | `distance_scale` | `double` | Emission probability distance scale (m). Default: 10.0 |
| `HeadingScale` | `heading_scale` | `double` | Emission probability heading scale (degrees). Default: 2.0 |
| `CutoffDistanceMeters` | `cutoff_distance` | `double` | Maximum candidate distance from GNSS position (m). Default: 500.0 |
| `HeadingCutoffDegrees` | `heading_cutoff` | `double` | Maximum heading difference for candidates (degrees). Default: 10.0 |
| `ProbabilityThreshold` | `probability_threshold` | `double` | Minimum probability threshold for candidates (0–1). Default: 0.02 |
| `ResamplingDistanceMeters` | `resampling_distance` | `double?` | GNSS resampling distance (m). Null = disabled |
| `MaxCandidates` | `max_candidates` | `int` | Maximum candidate netelements per GNSS position. Default: 3 |
| `PathOnly` | `path_only` | `bool` | Skip projecting positions onto path; `ProjectedPositions` will be empty. Default: false |
| `Beta` | `beta` | `double` | Transition probability scale β in meters (Newson & Krumm). Default: 50.0 |
| `EdgeZoneDistanceMeters` | `edge_zone_distance` | `double` | Distance threshold for edge-zone handling (m). Default: 50.0 |
| `TurnScaleDegrees` | `turn_scale` | `double` | Turn-angle scale (degrees). Default: 30.0 |
| `DetectionCutoffDistanceMeters` | `detection_cutoff_distance` | `double` | Max distance for resolving coordinate-only detections (m). Default: 2.5 |

Validation:
- All numeric fields ≥ 0
- `ProbabilityThreshold` ∈ [0.0, 1.0]

---

#### `DetectionRecord` *(input to `PrepareDetections`)*
A single train detection event (punctual or linear sensor).

| C# Property | Rust source | Type | Description |
|---|---|---|---|
| `Id` | `id` | `string?` | Optional detection identifier |
| `Source` | `source` | `string?` | Optional source system identifier |
| `SourceFile` | `source_file` | `string` | Source file path |
| `SourceRow` | `source_row` | `ulong` | Row index in source file |
| `Kind` | `kind` | `DetectionKind` | `Punctual` or `Linear` |
| `Timestamp` | `timestamp` | `DetectionTimestamp` | Single instant or time range (see below) |
| `NetelementId` | *(via status)* | `string?` | Pre-assigned netelement reference (optional) |
| `Metadata` | `metadata` | `IReadOnlyDictionary<string, string>` | Arbitrary key-value pairs from source file |

---

#### `DetectionTimestamp` *(discriminated union)*

| Variant | C# representation | Fields |
|---|---|---|
| `Single` | `DetectionTimestamp.Single` | `Timestamp: DateTimeOffset` |
| `Range` | `DetectionTimestamp.Range` | `From: DateTimeOffset`, `To: DateTimeOffset` |

Implementation: abstract base class with two sealed subclasses. Both carry timezone-aware `DateTimeOffset` (maps from Rust's `DateTime<FixedOffset>`).

---

#### `NetworkSegment` *(netelement — track geometry)*
A single railway track segment, wrapping the Rust `Netelement` struct.
Corresponds to GeoJSON features with `"type": "netelement"`.

| C# Property | Rust source | Type | Description |
|---|---|---|---|
| `Id` | `id` | `string` | Unique netelement identifier |
| `Coordinates` | `geometry` (`LineString<f64>`) | `IReadOnlyList<(double Longitude, double Latitude)>` | Ordered list of coordinate pairs (longitude first, GeoJSON convention). Minimum 2 points. |
| `Crs` | `crs` | `string` | Coordinate reference system. Default: `"EPSG:4326"` |

Validation:
- `Id` must be non-empty.
- `Coordinates` must have at least 2 points.

---

#### `NetworkRelation` *(netrelation — topology)*
A directed connection between two track segments, wrapping the Rust `NetRelation` struct.
Corresponds to GeoJSON features with `"type": "netrelation"`.

| C# Property | Rust source | GeoJSON field | Type | Description |
|---|---|---|---|---|
| `Id` | `id` | `id` | `string` | Unique netrelation identifier |
| `NetelementAId` | `from_netelement_id` | `netelementA` | `string` | ID of the first connected track segment |
| `NetelementBId` | `to_netelement_id` | `netelementB` | `string` | ID of the second connected track segment |
| `PositionOnA` | `position_on_a` | `positionOnA` | `int` | Endpoint of segment A used by this connection: `0` = start, `1` = end |
| `PositionOnB` | `position_on_b` | `positionOnB` | `int` | Endpoint of segment B used by this connection: `0` = start, `1` = end |
| `Navigability` | `navigable_forward`/`navigable_backward` | `navigability` | `Navigability` | Allowed travel directions |

**`Navigability` enum**:

| Value | GeoJSON string | Description |
|---|---|---|
| `Both` | `"both"` | Trains may travel in both directions |
| `Forward` | `"AB"` | Trains may travel from A to B only |
| `Backward` | `"BA"` | Trains may travel from B to A only |
| `None` | `"none"` | No train movement permitted |

Validation:
- `Id`, `NetelementAId`, `NetelementBId` must be non-empty.
- `PositionOnA` and `PositionOnB` must each be `0` or `1`.

---

`NetworkInput` is the wrapper that carries the chosen entry path (`FromRecords` or `FromGeoJson`). When `FromGeoJson` is called, tp-net passes the raw string to the Rust core unchanged. When `FromRecords` is called, tp-net serializes both collections into an equivalent GeoJSON FeatureCollection internally before crossing the FFI boundary — the two datasets are merged into a single FeatureCollection with mixed feature types, matching the format the Rust core expects.

---

### Output Entities

#### `ProjectedPosition`
A single GNSS position projected onto the railway network.

| C# Property | Rust source | Type | Description |
|---|---|---|---|
| `NetelementId` | `netelement_id` | `string` | Track segment ID |
| `MeasureMeters` | `measure_meters` | `double` | Distance along netelement from start (m) |
| `ProjectionDistanceMeters` | `projection_distance_meters` | `double` | Perpendicular projection distance (m) |
| `ProjectedX` | `projected_coords.x()` | `double` | Projected X coordinate in the output CRS |
| `ProjectedY` | `projected_coords.y()` | `double` | Projected Y coordinate in the output CRS |
| `Crs` | `crs` | `string` | Coordinate reference system |
| `OriginalLatitude` | `original.latitude` | `double` | Original WGS84 latitude |
| `OriginalLongitude` | `original.longitude` | `double` | Original WGS84 longitude |
| `Timestamp` | `original.timestamp` | `DateTimeOffset` | Observation time with timezone |
| `Intrinsic` | `intrinsic` | `double?` | Normalised position along the matched segment (0–1, from start to end). Populated when projecting onto a pre-calculated path (`ProjectOntoPath`); null for simple nearest-segment projection (`ProjectGnss`) |

---

#### `TrainPath`
Reconstructed path of a train across the railway network.

| C# Property | Rust source | Type | Description |
|---|---|---|---|
| `Segments` | `segments` | `IReadOnlyList<AssociatedNetElement>` | Ordered traversal segments |
| `OverallProbability` | `overall_probability` | `double` | Path quality score (0–1) |
| `CalculatedAt` | `calculated_at` | `DateTimeOffset?` | Calculation timestamp |

---

#### `AssociatedNetElement`
A single network element in a train path.

| C# Property | Rust source | Type | Description |
|---|---|---|---|
| `NetelementId` | `netelement_id` | `string` | Track segment ID |
| `Probability` | `probability` | `double` | Segment confidence score (0–1) |
| `StartIntrinsic` | `start_intrinsic` | `double` | Entry point (0–1) |
| `EndIntrinsic` | `end_intrinsic` | `double` | Exit point (0–1) |
| `GnssStartIndex` | `gnss_start_index` | `int` | First GNSS position index |
| `GnssEndIndex` | `gnss_end_index` | `int` | Last GNSS position index |
| `Origin` | `origin` | `PathOrigin` | Whether segment was placed by the algorithm or manually added/adjusted by a user in the webapp (see `PathOrigin`). Default: `Algorithm` |

---

#### `PathResult`
Full result from `CalculateTrainPath`, including optional path and diagnostics.

| C# Property | Rust source | Type | Description |
|---|---|---|---|
| `Path` | `path` | `TrainPath?` | Calculated path; null if calculation failed |
| `Mode` | `mode` | `PathCalculationMode` | `TopologyBased` or `FallbackIndependent` |
| `ProjectedPositions` | `projected_positions` | `IReadOnlyList<ProjectedPosition>` | All GNSS positions projected along the reconstructed path; empty when `PathConfig.PathOnly = true` |
| `Warnings` | `warnings` | `IReadOnlyList<string>` | Alerts emitted during calculation |
| `DetectionProvenance` | `detection_provenance` | `IReadOnlyList<DetectionRecord>` | Final provenance of every detection passed via `PreparedDetections`; empty when no detections were supplied |
| `HasPath` | computed | `bool` | `Path != null` |

---

#### `PreparedDetections`
Result from `PrepareDetections`, containing enriched detection records.

| C# Property | Rust source | Type | Description |
|---|---|---|---|
| `Records` | `records` | `IReadOnlyList<DetectionRecord>` | All records with status applied |
| `Warnings` | `warnings` | `IReadOnlyList<string>` | Non-fatal warnings emitted during preparation |

---

### Status & Reason Enumerations

#### `DetectionKind`
```
Punctual  — single-point sensor event
Linear    — span/range sensor event
```

#### `DetectionStatus` *(discriminated union on `DetectionRecord.Status`)*

| Variant | Extra fields | Description |
|---|---|---|
| `Applied` | `NetelementId: string`, `Intrinsic: double` | Detection directly matched to a network element |
| `Resolved` | `NetelementId: string`, `DistanceMeters: double` | Detection matched by proximity |
| `Discarded` | `Reason: DiscardReason` | Detection could not be matched |

#### `DiscardReason` *(discriminated union)*

| Variant | Extra fields | Description |
|---|---|---|
| `OutOfTimeRange` | `GnssFirst: DateTimeOffset`, `GnssLast: DateTimeOffset` | Timestamp outside GNSS coverage window |
| `OutOfReach` | `NearestDistanceMeters: double`, `CutoffMeters: double` | No network element within search radius |
| `UnknownNetelement` | `NetelementId: string` | Referenced element not in network |
| `IntrinsicOutOfRange` | `Value: double` | Computed intrinsic outside [0, 1] |
| `DuplicateOfPriorDetection` | `KeptIndex: int` | Duplicate of an earlier record |

#### `PathCalculationMode`
```
TopologyBased       — network graph used for HMM/Viterbi matching
FallbackIndependent — topology unavailable; segments matched independently
```

#### `PathOrigin`
```
Algorithm — segment selected by the Viterbi/HMM algorithm (default; backward-compatible with older saved path files)
Manual    — segment manually added or adjusted by a user in the webapp review interface
```

---

## Exception Hierarchy

All exceptions derive from `TpLibException` (base).

```
TpLibException
├── TpLibIoException           — file/stream read errors
├── TpLibParseException        — GeoJSON or CSV parse errors
├── TpLibConfigurationException — invalid parameter values
├── TpLibProjectionException    — projection failures
│   └── NoMatchWithinRadiusException
├── TpLibPathException          — path calculation failures
│   └── NoNavigablePathException
└── TpLibDetectionException     — detection preparation failures
```

Maps from Rust's `ProjectionError` variants (see tp-py `convert_error` for reference).

---

## Type Lifecycle & Memory Management

- All public C# types are **managed** (GC-owned); no `IDisposable` required by consumers.
- The native library is loaded once via `NativeLibrary.SetDllImportResolver` on first use (static initializer in `TpLibNative`).
- `ByteBuffer` allocations returned from native code are freed by the C# wrapper immediately after deserialization (via `tp_lib_net_free_buffer` FFI call).

---

## FFI Type Mapping Summary

| Rust core type | FFI boundary | C# public type |
|---|---|---|
| `ProjectionConfig` | `#[repr(C)]` struct | `ProjectionConfig` record |
| `PathConfig` | `#[repr(C)]` struct | `PathConfig` record |
| `Vec<ProjectedPosition>` | JSON `ByteBuffer` | `IReadOnlyList<ProjectedPosition>` |
| `PathResult` | JSON `ByteBuffer` | `PathResult` |
| `TrainPath` (input to `project_onto_path`) | JSON `ByteBuffer` | `TrainPath` |
| `Vec<DetectionRecord>` (input) | JSON `ByteBuffer` | `IEnumerable<DetectionRecord>` |
| `PreparedDetections` (output) | JSON `ByteBuffer` | `PreparedDetections` |
| `ProjectionError` | i32 error code + message buffer | `TpLibException` subclass |
| `DetectionError` | i32 error code + message buffer | `TpLibDetectionException` |
