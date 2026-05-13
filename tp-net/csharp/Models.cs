using System.Globalization;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace TpLib;

// ---------------------------------------------------------------------------
// Configuration records (mirror Rust ProjectionConfig / PathConfig defaults)
// ---------------------------------------------------------------------------

public sealed record ProjectionConfig
{
    public double MaxSearchRadiusMeters { get; init; } = 1000.0;
    public double ProjectionDistanceWarningThreshold { get; init; } = 50.0;
    public bool SuppressWarnings { get; init; } = false;
}

public sealed record PathConfig
{
    public double DistanceScale { get; init; } = 10.0;
    public double HeadingScale { get; init; } = 2.0;
    public double CutoffDistanceMeters { get; init; } = 500.0;
    public double HeadingCutoffDegrees { get; init; } = 10.0;
    public double ProbabilityThreshold { get; init; } = 0.02;
    public double? ResamplingDistanceMeters { get; init; }
    public int MaxCandidates { get; init; } = 3;
    public bool PathOnly { get; init; } = false;
    public double Beta { get; init; } = 50.0;
    public double EdgeZoneDistanceMeters { get; init; } = 50.0;
    public double TurnScaleDegrees { get; init; } = 30.0;
    public double DetectionCutoffDistanceMeters { get; init; } = 2.5;
}

// ---------------------------------------------------------------------------
// Output records (deserialized from FFI JSON — property names match Rust
// snake_case via [JsonPropertyName]).
// ---------------------------------------------------------------------------

public sealed record GnssOriginal(
    [property: JsonPropertyName("latitude")] double Latitude,
    [property: JsonPropertyName("longitude")] double Longitude,
    [property: JsonPropertyName("timestamp")] DateTimeOffset Timestamp,
    [property: JsonPropertyName("crs")] string Crs,
    [property: JsonPropertyName("heading")] double? Heading = null,
    [property: JsonPropertyName("distance")] double? Distance = null);

public sealed record ProjectedCoords(
    [property: JsonPropertyName("x")] double X,
    [property: JsonPropertyName("y")] double Y);

public sealed record ProjectedPosition(
    [property: JsonPropertyName("netelement_id")] string NetelementId,
    [property: JsonPropertyName("measure_meters")] double MeasureMeters,
    [property: JsonPropertyName("projection_distance_meters")] double ProjectionDistanceMeters,
    [property: JsonPropertyName("projected_coords")] ProjectedCoords ProjectedCoords,
    [property: JsonPropertyName("crs")] string Crs,
    [property: JsonPropertyName("original")] GnssOriginal Original,
    [property: JsonPropertyName("intrinsic")] double? Intrinsic = null)
{
    public double ProjectedX => ProjectedCoords.X;
    public double ProjectedY => ProjectedCoords.Y;
    public double OriginalLatitude => Original.Latitude;
    public double OriginalLongitude => Original.Longitude;
    public DateTimeOffset Timestamp => Original.Timestamp;
}

public sealed record AssociatedNetElement(
    [property: JsonPropertyName("netelement_id")] string NetelementId,
    [property: JsonPropertyName("probability")] double Probability,
    [property: JsonPropertyName("start_intrinsic")] double StartIntrinsic,
    [property: JsonPropertyName("end_intrinsic")] double EndIntrinsic,
    [property: JsonPropertyName("gnss_start_index")] int GnssStartIndex,
    [property: JsonPropertyName("gnss_end_index")] int GnssEndIndex,
    [property: JsonPropertyName("origin")] PathOrigin Origin = PathOrigin.Algorithm);

public sealed record TrainPath(
    [property: JsonPropertyName("segments")] IReadOnlyList<AssociatedNetElement> Segments,
    [property: JsonPropertyName("overall_probability")] double OverallProbability,
    [property: JsonPropertyName("calculated_at")] DateTimeOffset? CalculatedAt = null);

public sealed record PathResult(
    [property: JsonPropertyName("path")] TrainPath? Path,
    [property: JsonPropertyName("mode")] PathCalculationMode Mode,
    [property: JsonPropertyName("projected_positions")] IReadOnlyList<ProjectedPosition> ProjectedPositions,
    [property: JsonPropertyName("warnings")] IReadOnlyList<string> Warnings,
    [property: JsonPropertyName("detection_provenance")] IReadOnlyList<DetectionRecord> DetectionProvenance)
{
    public bool HasPath => Path is not null;
}

