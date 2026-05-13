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
    public static PreparedDetections PrepareDetections(
        NetworkInput network,
        GnssInput gnss,
        string detectionsGeoJson,
        DetectionKind kind,
        double cutoffDistanceMeters)
    {
        ArgumentNullException.ThrowIfNull(network);
        ArgumentNullException.ThrowIfNull(gnss);
        ArgumentException.ThrowIfNullOrEmpty(detectionsGeoJson);
        TpLibNative.EnsureInitialized();

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

    public static PreparedDetections PrepareDetections(
        string networkGeoJson,
        GnssInput gnss,
        string detectionsGeoJson,
        DetectionKind kind,
        double cutoffDistanceMeters = 2.5)
        => PrepareDetections(NetworkInput.FromGeoJson(networkGeoJson), gnss, detectionsGeoJson, kind, cutoffDistanceMeters);
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
