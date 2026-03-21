# REST API Contract: Train Path Review Webapp

**Crate**: `tp-webapp`  
**Feature**: `003-path-review-webapp`  
**Base URL**: `http://127.0.0.1:<port>` (port defaults to 8765; increments on conflict)  
**Protocol**: HTTP/1.1. All request and response bodies are `application/json` unless noted.  
**Authentication**: None (localhost-only tool, single session)

---

## Stability Contract

This API is considered **internal** to the tp-lib workspace. It is NOT a public library API and is not subject to semantic versioning. Changes must be co-ordinated between `tp-webapp` (server) and `tp-webapp/static/app.js` (client). The contract exists to stabilise the interface for testing purposes.

---

## Shared Types

### `ErrorResponse`

All endpoints that return a non-2xx status use this body:

```json
{ "ok": false, "error": "<human-readable error message>" }
```

### `SuccessAck`

Minimal success acknowledgement used by POST endpoints:

```json
{ "ok": true }
```

---

## Endpoints

### `GET /`

Serves the single-page application shell (`index.html`).

| Property | Value |
|----------|-------|
| Method | GET |
| Path | `/` |
| Authentication | None |
| Request body | None |
| Response status | 200 |
| Response content-type | `text/html; charset=utf-8` |
| Response body | Contents of `static/index.html` (embedded via rust-embed) |

All other static assets (`/app.js`, `/style.css`, `/leaflet/*`) are similarly served from the embedded `static/` directory at their respective paths.

---

### `GET /api/network`

Returns the complete railway network as a GeoJSON `FeatureCollection`.  
Each feature represents one netelement. The `in_path`, `origin`, and `confidence` properties reflect the **current** path state at the time of the request.

| Property | Value |
|----------|-------|
| Method | GET |
| Path | `/api/network` |
| Authentication | None |
| Request body | None |
| Response status | 200 |
| Response content-type | `application/json` |
| Response body | GeoJSON `FeatureCollection` (see schema below) |

#### Response Schema

```
FeatureCollection {
  type: "FeatureCollection"
  features: Feature[]
}

Feature {
  type: "Feature"
  geometry: {
    type: "LineString"
    coordinates: [number, number][]   // [lon, lat] pairs in WGS 84
  }
  properties: {
    netelement_id: string             // unique identifier from network file
    in_path:       boolean            // true if segment is in the current path
    origin:        "algorithm"        // present only when in_path == true
                 | "manual"
                 | null               // null when in_path == false
    confidence:    number | null      // 0.0–1.0; null when in_path == false
  }
}
```

#### Example Response (abbreviated)

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": { "type": "LineString", "coordinates": [[4.35, 50.85], [4.36, 50.86]] },
      "properties": { "netelement_id": "NE001", "in_path": true, "origin": "algorithm", "confidence": 0.87 }
    },
    {
      "type": "Feature",
      "geometry": { "type": "LineString", "coordinates": [[4.36, 50.86], [4.37, 50.87]] },
      "properties": { "netelement_id": "NE002", "in_path": false, "origin": null, "confidence": null }
    }
  ]
}
```

#### Error Cases

| Status | Condition |
|--------|-----------|
| 500 | State lock poisoned (internal server error) |

---

### `GET /api/path`

Returns the current ordered path as a JSON object.

| Property | Value |
|----------|-------|
| Method | GET |
| Path | `/api/path` |
| Authentication | None |
| Request body | None |
| Response status | 200 |
| Response content-type | `application/json` |

#### Response Schema

```
{
  segments: PathSegment[]
  overall_probability: number       // 0.0–1.0
  mode: "standalone" | "integrated"
}

