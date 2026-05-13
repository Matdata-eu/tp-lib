using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace TpLib;

/// <summary>
/// Project GNSS positions onto a railway network or a previously computed path.
/// </summary>
public static class Projection
{
    public static IReadOnlyList<ProjectedPosition> ProjectGnss(
        NetworkInput network,
        GnssInput gnss,
        ProjectionConfig? config = null)
    {
        ArgumentNullException.ThrowIfNull(network);
        ArgumentNullException.ThrowIfNull(gnss);
        config ??= new ProjectionConfig();
        TpLibNative.EnsureInitialized();
        var netBytes = Encoding.UTF8.GetBytes(network.AsJson());
        var gnssBytes = Encoding.UTF8.GetBytes(gnss.AsJson());

        unsafe
        {
            fixed (byte* netPtr = netBytes)
            fixed (byte* gnssPtr = gnssBytes)
            {
                var cfg = new ProjectionConfigFfi
                {
                    max_search_radius_meters = config.MaxSearchRadiusMeters,
                    projection_distance_warning_threshold = config.ProjectionDistanceWarningThreshold,
                    suppress_warnings = (byte)(config.SuppressWarnings ? 1 : 0),
                };
                var buf = NativeMethods.tp_net_project_gnss(netPtr, netBytes.Length, gnssPtr, gnssBytes.Length, cfg);
                return TpLibFfi.DeserializeOrThrow<List<ProjectedPosition>>(
                    buf,
                    "ProjectGnss failed (FFI returned an error sentinel).",
                    ex => ex is null
                        ? new TpLibProjectionException("ProjectGnss failed (FFI returned an error sentinel).")
                        : new TpLibProjectionException("ProjectGnss failed (FFI returned an error sentinel).", ex));
            }
        }
    }

    public static IReadOnlyList<ProjectedPosition> ProjectOntoPath(
        NetworkInput network,
        GnssInput gnss,
        TrainPath path,
        PathConfig? config = null)
    {
        ArgumentNullException.ThrowIfNull(network);
        ArgumentNullException.ThrowIfNull(gnss);
        ArgumentNullException.ThrowIfNull(path);
        config ??= new PathConfig();
        TpLibNative.EnsureInitialized();

        var netBytes = Encoding.UTF8.GetBytes(network.AsJson());
        var gnssBytes = Encoding.UTF8.GetBytes(gnss.AsJson());
        var pathBytes = JsonSerializer.SerializeToUtf8Bytes(path, TpLibJson.Options);

        unsafe
        {
            fixed (byte* netPtr = netBytes)
            fixed (byte* gnssPtr = gnssBytes)
            fixed (byte* pathPtr = pathBytes)
            {
                var cfg = TpLibFfi.ToFfi(config);
                var buf = NativeMethods.tp_net_project_onto_path(
                    netPtr, netBytes.Length,
                    gnssPtr, gnssBytes.Length,
                    pathPtr, pathBytes.Length,
                    cfg);
                return TpLibFfi.DeserializeOrThrow<List<ProjectedPosition>>(
                    buf,
                    "ProjectOntoPath failed (FFI returned an error sentinel).",
                    ex => ex is null
                        ? new TpLibProjectionException("ProjectOntoPath failed (FFI returned an error sentinel).")
                        : new TpLibProjectionException("ProjectOntoPath failed (FFI returned an error sentinel).", ex));
            }
        }
    }

    public static IReadOnlyList<ProjectedPosition> ProjectGnss(
        string networkGeoJson, GnssInput gnss, ProjectionConfig? config = null)
        => ProjectGnss(NetworkInput.FromGeoJson(networkGeoJson), gnss, config);

    public static IReadOnlyList<ProjectedPosition> ProjectGnss(
        string networkGeoJson, string gnssGeoJson, ProjectionConfig? config = null)
        => ProjectGnss(NetworkInput.FromGeoJson(networkGeoJson), GnssInput.FromGeoJson(gnssGeoJson), config);

