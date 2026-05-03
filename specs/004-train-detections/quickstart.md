# Quickstart — Absolute Train Position Detections

**Feature**: 004-train-detections
**Audience**: developers integrating detections into a `tp-cli` run or webapp review.

This walkthrough exercises every user story (US1, US2, US3) end-to-end against a small example.

All commands assume the working directory is the workspace root (`tp-lib/`). The CLI binary
package name is `tp-lib-cli`; the `calculate-path` subcommand performs path calculation
without coordinate projection.

---

## Prerequisites

- Built workspace: `cargo build --workspace`
- Sample fixtures shipped under `test-data/`:
  - [test-data/sample_gnss.geojson](../../test-data/sample_gnss.geojson) — 3 GNSS observations at `2024-01-15T10:30:00..10+01:00`
  - [test-data/sample_network.geojson](../../test-data/sample_network.geojson) — `NE001`, `NE002` (LineStrings)
  - [test-data/sample_detections_punctual.csv](../../test-data/sample_detections_punctual.csv) — punctual detection on `NE001` at `10:30:05+01:00`
  - [test-data/sample_detections_linear.geojson](../../test-data/sample_detections_linear.geojson) — linear detection on `NE001` from `10:30:00` to `10:30:10+01:00`

---

## Example 1 — Punctual detection on known netelement (US1)

[test-data/sample_detections_punctual.csv](../../test-data/sample_detections_punctual.csv):

```csv
timestamp,netelement_id,intrinsic,id,source
2024-01-15T10:30:05+01:00,NE001,0.5,beacon-7,BTM-A1
```

Run:

```powershell
cargo run -p tp-lib-cli -- calculate-path `
  --gnss test-data/sample_gnss.geojson `
  --network test-data/sample_network.geojson `
  --punctual-detections test-data/sample_detections_punctual.csv `
  -o target/tmp/path1.json -v
```

Expected stderr summary line:

```
detections: 1 applied, 0 discarded
```

Expected behaviour: at the GNSS index whose timestamp is closest to `10:30:05+01:00`, the Viterbi candidate set is forced to `NE001`. The resulting train path includes `NE001` in its segment chain.

---

## Example 2 — Punctual detection by coordinate (US2)

A coordinate-only punctual detection (GeoJSON Point geometry, no `netelement_id` property)
is resolved to the nearest netelement within `--cutoff-distance-detections` (meters,
default `2.5`). If the perpendicular projection distance exceeds the cutoff, the
detection is discarded with reason `out_of_reach` and reported in the summary.

```powershell
# Example fixture (not committed): test-data/sample_detections_punctual_coord.geojson
# {
#   "type": "FeatureCollection",
#   "features": [{
#     "type": "Feature",
#     "geometry": { "type": "Point", "coordinates": [4.3520, 50.8505] },
#     "properties": {
#       "kind": "punctual",
#       "timestamp": "2024-01-15T10:30:05+01:00",
#       "id": "gnss-fix-99",
#       "source": "external"
#     }
#   }]
# }

cargo run -p tp-lib-cli -- calculate-path `
  --gnss test-data/sample_gnss.geojson `
  --network test-data/sample_network.geojson `
  --punctual-detections test-data/sample_detections_punctual_coord.geojson `
  --cutoff-distance-detections 5.0 `
  -o target/tmp/path2_coord.json -v
```

---

## Example 3 — Linear detection (US3)

[test-data/sample_detections_linear.geojson](../../test-data/sample_detections_linear.geojson):

```json
{
  "type": "FeatureCollection",
  "features": [{
    "type": "Feature",
    "geometry": null,
    "properties": {
      "kind": "linear",
      "t_from": "2024-01-15T10:30:00+01:00",
      "t_to":   "2024-01-15T10:30:10+01:00",
      "netelement_id": "NE001",
      "id": "bsec-7",
      "source": "block-section-B7"
    }
  }]
}
```

Run:

```powershell
cargo run -p tp-lib-cli -- calculate-path `
  --gnss test-data/sample_gnss.geojson `
  --network test-data/sample_network.geojson `
  --linear-detections test-data/sample_detections_linear.geojson `
  -o target/tmp/path2.json -v
```

Expected stderr summary:

```
detections: 1 applied, 0 discarded
```

Expected behaviour: every GNSS observation whose timestamp ∈ `[10:30:00, 10:30:10]` has its candidate set restricted to `NE001`. The path follows `NE001` for the duration of the window.

---

## Combined run (all three stories)

```powershell
cargo run -p tp-lib-cli -- calculate-path `
  --gnss test-data/sample_gnss.geojson `
  --network test-data/sample_network.geojson `
  --punctual-detections test-data/sample_detections_punctual.csv `
  --linear-detections test-data/sample_detections_linear.geojson `
  --cutoff-distance-detections 2.5 `
  -o target/tmp/path3.json -v
```

Expected stderr summary:

```
detections: 2 applied, 0 discarded
```

Inspect the resulting train path GeoJSON (one feature per path segment with
`netelement_id`, `start_intrinsic`, `end_intrinsic`, `gnss_start_index`,
`gnss_end_index`, `probability`):

```powershell
Get-Content target/tmp/path3.json |
  ConvertFrom-Json |
  Select-Object -ExpandProperty features |
  ForEach-Object { $_.properties } |
  Format-Table netelement_id, start_intrinsic, end_intrinsic, probability
```

> **Note**: `PathResult.detection_provenance` is populated in memory during pipeline
> execution but is not yet serialized into the GeoJSON path file. Provenance
> inspection is currently available via the library API or the webapp review.

---

## Webapp review

Start the webapp pointing at the run directory:

```powershell
cargo run -p tp-webapp -- --runs-dir test-data/
```

Open `http://localhost:8080`, select the run, and toggle the **Detections** layer. Click any marker / highlight to see id, source, timestamp(s), status, reason, resolved netelement and intrinsic, and the full metadata table.

---

## Validation Checklist (mirrors success criteria)

- [ ] **SC-001**: A punctual detection on a different netelement than GNSS would suggest is honoured (Viterbi outputs the anchored netelement at that timestamp).
- [ ] **SC-002**: Coordinate-only punctual detection within cutoff is resolved to the expected netelement.
- [ ] **SC-003**: Linear detection forces all in-window GNSS samples onto the anchored netelement.
- [ ] **SC-004**: Out-of-time-range detections are discarded with the correct reason.
- [ ] **SC-005**: Combined run on 10k GNSS × 1k detections completes within 1.20× the no-detections baseline.
- [ ] **SC-006**: Conflicting detections (same timestamp, different netelements) abort with `DetectionError::ConflictingDetections`.
- [ ] **SC-007**: CLI summary line correctly counts applied + discarded buckets.
- [ ] **SC-008**: Provenance JSON round-trips and matches contract.
- [ ] **SC-009**: Webapp markers/highlights distinguish applied vs discarded.
- [ ] **SC-010**: Unsupported file extension yields `DetectionError::UnsupportedExtension`.
- [ ] **SC-011**: CSV ⇄ GeoJSON round-trip produces identical `Detection` values.
