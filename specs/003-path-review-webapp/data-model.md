# Data Model: Train Path Review Webapp

**Phase**: 1 — Design & Contracts  
**Feature**: `003-path-review-webapp`  
**Depends on**: [research.md](research.md), [spec.md](spec.md)

---

## tp-core Changes

### New: `PathOrigin` enum

**File**: `tp-core/src/models/path_origin.rs`

```rust
/// Indicates whether a path segment was selected by the algorithm or manually added by the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PathOrigin {
    /// Segment was selected by the path calculation algorithm (default for backward compatibility)
    #[default]
    Algorithm,
    /// Segment was manually added by a user in the review webapp
    Manual,
}
```

**Notes**:
- `#[default]` ensures existing CSV files without an `origin` column deserialize as `Algorithm`
- `rename_all = "lowercase"` serializes as `"algorithm"` / `"manual"` in CSV and JSON
- Added to `tp-core/src/models.rs` pub export: `pub use path_origin::PathOrigin;`

---

### Extended: `AssociatedNetElement`

**File**: `tp-core/src/models/associated_net_element.rs` — add one field

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssociatedNetElement {
    // --- existing fields (unchanged) ---
    pub netelement_id: String,
    pub probability: f64,
    pub start_intrinsic: f64,
    pub end_intrinsic: f64,
    pub gnss_start_index: usize,
    pub gnss_end_index: usize,

    // --- new field ---
    /// Provenance: whether this segment was placed by the algorithm or by a human reviewer.
    /// Defaults to `Algorithm` for backward-compatible deserialization of existing CSV files.
    #[serde(default)]
    pub origin: PathOrigin,
}
```

**Backward compatibility**: serde's `#[serde(default)]` means any existing CSV or JSON that omits `origin` will deserialize without error, treating the segment as `Algorithm`-selected. The new field is appended as an extra column in CSV output, which is forwards-compatible with the existing `parse_trainpath_csv` reader (it uses `csv::Reader` with `flexible = true` or header-matching, not positional column indexing).

**Manually-added segment invariants**:
- `probability`: always `1.0` (user is certain)
- `origin`: always `PathOrigin::Manual`
- `gnss_start_index` / `gnss_end_index`: both `0` (no associated GNSS positions; ignore when `origin == Manual`)
- `start_intrinsic`: `0.0` (full segment traversal assumed)
- `end_intrinsic`: `1.0`

---

## tp-webapp New Types

### `WebAppState`

**File**: `tp-webapp/src/server/state.rs`

```rust
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use tp_lib_core::{GnssPosition, RailwayNetwork, TrainPath};

pub struct WebAppState {
    /// Full railway network (netelements + netrelations), loaded at startup
    pub network: RailwayNetwork,

    /// Current train path being reviewed. Modified in place by PUT /api/path.
    pub path: TrainPath,

    /// Optional GNSS positions for overlay display (not editable)
    pub gnss: Option<Vec<GnssPosition>>,

    /// Operational mode — determines which UI buttons are shown and which
    /// endpoints are active.
    pub mode: AppMode,

    /// Output file path for standalone save (None = derive default name)
    pub output_path: Option<PathBuf>,

    /// One-shot sender used in integrated mode. Consumed by POST /confirm or
    /// POST /abort. `None` in standalone mode.
    pub confirm_tx: Option<oneshot::Sender<ConfirmResult>>,
}
```

### `AppMode`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Launched via `tp-cli webapp` — save outputs to file, server stays alive
    Standalone,
    /// Launched via `tp-cli --review` — confirm/abort signals CLI pipeline
    Integrated,
}
```

### `ConfirmResult`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmResult {
    /// User confirmed the path; pipeline should continue
    Confirmed,
    /// User aborted; pipeline should exit non-zero
    Aborted,
}
```

### `PathEditRequest`

**File**: `tp-webapp/src/server/routes.rs`

```rust
/// Request body for POST /api/path/add and POST /api/path/remove
#[derive(Deserialize)]
pub struct PathEditRequest {
    pub netelement_id: String,
}
```

Used by both `POST /api/path/add` and `POST /api/path/remove`. The server resolves the actual segment data and ordering via `edit::add_segment()` / `edit::remove_segment()` using the loaded network; the browser only needs to supply the target netelement ID.

> **Note**: `PUT /api/path` (full segment list replacement) from the original design was superseded by these two granular endpoints. See `contracts/api.md` for details.

---

## REST API JSON Shapes

### `GET /api/network` — Response

