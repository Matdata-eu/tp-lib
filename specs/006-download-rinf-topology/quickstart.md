# Quickstart: ERA RINF Network Download

**Feature**: `006-download-rinf-topology`  
**Audience**: developers validating automatic topology retrieval through CLI, Python, and .NET.

This walkthrough exercises the covered-area success path, the missing-coverage path, and the validation-failure path for auto-retrieved topology.

---

## Prerequisites

- Workspace builds locally.
- Outbound HTTPS access to `https://graph.data.era.europa.eu/repositories/rinf-plus`.
- A GNSS fixture located inside the known-good smoke-test polygon from `research.md`.
- Optional negative fixtures:
  - empty or invalid GNSS file
  - GNSS file outside available RINF coverage
  - mocked or recorded RINF response with coarse netelement geometry or zero netrelations

---

## Example 1: CLI path calculation without `--network`

```powershell
cargo run -p tp-cli -- calculate-path `
  --gnss test-data/rinf_smoke_gnss.geojson `
  --output target/tmp/rinf_path.geojson `
  --verbose
```

Expected behavior:
- CLI derives the retrieval polygon from GNSS.
- CLI downloads and validates RINF topology.
- CLI calculates the path and writes the output file.
- Stderr identifies that ERA RINF was used as the topology source.

---

## Example 2: CLI invalid-input failure

```powershell
cargo run -p tp-cli -- calculate-path `
  --gnss test-data/rinf_empty_gnss.geojson `
  --output target/tmp/should_not_exist.geojson
```

Expected behavior:
- Command fails before contacting the endpoint.
- Stderr reports invalid or empty GNSS input.

---

## Example 3: Python binding with automatic retrieval

```python
from tp_lib import calculate_train_path, RinfRetrievalOptions

result = calculate_train_path(
    gnss_positions=gnss_positions,
    network=None,
    rinf_options=RinfRetrievalOptions(
        endpoint_url="https://graph.data.era.europa.eu/repositories/rinf-plus",
        buffer_meters=1000.0,
    ),
)

print(len(result.projected_positions))
```

Expected behavior:
- No network file is required.
- Retrieval/validation happens inside the Rust core path.
- Failure categories surface as distinct Python exceptions
  (`InvalidGnssInputError`, `RinfMissingCoverageError`,
  `RinfIncompleteTopologyError`, `RinfRetrievalFailedError`).

---

## Example 4: .NET binding with automatic retrieval

```csharp
using TpLib;

var gnss = GnssInput.FromGeoJson(File.ReadAllText("test-data/rinf_smoke_gnss.geojson"));
var rinf = new RinfRetrievalOptions
{
    EndpointUrl  = "https://graph.data.era.europa.eu/repositories/rinf-plus",
    BufferMeters = 1000.0,
};

var result = PathCalculation.CalculateTrainPathAuto(network: null, gnss, rinfOptions: rinf);
Console.WriteLine(result.HasPath);
```

Expected behavior:
- Passing `null` for the network triggers ERA RINF retrieval.
- Missing coverage, incomplete topology, endpoint failure, and invalid
  GNSS each surface as distinct typed exceptions
  (`TpLibRinfMissingCoverageException`,
  `TpLibRinfIncompleteTopologyException`,
  `TpLibRinfRetrievalFailedException`,
  `TpLibInvalidGnssInputException`).

---

## Example 5: Endpoint smoke test using the known-good polygon

Use the polygon from `research.md` as the acceptance fixture for the external integration test. The test should confirm:
- netelements are returned
- netrelations are returned
- no returned netelement longer than 250 m is represented by only two points
  (validation only rejects the bundle when *every* netelement is coarse;
  individual coarse segments are reported but do not fail the workflow)

---

## Validation Checklist

- [ ] Covered-area CLI run succeeds without `--network`.
- [ ] Missing-coverage CLI run fails with the `missing_coverage` outcome.
- [ ] Invalid/empty GNSS input fails before any retrieval request.
- [ ] A mocked coarse-geometry response fails validation.
- [ ] A mocked response with netelements but zero netrelations fails validation.
- [ ] Python bindings can run a topology-dependent workflow without a supplied topology file.
- [ ] .NET bindings can run a topology-dependent workflow without a supplied topology file.
- [ ] Manual topology still takes precedence when explicitly supplied.