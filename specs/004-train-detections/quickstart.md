# Quickstart — Absolute Train Position Detections

**Feature**: 004-train-detections
**Audience**: developers integrating detections into a `tp-cli` run or webapp review.

This walkthrough exercises every user story (US1, US2, US3) end-to-end against a small example.

---

## Prerequisites

- Built workspace: `cargo build --workspace`
- Sample fixtures (suggested locations under `test-data/`):
  - `test-data/sample_gnss.geojson` — GNSS observations (existing)
  - `test-data/sample_network.geojson` — railway network (existing)
  - `test-data/sample_detections_punctual.csv` — to be added in Phase 2
  - `test-data/sample_detections_linear.geojson` — to be added in Phase 2

---

## Example 1 — Punctual detection on known netelement (US1)

`test-data/sample_detections_punctual.csv`:

```csv
timestamp,netelement_id,intrinsic,id,source
2026-05-01T08:15:30+02:00,NE-12345,0.5,beacon-7,BTM-A1
```

Run:

```powershell
cargo run -p tp-cli -- calculate `
  --gnss test-data/sample_gnss.geojson `
  --network test-data/sample_network.geojson `
  --punctual-detections test-data/sample_detections_punctual.csv
```

Expected stderr summary:

```
detections: 1 applied, 0 discarded
```

Expected behaviour: at the GNSS index whose timestamp is closest to `08:15:30+02:00`, the Viterbi candidate set is forced to `NE-12345`. The resulting `TrainPath` includes `NE-12345` in its `associated_net_elements` chain.

---

## Example 2 — Punctual detection by coordinate (US2)

`test-data/sample_detections_punctual_coord.geojson`:

```json
{
  "type": "FeatureCollection",
  "features": [{
    "type": "Feature",
    "geometry": { "type": "Point", "coordinates": [4.34878, 50.85045] },
    "properties": {
      "kind": "punctual",
      "timestamp": "2026-05-01T08:17:00+02:00",
      "id": "gnss-fix-99",
      "source": "external"
    }
  }]
}
```

Run with a non-default detection cutoff:

```powershell
cargo run -p tp-cli -- calculate `
  --gnss test-data/sample_gnss.geojson `
  --network test-data/sample_network.geojson `
  --punctual-detections test-data/sample_detections_punctual_coord.geojson `
  --cutoff-distance-detections 5.0
```

Expected: the point is projected onto the nearest netelement; if perpendicular distance ≤ 5.0 m, anchor is applied. Otherwise the detection is discarded with `out_of_reach` and reported in the summary.

---

## Example 3 — Linear detection (US3)

`test-data/sample_detections_linear.geojson`:

```json
{
  "type": "FeatureCollection",
  "features": [{
    "type": "Feature",
    "geometry": null,
    "properties": {
      "kind": "linear",
      "t_from": "2026-05-01T08:18:00+02:00",
      "t_to":   "2026-05-01T08:19:00+02:00",
      "netelement_id": "NE-9002",
      "id": "bsec-7",
      "source": "block-section-B7"
    }
  }]
}
```

Run:

```powershell
cargo run -p tp-cli -- calculate `
  --gnss test-data/sample_gnss.geojson `
  --network test-data/sample_network.geojson `
  --linear-detections test-data/sample_detections_linear.geojson
```

Expected: every GNSS observation whose timestamp ∈ `[08:18:00, 08:19:00]` has its candidate set restricted to `NE-9002`. The path crosses `NE-9002` for the duration of the window.

---

## Combined run (all three stories)

```powershell
cargo run -p tp-cli -- calculate `
  --gnss test-data/sample_gnss.geojson `
  --network test-data/sample_network.geojson `
  --punctual-detections test-data/sample_detections_punctual.csv `
  --linear-detections test-data/sample_detections_linear.geojson `
  --cutoff-distance-detections 2.5 `
  --json-output test-data/run.json
```

Inspect provenance:

```powershell
Get-Content test-data/run.json |
  ConvertFrom-Json |
  Select-Object -ExpandProperty detection_provenance |
  Format-Table id, kind, status
```

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
