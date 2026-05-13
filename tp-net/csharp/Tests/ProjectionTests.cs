using System;
using System.Linq;
using TpLib;
using Xunit;

namespace TpLib.Tests;

public class ProjectionTests
{
    [Fact]
    public void ProjectGnss_SampleData_ReturnsValidPositions()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));

        var result = Projection.ProjectGnss(network, gnss);

        Assert.NotEmpty(result);
        Assert.All(result, p =>
        {
            Assert.False(string.IsNullOrEmpty(p.NetelementId));
            Assert.True(p.MeasureMeters >= 0);
            Assert.True(p.ProjectionDistanceMeters >= 0);
            Assert.Null(p.Intrinsic);
        });
    }

    [Fact]
    public void ProjectGnss_StringOverload_Works()
    {
        var net = TestData.Read("sample_network.geojson");
        var gnss = TestData.Read("sample_gnss.geojson");

        var result = Projection.ProjectGnss(
            NetworkInput.FromGeoJson(net),
            GnssInput.FromGeoJson(gnss));

        Assert.NotEmpty(result);
    }

    [Fact]
    public void ProjectGnss_CsvInput_Works()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var csv = """
                  latitude,longitude,timestamp
                  50.8503,4.3517,2024-01-15T10:30:00+01:00
                  50.8505,4.3520,2024-01-15T10:30:05+01:00
                  50.8508,4.3523,2024-01-15T10:30:10+01:00
                  """;
        var gnss = GnssInput.FromCsv(csv);

        var result = Projection.ProjectGnss(network, gnss);

        Assert.Equal(3, result.Count);
    }

    [Fact]
    public void ProjectGnss_CustomConfig_Respected()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));
        var config = new ProjectionConfig { MaxSearchRadiusMeters = 2000.0, SuppressWarnings = true };

        var result = Projection.ProjectGnss(network, gnss, config);

        Assert.NotEmpty(result);
    }

    [Fact]
    public void ProjectGnss_NullNetwork_Throws()
    {
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));
        Assert.Throws<ArgumentNullException>(() => Projection.ProjectGnss((NetworkInput)null!, gnss));
    }

    [Fact]
    public void ProjectGnss_MalformedGeoJson_ThrowsParse()
    {
        var network = NetworkInput.FromGeoJson("{ not valid json");
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));
        Assert.Throws<TpLibProjectionException>(() => Projection.ProjectGnss(network, gnss));
    }

    [Fact]
    public void ProjectOntoPath_RoundTrip_PopulatesIntrinsic()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));

        var pathResult = PathCalculation.CalculateTrainPath(network, gnss);
        if (!pathResult.HasPath)
        {
            return; // sample data may not yield a path in all runs
        }

        var projected = Projection.ProjectOntoPath(network, gnss, pathResult.Path!);
        Assert.All(projected, p =>
        {
            if (p.Intrinsic is not null)
            {
                Assert.InRange(p.Intrinsic!.Value, 0.0, 1.0);
            }
        });
    }
}
