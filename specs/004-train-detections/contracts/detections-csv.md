# Contract — Detections CSV Format

**Feature**: 004-train-detections
**Applies to**: `--punctual-detections <FILE.csv>`, `--linear-detections <FILE.csv>`

CSV files are dispatched by file extension (`.csv`). Other extensions are rejected with `DetectionError::UnsupportedExtension` before any I/O.

---

## Punctual Detections — CSV Schema

**Required header row** (column order is free; presence is required):

| Column            | Type          | Required               | Notes |
|-------------------|---------------|------------------------|-------|
| `timestamp`       | RFC3339 string| **yes**                | Must include explicit timezone offset, e.g. `2026-05-01T08:15:30+02:00`. Parsed as `DateTime<FixedOffset>`. |
| `netelement_id`   | string        | conditional            | Required if `lat`/`lon` are absent. Mutually exclusive with `lat`/`lon` per row. |
| `intrinsic`       | float         | optional               | ∈ [0.0, 1.0]. Informational only (D8). |
| `lat`             | float         | conditional            | Required if `netelement_id` is absent. |
| `lon`             | float         | conditional            | Required if `netelement_id` is absent. |
| `crs`             | string        | conditional            | Required iff `lat`/`lon` are present (e.g. `EPSG:4326`). |
| `id`              | string        | optional               | If present, non-empty. |
| `source`          | string        | optional               | Free-form, e.g. `BTM-A1`, `axle-counter-12`. |

**Additional columns**: any unrecognised column is captured into `metadata` as `key → value` strings. Empty cells are treated as missing.

**Row-level rules**:
- Exactly one of `{netelement_id}` or `{lat, lon, crs}` must be supplied. Mixing raises `DetectionError::InvalidSchema`.
- Empty `timestamp` → `DetectionError::InvalidTimestamp`.

### Example

```csv
timestamp,netelement_id,intrinsic,id,source
2026-05-01T08:15:30+02:00,NE-12345,0.5,beacon-7,BTM-A1
2026-05-01T08:16:00+02:00,,, ,axle-counter-12
2026-05-01T08:16:00+02:00,,,,
```

```csv
timestamp,lat,lon,crs,id,source
2026-05-01T08:17:00+02:00,50.85045,4.34878,EPSG:4326,gnss-fix-99,external
```

---

## Linear Detections — CSV Schema

| Column            | Type          | Required | Notes |
|-------------------|---------------|----------|-------|
| `t_from`          | RFC3339 string| **yes**  | Same parsing rules as `timestamp`. |
| `t_to`            | RFC3339 string| **yes**  | Must satisfy `t_to >= t_from`. |
| `netelement_id`   | string        | **yes**  | |
| `start_intrinsic` | float         | optional | ∈ [0.0, 1.0]. |
| `end_intrinsic`   | float         | optional | ∈ [0.0, 1.0]. If both present, `start_intrinsic <= end_intrinsic`. |
| `id`              | string        | optional | |
| `source`          | string        | optional | E.g. `track-circuit-A12`, `block-section-B7`. |

Unknown columns → `metadata`. Empty cells → missing.

### Example

```csv
t_from,t_to,netelement_id,start_intrinsic,end_intrinsic,id,source
2026-05-01T08:15:00+02:00,2026-05-01T08:17:30+02:00,NE-9001,,,,track-circuit-A12
2026-05-01T08:18:00+02:00,2026-05-01T08:19:00+02:00,NE-9002,0.0,0.5,bsec-7,block-section-B7
```

---

## Encoding & Delimiters

- UTF-8 (BOM tolerated, stripped on read).
- Delimiter: comma (`,`). Quoting per RFC 4180.
- Line endings: `\n` or `\r\n`.

## Error Handling

| Condition                                     | Result |
|-----------------------------------------------|--------|
| Missing required column                       | `DetectionError::InvalidSchema` |
| Empty/unparseable required cell               | `DetectionError::Parse` (with row index) |
| Timestamp without timezone                    | `DetectionError::InvalidTimestamp` |
| `intrinsic` out of `[0, 1]`                   | `DetectionError::InvalidIntrinsic` |
| Both `netelement_id` and `lat/lon` per row    | `DetectionError::InvalidSchema` |
| Neither `netelement_id` nor `lat/lon` per row | `DetectionError::InvalidSchema` |
| `lat`/`lon` present without `crs`             | `DetectionError::MissingCrs` |
