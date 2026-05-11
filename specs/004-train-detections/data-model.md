# Data Model — Absolute Train Position Detections

**Feature**: 004-train-detections
**Date**: 2026-05-01

All entities below live in `tp-core/src/detections/` unless otherwise noted.
Existing types (`PathResult`, `PathConfig`, `PathMetadata`) receive additive fields only — no breaking changes.

---

## New Entities

### `Detection` (discriminated union)

```rust
pub enum Detection {
    Punctual(PunctualDetection),
    Linear(LinearDetection),
}
```

Returned from the file-format parsers after loading and before validation/resolution.

---

### `PunctualDetection`

```rust
pub struct PunctualDetection {
    /// Absolute timestamp (timezone-aware; naive timestamps rejected as InvalidTimestamp).
    pub timestamp: DateTime<FixedOffset>,
    /// Topological location — exactly one of `location` xor `coordinates` is `Some`.
    pub location: Option<TopologicalLocation>,
    pub coordinates: Option<GeographicLocation>,
    /// Informational only (D8); validated ∈ [0, 1] but not used by Viterbi.
    pub intrinsic: Option<f64>,
    /// Optional stable ID supplied by the operator.
    pub id: Option<String>,
    /// Free-form source label (e.g. "BTM-A1", "axle-counter-12").
    pub source: Option<String>,
    /// Unrecognised input columns/properties captured verbatim.
    pub metadata: HashMap<String, String>,
}
```

### `TopologicalLocation`

```rust
pub struct TopologicalLocation {
    pub netelement_id: String,
}
```

### `GeographicLocation`

```rust
pub struct GeographicLocation {
    pub lat: f64,
    pub lon: f64,
    /// Explicit CRS required (e.g. "EPSG:4326"); MissingCrs error if absent.
    pub crs: String,
}
```

---

### `LinearDetection`

```rust
pub struct LinearDetection {
    /// Window start; must satisfy t_from <= t_to.
    pub t_from: DateTime<FixedOffset>,
    /// Window end.
    pub t_to: DateTime<FixedOffset>,
    pub netelement_id: String,
    /// Informational only (D8); validated ∈ [0, 1] if present.
    pub start_intrinsic: Option<f64>,
    /// If both present: start_intrinsic <= end_intrinsic.
    pub end_intrinsic: Option<f64>,
    pub id: Option<String>,
    pub source: Option<String>,
    pub metadata: HashMap<String, String>,
}
```

---

### `ResolvedAnchor`

Produced by the resolution stage (after validation + coordinate projection). This is what the Viterbi receives via `PathConfig.anchors`.

```rust
pub enum ResolvedAnchor {
    /// Constrains all Viterbi states at `gnss_index` to the single given netelement+intrinsic.
    Punctual {
        netelement_id: String,
        intrinsic: Option<f64>,
        gnss_index: usize,
    },
    /// Filters Viterbi candidates at every GNSS index in `gnss_range` to the given netelement.
    Linear {
        netelement_id: String,
        start_intrinsic: Option<f64>,
        end_intrinsic: Option<f64>,
        /// Inclusive range of GNSS observation indices within the [t_from, t_to] window.
        gnss_range: std::ops::RangeInclusive<usize>,
    },
}
```

Sorted by `gnss_index` / `gnss_range.start()` before being passed to `PathConfig`.

---

### `DetectionRecord`

One record per input detection (including discarded ones). Written to `PathResult.detection_provenance`.

```rust
pub struct DetectionRecord {
    /// File path (or "<inline>" for tests).
    pub source_file: String,
    /// CSV row (1-based, header = row 1) or GeoJSON feature index (0-based); None if unavailable.
    pub source_row: Option<usize>,
    pub kind: DetectionKind,
    pub timestamp: TimestampOrRange,
    pub status: DetectionStatus,
    pub id: Option<String>,
    pub source: Option<String>,
    pub metadata: HashMap<String, String>,
}

pub enum DetectionKind {
    Punctual,
    Linear,
}

pub enum TimestampOrRange {
    Instant(DateTime<FixedOffset>),
    Range {
        from: DateTime<FixedOffset>,
        to: DateTime<FixedOffset>,
    },
}
```