Returns the complete network as GeoJSON `FeatureCollection`. Each feature is a netelement.

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.35, 50.85], [4.36, 50.86]]
      },
      "properties": {
        "netelement_id": "NE001",
        "in_path": false,
        "origin": "algorithm",
        "confidence": null
      }
    },
    {
      "type": "Feature",
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.36, 50.86], [4.37, 50.87]]
      },
      "properties": {
        "netelement_id": "NE002",
        "in_path": true,
        "origin": "manual",
        "confidence": 1.0
      }
    }
  ]
}
```

**Property fields**:
| Field | Type | Description |
|-------|------|-------------|
| `netelement_id` | `string` | Unique identifier from network file |
| `in_path` | `boolean` | Whether this segment is currently in the reviewed path |
| `origin` | `"algorithm" \| "manual" \| null` | Provenance (null when `in_path == false`) |
| `confidence` | `number \| null` | Probability score 0.0–1.0 (null when `in_path == false`) |

---

### `GET /api/path` — Response

Returns the current ordered path.

```json
{
  "segments": [
    {
      "netelement_id": "NE001",
      "probability": 0.87,
      "start_intrinsic": 0.0,
      "end_intrinsic": 1.0,
      "gnss_start_index": 0,
      "gnss_end_index": 12,
      "origin": "algorithm",
      "path_index": 0
    },
    {
      "netelement_id": "NE002",
      "probability": 1.0,
      "start_intrinsic": 0.0,
      "end_intrinsic": 1.0,
      "gnss_start_index": 0,
      "gnss_end_index": 0,
      "origin": "manual",
      "path_index": 1
    }
  ],
  "overall_probability": 0.89,
  "mode": "standalone"
}
```

**Top-level fields**:
| Field | Type | Description |
|-------|------|-------------|
| `segments` | `array` | Ordered `AssociatedNetElement` objects with `path_index` appended |
| `overall_probability` | `number` | Length-weighted average probability |
| `mode` | `"standalone" \| "integrated"` | Current app mode |

---

### ~~`PUT /api/path`~~ — ~~superseded~~

> **Post-implementation note**: `PUT /api/path` was superseded by two granular POST endpoints (see below). The client-managed full-list approach was abandoned because it required the browser to know the correct snap-insertion position, duplicating server-side logic. See `contracts/api.md` and `plan.md § Post-Implementation Changes` for details.

---

### `POST /api/path/add` — Request

Adds a single netelement to the path using server-side snap insertion.

```json
{ "netelement_id": "NE003" }
```

**Response** (200 OK):

```json
{ "ok": true, "segments_count": 3 }
```

**Error** (404 Not Found — netelement not in network):

```json
{ "ok": false, "error": "netelement not found: NE999" }
```

---

### `POST /api/path/remove` — Request

Removes a single netelement from the path.

```json
{ "netelement_id": "NE002" }
```

**Response** (200 OK):

```json
{ "ok": true, "segments_count": 2 }
```

---

### `POST /api/save` — Request (standalone mode only)

Triggers writing the current path to the output file.

```json
{}
```

**Response** (200 OK):

```json
{ "ok": true, "path": "/home/user/modified_path.csv" }
```

**Error** (409 Conflict — called in integrated mode):

```json
{ "ok": false, "error": "save is not available in integrated mode; use confirm instead" }
```

---

### `POST /api/confirm` — Request (integrated mode only)

Signals the CLI to continue pipeline execution with the current path.

```json
{}
```

**Response** (200 OK):

```json
{ "ok": true }
```

**Error** (409 Conflict — called in standalone mode):

```json
{ "ok": false, "error": "confirm is not available in standalone mode; use save instead" }
```

**Error** (409 Conflict — already confirmed):

```json
{ "ok": false, "error": "already confirmed" }
```

---

### `POST /api/abort` — Request (integrated mode only)

Signals the CLI to abort and exit non-zero.

```json
{}
```

**Response** (200 OK):

```json
{ "ok": true }
```

**Error** (409 Conflict — called in standalone mode):

```json
{ "ok": false, "error": "abort is not available in standalone mode" }
```

---

## Entity Relationships

```
RailwayNetwork ─────────────────────────────────────┐
  ├── Vec<Netelement>                                │ loaded at startup
  └── Vec<NetRelation>                               │ (existing tp-core types)
                                                     │
WebAppState ◄────────────────────────────────────── arc shared across handlers
  ├── network: RailwayNetwork
  ├── path: TrainPath
  │     └── segments: Vec<AssociatedNetElement>
  │           └── origin: PathOrigin  ← NEW field
  ├── gnss: Option<Vec<GnssPosition>>
  ├── mode: AppMode
  ├── output_path: Option<PathBuf>
  └── confirm_tx: Option<oneshot::Sender<ConfirmResult>>

AppMode           ConfirmResult         PathOrigin
  Standalone        Confirmed             Algorithm (default)
  Integrated        Aborted               Manual
```
