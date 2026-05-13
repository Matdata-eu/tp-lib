# API Contracts: TpLib C# Public API

**Phase**: Phase 1 — Design & Contracts  
**Date**: 2026-05-13  
**Feature**: `005-dotnet-bindings`

This document specifies the **public C# API surface** for the `TpLib` NuGet package. These contracts define the boundary that downstream .NET consumers depend on and must remain stable across minor versions.

---

## Namespace: `TpLib`

---

### Class: `TpLib.Projection`

Static utility class for GNSS projection operations.

```csharp
namespace TpLib;

/// <summary>
/// Projects GNSS positions onto a railway network.
/// </summary>
public static class Projection
{
    /// <summary>
    /// Projects a sequence of GNSS positions onto the railway network using simple
    /// nearest-segment matching (feature 001 — topology-free).
    /// Each GNSS point is matched independently to its geometrically nearest netelement;
    /// network relations are not used. Use this for quick analysis, data quality checks,
    /// or when netrelations are unavailable.
    /// For topology-aware path reconstruction, see <see cref="PathCalculation.CalculateTrainPath"/>.
    /// </summary>
    /// <param name="network">Railway network as a <see cref="NetworkInput"/> (structured segments or GeoJSON).</param>
    /// <param name="gnss">GNSS positions as a <see cref="GnssInput"/> (structured records, GeoJSON, or CSV).</param>
    /// <param name="config">Projection configuration. Uses defaults when null.</param>
    /// <returns>List of projected positions, one per valid GNSS point.</returns>
    /// <exception cref="TpLibParseException">Network or GNSS data is malformed.</exception>
    /// <exception cref="TpLibProjectionException">Projection failed (e.g. empty network).</exception>
    public static IReadOnlyList<ProjectedPosition> ProjectGnss(
        NetworkInput network,
        GnssInput gnss,
        ProjectionConfig? config = null);

    /// <summary>Convenience overload: accepts the railway network as a GeoJSON string directly.</summary>
    public static IReadOnlyList<ProjectedPosition> ProjectGnss(
        string networkGeoJson,
        GnssInput gnss,
        ProjectionConfig? config = null)
        => ProjectGnss(NetworkInput.FromGeoJson(networkGeoJson), gnss, config);

    /// <summary>Convenience overload: accepts both network and GNSS as GeoJSON strings directly.</summary>
    public static IReadOnlyList<ProjectedPosition> ProjectGnss(
        string networkGeoJson,
        string gnssGeoJson,
        ProjectionConfig? config = null)
        => ProjectGnss(NetworkInput.FromGeoJson(networkGeoJson), GnssInput.FromGeoJson(gnssGeoJson), config);

    /// <summary>
    /// Projects a sequence of GNSS positions onto a <b>pre-calculated</b> railway path.
    /// Use this when you already have a <see cref="TrainPath"/> (from a prior call to
    /// <see cref="PathCalculation.CalculateTrainPath"/> or loaded from a saved/reviewed file)
    /// and want to project GNSS positions along that fixed path.
    ///
    /// <para>The resulting <see cref="ProjectedPosition.Intrinsic"/> field is populated for
    /// every position (normalised 0–1 offset along the matched segment).</para>
    ///
    /// <para>Common workflows:</para>
    /// <list type="bullet">
    ///   <item>Save the <see cref="TrainPath"/> after <see cref="PathCalculation.CalculateTrainPath"/>,
    ///   let a user review it in the webapp, then call this method to re-project using the reviewed path.</item>
    ///   <item>Re-use a path calculated for one GNSS recording to project another recording
    ///   on the same route.</item>
    /// </list>
    ///
    /// <para>For topology-free nearest-segment projection see <see cref="ProjectGnss"/>.<br/>
    /// For calculating a new path and projecting in one step see
    /// <see cref="PathCalculation.CalculateTrainPath"/> (returns
    /// <see cref="PathResult.ProjectedPositions"/>).</para>
    /// </summary>
    /// <param name="network">Railway network containing all netelements referenced by <paramref name="path"/>.</param>
    /// <param name="gnss">GNSS positions to project.</param>
    /// <param name="path">Pre-calculated train path. All netelement IDs must exist in <paramref name="network"/>.</param>
    /// <param name="config">Path configuration. Uses defaults when null.</param>
    /// <returns>One <see cref="ProjectedPosition"/> per GNSS input point, ordered as the input.
    /// <see cref="ProjectedPosition.Intrinsic"/> is populated for all returned positions.</returns>
    /// <exception cref="TpLibParseException">Network or GNSS data is malformed.</exception>
    /// <exception cref="TpLibPathException">A netelement in <paramref name="path"/> does not exist
    /// in <paramref name="network"/>, or projection onto the path fails.</exception>
    public static IReadOnlyList<ProjectedPosition> ProjectOntoPath(
        NetworkInput network,
        GnssInput gnss,
        TrainPath path,
        PathConfig? config = null);

    /// <summary>Convenience overload: accepts the railway network as a GeoJSON string directly.</summary>
    public static IReadOnlyList<ProjectedPosition> ProjectOntoPath(
        string networkGeoJson,
        GnssInput gnss,
        TrainPath path,
        PathConfig? config = null)
        => ProjectOntoPath(NetworkInput.FromGeoJson(networkGeoJson), gnss, path, config);
}
```

