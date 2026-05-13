# Quickstart: TpLib C#/.NET Bindings

**Feature**: `005-dotnet-bindings`  
**Target audience**: .NET developers consuming tp-lib from C#

---

## Prerequisites

- .NET 8 SDK or later
- A supported platform: Windows x64, Linux x64, macOS x64, or macOS arm64
- Railway network data available in one of two forms: 
  - a GeoJSON string (file or database text/jsonb column — see `test-data/sample_network.geojson` for format), 
  - or structured rows from a relational `network_segments` table. Pass either form via `NetworkInput.FromGeoJson()` or `NetworkInput.FromRecords()` respectively — or use the raw-string convenience overloads for quick scripting.
- GNSS positions in one of three forms:
  - **GeoJSON string**: `GnssInput.FromGeoJson(string)`
  - **CSV string** with `latitude`, `longitude`, and `timestamp` columns: `GnssInput.FromCsv(string)` 
  - **Typed objects** mapped from database rows: `GnssInput.FromRecords(IEnumerable<GnssRecord>)` — no serialization required on the caller's side

---

## 1. Add the NuGet Package

```bash
dotnet add package TpLib
```

Or in your `.csproj`:

```xml
<PackageReference Include="TpLib" Version="*" />
```

The package includes pre-built native binaries for all supported platforms. No additional setup is required — the correct binary is loaded automatically at runtime.

---

## 2. Project GNSS Positions (Simple Projection)

`Projection.ProjectGnss()` performs **simple, topology-free projection** (feature 001): each GNSS position is matched independently to its geometrically nearest netelement using an R-tree spatial index. Network relations are not used and do not need to be present in the network input.

Use this when:
- You want a quick per-point nearest-segment match without full path reconstruction.
- You are doing data quality analysis or debugging GNSS accuracy.
- Netrelations are not available.

