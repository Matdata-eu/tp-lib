# Phase 0 Research — Absolute Train Position Detections

**Feature**: 004-train-detections
**Date**: 2026-05-01

All clarifications from `spec.md` § Clarifications/Session 2026-05-01 are integrated. This document records the technical decisions taken to satisfy each functional requirement. No `NEEDS CLARIFICATION` markers remain.

---

## D1. File format auto-detection by extension (FR-002a)

**Decision**: Detect format from the file extension only.

- `.geojson`, `.json` → GeoJSON parser
- `.csv` → CSV parser
- Any other extension (or no extension) → fatal error before parsing

**Rationale**: User clarification (Q3) chose extension-based detection over MIME sniffing or explicit `--format`. Matches existing GNSS parser conventions (CSV vs GeoJSON dispatch in `tp-core/src/io/`). Predictable, no I/O before dispatch, fails fast.

**Alternatives considered**:
- Content sniffing (first bytes `{` vs delimiter) — rejected: fragile (CSV first row may start with `{`); ambiguous for empty files.
- Explicit `--punctual-detections-format` flag — rejected: more CLI surface area, and the user explicitly preferred auto-detection.

---

## D2. Equivalent CSV ⇄ GeoJSON serialization (FR-002b)

**Decision**: Both formats encode the same logical schema; round-trip equivalence is part of contract tests.

- Punctual detection: required `timestamp` + EITHER `netelement_id` (+ optional `intrinsic`) OR (`lat`, `lon`, `crs`). Optional `id`, `source`, free-form metadata.
- Linear detection: required `t_from`, `t_to`, `netelement_id`. Optional `start_intrinsic`, `end_intrinsic`, `id`, `source`, metadata.

CSV: one row per detection, fixed column names (see contracts/). GeoJSON: `FeatureCollection` of `Feature`s; `properties.kind = "punctual" | "linear"`; geometry is `Point` (punctual coord-only) or omitted (topological / linear).

**Rationale**: Mirrors the existing GNSS dual-format convention. Reusing `geojson` and `csv` crates already in workspace dependencies — no new deps.

**Alternatives**: separate CLI flags per format/kind (rejected — explosion of flags); single combined file holding both kinds (rejected — `kind` discriminator field works cleanly in GeoJSON via `properties.kind` but is awkward in CSV; user clarified separate `--punctual-detections` and `--linear-detections` flags).

---

## D3. Coordinate resolution to netelement (FR-008, FR-009)

**Decision**: Reuse `RailwayNetwork::find_nearest()` (R-tree, O(log n)) with the dedicated `--cutoff-distance-detections` cutoff (default 2.5 m).

- For coord-only punctual detections: project the (lat, lon) onto the nearest netelement; if perpendicular distance > cutoff, **discard** detection with reason `OutOfReach`. Record in provenance.
- The resolved netelement_id and `intrinsic` are stored on the resolved record but only used for anchoring (the original GNSS projection pipeline is untouched per FR-016).

**Rationale**: User clarified (Q4) that the detection cutoff (meters) is **separate** from `--cutoff-distance` because detector locations are surveyed (high-precision) whereas GNSS samples are noisy. The 2.5 m default reflects typical survey precision. Reuse of the R-tree avoids any new spatial-index code.

**Alternatives**:
- Single shared cutoff with GNSS — rejected: forces user to compromise between strict (detections) and loose (GNSS) tolerances.
- Dimensionless / fractional cutoff — rejected: not interpretable for surveyors; meters match other CLI distance flags.

---

## D4. Same-timestamp conflict handling (FR-007a)

**Decision**:
- **Same timestamp + same netelement** (and topologically compatible — if both have `intrinsic`, they round-equal at 1e-6) ⇒ silently deduplicate (keep first, count as duplicate in provenance metadata).
- **Same timestamp + different netelement** (or coord-only resolving to different netelements) ⇒ **fatal**: abort the entire run with a typed `DetectionError::ConflictingDetections { timestamp, netelement_a, netelement_b }`.

**Rationale**: User clarified (Q5) that conflicts are unrecoverable — the absolute-position channel cannot have ambiguous truth. Better to halt than silently pick one.

**Alternatives**:
- Discard both with a warning — rejected: leaves user unaware of an upstream data-quality bug.
- Pick highest priority by `source` — rejected: introduces priority semantics not in scope.

---

## D5. Time-range filtering (FR-010, FR-011)

**Decision**: After validation, filter detections whose timestamp(s) fall outside `[gnss[0].timestamp, gnss[last].timestamp]`.

- Punctual: discard if `timestamp < first || timestamp > last`.
- Linear: discard if `t_to < first || t_from > last`. Clip overlap is **not** performed in this feature (out-of-window means out).

Discarded detections are recorded with reason `OutOfTimeRange { gnss_window }` in provenance.

**Rationale**: Clear, deterministic, no clipping ambiguity. Matches user clarification on window semantics for linear detections.

---

## D6. Anchor injection into Viterbi (FR-013, FR-015)

**Decision**: Two integration points in `tp-core/src/path/viterbi.rs`:

- **Punctual anchor at GNSS index `i`** (mapped to nearest GNSS observation by timestamp, with linear interpolation for "between observations"): replace the candidate set at step `i` with a single forced state for the anchor's netelement+intrinsic. Emission probability for that state = 1.0; all other candidates pruned. Forward variable is initialised exclusively from the forced state.
- **Linear anchor over `[t_from, t_to]`**: at every GNSS index `i` whose timestamp falls in the window, the candidate set at step `i` is filtered to **only those candidates on the anchor's netelement** (intrinsic optionally constrained by `[start_intrinsic, end_intrinsic]` if given). Other candidates pruned.