    public static IReadOnlyList<ProjectedPosition> ProjectOntoPath(
        string networkGeoJson, GnssInput gnss, TrainPath path, PathConfig? config = null)
        => ProjectOntoPath(NetworkInput.FromGeoJson(networkGeoJson), gnss, path, config);

    public static IReadOnlyList<ProjectedPosition> ProjectOntoPath(
        string networkGeoJson, string gnssGeoJson, TrainPath path, PathConfig? config = null)
        => ProjectOntoPath(NetworkInput.FromGeoJson(networkGeoJson), GnssInput.FromGeoJson(gnssGeoJson), path, config);
}

/// <summary>
/// Train path calculation entry points.
/// </summary>
public static class PathCalculation
{
    public static PathResult CalculateTrainPath(
        NetworkInput network,
        GnssInput gnss,
        PathConfig? config = null,
        PreparedDetections? detections = null)
    {
        ArgumentNullException.ThrowIfNull(network);
        ArgumentNullException.ThrowIfNull(gnss);
        config ??= new PathConfig();
        TpLibNative.EnsureInitialized();

        var netBytes = Encoding.UTF8.GetBytes(network.AsJson());
        var gnssBytes = Encoding.UTF8.GetBytes(gnss.AsJson());
        var detBytes = detections is null
            ? Array.Empty<byte>()
            : JsonSerializer.SerializeToUtf8Bytes(detections, TpLibJson.Options);

        unsafe
        {
            fixed (byte* netPtr = netBytes)
            fixed (byte* gnssPtr = gnssBytes)
            fixed (byte* detPtr = detBytes)
            {
                var cfg = TpLibFfi.ToFfi(config);
                byte* detArg = detBytes.Length == 0 ? null : detPtr;
                var buf = NativeMethods.tp_net_calculate_train_path(
                    netPtr, netBytes.Length,
                    gnssPtr, gnssBytes.Length,
                    detArg, detBytes.Length,
                    cfg);
                return TpLibFfi.DeserializeOrThrow<PathResult>(
                    buf,
                    "CalculateTrainPath failed (FFI returned an error sentinel).",
                    ex => ex is null
                        ? new TpLibPathException("CalculateTrainPath failed (FFI returned an error sentinel).")
                        : new TpLibPathException("CalculateTrainPath failed (FFI returned an error sentinel).", ex));
            }
        }
    }

    public static PathResult CalculateTrainPath(
        string networkGeoJson, GnssInput gnss, PathConfig? config = null, PreparedDetections? detections = null)
        => CalculateTrainPath(NetworkInput.FromGeoJson(networkGeoJson), gnss, config, detections);

    public static PathResult CalculateTrainPath(
        string networkGeoJson, string gnssGeoJson, PathConfig? config = null, PreparedDetections? detections = null)
        => CalculateTrainPath(NetworkInput.FromGeoJson(networkGeoJson), GnssInput.FromGeoJson(gnssGeoJson), config, detections);
}

/// <summary>
/// Detection preparation: validates and resolves user-supplied detections into anchors.
/// </summary>
public static class DetectionPreparation
{
    /// <summary>
    /// Validate, time-filter and resolve detections against the network.
    /// </summary>
    /// <param name="network">Network input (GeoJSON or constructed from records).</param>
    /// <param name="gnss">GNSS input used to derive the time window.</param>
    /// <param name="detections">Detection events. Each record's <see cref="DetectionRecord.Kind"/> selects the
    /// punctual/linear schema and must be paired with the required positional fields (see
    /// <see cref="DetectionRecord"/>).</param>
    /// <param name="cutoffDistanceMeters">Max projection distance for coordinate-only punctual detections.</param>
    public static PreparedDetections PrepareDetections(
        NetworkInput network,
        GnssInput gnss,
        IEnumerable<DetectionRecord> detections,
        double cutoffDistanceMeters = 2.5)
    {
        ArgumentNullException.ThrowIfNull(network);
        ArgumentNullException.ThrowIfNull(gnss);
        ArgumentNullException.ThrowIfNull(detections);
        TpLibNative.EnsureInitialized();

        var records = detections as IReadOnlyList<DetectionRecord> ?? detections.ToList();

        var allRecords = new List<DetectionRecord>();
        var allWarnings = new List<string>();
        using var anchorsStream = new MemoryStream();
        using (var aw = new Utf8JsonWriter(anchorsStream))
        {
            aw.WriteStartArray();
            foreach (var kind in new[] { DetectionKind.Punctual, DetectionKind.Linear })
            {
                var subset = records.Where(r => r.Kind == kind).ToList();
                if (subset.Count == 0)
                {
                    continue;
                }
                var partial = PrepareDetectionsForKind(network, gnss, subset, kind, cutoffDistanceMeters);
                allRecords.AddRange(partial.Records);
                allWarnings.AddRange(partial.Warnings);
                if (partial.Anchors.ValueKind == JsonValueKind.Array)
                {
                    foreach (var anchor in partial.Anchors.EnumerateArray())
                    {
                        anchor.WriteTo(aw);
                    }
                }
            }
            aw.WriteEndArray();
            aw.Flush();
        }

        using var anchorsDoc = JsonDocument.Parse(anchorsStream.ToArray());
        return new PreparedDetections(allRecords, allWarnings, anchorsDoc.RootElement.Clone());
    }

