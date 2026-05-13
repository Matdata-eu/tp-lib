using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using TpLib;
using Xunit;

namespace TpLib.Tests;

public class DetectionPreparationTests
{
    private static (NetworkInput network, GnssInput gnss) LoadFixtures()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));
        return (network, gnss);
    }

    [Fact]
    public void PrepareDetections_Punctual_TypedRecords_ReturnsRecords()
    {
        var (network, gnss) = LoadFixtures();

        var detections = new List<DetectionRecord>
        {
            new(
                SourceFile: "test",
                SourceRow: 0,
                Kind: DetectionKind.Punctual,
                Timestamp: new DetectionTimestamp.Single(
                    DateTimeOffset.Parse("2024-01-15T10:30:05+01:00")),
                Id: "D001",
                NetelementId: "NE001"),
        };

        var prepared = DetectionPreparation.PrepareDetections(network, gnss, detections);

        Assert.NotNull(prepared);
        Assert.NotNull(prepared.Records);
        Assert.Single(prepared.Records);
    }

    [Fact]
    public void PrepareDetections_Linear_TypedRecords_ReturnsRecords()
    {
        var (network, gnss) = LoadFixtures();

        var detections = new List<DetectionRecord>
        {
            new(
                SourceFile: "test",
                SourceRow: 0,
                Kind: DetectionKind.Linear,
                Timestamp: new DetectionTimestamp.Range(
                    From: DateTimeOffset.Parse("2024-01-15T10:30:00+01:00"),
                    To: DateTimeOffset.Parse("2024-01-15T10:30:10+01:00")),
                Id: "L001",
                NetelementId: "NE001",
                StartIntrinsic: 0.0,
                EndIntrinsic: 1.0),
        };

        var prepared = DetectionPreparation.PrepareDetections(network, gnss, detections);

        Assert.NotNull(prepared);
        Assert.Single(prepared.Records);
    }

    [Fact]
    public void PrepareDetections_MixedKinds_AreCombined()
    {
        var (network, gnss) = LoadFixtures();

        var detections = new List<DetectionRecord>
        {
            new(
                SourceFile: "test",
                SourceRow: 0,
                Kind: DetectionKind.Punctual,
                Timestamp: new DetectionTimestamp.Single(
                    DateTimeOffset.Parse("2024-01-15T10:30:02+01:00")),
                NetelementId: "NE001"),
            new(
                SourceFile: "test",
                SourceRow: 1,
                Kind: DetectionKind.Linear,
                Timestamp: new DetectionTimestamp.Range(
                    From: DateTimeOffset.Parse("2024-01-15T10:30:04+01:00"),
                    To: DateTimeOffset.Parse("2024-01-15T10:30:08+01:00")),
                NetelementId: "NE002",
                StartIntrinsic: 0.0,
                EndIntrinsic: 1.0),
        };

        var prepared = DetectionPreparation.PrepareDetections(network, gnss, detections);

        Assert.Equal(2, prepared.Records.Count);
    }

    [Fact]
    public void PrepareDetections_EmptySequence_ReturnsEmpty()
    {
        var (network, gnss) = LoadFixtures();

        var prepared = DetectionPreparation.PrepareDetections(
            network, gnss, Enumerable.Empty<DetectionRecord>());

        Assert.NotNull(prepared);
        Assert.Empty(prepared.Records);
        Assert.Empty(prepared.Warnings);
    }

    [Fact]
    public void PrepareDetections_NullDetections_Throws()
    {
        var (network, gnss) = LoadFixtures();

        Assert.Throws<ArgumentNullException>(
            () => DetectionPreparation.PrepareDetections(
                network, gnss, (IEnumerable<DetectionRecord>)null!));
    }

    [Fact]
    public void DetectionStatus_Applied_RoundTrips()
    {
        var options = TpLibJson.Options;
        DetectionStatus status = new DetectionStatus.Applied("ne-1", 0.5);
        var json = JsonSerializer.Serialize(status, options);
        var roundTripped = JsonSerializer.Deserialize<DetectionStatus>(json, options);
        var applied = Assert.IsType<DetectionStatus.Applied>(roundTripped);
        Assert.Equal("ne-1", applied.NetelementId);
        Assert.Equal(0.5, applied.Intrinsic);
    }

    [Fact]
    public void DetectionStatus_Resolved_RoundTrips()
    {
        var options = TpLibJson.Options;
        DetectionStatus status = new DetectionStatus.Resolved("ne-2", 1.25);
        var json = JsonSerializer.Serialize(status, options);
        var roundTripped = JsonSerializer.Deserialize<DetectionStatus>(json, options);
        var resolved = Assert.IsType<DetectionStatus.Resolved>(roundTripped);
        Assert.Equal("ne-2", resolved.NetelementId);
        Assert.Equal(1.25, resolved.DistanceMeters);
    }

    [Fact]
    public void DetectionStatus_Discarded_OutOfTimeRange_RoundTrips()
    {
        var options = TpLibJson.Options;
        var first = DateTimeOffset.Parse("2026-03-13T17:00:00+01:00");
        var last = DateTimeOffset.Parse("2026-03-13T18:00:00+01:00");
        DetectionStatus status = new DetectionStatus.Discarded(
            new DiscardReason.OutOfTimeRange(first, last));
        var json = JsonSerializer.Serialize(status, options);
        var roundTripped = JsonSerializer.Deserialize<DetectionStatus>(json, options);
        var discarded = Assert.IsType<DetectionStatus.Discarded>(roundTripped);
        var reason = Assert.IsType<DiscardReason.OutOfTimeRange>(discarded.Reason);
        Assert.Equal(first, reason.GnssFirst);
        Assert.Equal(last, reason.GnssLast);
    }

    [Fact]
    public void DetectionStatus_Discarded_OutOfReach_RoundTrips()
    {
        var options = TpLibJson.Options;
        DetectionStatus status = new DetectionStatus.Discarded(
            new DiscardReason.OutOfReach(12.5, 2.5));
        var json = JsonSerializer.Serialize(status, options);
        var roundTripped = JsonSerializer.Deserialize<DetectionStatus>(json, options);
        var discarded = Assert.IsType<DetectionStatus.Discarded>(roundTripped);
        var reason = Assert.IsType<DiscardReason.OutOfReach>(discarded.Reason);
        Assert.Equal(12.5, reason.NearestDistanceMeters);
        Assert.Equal(2.5, reason.CutoffMeters);
    }
}
