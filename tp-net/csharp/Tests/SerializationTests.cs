using System.Text.Json;
using TpLib;
using Xunit;

namespace TpLib.Tests;

public class SerializationTests
{
    [Fact]
    public void Enums_SerializeAs_SnakeCaseLower()
    {
        var json = JsonSerializer.Serialize(PathCalculationMode.TopologyBased, TpLibJson.Options);
        Assert.Equal("\"topology_based\"", json);

        var nav = JsonSerializer.Serialize(Navigability.Forward, TpLibJson.Options);
        Assert.Equal("\"forward\"", nav);

        var kind = JsonSerializer.Serialize(DetectionKind.Linear, TpLibJson.Options);
        Assert.Equal("\"linear\"", kind);
    }

    [Fact]
    public void PathConfig_SerializesUsing_SnakeCase()
    {
        var cfg = new PathConfig { CutoffDistanceMeters = 600.0, PathOnly = true };
        var json = JsonSerializer.Serialize(cfg, TpLibJson.Options);

        Assert.Contains("cutoff_distance_meters", json);
        Assert.Contains("path_only", json);
        Assert.DoesNotContain("CutoffDistanceMeters", json);
    }

    [Fact]
    public void DetectionTimestamp_Single_RoundTrips()
    {
        var ts = new System.DateTimeOffset(2024, 6, 1, 12, 0, 0, System.TimeSpan.Zero);
        DetectionTimestamp value = new DetectionTimestamp.Single(ts);
        var json = JsonSerializer.Serialize(value, TpLibJson.Options);
        var back = JsonSerializer.Deserialize<DetectionTimestamp>(json, TpLibJson.Options);

        var single = Assert.IsType<DetectionTimestamp.Single>(back);
        Assert.Equal(ts, single.Timestamp);
    }

    [Fact]
    public void DetectionTimestamp_Range_RoundTrips()
    {
        var from = new System.DateTimeOffset(2024, 6, 1, 12, 0, 0, System.TimeSpan.Zero);
        var to = new System.DateTimeOffset(2024, 6, 1, 12, 0, 30, System.TimeSpan.Zero);
        DetectionTimestamp value = new DetectionTimestamp.Range(from, to);
        var json = JsonSerializer.Serialize(value, TpLibJson.Options);
        var back = JsonSerializer.Deserialize<DetectionTimestamp>(json, TpLibJson.Options);

        var range = Assert.IsType<DetectionTimestamp.Range>(back);
        Assert.Equal(from, range.From);
        Assert.Equal(to, range.To);
    }
}
