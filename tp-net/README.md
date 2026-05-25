# TpLib — .NET bindings for tp-lib

C#/.NET bindings for `tp-lib`, providing GNSS projection onto railway networks
and train-path calculation from a managed API.

## Prerequisites

- .NET 8 SDK (or newer)
- One of the supported platforms (see table below)

## Install

```sh
dotnet add package TpLib
```

The package ships pre-built native binaries for every supported platform; the
correct one is selected automatically at runtime.

## Quick example

```csharp
using TpLib;

var networkGeoJson = File.ReadAllText("network.geojson");
var gnssGeoJson    = File.ReadAllText("gnss.geojson");

// 1. Project GNSS points onto the closest network elements.
var projections = Projection.ProjectGnss(networkGeoJson, gnssGeoJson);
Console.WriteLine($"Projected {projections.Count} points");

// 2. Calculate the most likely train path.
var result = PathCalculation.CalculateTrainPath(networkGeoJson, gnssGeoJson);
if (result.HasPath)
{
    Console.WriteLine($"Path probability: {result.Path!.OverallProbability:F3}");
    foreach (var segment in result.Path.Segments)
    {
        Console.WriteLine($"  {segment.NetelementId}  p={segment.Probability:F3}");
    }
}
```

## Automatic RINF Topology Retrieval

When you do not have a local network GeoJSON, omit it and let the library
download a bounding-box subset of the ERA RINF topology on demand:

```csharp
using TpLib;

var gnss = GnssInput.FromGeoJson(File.ReadAllText("gnss.geojson"));
var rinf = new RinfRetrievalOptions
{
    EndpointUrl  = "https://graph.data.era.europa.eu/repositories/rinf-plus",
    BufferMeters = 1000.0,
};

// Pass null for the network to trigger auto-retrieval.
var projections = Projection.ProjectGnssAuto(network: null, gnss, rinfOptions: rinf);
var path        = PathCalculation.CalculateTrainPathAuto(network: null, gnss, rinfOptions: rinf);
```

Typed exceptions are raised for retrieval failures:
`TpLibInvalidGnssInputException`, `TpLibRinfMissingCoverageException`,
`TpLibRinfIncompleteTopologyException`, `TpLibRinfRetrievalFailedException`.

## Supported platforms

| RID         | OS / Architecture            | Native library         |
|-------------|------------------------------|------------------------|
| `win-x64`   | Windows 10+ on x86_64        | `tp_lib_net.dll`       |
| `linux-x64` | Linux glibc on x86_64        | `libtp_lib_net.so`     |
| `osx-x64`   | macOS on Intel               | `libtp_lib_net.dylib`  |
| `osx-arm64` | macOS on Apple Silicon       | `libtp_lib_net.dylib`  |

## See also

- [quickstart.md](../../specs/005-dotnet-bindings/quickstart.md) — end-to-end walkthrough
- [contracts/api.md](../../specs/005-dotnet-bindings/contracts/api.md) — full API reference
- [tp-lib root README](../../README.md)

## License

Apache-2.0