Anchors are passed to `PathConfig` as `anchors: Vec<ResolvedAnchor>` sorted by GNSS index. The Viterbi loop receives them via a borrow.

**Rationale**:
- Forced-state injection is the standard textbook HMM constrained-decoding technique.
- Linear filtering preserves Viterbi structure (still picks best intrinsic + transitions); restricts only the netelement axis.
- Reduces the candidate state space at anchored steps ⇒ speed ≥ baseline, comfortably within SC-005's 20% budget.

**Alternatives**:
- Boost emission probability without forcing — rejected: not a hard anchor; violates FR-013/015.
- Pre-segment the GNSS series at anchor boundaries and run independent Viterbi per segment — rejected: loses transition probability across anchor boundaries; complicates path reconstruction.

---

## D7. Anchor mapping to GNSS indices (FR-010 corollary)

**Decision**: For each anchor timestamp `t_a`, locate the GNSS index `i*` minimising `|gnss[i].timestamp − t_a|`. Punctual anchor injected at `i*`. Linear anchor active for all `i` such that `gnss[i].timestamp ∈ [t_from, t_to]`.

**Rationale**: Detections are absolute references; the Viterbi works on the GNSS observation sequence. Snapping to nearest index avoids inserting synthetic observations and keeps the index-based path output (`gnss_start_index` / `gnss_end_index` in `AssociatedNetElement`) consistent.

**Alternatives**: insert a virtual GNSS observation at `t_a` — rejected: changes index semantics for downstream consumers.

---

## D8. Intrinsic fields are informational only (FR-014)

**Decision**: `intrinsic`, `start_intrinsic`, `end_intrinsic` are validated (must be in `[0, 1]`) and recorded in provenance, but **not** used by the Viterbi or path reconstruction in this feature. Reserved for a future feature (likely on-segment positioning refinement).

**Rationale**: User clarified (Q1) to defer intrinsic-aware anchoring to keep this feature focused. Anchors only constrain the **netelement** axis, not the position along it.

---

## D9. Provenance record shape (FR-017, FR-020)

**Decision**: New struct `DetectionRecord` on `PathResult.detection_provenance: Vec<DetectionRecord>`:

```rust
pub struct DetectionRecord {
    pub source_file: String,        // path or "<inline>"
    pub source_row: Option<usize>,  // CSV row or GeoJSON feature index
    pub kind: DetectionKind,        // Punctual | Linear
    pub timestamp: TimestampOrRange,
    pub status: DetectionStatus,    // Applied { netelement_id } | Discarded { reason } | Resolved { netelement_id, distance_m }
    pub id: Option<String>,
    pub source: Option<String>,
    pub metadata: HashMap<String, String>,
}
```

`DetectionStatus::Applied` implies the detection successfully constrained Viterbi. `Discarded` carries a reason enum (`OutOfTimeRange`, `OutOfReach`, `UnknownNetelement`, `IntrinsicOutOfRange`, `DuplicateOfPriorDetection`). Conflicting detections never reach this list — they are fatal (D4).

**Rationale**: Mirrors existing `PathMetadata`/`DebugInfo` patterns in `tp-core`. Easy to serialize for both CLI summary and webapp API.

---

## D10. CLI summary line (FR-020)

**Decision**: After path calculation, the CLI emits a one-line summary to stderr:

```
detections: 23 applied, 4 discarded (2 out-of-range, 1 out-of-reach, 1 duplicate)
```

JSON details (full provenance) are written when `--json-output` is in effect (existing flag).

**Rationale**: Operator-friendly default; machine-readable available on demand. Aligns with Principle II.

---

## D11. Webapp rendering (FR-021..FR-024)

**Decision**:
- New endpoint `GET /api/detections` returns `{ punctual: [...], linear: [...], discarded: [...] }` from the persisted `PathResult`.
- Frontend (Leaflet): a "Detections" layer toggle. Punctual = circle markers (filled = applied, hollow with X = discarded); Linear = highlighted polyline along the netelement (semi-transparent overlay). Discarded items rendered with a muted color/dashed style.
- Click on any marker / highlight ⇒ details panel (right sidebar) displays: id, source, timestamp(s), status, reason (if discarded), resolved netelement_id + intrinsic, raw metadata key/value table.
- **Read-only**: no upload, edit, delete, or filter UI in this feature (per user clarification Q2).

**Rationale**: Smallest webapp surface that satisfies operator review needs; avoids edit/upload complexity outside the CLI's authoritative path. Reuses existing path-review layout patterns in `tp-webapp/static/`.

**Alternatives**: in-browser detection upload — rejected: bypasses CLI provenance; edit/delete UI — rejected: not in user-clarified scope.

---

## D12. Test strategy & performance benchmarks

**Decision**:
- Unit tests per parser, validator, filter, resolver (in respective files / `#[cfg(test)]` modules).
- Integration tests in `tp-core/tests/detections_*.rs` covering each user story (US1 punctual topo, US2 punctual coord, US3 linear) end-to-end.
- Contract tests in `tp-cli/tests/cli_detections.rs` (CLI flags + summary).
- Webapp contract tests in `tp-webapp/tests/api_detections.rs`.
- Criterion benchmark `tp-core/benches/detections_overhead.rs` measuring SC-005 (≤20% overhead at 10k GNSS × 1k detections).

**Rationale**: Aligns with Principle IV (TDD) and V (full coverage). Existing benchmark harness patterns reused.

---

## Summary — All Clarifications Resolved

| Clarification | Resolved by |
|---|---|
| Q1: intrinsic anchoring deferred | D8 |
| Q2: webapp scope (read-only) | D11 |
| Q3: extension-based format | D1 |
| Q4: meters cutoff, separate flag | D3 |
| Q5: same-timestamp conflict fatal | D4 |

No `NEEDS CLARIFICATION` remain.