**Preconditions**:
- `network` is non-null; its content is non-empty and valid for the declared format.
- `gnss` is non-null; its content is non-empty and valid for the declared format.
- `config.MaxSearchRadiusMeters > 0` when config is non-null.

**Postconditions**:
- Returns a list of `ProjectedPosition` objects, ≥0 elements.
- Each returned position has `MeasureMeters >= 0` and `ProjectionDistanceMeters >= 0`.

---

### Class: `TpLib.PathCalculation`

Static utility class for train path calculation.

```csharp
namespace TpLib;

/// <summary>
/// Calculates train paths from projected GNSS positions.
/// </summary>
public static class PathCalculation
{
    /// <summary>
    /// Calculates the most probable train path through the railway network using a
    /// Hidden Markov Model (Viterbi algorithm) over network topology (feature 002).
    /// The returned <see cref="PathResult.ProjectedPositions"/> are projected along
    /// the reconstructed path — more accurate than simple nearest-segment projection.
    /// Check <see cref="PathResult.Mode"/> to determine which mode was used:
    /// <see cref="PathCalculationMode.TopologyBased"/> (normal) or
    /// <see cref="PathCalculationMode.FallbackIndependent"/> (topology unavailable;
    /// falls back to per-point nearest-segment matching, equivalent to
    /// <see cref="Projection.ProjectGnss"/>).
    /// </summary>
    /// <param name="network">Railway network as a <see cref="NetworkInput"/> (structured segments or GeoJSON).</param>
    /// <param name="gnss">GNSS positions as a <see cref="GnssInput"/> (structured records, GeoJSON, or CSV).</param>
    /// <param name="config">Path calculation configuration. Uses defaults when null.</param>
    /// <param name="detections">Optional prepared detections to anchor the HMM calculation.
    /// Obtain via <see cref="DetectionPreparation.PrepareDetections"/>.
    /// When provided, resolved anchors constrain the Viterbi path.</param>
    /// <returns>PathResult with the calculated path and diagnostics.</returns>
    /// <exception cref="TpLibParseException">Network or GNSS data is malformed.</exception>
    /// <exception cref="TpLibPathException">Path calculation encountered an unrecoverable error.</exception>
    public static PathResult CalculateTrainPath(
        NetworkInput network,
        GnssInput gnss,
        PathConfig? config = null,
        PreparedDetections? detections = null);

    /// <summary>Convenience overload: accepts the railway network as a GeoJSON string directly.</summary>
    public static PathResult CalculateTrainPath(
        string networkGeoJson,
        GnssInput gnss,
        PathConfig? config = null,
        PreparedDetections? detections = null)
        => CalculateTrainPath(NetworkInput.FromGeoJson(networkGeoJson), gnss, config, detections);

    /// <summary>Convenience overload: accepts both network and GNSS as GeoJSON strings directly.</summary>
    public static PathResult CalculateTrainPath(
        string networkGeoJson,
        string gnssGeoJson,
        PathConfig? config = null,
        PreparedDetections? detections = null)
        => CalculateTrainPath(NetworkInput.FromGeoJson(networkGeoJson), GnssInput.FromGeoJson(gnssGeoJson), config, detections);
}
```

**Preconditions**:
- `network` is non-null; its content is non-empty and valid for the declared format.
- `gnss` is non-null; its content is non-empty and valid for the declared format.

**Postconditions**:
- Always returns a `PathResult` (never null); `PathResult.Path` may be null if no path was found.
- `PathResult.ProjectedPositions` is never null.

---

### Class: `TpLib.DetectionPreparation`

Static utility class for detection record preparation.