For topology-aware path reconstruction that leverages network connectivity, see [Section 3: Calculate a Train Path](#3-calculate-a-train-path).

```csharp
using TpLib;

// Load your network and GNSS data
string networkGeoJson = File.ReadAllText("sample_network.geojson");
string gnssGeoJson = File.ReadAllText("sample_gnss.geojson");

// Project GNSS onto the network with default settings
IReadOnlyList<ProjectedPosition> positions = Projection.ProjectGnss(
    networkGeoJson,
    gnssGeoJson);

foreach (var pos in positions)
{
    Console.WriteLine($"[{pos.Timestamp:O}] {pos.NetelementId} @ {pos.MeasureMeters:F1}m " +
                      $"(dist={pos.ProjectionDistanceMeters:F2}m)");
}
```

Custom configuration:

```csharp
var config = new ProjectionConfig
{
    MaxSearchRadiusMeters = 500.0,
    ProjectionDistanceWarningThreshold = 30.0,
    SuppressWarnings = true
};

IReadOnlyList<ProjectedPosition> positions = Projection.ProjectGnss(
    networkGeoJson,
    gnssGeoJson,
    config);
```

---

## 3. Calculate a Train Path

`PathCalculation.CalculateTrainPath()` uses a **Hidden Markov Model (Viterbi algorithm)** over network topology to find the most probable train path (feature 002). The returned `ProjectedPositions` are the GNSS points projected *along the reconstructed path*, which is more accurate than simple nearest-segment projection.

Check `result.Mode` to know which algorithm was used:
- `PathCalculationMode.TopologyBased` — the HMM used network topology successfully (normal case).
- `PathCalculationMode.FallbackIndependent` — topology was insufficient; the library fell back to per-point nearest-segment matching (equivalent to `Projection.ProjectGnss()`). Check `result.Warnings` for details.

If detection anchors are available, prepare them first (see [Section 4](#4-prepare-detection-records)) and pass the `PreparedDetections` result to this call so anchors constrain the HMM path.

```csharp
using TpLib;

string networkGeoJson = File.ReadAllText("sample_network.geojson");
string gnssGeoJson = File.ReadAllText("sample_gnss.geojson");

PathResult result = PathCalculation.CalculateTrainPath(networkGeoJson, gnssGeoJson);

if (result.HasPath)
{
    TrainPath path = result.Path!;
    Console.WriteLine($"Path probability: {path.OverallProbability:P1}");
    Console.WriteLine($"Segments: {path.Segments.Count}");

    foreach (var seg in path.Segments)
    {
        Console.WriteLine($"  {seg.NetelementId}: {seg.StartIntrinsic:F3} → {seg.EndIntrinsic:F3}");
    }
}
else
{
    Console.WriteLine("No path found.");
    foreach (var warning in result.Warnings)
        Console.WriteLine($"  Warning: {warning}");
}
```

Custom path configuration:

```csharp
var pathConfig = new PathConfig
{
    CutoffDistanceMeters = 500.0,
    ProbabilityThreshold = 0.05,
    ResamplingDistanceMeters = 10.0
};

PathResult result = PathCalculation.CalculateTrainPath(
    networkGeoJson,
    gnssGeoJson,
    pathConfig);
```

---

## 4. Project Onto a Pre-Calculated Path

`Projection.ProjectOntoPath()` projects GNSS positions along a **pre-calculated** `TrainPath`. This is the equivalent of the tp-cli `--train-path` flag:

```bash
# Calculate path and project in one step (Section 3)
tp-cli --gnss positions.csv --network network.geojson --output result.csv

# Project onto a previously saved/reviewed path (this section)
tp-cli --gnss positions.csv --network network.geojson --train-path path.csv --output result.csv
```

Use this workflow when you:
- Have a path that was reviewed and edited in the webapp and want to re-project GNSS positions onto it.
- Want to re-use a path calculated from one GNSS recording to project another recording on the same route.

Unlike `ProjectGnss`, **every returned `ProjectedPosition` has `Intrinsic` populated** (normalised 0–1 offset along the matched segment).

```csharp
using TpLib;

string networkGeoJson = File.ReadAllText("sample_network.geojson");
string gnssGeoJson    = File.ReadAllText("sample_gnss.geojson");

// --- Option A: path comes from a prior CalculateTrainPath call ---
PathResult calc = PathCalculation.CalculateTrainPath(networkGeoJson, gnssGeoJson);
TrainPath reviewedPath = calc.Path ?? throw new InvalidOperationException("No path found");

// Optionally: persist reviewedPath to JSON, send to webapp for review, reload later.
// string json = JsonSerializer.Serialize(reviewedPath);
// reviewedPath = JsonSerializer.Deserialize<TrainPath>(json)!;

// --- Option B: path loaded from a previously saved file ---
// TrainPath reviewedPath = JsonSerializer.Deserialize<TrainPath>(
//     File.ReadAllText("reviewed_path.json"))!;

// Project GNSS positions onto the reviewed path.
IReadOnlyList<ProjectedPosition> positions = Projection.ProjectOntoPath(
    networkGeoJson,
    GnssInput.FromGeoJson(gnssGeoJson),
    reviewedPath);

foreach (var pos in positions)
{
    // Intrinsic is always non-null when using ProjectOntoPath
    Console.WriteLine(
        $"{pos.Timestamp:u}  {pos.NetelementId}  " +
        $"measure={pos.MeasureMeters:F1}m  intrinsic={pos.Intrinsic:F4}  " +
        $"dist={pos.ProjectionDistanceMeters:F1}m");
}
```

**Key differences from `CalculateTrainPath`:**

| | `CalculateTrainPath` | `ProjectOntoPath` |
|---|---|---|
| Input path | Calculated internally | Provided by caller |
| `Intrinsic` | Populated | Populated |
| `PathResult.Mode` | Returned | N/A (no topology step) |
| Detections | Supported | Not applicable |
| Use case | First run, unknown path | Re-projection on known path |

**Preconditions:**
- All `NetelementId` values in `path.Segments` must exist in the provided network.
- `TpLibPathException` is thrown when any referenced netelement is missing.

---

## 5. Prepare Detection Records

Detection records (axle counters, loop sensors, etc.) serve as **spatial anchors** that constrain the HMM path calculation. The correct workflow is:

1. **Prepare detections first** — time-filter, validate, and resolve detection events against the network.
2. **Pass the result to `CalculateTrainPath`** — resolved anchors constrain the Viterbi path.

```csharp
using TpLib;

string networkGeoJson = File.ReadAllText("sample_network.geojson");
string gnssGeoJson = File.ReadAllText("sample_gnss.geojson");

// Step 1: Build detection records (e.g. from CSV or DB)
var detections = new List<DetectionRecord>
{
    new DetectionRecord(
        SourceFile: "sensors.csv",
        SourceRow: 0,
        Kind: DetectionKind.Punctual,
        Timestamp: new DetectionTimestamp.Single(
            DateTimeOffset.Parse("2026-03-13T17:15:00+01:00")),
        Id: "D001",
        Source: "axle-counter-A",
        Metadata: new Dictionary<string, string> { ["confidence"] = "0.95" }),

    new DetectionRecord(
        SourceFile: "sensors.csv",
        SourceRow: 1,
        Kind: DetectionKind.Linear,
        Timestamp: new DetectionTimestamp.Range(
            From: DateTimeOffset.Parse("2026-03-13T17:15:30+01:00"),
            To: DateTimeOffset.Parse("2026-03-13T17:15:45+01:00")),
        Id: "D002",
        Source: "loop-sensor-B",
        Metadata: ImmutableDictionary<string, string>.Empty),
};

// Step 2: Prepare detections — time-filter and resolve to network elements.
// Uses the GNSS window to discard out-of-range events.
PreparedDetections prepared = DetectionPreparation.PrepareDetections(
    networkGeoJson,
    GnssInput.FromGeoJson(gnssGeoJson),
    detections);

foreach (var warning in prepared.Warnings)
    Console.WriteLine($"Detection warning: {warning}");

// Step 3: Calculate path anchored by the prepared detections.
PathResult pathResult = PathCalculation.CalculateTrainPath(
    networkGeoJson,
    gnssGeoJson,
    config: null,
    detections: prepared);

// Step 4: Inspect detection provenance from the path result.
foreach (var record in pathResult.DetectionProvenance)
{
    switch (record.Status)
    {
        case DetectionStatus.Applied applied:
            Console.WriteLine($"{record.Id}: Applied to {applied.NetelementId}");
            break;
        case DetectionStatus.Resolved resolved:
            Console.WriteLine($"{record.Id}: Resolved to {resolved.NetelementId} ({resolved.DistanceMeters:F1}m away)");
            break;
        case DetectionStatus.Discarded { Reason: DiscardReason.OutOfTimeRange otr }:
            Console.WriteLine($"{record.Id}: Discarded — outside GNSS window [{otr.GnssFirst:t} – {otr.GnssLast:t}]");
            break;
        case DetectionStatus.Discarded { Reason: DiscardReason.OutOfReach oor }:
            Console.WriteLine($"{record.Id}: Discarded — nearest element {oor.NearestDistanceMeters:F0}m (cutoff {oor.CutoffMeters:F0}m)");
            break;
        case DetectionStatus.Discarded discarded:
            Console.WriteLine($"{record.Id}: Discarded — {discarded.Reason.GetType().Name}");
            break;
    }
}
```

---

## 6. Error Handling

All TpLib operations throw typed exceptions. Catch the base `TpLibException` or specific subclasses:

```csharp
using TpLib;

try
{
    var positions = Projection.ProjectGnss(networkGeoJson, gnssGeoJson);
}
catch (TpLibParseException ex)
{
    Console.Error.WriteLine($"Failed to parse input: {ex.Message}");
}
catch (NoMatchWithinRadiusException ex)
{
    // All GNSS points fell outside the search radius
    Console.Error.WriteLine($"Projection failed: {ex.Message}");
}
catch (TpLibException ex)
{
    // Catch-all for other tp-lib errors
    Console.Error.WriteLine($"tp-lib error: {ex.Message}");
}
```

---

## 7. Timezone Awareness

All timestamps in TpLib use `DateTimeOffset`, preserving the original UTC offset.  
When parsing timestamps from CSV/text, always specify the timezone:

```csharp
// Good — timezone preserved
var ts = DateTimeOffset.Parse("2026-03-13T17:15:00+01:00");

// Avoid — loses timezone info
var ts = DateTime.Parse("2026-03-13T17:15:00");
```

GNSS timestamps returned in `ProjectedPosition.Timestamp` carry the original UTC offset from the GeoJSON source.

---

## 8. Full Pipeline Example

This example shows two common workflows:

**Workflow A — Simple projection only** (no topology, no detections): fast nearest-segment match, good for diagnostics.

**Workflow B — Topology-aware path with detections** (recommended for production): prepare detection anchors first, then calculate the path with those anchors.

```csharp
using TpLib;

// Load data
string networkGeoJson = File.ReadAllText("network.geojson");
string gnssGeoJson    = File.ReadAllText("gnss_log.geojson");

// ── Workflow A: Simple projection (topology-free) ──
IReadOnlyList<ProjectedPosition> projected = Projection.ProjectGnss(
    networkGeoJson, gnssGeoJson,
    new ProjectionConfig { ProjectionDistanceWarningThreshold = 30.0 });

Console.WriteLine($"Simple projection: {projected.Count} positions.");

// ── Workflow B: Topology-aware path calculation with detection anchors ──

// Step 1: Prepare detections (must happen before CalculateTrainPath)
var detections = ParseDetections("detections.csv");  // your CSV parser
PreparedDetections prepared = DetectionPreparation.PrepareDetections(
    networkGeoJson, GnssInput.FromGeoJson(gnssGeoJson), detections);

Console.WriteLine($"Detection preparation: {prepared.Records.Count} records, {prepared.Warnings.Count} warnings.");

// Step 2: Calculate path anchored by the prepared detections
PathResult pathResult = PathCalculation.CalculateTrainPath(
    networkGeoJson, gnssGeoJson,
    config: null,
    detections: prepared);

Console.WriteLine(pathResult.HasPath
    ? $"Path ({pathResult.Mode}): {pathResult.Path!.Segments.Count} segments, p={pathResult.Path.OverallProbability:P1}"
    : $"No path found (mode={pathResult.Mode}).");

// Step 3: Inspect detection provenance
int applied   = pathResult.DetectionProvenance.Count(r => r.Status is DetectionStatus.Applied);
int resolved  = pathResult.DetectionProvenance.Count(r => r.Status is DetectionStatus.Resolved);
int discarded = pathResult.DetectionProvenance.Count(r => r.Status is DetectionStatus.Discarded);

Console.WriteLine($"Detections: {applied} applied, {resolved} resolved, {discarded} discarded.");
```

---

## 9. Database-Backed Service Integration

When tp-lib is used inside a service that stores railway data in PostgreSQL (or any relational database), all inputs are available as in-memory data — no files or string serialization steps are required on the caller's side.

Use `GnssInput.FromRecords()` to map database rows directly to typed `GnssRecord` objects. The library handles everything across the FFI boundary transparently.

### Prerequisites

- Network topology available in one of two forms:
  - **GeoJSON text/jsonb column**: wrap with `NetworkInput.FromGeoJson(fetchedString)`.
  - **Structured tables**: a `netelements` table (`id`, `crs`, `coordinates`) and a `netrelations` table (`id`, `netelement_a_id`, `netelement_b_id`, `position_on_a`, `position_on_b`, `navigability`). Map rows to `NetworkSegment` + `NetworkRelation` and wrap with `NetworkInput.FromRecords(netelements, netrelations)`.
- GNSS readings stored as structured rows: `latitude DOUBLE PRECISION`, `longitude DOUBLE PRECISION`, `timestamp TIMESTAMPTZ`.
- Detection events stored as structured rows (or a joined view).

The examples below use [Dapper](https://github.com/DapperLib/Dapper) for brevity; any ADO.NET-compatible library works.

### Fetching and projecting

```csharp
using TpLib;
using Dapper;
using Npgsql;

await using var connection = new NpgsqlConnection(connectionString);

// Option A: network stored as a GeoJSON text/jsonb column
string rawGeoJson = await connection.QuerySingleAsync<string>(
    "SELECT geojson FROM railway_networks WHERE id = @networkId",
    new { networkId });
NetworkInput networkInput = NetworkInput.FromGeoJson(rawGeoJson);

// Option B: network stored as structured rows — two tables (netelements + netrelations)
var elementRows = await connection.QueryAsync(
    "SELECT id, crs, coordinates FROM netelements WHERE network_id = @networkId",
    new { networkId });
var relationRows = await connection.QueryAsync(
    """SELECT id, netelement_a_id, netelement_b_id,
              position_on_a, position_on_b, navigability
       FROM netrelations WHERE network_id = @networkId""",
    new { networkId });
NetworkInput networkInput = NetworkInput.FromRecords(
    netelements: elementRows.Select(r => new NetworkSegment(
        Id: r.id,
        Coordinates: ParseCoordinates(r.coordinates), // your geometry deserializer
        Crs: r.crs)),
    netrelations: relationRows.Select(r => new NetworkRelation(
        Id: r.id,
        NetelementAId: r.netelement_a_id,
        NetelementBId: r.netelement_b_id,
        PositionOnA: r.position_on_a,
        PositionOnB: r.position_on_b,
        Navigability: Enum.Parse<Navigability>(r.navigability, ignoreCase: true))));

// 2. Fetch GNSS rows and map directly to typed records — no serialization required
var gnssRows = await connection.QueryAsync(
    """
    SELECT latitude, longitude, timestamp
    FROM gnss_logs
    WHERE task_id = @taskId
    ORDER BY timestamp
    """,
    new { taskId });

GnssInput gnssInput = GnssInput.FromRecords(
    gnssRows.Select(r => new GnssRecord(r.latitude, r.longitude, r.timestamp)));

// 3. Project GNSS onto the network
IReadOnlyList<ProjectedPosition> positions = Projection.ProjectGnss(networkInput, gnssInput);
```

### Full pipeline

```csharp
// 5. Fetch detection records and map to tp-lib types
var detectionRows = await connection.QueryAsync(
    """
    SELECT id, source, source_file, source_row, kind,
           timestamp_from, timestamp_to
    FROM detections
    WHERE task_id = @taskId
    """,
    new { taskId });

var detections = detectionRows.Select(r => new DetectionRecord
{
    Id          = r.id?.ToString(),
    Source      = r.source,
    SourceFile  = r.source_file,
    SourceRow   = (ulong)r.source_row,
    Kind        = Enum.Parse<DetectionKind>(r.kind, ignoreCase: true),
    Timestamp   = r.timestamp_to is null
                  ? new DetectionTimestamp.Single(r.timestamp_from)
                  : new DetectionTimestamp.Range(r.timestamp_from, r.timestamp_to),
    Metadata    = new Dictionary<string, string>()
}).ToList();

// 6. Prepare detections first — anchors constrain the HMM path calculation.
PreparedDetections prepared = DetectionPreparation.PrepareDetections(
    networkInput, gnssInput, detections);

// 7. Calculate path with detection anchors
PathResult pathResult = PathCalculation.CalculateTrainPath(
    networkInput, gnssInput, config: null, detections: prepared);

if (!pathResult.HasPath)
{
    logger.LogWarning("No path found for task {TaskId}", taskId);
    return;
}

// 8. Persist results
await WriteResultsAsync(connection, taskId, pathResult, prepared);
```

### Key points

| Concern | Approach |
|---|---|
| Network from GeoJSON column | `NetworkInput.FromGeoJson(fetchedString)` — no conversion needed |
| Network from structured tables | Map rows to `NetworkSegment` + `NetworkRelation` → `NetworkInput.FromRecords(netelements, netrelations)` — no serialization |
| GNSS data from DB | Map rows to `GnssRecord` → `GnssInput.FromRecords()` — no serialization |
| GNSS data from CSV file | `GnssInput.FromCsv(File.ReadAllText(...))` |
| GNSS data as GeoJSON | `GnssInput.FromGeoJson(string)` or the `string gnssGeoJson` convenience overload |
| Timestamps | Use `DateTimeOffset` (Dapper maps `TIMESTAMPTZ` automatically) — timezone offset is required |
| No temp files | The entire pipeline runs in-memory |

---

## Supported Platforms

| RID | OS | Architecture |
|---|---|---|
| `win-x64` | Windows 10/11, Windows Server 2019+ | x86-64 |
| `linux-x64` | Ubuntu 20.04+, Debian 11+, RHEL 8+ | x86-64 |
| `osx-x64` | macOS 12+ | Intel |
| `osx-arm64` | macOS 12+ | Apple Silicon |

---

## Building from Source

To build the native library and C# wrapper from source:

```bash
# Clone the repository
git clone https://github.com/infrabel/tp-lib.git
cd tp-lib

# Build the Rust native library for your platform
cargo build --release --package tp-net

# Build and test the C# project
cd tp-net/csharp
dotnet build
dotnet test
```

To build the NuGet package locally:

```bash
cd tp-net/csharp
dotnet pack -c Release
```