// ---------------------------------------------------------------------------
// Detection records (Rust models::detection_record).
// ---------------------------------------------------------------------------

public abstract record DetectionTimestamp
{
    public sealed record Single(
        [property: JsonPropertyName("timestamp")] DateTimeOffset Timestamp) : DetectionTimestamp;

    public sealed record Range(
        [property: JsonPropertyName("t_from")] DateTimeOffset From,
        [property: JsonPropertyName("t_to")] DateTimeOffset To) : DetectionTimestamp;
}

internal sealed class DetectionTimestampJsonConverter : JsonConverter<DetectionTimestamp>
{
    public override DetectionTimestamp Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var doc = JsonDocument.ParseValue(ref reader);
        var root = doc.RootElement;
        if (root.TryGetProperty("timestamp", out var ts))
        {
            return new DetectionTimestamp.Single(ts.GetDateTimeOffset());
        }
        if (root.TryGetProperty("t_from", out var from) && root.TryGetProperty("t_to", out var to))
        {
            return new DetectionTimestamp.Range(from.GetDateTimeOffset(), to.GetDateTimeOffset());
        }
        throw new JsonException("Unrecognized DetectionTimestamp shape");
    }

    public override void Write(Utf8JsonWriter writer, DetectionTimestamp value, JsonSerializerOptions options)
    {
        switch (value)
        {
            case DetectionTimestamp.Single s:
                writer.WriteStartObject();
                writer.WriteString("timestamp", s.Timestamp);
                writer.WriteEndObject();
                break;
            case DetectionTimestamp.Range r:
                writer.WriteStartObject();
                writer.WriteString("t_from", r.From);
                writer.WriteString("t_to", r.To);
                writer.WriteEndObject();
                break;
            default:
                throw new JsonException();
        }
    }
}

/// <summary>
/// Disposition of an ingested detection (output, populated by <see cref="DetectionPreparation.PrepareDetections(NetworkInput, GnssInput, IEnumerable{DetectionRecord}, double)"/>).
/// Mirrors Rust's <c>DetectionStatus</c> tagged enum (<c>tag = "status"</c>).
/// </summary>
public abstract record DetectionStatus
{
    /// <summary>Detection was applied as a Viterbi anchor.</summary>
    public sealed record Applied(
        [property: JsonPropertyName("netelement_id")] string NetelementId,
        [property: JsonPropertyName("intrinsic")] double Intrinsic) : DetectionStatus;

    /// <summary>Coordinate-only detection successfully resolved within the cutoff.</summary>
    public sealed record Resolved(
        [property: JsonPropertyName("netelement_id")] string NetelementId,
        [property: JsonPropertyName("distance_m")] double DistanceMeters) : DetectionStatus;

    /// <summary>Detection was discarded; see <see cref="Reason"/>.</summary>
    public sealed record Discarded(
        [property: JsonPropertyName("reason")] DiscardReason Reason) : DetectionStatus;
}

/// <summary>
/// Reason a detection was discarded. Mirrors Rust's <c>DiscardReason</c> tagged enum (<c>tag = "kind"</c>).
/// </summary>
public abstract record DiscardReason
{
    public sealed record OutOfTimeRange(
        [property: JsonPropertyName("gnss_first")] DateTimeOffset GnssFirst,
        [property: JsonPropertyName("gnss_last")] DateTimeOffset GnssLast) : DiscardReason;

    public sealed record OutOfReach(
        [property: JsonPropertyName("nearest_distance_m")] double NearestDistanceMeters,
        [property: JsonPropertyName("cutoff_m")] double CutoffMeters) : DiscardReason;

    public sealed record UnknownNetelement(
        [property: JsonPropertyName("netelement_id")] string NetelementId) : DiscardReason;

    public sealed record IntrinsicOutOfRange(
        [property: JsonPropertyName("value")] double Value) : DiscardReason;

