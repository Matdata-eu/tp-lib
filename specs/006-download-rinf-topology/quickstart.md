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
from tp_lib import calculate_train_path

result = calculate_train_path(
    "test-data/rinf_smoke_gnss.geojson",
    network_file=None,
)

print(result.mode)
print(len(result.projected_positions))
```

Expected behavior:
- No network file is required.
- Retrieval/validation happens inside the Rust core path.
- Failure categories remain distinct Python exceptions.

---

## Example 4: .NET binding with automatic retrieval

```csharp
using TpLib;

var gnss = GnssInput.FromGeoJsonFile("test-data/rinf_smoke_gnss.geojson");
var result = PathCalculation.CalculateTrainPath(gnss, network: null);

Console.WriteLine(result.Mode);
Console.WriteLine(result.ProjectedPositions.Count);
```

Expected behavior:
- Nullable network input triggers ERA RINF retrieval.
- Missing coverage and endpoint failures remain distinguishable from invalid GNSS input.

---

## Example 5: Endpoint smoke test using the known-good polygon

Use the polygon from `research.md` as the acceptance fixture for the external integration test. The test should confirm:
- netelements are returned
- netrelations are returned
- no returned netelement longer than 250 m is represented by only two points

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