PathSegment {
  netelement_id:    string
  probability:      number          // 0.0–1.0 (always 1.0 for manual segments)
  start_intrinsic:  number          // 0.0–1.0
  end_intrinsic:    number          // 0.0–1.0
  gnss_start_index: integer         // 0 for manual segments
  gnss_end_index:   integer         // 0 for manual segments
  origin:           "algorithm" | "manual"
  path_index:       integer         // 0-based position in ordered path
}
```

#### Example Response

```json
{
  "segments": [
    {
      "netelement_id": "NE001", "probability": 0.87,
      "start_intrinsic": 0.0, "end_intrinsic": 1.0,
      "gnss_start_index": 0, "gnss_end_index": 12,
      "origin": "algorithm", "path_index": 0
    },
    {
      "netelement_id": "NE003", "probability": 1.0,
      "start_intrinsic": 0.0, "end_intrinsic": 1.0,
      "gnss_start_index": 0, "gnss_end_index": 0,
      "origin": "manual", "path_index": 1
    }
  ],
  "overall_probability": 0.89,
  "mode": "standalone"
}
```

---

### ~~`PUT /api/path`~~ *(superseded — see `POST /api/path/add` and `POST /api/path/remove`)*

> **Implementation note**: The original design sent the full ordered segment list from the client on every edit. During implementation this was replaced by two granular endpoints that trigger server-side snap insertion directly. `PUT /api/path` is no longer used by the browser frontend.

---

### `POST /api/path/add`

Adds a single netelement to the in-memory path. The server calls `edit::add_segment()` which performs snap insertion using netrelations topology (FR-009). The browser does **not** need to manage segment ordering.

| Property | Value |
|----------|-------|
| Method | POST |
| Path | `/api/path/add` |
| Authentication | None |
| Request content-type | `application/json` |
| Request body | See schema below |
| Response status | 200 (success) / 404 (netelement not found) / 500 (internal error) |
| Response content-type | `application/json` |

#### Request Schema

```json
{ "netelement_id": "NE001" }
```

#### Response (200 OK)

```json
{ "ok": true }
```

#### Response (404 Not Found)

```json
{ "ok": false, "error": "netelement NE999 not found in loaded network" }
```

After a successful response, the browser refreshes both `GET /api/path` and `GET /api/network` to reflect the updated state.

---

### `POST /api/path/remove`

Removes a single netelement from the in-memory path. The server calls `edit::remove_segment()`.

| Property | Value |
|----------|-------|
| Method | POST |
| Path | `/api/path/remove` |
| Authentication | None |
| Request content-type | `application/json` |
| Request body | See schema below |
| Response status | 200 (success) / 500 (internal error) |
| Response content-type | `application/json` |

#### Request Schema

```json
{ "netelement_id": "NE001" }
```

#### Response (200 OK)

```json
{ "ok": true }
```

After a successful response, the browser refreshes both `GET /api/path` and `GET /api/network` to reflect the updated state.

---

### `POST /api/save`

Writes the current in-memory path to the output file. **Standalone mode only.** The server remains running after the write.

| Property | Value |
|----------|-------|
| Method | POST |
| Path | `/api/save` |
| Authentication | None |
| Request body | `{}` or empty |
| Response status | 200 (written) / 409 (wrong mode) / 500 (I/O error) |
| Response content-type | `application/json` |

#### Response (200 OK)

```json
{ "ok": true, "path": "/home/user/reviewed_path.csv" }
```

The `path` field contains the absolute path of the file that was written.

#### Response (409 Conflict — integrated mode)

```json
{ "ok": false, "error": "save is not available in integrated mode; use /api/confirm instead" }
```

#### Response (500 Internal Server Error — I/O failure)

```json
{ "ok": false, "error": "failed to write output file: permission denied" }
```

---

### `POST /api/confirm`

Signals the CLI process to continue pipeline execution with the current in-memory path. **Integrated mode only.** After this call succeeds, the server shuts down gracefully.

| Property | Value |
|----------|-------|
| Method | POST |
| Path | `/api/confirm` |
| Authentication | None |
| Request body | `{}` or empty |
| Response status | 200 (confirmed) / 409 (wrong mode or already confirmed) |
| Response content-type | `application/json` |

#### Response (200 OK)

```json
{ "ok": true }
```

The response is sent **before** the server shuts down — the browser receives the acknowledgement.

#### Response (409 Conflict — standalone mode)

```json
{ "ok": false, "error": "confirm is not available in standalone mode; use /api/save instead" }
```

#### Response (409 Conflict — already confirmed)

```json
{ "ok": false, "error": "already confirmed" }
```

---

### `POST /api/abort`

Signals the CLI process to abort and exit with a non-zero exit code. **Integrated mode only.** After this call succeeds, the server shuts down gracefully.

| Property | Value |
|----------|-------|
| Method | POST |
| Path | `/api/abort` |
| Authentication | None |
| Request body | `{}` or empty |
| Response status | 200 (aborting) / 409 (wrong mode) |
| Response content-type | `application/json` |

#### Response (200 OK)

```json
{ "ok": true }
```

The CLI will print a cancellation message to stderr and exit with exit code 1.

#### Response (409 Conflict — standalone mode)

```json
{ "ok": false, "error": "abort is not available in standalone mode" }
```

---

## Mode × Endpoint Matrix

| Endpoint | Standalone | Integrated |
|----------|-----------|-----------|
| `GET /` | ✅ | ✅ |
| `GET /api/network` | ✅ | ✅ |
| `GET /api/path` | ✅ | ✅ |
| `POST /api/path/add` | ✅ | ✅ |
| `POST /api/path/remove` | ✅ | ✅ |
| `POST /api/save` | ✅ | ❌ 409 |
| `POST /api/confirm` | ❌ 409 | ✅ |
| `POST /api/abort` | ❌ 409 | ✅ |

---

## Server Lifecycle

```
startup:
  1. Bind to 127.0.0.1:<port>  (try 8765..8774)
  2. Print URL to terminal
  3. Attempt to open browser (ignore failure, URL already printed)
  4. Begin accepting requests

standalone shutdown:
  - Only on CLI process termination (Ctrl+C / SIGINT)

integrated shutdown sequence:
  POST /confirm or POST /abort
  → Response 200 sent to browser
  → CancellationToken triggered
  → Axum server task ends
  → oneshot result delivered to CLI await point
  → CLI continues (confirm) or exits non-zero (abort)
```