    public sealed record DuplicateOfPriorDetection(
        [property: JsonPropertyName("kept_index")] int KeptIndex) : DiscardReason;
}

internal sealed class DetectionStatusJsonConverter : JsonConverter<DetectionStatus>
{
    public override DetectionStatus Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var doc = JsonDocument.ParseValue(ref reader);
        var root = doc.RootElement;
        var tag = root.GetProperty("status").GetString();
        return tag switch
        {
            "applied" => new DetectionStatus.Applied(
                root.GetProperty("netelement_id").GetString()!,
                root.GetProperty("intrinsic").GetDouble()),
            "resolved" => new DetectionStatus.Resolved(
                root.GetProperty("netelement_id").GetString()!,
                root.GetProperty("distance_m").GetDouble()),
            "discarded" => new DetectionStatus.Discarded(
                JsonSerializer.Deserialize<DiscardReason>(root.GetProperty("reason").GetRawText(), options)!),
            _ => throw new JsonException($"Unknown DetectionStatus tag '{tag}'"),
        };
    }

    public override void Write(Utf8JsonWriter writer, DetectionStatus value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        switch (value)
        {
            case DetectionStatus.Applied a:
                writer.WriteString("status", "applied");
                writer.WriteString("netelement_id", a.NetelementId);
                writer.WriteNumber("intrinsic", a.Intrinsic);
                break;
            case DetectionStatus.Resolved r:
                writer.WriteString("status", "resolved");
                writer.WriteString("netelement_id", r.NetelementId);
                writer.WriteNumber("distance_m", r.DistanceMeters);
                break;
            case DetectionStatus.Discarded d:
                writer.WriteString("status", "discarded");
                writer.WritePropertyName("reason");
                JsonSerializer.Serialize(writer, d.Reason, options);
                break;
            default:
                throw new JsonException($"Unsupported DetectionStatus variant: {value.GetType()}");
        }
        writer.WriteEndObject();
    }
}

internal sealed class DiscardReasonJsonConverter : JsonConverter<DiscardReason>
{
    public override DiscardReason Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var doc = JsonDocument.ParseValue(ref reader);
        var root = doc.RootElement;
        var tag = root.GetProperty("kind").GetString();
        return tag switch
        {
            "out_of_time_range" => new DiscardReason.OutOfTimeRange(
                root.GetProperty("gnss_first").GetDateTimeOffset(),
                root.GetProperty("gnss_last").GetDateTimeOffset()),
            "out_of_reach" => new DiscardReason.OutOfReach(
                root.GetProperty("nearest_distance_m").GetDouble(),
                root.GetProperty("cutoff_m").GetDouble()),
            "unknown_netelement" => new DiscardReason.UnknownNetelement(
                root.GetProperty("netelement_id").GetString()!),
            "intrinsic_out_of_range" => new DiscardReason.IntrinsicOutOfRange(
                root.GetProperty("value").GetDouble()),
            "duplicate_of_prior_detection" => new DiscardReason.DuplicateOfPriorDetection(
                root.GetProperty("kept_index").GetInt32()),
            _ => throw new JsonException($"Unknown DiscardReason kind '{tag}'"),
        };
    }

    public override void Write(Utf8JsonWriter writer, DiscardReason value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        switch (value)
        {
            case DiscardReason.OutOfTimeRange r:
                writer.WriteString("kind", "out_of_time_range");
                writer.WriteString("gnss_first", r.GnssFirst);
                writer.WriteString("gnss_last", r.GnssLast);
                break;
            case DiscardReason.OutOfReach r:
                writer.WriteString("kind", "out_of_reach");
                writer.WriteNumber("nearest_distance_m", r.NearestDistanceMeters);
                writer.WriteNumber("cutoff_m", r.CutoffMeters);
                break;
            case DiscardReason.UnknownNetelement r:
                writer.WriteString("kind", "unknown_netelement");
                writer.WriteString("netelement_id", r.NetelementId);
                break;
            case DiscardReason.IntrinsicOutOfRange r:
                writer.WriteString("kind", "intrinsic_out_of_range");
                writer.WriteNumber("value", r.Value);
                break;
            case DiscardReason.DuplicateOfPriorDetection r:
                writer.WriteString("kind", "duplicate_of_prior_detection");
                writer.WriteNumber("kept_index", r.KeptIndex);
                break;
            default:
                throw new JsonException($"Unsupported DiscardReason variant: {value.GetType()}");
        }
        writer.WriteEndObject();
    }
}