---

### `DetectionStatus`

```rust
pub enum DetectionStatus {
    /// Detection was accepted and injected as a Viterbi anchor.
    Applied {
        netelement_id: String,
        intrinsic: Option<f64>,
    },
    /// Detection was successfully resolved from coordinates to a netelement but a later
    /// stage produced a `Discarded` outcome (transient intermediate; can appear in provenance
    /// when a resolved detection is then out-of-range or a duplicate).
    Resolved {
        netelement_id: String,
        distance_m: f64,
    },
    /// Detection was not used; carries a typed reason.
    Discarded {
        reason: DiscardReason,
    },
}
```

---

### `DiscardReason`

```rust
pub enum DiscardReason {
    /// Timestamp(s) fall outside the GNSS observation window.
    OutOfTimeRange {
        gnss_first: DateTime<FixedOffset>,
        gnss_last: DateTime<FixedOffset>,
    },
    /// Coordinate-only punctual: nearest netelement exceeded cutoff.
    OutOfReach {
        nearest_distance_m: f64,
        cutoff_m: f64,
    },
    /// `netelement_id` not found in the supplied network (fatal when encountered at validation
    /// time, but represented here if the detection reaches this stage via another path).
    UnknownNetelement {
        netelement_id: String,
    },
    /// `intrinsic`, `start_intrinsic`, or `end_intrinsic` outside `[0, 1]`.
    IntrinsicOutOfRange {
        value: f64,
    },
    /// Same timestamp + same netelement as an earlier detection; redundant row deduplicated.
    DuplicateOfPriorDetection {
        /// 0-based index of the surviving detection in `detection_provenance`.
        kept_index: usize,
    },
}
```

---

## Modified Existing Types

### `PathConfig` (additive fields)

**File**: `tp-core/src/path.rs` (or `path/config.rs` depending on module split)

```rust
pub struct PathConfig {
    // … existing fields unchanged …

    /// Resolved detection anchors, sorted by first GNSS index.
    /// Empty when no detection files are supplied.
    pub anchors: Vec<ResolvedAnchor>,

    /// Perpendicular-distance cutoff for coordinate-only punctual detection resolution.
    /// Corresponds to `--cutoff-distance-detections` CLI flag.
    /// Default: 2.5 m.
    pub detection_cutoff_distance: f64,
}
```

`Default` for `anchors` is `vec![]`; `Default` for `detection_cutoff_distance` is `2.5`.
Backward-compatible: existing callers that construct `PathConfig` with struct-update syntax or `..Default::default()` are unaffected.

---

### `PathResult` (additive field)

**File**: `tp-core/src/path.rs`

```rust
pub struct PathResult {
    // … existing fields unchanged …

    /// One record per input detection (including discarded). Always present;
    /// empty when no detection files were supplied.
    pub detection_provenance: Vec<DetectionRecord>,
}
```

JSON serialization of this field is defined in `contracts/path-result-provenance.md`.

---

## Error Types

### `DetectionError`

