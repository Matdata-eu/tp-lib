# Contract — Webapp Detections API

**Feature**: 004-train-detections
**Applies to**: `tp-webapp` HTTP endpoint serving detection overlays for the path-review UI.

The webapp is **read-only** for detections (per user clarification Q2). No upload, edit, delete, or filter endpoints exist.

---

## Endpoint

```
GET /api/detections
```

### Response — `200 OK`

```jsonc
{
  "punctual": [
    {
      "provenance_index": 0,                       // index into PathResult.detection_provenance
      "id": "beacon-7",
      "source": "BTM-A1",
      "timestamp": "2026-05-01T08:15:30+02:00",
      "status": "applied",                          // "applied" | "discarded"
      "discard_reason": null,                       // populated iff status == "discarded"
      "netelement_id": "NE-12345",                  // resolved netelement (null if Discarded with no resolution)
      "intrinsic": 0.5,                             // null if not provided
      "marker": {
        "lat": 50.85045,                            // for rendering on Leaflet
        "lon": 4.34878
      }
    }
  ],
  "linear": [
    {
      "provenance_index": 12,
      "id": "bsec-7",
      "source": "block-section-B7",
      "t_from": "2026-05-01T08:18:00+02:00",
      "t_to":   "2026-05-01T08:19:00+02:00",
      "status": "applied",
      "discard_reason": null,
      "netelement_id": "NE-9002",
      "start_intrinsic": 0.0,
      "end_intrinsic":   0.5,
      "highlight": {
        "geometry": {
          "type": "LineString",
          "coordinates": [[4.34878, 50.85045], [4.34920, 50.85101]]
        }
      }
    }
  ]
}
```

### `discard_reason` values (when `status == "discarded"`)

| Value                          | Meaning |
|--------------------------------|---------|
| `"out_of_time_range"`          | Outside GNSS observation window |
| `"out_of_reach"`               | No netelement within `--cutoff-distance-detections` |
| `"unknown_netelement"`         | `netelement_id` not in network |
| `"intrinsic_out_of_range"`     | Intrinsic value outside `[0, 1]` |
| `"duplicate_of_prior_detection"` | Same timestamp & netelement as an earlier detection |

### `marker.lat`/`lon` derivation (punctual)

- If the input was `Geographic`, return the resolved point on the netelement (R-tree projection target) when `Resolved`/`Applied`. If `Discarded(out_of_reach)`, return the original input lat/lon (so the operator sees where the bad detection landed).
- If the input was `Topological`, compute `(lat, lon)` by interpolating along the netelement geometry at `intrinsic` (or `0.5` if intrinsic absent).

### `highlight.geometry` derivation (linear)

A `LineString` clipped along the netelement between `start_intrinsic` (default `0.0`) and `end_intrinsic` (default `1.0`), projected to `EPSG:4326`. Discarded linear detections still receive geometry so the operator can locate them.

### Errors

| Status | Body                          | Meaning |
|--------|-------------------------------|---------|
| `500`  | `{ "error": "internal" }`     | Server failure |

Empty arrays are valid and returned as `200` — they convey "feature was enabled, no detections loaded".

---

## Frontend rendering rules (FR-021..024)

| Element            | Style |
|--------------------|-------|
| Punctual applied   | Filled circle marker, tooltip with id+timestamp |
| Punctual discarded | Hollow marker with `×`, muted color, dashed border |
| Linear applied     | Solid polyline overlay, opacity 0.6, distinct color |
| Linear discarded   | Dashed polyline, muted color, opacity 0.3 |

Click on any rendered detection ⇒ details panel populated from the JSON record above (id, source, timestamp(s), status, reason, netelement, intrinsic(s), full `metadata`).

---

## Caching

The endpoint is read-only and idempotent for the currently loaded run state. Webapp may apply standard HTTP caching headers keyed to the in-memory `path_result.hash`.