    /// <summary>Convenience overload accepting the raw network GeoJSON string.</summary>
    public static PreparedDetections PrepareDetections(
        string networkGeoJson,
        GnssInput gnss,
        IEnumerable<DetectionRecord> detections,
        double cutoffDistanceMeters = 2.5)
        => PrepareDetections(NetworkInput.FromGeoJson(networkGeoJson), gnss, detections, cutoffDistanceMeters);

    private static PreparedDetections PrepareDetectionsForKind(
        NetworkInput network,
        GnssInput gnss,
        IReadOnlyList<DetectionRecord> records,
        DetectionKind kind,
        double cutoffDistanceMeters)
    {
        var detectionsGeoJson = BuildDetectionsGeoJson(records, kind);

        var netBytes = Encoding.UTF8.GetBytes(network.AsJson());
        var gnssBytes = Encoding.UTF8.GetBytes(gnss.AsJson());
        var detBytes = Encoding.UTF8.GetBytes(detectionsGeoJson);

        unsafe
        {
            fixed (byte* netPtr = netBytes)
            fixed (byte* gnssPtr = gnssBytes)
            fixed (byte* detPtr = detBytes)
            {
                var buf = NativeMethods.tp_net_prepare_detections(
                    netPtr, netBytes.Length,
                    gnssPtr, gnssBytes.Length,
                    detPtr, detBytes.Length,
                    (byte)(kind == DetectionKind.Linear ? 1 : 0),
                    cutoffDistanceMeters);
                return TpLibFfi.DeserializeOrThrow<PreparedDetections>(
                    buf,
                    "PrepareDetections failed (FFI returned an error sentinel).",
                    ex => ex is null
                        ? new TpLibDetectionException("PrepareDetections failed (FFI returned an error sentinel).")
                        : new TpLibDetectionException("PrepareDetections failed (FFI returned an error sentinel).", ex));
            }
        }
    }

