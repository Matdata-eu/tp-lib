# Contract — Detections GeoJSON Format

**Feature**: 004-train-detections
**Applies to**: `--punctual-detections <FILE.geojson|.json>`, `--linear-detections <FILE.geojson|.json>`

A detections GeoJSON file is a `FeatureCollection`. Each feature represents one detection. The detection kind is encoded in `properties.kind` and **must match** the CLI flag being used to load the file (mismatch → `DetectionError::InvalidSchema`).

---

## Common Feature Shape

```json
{
  "type": "Feature",
  "geometry": <Geometry | null>,
  "properties": {
    "kind": "punctual" | "linear",
    "id": "<optional string>",
    "source": "<optional string>",
    ...
  }
}
```

Top-level:

```json
{ "type": "FeatureCollection", "features": [ /* … */ ] }
```

---

## Punctual Detection Feature

**`properties.kind = "punctual"`**

| Property          | Type    | Required               | Notes |
|-------------------|---------|------------------------|-------|
| `kind`            | string  | **yes** (`"punctual"`) | |
| `timestamp`       | string  | **yes**                | RFC3339 with timezone. |
| `netelement_id`   | string  | conditional            | Required if no `geometry`. |
| `intrinsic`       | number  | optional               | ∈ [0, 1]. |
| `id`              | string  | optional               | |
| `source`          | string  | optional               | |

Other properties → captured into `metadata`.

**Geometry rules**:
- If `geometry` is `Point` ⇒ `[lon, lat]` interpreted in CRS `EPSG:4326` (GeoJSON default). The optional `properties.crs` overrides only if explicitly set.
- If `geometry` is `null` ⇒ `properties.netelement_id` **must** be present (topological-only).
- Any other geometry type ⇒ `DetectionError::InvalidSchema`.

**`properties.crs`** (optional override): a string like `"EPSG:31370"`. If present, `geometry` coordinates are interpreted in that CRS instead of EPSG:4326.

### Example

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": null,
      "properties": {
        "kind": "punctual",
        "timestamp": "2026-05-01T08:15:30+02:00",
        "netelement_id": "NE-12345",
        "intrinsic": 0.5,
        "id": "beacon-7",
        "source": "BTM-A1"
      }
    },
    {
      "type": "Feature",
      "geometry": { "type": "Point", "coordinates": [4.34878, 50.85045] },
      "properties": {
        "kind": "punctual",
        "timestamp": "2026-05-01T08:17:00+02:00",
        "id": "gnss-fix-99",
        "source": "external"
      }
    }
  ]
}
```

---

## Linear Detection Feature

**`properties.kind = "linear"`**

| Property          | Type    | Required             | Notes |
|-------------------|---------|----------------------|-------|
| `kind`            | string  | **yes** (`"linear"`) | |
| `t_from`          | string  | **yes**              | RFC3339 with timezone. |
| `t_to`            | string  | **yes**              | RFC3339 with timezone; `t_to >= t_from`. |
| `netelement_id`   | string  | **yes**              | |
| `start_intrinsic` | number  | optional             | ∈ [0, 1]. |
| `end_intrinsic`   | number  | optional             | ∈ [0, 1]; if both present, `start_intrinsic <= end_intrinsic`. |
| `id`              | string  | optional             | |
| `source`          | string  | optional             | |

**Geometry**: ignored for linear detections (`null` recommended). If a `LineString` is present it is silently dropped (the spatial extent is derived from `netelement_id` + intrinsics).

### Example

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": null,
      "properties": {
        "kind": "linear",
        "t_from": "2026-05-01T08:15:00+02:00",
        "t_to":   "2026-05-01T08:17:30+02:00",
        "netelement_id": "NE-9001",
        "source": "track-circuit-A12"
      }
    },
    {
      "type": "Feature",
      "geometry": null,
      "properties": {
        "kind": "linear",
        "t_from": "2026-05-01T08:18:00+02:00",
        "t_to":   "2026-05-01T08:19:00+02:00",
        "netelement_id": "NE-9002",
        "start_intrinsic": 0.0,
        "end_intrinsic":   0.5,
        "id": "bsec-7",
        "source": "block-section-B7"
      }
    }
  ]
}
```

---

## Cross-Format Equivalence (FR-002b)

For any detection set:
- Round-tripping CSV ⇄ GeoJSON yields identical `Detection` values, **except** that:
  - GeoJSON CSV-only metadata becomes top-level `properties`.
  - Numeric precision is preserved to ≥ 9 significant digits.

This property is enforced by contract tests `tp-core/tests/detections_load.rs::roundtrip_*`.

---

## Error Handling

| Condition                                                  | Result |
|------------------------------------------------------------|--------|
| Top-level not a `FeatureCollection`                        | `DetectionError::InvalidSchema` |
| `properties.kind` missing or not `"punctual"`/`"linear"`   | `DetectionError::InvalidSchema` |
| `kind` does not match CLI flag                             | `DetectionError::InvalidSchema` |
| Required property missing                                  | `DetectionError::InvalidSchema` |
| Timestamp without timezone                                 | `DetectionError::InvalidTimestamp` |
| `intrinsic` / `start_intrinsic` / `end_intrinsic` ∉ [0, 1] | `DetectionError::InvalidIntrinsic` |
| Punctual: both geometry and `netelement_id`                | `DetectionError::InvalidSchema` |
| Punctual: neither geometry nor `netelement_id`             | `DetectionError::InvalidSchema` |
| Punctual geometry not `Point`/`null`                       | `DetectionError::InvalidSchema` |
