using System.Linq;
using TpLib;
using Xunit;

namespace TpLib.Tests;

public class PathCalculationTests
{
    [Fact]
    public void CalculateTrainPath_SampleData_ReturnsPath()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));

        var result = PathCalculation.CalculateTrainPath(network, gnss);

        Assert.NotNull(result);
        if (result.HasPath)
        {
            Assert.InRange(result.Path!.OverallProbability, 0.0, 1.0);
        }
    }

    [Fact]
    public void CalculateTrainPath_ModeIsTopologyOrFallback()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));

        var result = PathCalculation.CalculateTrainPath(network, gnss);

        Assert.True(result.Mode == PathCalculationMode.TopologyBased
                 || result.Mode == PathCalculationMode.FallbackIndependent);
    }

    [Fact]
    public void CalculateTrainPath_PathOnlyTrue_EmptyProjections()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));
        var config = new PathConfig { PathOnly = true };

        var result = PathCalculation.CalculateTrainPath(network, gnss, config);

        Assert.Empty(result.ProjectedPositions);
    }

    [Fact]
    public void CalculateTrainPath_MalformedGeoJson_ThrowsParse()
    {
        var network = NetworkInput.FromGeoJson("{ broken");
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));

        Assert.Throws<TpLibPathException>(() => PathCalculation.CalculateTrainPath(network, gnss));
    }

    [Fact]
    public void CalculateTrainPathAuto_SuppliedNetwork_TakesPrecedence()
    {
        // Supplying a network must short-circuit RINF retrieval, even with an
        // intentionally unreachable endpoint configured.
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));
        var rinf = new RinfRetrievalOptions
        {
            EndpointUrl = "http://127.0.0.1:1/never",
            BufferMeters = 250.0,
        };

        var result = PathCalculation.CalculateTrainPathAuto(network, gnss, rinfOptions: rinf);

        Assert.NotNull(result);
    }

    [Fact]
    public void CalculateTrainPathAuto_NullNetworkUnreachableEndpoint_ThrowsRetrievalFailed()
    {
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));
        var rinf = new RinfRetrievalOptions
        {
            EndpointUrl = "http://127.0.0.1:1/never",
            BufferMeters = 250.0,
        };

        Assert.Throws<TpLibRinfRetrievalFailedException>(
            () => PathCalculation.CalculateTrainPathAuto(null, gnss, rinfOptions: rinf));
    }
}