**File**: `tp-core/src/detections/error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    /// File extension is not `.csv`, `.geojson`, or `.json`.
    #[error("unsupported detection file extension: {extension}")]
    UnsupportedExtension { extension: String },

    /// Required column absent or `properties.kind` mismatch.
    #[error("invalid detection schema in {file}: {detail}")]
    InvalidSchema { file: String, detail: String },

    /// Cell / JSON value failed to parse as the expected type.
    #[error("parse error in {file} row {row}: {detail}")]
    Parse { file: String, row: usize, detail: String },

    /// Timestamp string is missing a timezone offset (naive datetime rejected).
    #[error("timestamp without timezone in {file} row {row}: {value:?}")]
    InvalidTimestamp { file: String, row: usize, value: String },

    /// `intrinsic` / `start_intrinsic` / `end_intrinsic` value outside `[0, 1]`.
    #[error("intrinsic {value} out of range [0, 1] in {file} row {row}")]
    InvalidIntrinsic { file: String, row: usize, value: f64 },

    /// Coordinate-only punctual without a `crs` field.
    #[error("coordinate detection missing crs in {file} row {row}")]
    MissingCrs { file: String, row: usize },

    /// Two punctual detections share the same timestamp but reference different netelements.
    /// Fatal — aborts the run.
    #[error(
        "conflicting detections at {timestamp}: netelement {netelement_a} vs {netelement_b}"
    )]
    ConflictingDetections {
        timestamp: DateTime<FixedOffset>,
        netelement_a: String,
        netelement_b: String,
    },

    /// `t_from > t_to` for a linear detection.
    #[error("t_from > t_to in {file} row {row}")]
    InvalidTimeRange { file: String, row: usize },

    /// `netelement_id` not found in the supplied network. Fatal.
    #[error("unknown netelement {netelement_id} in {file} row {row}")]
    UnknownNetelement {
        file: String,
        row: usize,
        netelement_id: String,
    },

    /// Underlying I/O failure.
    #[error("I/O error reading {file}: {source}")]
    Io {
        file: String,
        #[source]
        source: std::io::Error,
    },
}
```

`DetectionError` is a variant of the existing top-level `TpError` (or equivalent) so CLI/webapp error-handling is uniform.

---

## Module Layout

```text
tp-core/src/
  detections.rs                  # pub mod declaration (no mod.rs — Principle XI)
  detections/
    error.rs                     # DetectionError
    load.rs                      # parse CSV / GeoJSON → Vec<Detection>; format dispatch
    validate.rs                  # schema checks, temporal order, conflict detection
    filter.rs                    # time-range filtering (OutOfTimeRange discard)
    resolve.rs                   # coordinate→netelement resolution, anchor building
```

Public API surface (re-exported from `tp-core::detections`):

```rust
pub use detections::{
    Detection, PunctualDetection, LinearDetection,
    TopologicalLocation, GeographicLocation,
    ResolvedAnchor, DetectionRecord, DetectionKind,
    TimestampOrRange, DetectionStatus, DiscardReason,
    DetectionError,
};

/// Entry point: load, validate, filter, resolve in one call.
/// Returns (Vec<ResolvedAnchor>, Vec<DetectionRecord>) ready for PathConfig injection.
pub fn prepare_detections(
    punctual_file: Option<&Path>,
    linear_file: Option<&Path>,
    network: &RailwayNetwork,
    gnss_window: (DateTime<FixedOffset>, DateTime<FixedOffset>),
    cutoff_distance: f64,
) -> Result<(Vec<ResolvedAnchor>, Vec<DetectionRecord>), DetectionError>;
```

---

## Data Flow

```
CLI flags
  --punctual-detections <FILE>  →  load::parse_punctual(file)   → Vec<PunctualDetection>
  --linear-detections   <FILE>  →  load::parse_linear(file)     → Vec<LinearDetection>
  --cutoff-distance-detections  →  PathConfig.detection_cutoff_distance

validate::validate_all(punctual, linear, network)
  → DetectionError (fatal: InvalidSchema, ConflictingDetections, UnknownNetelement, ...)
  → validated Vec<Detection>

filter::filter_by_time_range(detections, gnss_window)
  → Discarded(OutOfTimeRange) records
  → remaining Vec<Detection>

resolve::resolve(remaining, network, cutoff)
  → coord-only punctual  →  R-tree find_nearest  →  Resolved / Discarded(OutOfReach)
  → topological          →  netelement lookup (already validated)
  → Vec<ResolvedAnchor> + Vec<DetectionRecord>

PathConfig { anchors: Vec<ResolvedAnchor>, detection_cutoff_distance, ..existing }
  ↓ passed to Viterbi
PathResult { ..existing, detection_provenance: Vec<DetectionRecord> }
  ↓ serialized via contracts/path-result-provenance.md
```