/// <summary>
/// A single detection event passed to <see cref="DetectionPreparation.PrepareDetections(NetworkInput, GnssInput, IEnumerable{DetectionRecord}, double)"/>,
/// and also returned in <see cref="PreparedDetections.Records"/> /
/// <see cref="PathResult.DetectionProvenance"/> with <see cref="Status"/> populated.
///
/// On input, supply either topological position (<see cref="NetelementId"/> [+ <see cref="Intrinsic"/>])
/// or geographic position (<see cref="Latitude"/>, <see cref="Longitude"/>) — never both.
/// <see cref="Status"/> is ignored on input and populated on output.
/// </summary>
public sealed record DetectionRecord(
    [property: JsonPropertyName("source_file")] string SourceFile,
    [property: JsonPropertyName("source_row")] ulong SourceRow,
    [property: JsonPropertyName("kind")] DetectionKind Kind,
    [property: JsonPropertyName("timestamp")] DetectionTimestamp Timestamp,
    [property: JsonPropertyName("id")] string? Id = null,
    [property: JsonPropertyName("source")] string? Source = null,
    [property: JsonPropertyName("metadata")] IReadOnlyDictionary<string, string>? Metadata = null,
    [property: JsonPropertyName("status")] DetectionStatus? Status = null,
    [property: JsonPropertyName("netelement_id")] string? NetelementId = null,
    [property: JsonPropertyName("intrinsic")] double? Intrinsic = null,
    [property: JsonPropertyName("start_intrinsic")] double? StartIntrinsic = null,
    [property: JsonPropertyName("end_intrinsic")] double? EndIntrinsic = null,
    [property: JsonPropertyName("latitude")] double? Latitude = null,
    [property: JsonPropertyName("longitude")] double? Longitude = null,
    [property: JsonPropertyName("crs")] string? Crs = null);

public sealed record PreparedDetections(
    [property: JsonPropertyName("records")] IReadOnlyList<DetectionRecord> Records,
    [property: JsonPropertyName("warnings")] IReadOnlyList<string> Warnings,
    [property: JsonPropertyName("anchors")] JsonElement Anchors);

// ---------------------------------------------------------------------------
// Input record types (consumer-facing).
// ---------------------------------------------------------------------------

public sealed record NetworkSegment(
    string Id,
    IReadOnlyList<(double Longitude, double Latitude)> Coordinates,
    string Crs = "EPSG:4326");

public sealed record NetworkRelation(
    string Id,
    string NetelementAId,
    string NetelementBId,
    int PositionOnA,
    int PositionOnB,
    Navigability Navigability);

public sealed record GnssRecord(
    double Latitude,
    double Longitude,
    DateTimeOffset Timestamp);

// ---------------------------------------------------------------------------
// Input wrappers — carry raw JSON/CSV ready for the Rust core.
// ---------------------------------------------------------------------------

public sealed class NetworkInput
{
    private readonly string _json;
    private NetworkInput(string json) { _json = json; }

    public static NetworkInput FromGeoJson(string geoJson)
    {
        ArgumentException.ThrowIfNullOrEmpty(geoJson);
        return new NetworkInput(geoJson);
    }