```csharp
namespace TpLib;

/// <summary>
/// Loads, validates, time-filters, and spatially resolves train detection records
/// against the railway network.
/// </summary>
public static class DetectionPreparation
{
    /// <summary>
    /// Prepares detection records for use as anchors in
    /// <see cref="PathCalculation.CalculateTrainPath"/>.
    /// Detection timestamps are filtered to the GNSS time window, and each detection
    /// is resolved to its nearest netelement (punctual) or span (linear).
    /// The resulting <see cref="PreparedDetections"/> should be passed to
    /// <see cref="PathCalculation.CalculateTrainPath"/> via the <c>detections</c> parameter
    /// so that resolved anchors constrain the HMM path calculation.
    /// </summary>
    /// <param name="network">Railway network as a <see cref="NetworkInput"/> (structured segments or GeoJSON).</param>
    /// <param name="gnss">GNSS positions as a <see cref="GnssInput"/>; used to establish the time window for filtering.</param>
    /// <param name="detections">Detection records to prepare. Order is preserved.</param>
    /// <param name="cutoffDistanceMeters">Maximum distance (m) for resolving coordinate-only detections. Default: 2.5.</param>
    /// <returns>PreparedDetections with all records annotated with status, ready to anchor path calculation.</returns>
    /// <exception cref="ArgumentNullException">Any required parameter is null.</exception>
    /// <exception cref="TpLibDetectionException">Preparation failed (e.g. incompatible network).</exception>
    public static PreparedDetections PrepareDetections(
        NetworkInput network,
        GnssInput gnss,
        IEnumerable<DetectionRecord> detections,
        double cutoffDistanceMeters = 2.5);

    /// <summary>Convenience overload: accepts the railway network as a GeoJSON string directly.</summary>
    public static PreparedDetections PrepareDetections(
        string networkGeoJson,
        GnssInput gnss,
        IEnumerable<DetectionRecord> detections,
        double cutoffDistanceMeters = 2.5)
        => PrepareDetections(NetworkInput.FromGeoJson(networkGeoJson), gnss, detections, cutoffDistanceMeters);
}
```

**Preconditions**:
- `network` is non-null; its content is non-empty and valid for the declared format.
- `gnss` is non-null; used to establish the time window for filtering.
- `detections` is non-null (may be empty; returns empty `PreparedDetections`).
- `cutoffDistanceMeters > 0`.

**Postconditions**:
- `result.Records.Count == detections.Count()`.
- Each record has a non-null `Status` (one of `Applied`, `Resolved`, `Discarded`).

---

## Input Format Types

