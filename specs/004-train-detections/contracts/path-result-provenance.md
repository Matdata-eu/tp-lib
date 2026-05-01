# Contract — `PathResult.detection_provenance` JSON

**Feature**: 004-train-detections
**Applies to**: `tp-cli` `--json-output`, `tp-webapp` `/api/path` payload, library consumers serializing `PathResult`.

This document defines the JSON shape of the new `detection_provenance` field added to `PathResult`. All other `PathResult` fields are unchanged.

---

## Schema

```jsonc
{
  // ... other PathResult fields (unchanged) ...

  "detection_provenance": [
    {
      "source_file": "<string>",
      "source_row":  <integer | null>,        // CSV row (1-based, header = row 1) or GeoJSON feature index (0-based)
      "kind":        "punctual" | "linear",
      "id":          "<string | null>",
      "source":      "<string | null>",
      "timestamp":   <Timestamp | TimestampRange>,
      "status":      <Status>,
      "metadata":    { "<key>": "<value>", ... }
    }
  ]
}
```

### `Timestamp` (punctual)

```json
{ "instant": "2026-05-01T08:15:30+02:00" }
```

### `TimestampRange` (linear)

```json
{ "from": "2026-05-01T08:15:00+02:00", "to": "2026-05-01T08:17:30+02:00" }
```

### `Status` variants

#### Applied

```json
{
  "applied": {
    "netelement_id": "NE-12345",
    "intrinsic":     0.5
  }
}
```

`intrinsic` may be `null`.

#### Resolved (transient — only appears when an intermediate stage discards a successfully resolved detection)

```json
{
  "resolved": {
    "netelement_id": "NE-12345",
    "distance_m":    0.83
  }
}
```

#### Discarded

```json
{ "discarded": { "out_of_time_range": { "gnss_first": "...", "gnss_last": "..." } } }
{ "discarded": { "out_of_reach":       { "nearest_distance_m": 4.12, "cutoff_m": 2.5 } } }
{ "discarded": { "unknown_netelement": { "netelement_id": "NE-DOES-NOT-EXIST" } } }
{ "discarded": { "intrinsic_out_of_range": { "value": 1.5 } } }
{ "discarded": { "duplicate_of_prior_detection": { "kept_index": 7 } } }
```

`kept_index` refers to the position in `detection_provenance` of the surviving detection.

---

## Ordering Guarantee

`detection_provenance` is ordered **by input order** (concatenating `--punctual-detections` then `--linear-detections`, each in the order rows/features appear in the source file). This index is referenced by `kept_index` for duplicates.

## Cardinality

`detection_provenance.length == total_input_detections` (one record per input detection). Conflicting detections never appear here — they cause a fatal error before serialization.

## CLI Summary Mapping (FR-020)

The stderr summary line aggregates these statuses:

```
detections: <Applied count> applied, <Discarded count> discarded
   (<n out_of_time_range> out-of-range, <n out_of_reach> out-of-reach,
    <n unknown_netelement> unknown-netelement, <n duplicate> duplicate,
    <n intrinsic_out_of_range> bad-intrinsic)
```

Empty buckets are omitted from the parenthesised breakdown.

---

## Backward Compatibility

Consumers that ignore unknown fields are unaffected. The field is **always present** when the feature is compiled in (default), even when no detections were loaded — in which case it is `[]`.
