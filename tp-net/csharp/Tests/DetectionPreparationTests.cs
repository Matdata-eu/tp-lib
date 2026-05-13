using System.IO;
using System.Linq;
using TpLib;
using Xunit;

namespace TpLib.Tests;

public class DetectionPreparationTests
{
    [Fact]
    public void PrepareDetections_Punctual_GeoJson_ReturnsRecords()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));
        var detPath = TestData.Path("sample_detections_punctual_coord.geojson");
        if (!File.Exists(detPath))
        {
            return; // skip if fixture missing
        }
        var detections = File.ReadAllText(detPath);

        var prepared = DetectionPreparation.PrepareDetections(
            network, gnss, detections, DetectionKind.Punctual, cutoffDistanceMeters: 2.5);

        Assert.NotNull(prepared);
        Assert.NotNull(prepared.Records);
    }

    [Fact]
    public void PrepareDetections_Linear_GeoJson_ReturnsRecords()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));
        var detPath = TestData.Path("sample_detections_linear.geojson");
        if (!File.Exists(detPath))
        {
            return;
        }
        var detections = File.ReadAllText(detPath);

        var prepared = DetectionPreparation.PrepareDetections(
            network, gnss, detections, DetectionKind.Linear, cutoffDistanceMeters: 2.5);

        Assert.NotNull(prepared);
    }

    [Fact]
    public void PrepareDetections_NullDetections_Throws()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));

        Assert.Throws<System.ArgumentNullException>(
            () => DetectionPreparation.PrepareDetections(network, gnss, (string)null!, DetectionKind.Punctual, 2.5));
    }

    [Fact]
    public void PrepareDetections_MalformedGeoJson_ThrowsDetection()
    {
        var network = NetworkInput.FromGeoJson(TestData.Read("sample_network.geojson"));
        var gnss = GnssInput.FromGeoJson(TestData.Read("sample_gnss.geojson"));

        Assert.Throws<TpLibDetectionException>(
            () => DetectionPreparation.PrepareDetections(network, gnss, "{not json", DetectionKind.Punctual, 2.5));
    }
}