```csharp
namespace TpLib;

/// <summary>
/// A railway track segment (netelement): a LineString geometry with an identifier.
/// Coordinates are in GeoJSON order — (Longitude, Latitude) — matching the WGS-84
/// convention used by the tp-lib Rust core.
/// Corresponds to features with <c>"type": "netelement"</c> in a tp-lib GeoJSON network file.
/// </summary>
/// <param name="Id">Unique identifier for the segment (matches the <c>id</c> GeoJSON property).</param>
/// <param name="Coordinates">Ordered sequence of (Longitude, Latitude) pairs forming the track centerline. Minimum 2 points.</param>
/// <param name="Crs">Coordinate reference system (default: "EPSG:4326").</param>
public sealed record NetworkSegment(
    string Id,
    IReadOnlyList<(double Longitude, double Latitude)> Coordinates,
    string Crs = "EPSG:4326");

/// <summary>
/// A topological connection between two track segments (netrelation), describing whether
/// trains may travel from one segment to another and at which endpoints they connect.
/// Corresponds to features with <c>"type": "netrelation"</c> in a tp-lib GeoJSON network file.
/// </summary>
/// <param name="Id">Unique identifier for this relation.</param>
/// <param name="NetelementAId">ID of the first connected track segment.</param>
/// <param name="NetelementBId">ID of the second connected track segment.</param>
/// <param name="PositionOnA">Endpoint of segment A used by this connection: 0 = start, 1 = end.</param>
/// <param name="PositionOnB">Endpoint of segment B used by this connection: 0 = start, 1 = end.</param>
/// <param name="Navigability">Allowed train travel directions across this connection.</param>
public sealed record NetworkRelation(
    string Id,
    string NetelementAId,
    string NetelementBId,
    int PositionOnA,
    int PositionOnB,
    Navigability Navigability);

/// <summary>
/// Allowed train travel directions across a <see cref="NetworkRelation"/>.
/// Maps to the <c>navigability</c> string property in the tp-lib GeoJSON format.
/// </summary>
public enum Navigability
{
    /// <summary>Trains may travel in both directions between the two segments.</summary>
    Both,
    /// <summary>Trains may travel from segment A to segment B only.</summary>
    Forward,
    /// <summary>Trains may travel from segment B to segment A only.</summary>
    Backward,
    /// <summary>No train movement is permitted (administrative connection only).</summary>
    None,
}

/// <summary>
/// Wraps railway network input data for tp-lib processing.
/// Callers choose the most convenient entry point; internal serialization is
/// handled by the library and is not visible to consumers.
/// </summary>
public sealed class NetworkInput
{
    /// <summary>
    /// Creates a <see cref="NetworkInput"/> from in-memory collections of typed track
    /// segments and their topological relations.
    /// This is the preferred entry point when network data originates from a relational
    /// database with separate <c>netelements</c> and <c>netrelations</c> tables —
    /// no serialization is required on the caller's side.
    /// </summary>
    /// <param name="netelements">Non-empty sequence of railway track segments.</param>
    /// <param name="netrelations">
    /// Sequence of connections between track segments.
    /// May be empty for topology-free operations (e.g., <see cref="Projection.ProjectGnss"/>),
    /// but is required for path calculation (<see cref="PathCalculation.CalculateTrainPath"/>).
    /// </param>
    public static NetworkInput FromRecords(
        IEnumerable<NetworkSegment> netelements,
        IEnumerable<NetworkRelation> netrelations);

    /// <summary>
    /// Creates a <see cref="NetworkInput"/> from a GeoJSON FeatureCollection string.
    /// The collection must contain both <c>netelement</c> (LineString) and
    /// <c>netrelation</c> (Point) features, as produced by tp-lib's export tools.
    /// Use this when network data is stored as a GeoJSON text/jsonb column or loaded from a file.
    /// Prefer <see cref="FromRecords"/> when data is available as structured rows.
    /// </summary>
    /// <param name="geoJson">Non-null, non-empty GeoJSON FeatureCollection string.</param>
    public static NetworkInput FromGeoJson(string geoJson);
}
```

```csharp
namespace TpLib;

/// <summary>
/// A GNSS point: geographic position with timestamp.
/// </summary>
/// <param name="Latitude">WGS-84 latitude in decimal degrees.</param>
/// <param name="Longitude">WGS-84 longitude in decimal degrees.</param>
/// <param name="Timestamp">Observation time — must include UTC offset.</param>
public sealed record GnssRecord(
    double Latitude,
    double Longitude,
    DateTimeOffset Timestamp);

/// <summary>
/// Wraps GNSS input data for tp-lib processing.
/// Callers choose the most convenient entry point; internal serialization is
/// handled by the library and is not visible to consumers.
/// </summary>
public sealed class GnssInput
{
    /// <summary>
    /// Creates a <see cref="GnssInput"/> from an in-memory collection of typed records.
    /// This is the preferred entry point when GNSS data originates from a relational
    /// database or any in-process data structure — no serialization is required on the
    /// caller's side.
    /// </summary>
    /// <param name="records">Non-empty sequence of GNSS points.</param>
    /// <param name="crs">Coordinate reference system (default: "EPSG:4326").</param>
    public static GnssInput FromRecords(IEnumerable<GnssRecord> records, string crs = "EPSG:4326");

    /// <summary>
    /// Creates a <see cref="GnssInput"/> from a GeoJSON FeatureCollection string.
    /// Each Feature must be a Point geometry with a <c>timestamp</c> property.
    /// Prefer <see cref="FromRecords"/> when data is already available in memory.
    /// </summary>
    /// <param name="geoJson">Non-null, non-empty GeoJSON FeatureCollection string.</param>
    /// <param name="crs">Coordinate reference system (default: "EPSG:4326").</param>
    public static GnssInput FromGeoJson(string geoJson, string crs = "EPSG:4326");

    /// <summary>
    /// Creates a <see cref="GnssInput"/> from a CSV string with a header row.
    /// Useful when reading from a CSV file or stream.
    /// Prefer <see cref="FromRecords"/> when data is already available in memory.
    /// </summary>
    /// <param name="csv">Non-null, non-empty CSV string with a header row.</param>
    /// <param name="crs">Coordinate reference system (default: "EPSG:4326").</param>
    /// <param name="latitudeColumn">Latitude column name (default: "latitude").</param>
    /// <param name="longitudeColumn">Longitude column name (default: "longitude").</param>
    /// <param name="timestampColumn">Timestamp column name (default: "timestamp"). Values must be ISO-8601.</param>
    public static GnssInput FromCsv(
        string csv,
        string crs = "EPSG:4326",
        string latitudeColumn = "latitude",
        string longitudeColumn = "longitude",
        string timestampColumn = "timestamp");
}
```