    private static string BuildDetectionsGeoJson(IReadOnlyList<DetectionRecord> records, DetectionKind kind)
    {
        using var stream = new MemoryStream();
        using (var w = new Utf8JsonWriter(stream))
        {
            w.WriteStartObject();
            w.WriteString("type", "FeatureCollection");
            w.WriteStartArray("features");
            foreach (var rec in records)
            {
                w.WriteStartObject();
                w.WriteString("type", "Feature");
                w.WriteStartObject("properties");
                w.WriteString("kind", kind == DetectionKind.Linear ? "linear" : "punctual");

                switch (rec.Timestamp)
                {
                    case DetectionTimestamp.Single single when kind == DetectionKind.Punctual:
                        w.WriteString("timestamp", single.Timestamp);
                        break;
                    case DetectionTimestamp.Range range when kind == DetectionKind.Linear:
                        w.WriteString("t_from", range.From);
                        w.WriteString("t_to", range.To);
                        break;
                    default:
                        throw new TpLibDetectionException(
                            $"Detection (row {rec.SourceRow}, kind {kind}) has incompatible Timestamp type.");
                }

                if (rec.Id is not null) w.WriteString("id", rec.Id);
                if (rec.Source is not null) w.WriteString("source", rec.Source);

                if (kind == DetectionKind.Punctual)
                {
                    if (rec.NetelementId is not null)
                    {
                        w.WriteString("netelement_id", rec.NetelementId);
                        if (rec.Intrinsic.HasValue) w.WriteNumber("intrinsic", rec.Intrinsic.Value);
                    }
                    if (rec.Crs is not null) w.WriteString("crs", rec.Crs);
                }
                else
                {
                    if (rec.NetelementId is null)
                    {
                        throw new TpLibDetectionException(
                            $"Linear detection (row {rec.SourceRow}) requires NetelementId.");
                    }
                    w.WriteString("netelement_id", rec.NetelementId);
                    if (rec.StartIntrinsic.HasValue) w.WriteNumber("start_intrinsic", rec.StartIntrinsic.Value);
                    if (rec.EndIntrinsic.HasValue) w.WriteNumber("end_intrinsic", rec.EndIntrinsic.Value);
                }

                if (rec.Metadata is not null)
                {
                    foreach (var (k, v) in rec.Metadata)
                    {
                        // Reserved property names handled above; user metadata uses other keys.
                        w.WriteString(k, v);
                    }
                }
                w.WriteEndObject(); // properties

                if (kind == DetectionKind.Punctual && rec.Latitude.HasValue && rec.Longitude.HasValue)
                {
                    w.WriteStartObject("geometry");
                    w.WriteString("type", "Point");
                    w.WriteStartArray("coordinates");
                    w.WriteNumberValue(rec.Longitude.Value);
                    w.WriteNumberValue(rec.Latitude.Value);
                    w.WriteEndArray();
                    w.WriteEndObject();
                }
                else
                {
                    w.WriteNull("geometry");
                }
                w.WriteEndObject(); // feature
            }
            w.WriteEndArray();
            w.WriteEndObject();
        }
        return Encoding.UTF8.GetString(stream.ToArray());
    }
}

internal static class TpLibFfi
{
    internal static PathConfigFfi ToFfi(PathConfig c) => new()
    {
        distance_scale = c.DistanceScale,
        heading_scale = c.HeadingScale,
        cutoff_distance = c.CutoffDistanceMeters,
        heading_cutoff = c.HeadingCutoffDegrees,
        probability_threshold = c.ProbabilityThreshold,
        resampling_distance = c.ResamplingDistanceMeters ?? 0.0,
        has_resampling_distance = (byte)(c.ResamplingDistanceMeters.HasValue ? 1 : 0),
        max_candidates = (ulong)c.MaxCandidates,
        path_only = (byte)(c.PathOnly ? 1 : 0),
        debug_mode = 0,
        beta = c.Beta,
        edge_zone_distance = c.EdgeZoneDistanceMeters,
        turn_scale = c.TurnScaleDegrees,
        detection_cutoff_distance = c.DetectionCutoffDistanceMeters,
    };

    internal static unsafe T DeserializeOrThrow<T>(
        ByteBuffer buf,
        string errorMessage,
        Func<Exception?, TpLibException> errorFactory)
    {
        try
        {
            if (buf.ptr == null || buf.len < 0)
            {
                throw errorFactory(null);
            }
            if (buf.len == 0)
            {
                throw errorFactory(new InvalidOperationException("FFI returned an empty buffer."));
            }
            var span = new ReadOnlySpan<byte>(buf.ptr, buf.len);
            try
            {
                var value = JsonSerializer.Deserialize<T>(span, TpLibJson.Options);
                if (value is null)
                {
                    throw errorFactory(new InvalidOperationException("FFI payload deserialized to null."));
                }
                return value;
            }
            catch (JsonException jex)
            {
                throw errorFactory(jex);
            }
        }
        finally
        {
            if (buf.ptr != null)
            {
                TpLibNative.FreeByteBuffer(buf);
            }
        }
    }
}