    public static NetworkInput FromRecords(
        IEnumerable<NetworkSegment> segments,
        IEnumerable<NetworkRelation> relations)
    {
        ArgumentNullException.ThrowIfNull(segments);
        ArgumentNullException.ThrowIfNull(relations);

        using var stream = new MemoryStream();
        using var w = new Utf8JsonWriter(stream);
        w.WriteStartObject();
        w.WriteString("type", "FeatureCollection");
        w.WriteStartArray("features");

        foreach (var seg in segments)
        {
            w.WriteStartObject();
            w.WriteString("type", "Feature");
            w.WriteStartObject("properties");
            w.WriteString("id", seg.Id);
            w.WriteString("type", "netelement");
            w.WriteString("crs", seg.Crs);
            w.WriteEndObject();
            w.WriteStartObject("geometry");
            w.WriteString("type", "LineString");
            w.WriteStartArray("coordinates");
            foreach (var (lon, lat) in seg.Coordinates)
            {
                w.WriteStartArray();
                w.WriteNumberValue(lon);
                w.WriteNumberValue(lat);
                w.WriteEndArray();
            }
            w.WriteEndArray();
            w.WriteEndObject();
            w.WriteEndObject();
        }

        foreach (var rel in relations)
        {
            w.WriteStartObject();
            w.WriteString("type", "Feature");
            w.WriteStartObject("properties");
            w.WriteString("id", rel.Id);
            w.WriteString("type", "netrelation");
            w.WriteString("netelementA", rel.NetelementAId);
            w.WriteString("netelementB", rel.NetelementBId);
            w.WriteNumber("positionOnA", rel.PositionOnA);
            w.WriteNumber("positionOnB", rel.PositionOnB);
            w.WriteString("navigability", rel.Navigability switch
            {
                Navigability.Both => "both",
                Navigability.Forward => "AB",
                Navigability.Backward => "BA",
                Navigability.None => "none",
                _ => "both",
            });
            w.WriteEndObject();
            w.WriteNull("geometry");
            w.WriteEndObject();
        }

        w.WriteEndArray();
        w.WriteEndObject();
        w.Flush();
        return new NetworkInput(Encoding.UTF8.GetString(stream.ToArray()));
    }

    internal string AsJson() => _json;
}

public sealed class GnssInput
{
    private readonly string _payload;
    private readonly bool _isCsv;

    private GnssInput(string payload, bool isCsv) { _payload = payload; _isCsv = isCsv; }

    public static GnssInput FromGeoJson(string geoJson)
    {
        ArgumentException.ThrowIfNullOrEmpty(geoJson);
        return new GnssInput(geoJson, isCsv: false);
    }

    public static GnssInput FromCsv(string csv)
    {
        ArgumentException.ThrowIfNullOrEmpty(csv);
        return new GnssInput(csv, isCsv: true);
    }

    public static GnssInput FromRecords(IEnumerable<GnssRecord> records)
    {
        ArgumentNullException.ThrowIfNull(records);

        using var stream = new MemoryStream();
        using var w = new Utf8JsonWriter(stream);
        w.WriteStartObject();
        w.WriteString("type", "FeatureCollection");
        w.WriteStartArray("features");
        foreach (var r in records)
        {
            w.WriteStartObject();
            w.WriteString("type", "Feature");
            w.WriteStartObject("properties");
            w.WriteNumber("latitude", r.Latitude);
            w.WriteNumber("longitude", r.Longitude);
            w.WriteString("timestamp", r.Timestamp.ToString("o", CultureInfo.InvariantCulture));
            w.WriteEndObject();
            w.WriteStartObject("geometry");
            w.WriteString("type", "Point");
            w.WriteStartArray("coordinates");
            w.WriteNumberValue(r.Longitude);
            w.WriteNumberValue(r.Latitude);
            w.WriteEndArray();
            w.WriteEndObject();
            w.WriteEndObject();
        }
        w.WriteEndArray();
        w.WriteEndObject();
        w.Flush();
        return new GnssInput(Encoding.UTF8.GetString(stream.ToArray()), isCsv: false);
    }

    internal string AsJson() => _payload;
    internal bool IsCsv => _isCsv;
}

// ---------------------------------------------------------------------------
// Shared JSON options used by the C# wrappers when (de)serializing FFI payloads.
// ---------------------------------------------------------------------------

internal static class TpLibJson
{
    internal static readonly JsonSerializerOptions Options = BuildOptions();

    private static JsonSerializerOptions BuildOptions()
    {
        var o = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
            PropertyNameCaseInsensitive = false,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        };
        o.Converters.Add(new JsonStringEnumConverter(JsonNamingPolicy.SnakeCaseLower));
        o.Converters.Add(new DetectionTimestampJsonConverter());
        o.Converters.Add(new DetectionStatusJsonConverter());
        o.Converters.Add(new DiscardReasonJsonConverter());
        return o;
    }
}
