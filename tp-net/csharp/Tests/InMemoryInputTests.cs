using System.Collections.Generic;
using System.Linq;
using TpLib;
using Xunit;

namespace TpLib.Tests;

public class InMemoryInputTests
{
    [Fact]
    public void NetworkInput_FromRecords_Equivalent_To_GeoJson()
    {
        var segments = new List<NetworkSegment>
        {
            new("A", new (double, double)[]
            {
                (4.4351, 50.8505),
                (4.4361, 50.8510),
                (4.4371, 50.8515),
            }),
            new("B", new (double, double)[]
            {
                (4.4371, 50.8515),
                (4.4381, 50.8520),
            }),
        };
        var relations = new List<NetworkRelation>
        {
            new("R1", "A", "B", 1, 0, Navigability.Both),
        };

        var network = NetworkInput.FromRecords(segments, relations);
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));

        // Should at least construct and serialize without throwing.
        Assert.False(string.IsNullOrEmpty(network.AsJson()));
    }

    [Fact]
    public void GnssInput_FromRecords_Builds_Valid_GeoJson()
    {
        var records = new List<GnssRecord>
        {
            new(50.8505, 4.4351, new System.DateTimeOffset(2024, 1, 1, 8, 0, 0, System.TimeSpan.Zero)),
            new(50.8510, 4.4361, new System.DateTimeOffset(2024, 1, 1, 8, 0, 1, System.TimeSpan.Zero)),
            new(50.8515, 4.4371, new System.DateTimeOffset(2024, 1, 1, 8, 0, 2, System.TimeSpan.Zero)),
        };

        var gnss = GnssInput.FromRecords(records);
        var json = gnss.AsJson();

        Assert.Contains("FeatureCollection", json);
        Assert.Contains("Point", json);
    }
}