**CSV format requirements**:
- Header row required; column order is irrelevant — columns are identified by name.
- Latitude and longitude: decimal degrees (e.g. `50.8503`, `4.3517`).
- Timestamp: ISO-8601 string with timezone offset (e.g. `2026-03-13T17:15:00+01:00` or `2026-03-13T16:15:00Z`).
- Additional columns are preserved as metadata.

---

## Configuration Records

```csharp
namespace TpLib;

/// <summary>Configures GNSS projection behavior.</summary>
public sealed record ProjectionConfig
{
    public double MaxSearchRadiusMeters { get; init; } = 1000.0;
    public double ProjectionDistanceWarningThreshold { get; init; } = 50.0;
    public bool SuppressWarnings { get; init; } = false;
}

/// <summary>Configures train path calculation.</summary>
public sealed record PathConfig
{
    /// <summary>Emission probability distance scale (m). Default: 10.0</summary>
    public double DistanceScale { get; init; } = 10.0;
    /// <summary>Emission probability heading scale (degrees). Default: 2.0</summary>
    public double HeadingScale { get; init; } = 2.0;
    /// <summary>Maximum candidate distance from GNSS position (m). Default: 500.0</summary>
    public double CutoffDistanceMeters { get; init; } = 500.0;
    /// <summary>Maximum heading difference for candidates (degrees). Default: 10.0</summary>
    public double HeadingCutoffDegrees { get; init; } = 10.0;
    /// <summary>Minimum probability threshold for candidates (0–1). Default: 0.02</summary>
    public double ProbabilityThreshold { get; init; } = 0.02;
    /// <summary>Resampling distance between GNSS positions (m). Null disables resampling.</summary>
    public double? ResamplingDistanceMeters { get; init; } = null;
    /// <summary>Maximum candidate netelements per GNSS position. Default: 3</summary>
    public int MaxCandidates { get; init; } = 3;
    /// <summary>When true, skip projecting positions onto the path; ProjectedPositions will be empty. Default: false</summary>
    public bool PathOnly { get; init; } = false;
    /// <summary>Transition probability scale β in meters (Newson &amp; Krumm). Default: 50.0</summary>
    public double Beta { get; init; } = 50.0;
    /// <summary>Distance threshold for edge-zone handling (m). Default: 50.0</summary>
    public double EdgeZoneDistanceMeters { get; init; } = 50.0;
    /// <summary>Turn-angle scale (degrees). Default: 30.0</summary>
    public double TurnScaleDegrees { get; init; } = 30.0;
    /// <summary>Max distance for resolving coordinate-only detections (m). Default: 2.5</summary>
    public double DetectionCutoffDistanceMeters { get; init; } = 2.5;
}
```

---

## Result Types

```csharp
namespace TpLib;

public sealed record ProjectedPosition(
    string NetelementId,
    double MeasureMeters,
    double ProjectionDistanceMeters,
    /// <summary>Projected X coordinate in the output CRS.</summary>
    double ProjectedX,
    /// <summary>Projected Y coordinate in the output CRS.</summary>
    double ProjectedY,
    string Crs,
    double OriginalLatitude,
    double OriginalLongitude,
    DateTimeOffset Timestamp,
    /// <summary>
    /// Normalised position along the matched segment (0–1, from start to end).
    /// Populated when projecting onto a pre-calculated path (<see cref="Projection.ProjectOntoPath"/>);
    /// null for simple nearest-segment projection (<see cref="Projection.ProjectGnss"/>).
    /// </summary>
    double? Intrinsic = null);

public sealed record TrainPath(
    IReadOnlyList<AssociatedNetElement> Segments,
    double OverallProbability,
    DateTimeOffset? CalculatedAt);

public sealed record AssociatedNetElement(
    string NetelementId,
    double Probability,
    double StartIntrinsic,
    double EndIntrinsic,
    int GnssStartIndex,
    int GnssEndIndex,
    /// <summary>
    /// Whether this segment was placed by the algorithm or manually added/adjusted
    /// by a user in the webapp review interface. Defaults to <see cref="PathOrigin.Algorithm"/>;
    /// backward-compatible with older saved path files.
    /// </summary>
    PathOrigin Origin = PathOrigin.Algorithm);

public sealed record PathResult(
    TrainPath? Path,
    PathCalculationMode Mode,
    IReadOnlyList<ProjectedPosition> ProjectedPositions,
    IReadOnlyList<string> Warnings,
    /// <summary>
    /// Per-detection provenance after path calculation. Contains the final status
    /// of every detection record that was passed in via <c>PreparedDetections</c>.
    /// </summary>
    IReadOnlyList<DetectionRecord> DetectionProvenance)
{
    public bool HasPath => Path is not null;
}

public sealed record PreparedDetections(
    IReadOnlyList<DetectionRecord> Records,
    /// <summary>Non-fatal warnings emitted during detection preparation.</summary>
    IReadOnlyList<string> Warnings);
```

---

## Detection Input Types

```csharp
namespace TpLib;

public sealed record DetectionRecord(
    string SourceFile,
    ulong SourceRow,
    DetectionKind Kind,
    DetectionTimestamp Timestamp,
    string? Id,
    string? Source,
    IReadOnlyDictionary<string, string> Metadata,
    DetectionStatus? Status = null);   // null on input; populated after PrepareDetections

public enum DetectionKind { Punctual, Linear }

public abstract record DetectionTimestamp
{
    public sealed record Single(DateTimeOffset Timestamp) : DetectionTimestamp;
    public sealed record Range(DateTimeOffset From, DateTimeOffset To) : DetectionTimestamp;
}
```

---

## Status & Reason Types

```csharp
namespace TpLib;

public abstract record DetectionStatus
{
    public sealed record Applied(string NetelementId, double Intrinsic) : DetectionStatus;
    public sealed record Resolved(string NetelementId, double DistanceMeters) : DetectionStatus;
    public sealed record Discarded(DiscardReason Reason) : DetectionStatus;
}

public abstract record DiscardReason
{
    public sealed record OutOfTimeRange(
        DateTimeOffset GnssFirst,
        DateTimeOffset GnssLast) : DiscardReason;

    public sealed record OutOfReach(
        double NearestDistanceMeters,
        double CutoffMeters) : DiscardReason;

    public sealed record UnknownNetelement(string NetelementId) : DiscardReason;
    public sealed record IntrinsicOutOfRange(double Value) : DiscardReason;
    public sealed record DuplicateOfPriorDetection(int KeptIndex) : DiscardReason;
}

public enum PathCalculationMode { TopologyBased, FallbackIndependent }

/// <summary>
/// Indicates whether a path segment was placed by the path calculation algorithm
/// or manually added/adjusted by a user in the webapp review interface.
/// </summary>
public enum PathOrigin
{
    /// <summary>Segment selected by the Viterbi/HMM algorithm (default; backward-compatible with older path files).</summary>
    Algorithm,
    /// <summary>Segment manually added or adjusted by a user in the webapp.</summary>
    Manual,
}
```

---

## Exception Hierarchy

```csharp
namespace TpLib;

/// <summary>Base for all tp-lib exceptions.</summary>
public class TpLibException : Exception
{
    public TpLibException(string message) : base(message) { }
    public TpLibException(string message, Exception inner) : base(message, inner) { }
}

/// <summary>File or stream read error.</summary>
public class TpLibIoException : TpLibException { ... }

/// <summary>GeoJSON or CSV parse error.</summary>
public class TpLibParseException : TpLibException { ... }

/// <summary>Invalid parameter value.</summary>
public class TpLibConfigurationException : TpLibException { ... }

/// <summary>GNSS projection failure.</summary>
public class TpLibProjectionException : TpLibException { ... }

/// <summary>No projection found within the search radius.</summary>
public class NoMatchWithinRadiusException : TpLibProjectionException { ... }

/// <summary>Train path calculation failure.</summary>
public class TpLibPathException : TpLibException { ... }

/// <summary>No navigable path exists in the network.</summary>
public class NoNavigablePathException : TpLibPathException { ... }

/// <summary>Detection preparation failure.</summary>
public class TpLibDetectionException : TpLibException { ... }
```

---

## Stability Guarantees

| API element | Stability |
|---|---|
| All `public` types/members above | **Stable** — breaking changes require major version bump |
| `NativeMethods.g.cs` (generated, `internal`) | **Internal** — not part of public contract |
| JSON interchange format between FFI layers | **Internal** — may change between any versions |
| Error message strings | **Unstable** — use exception types for branching, not message text |
