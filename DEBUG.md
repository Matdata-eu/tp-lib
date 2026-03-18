# Debug Output Reference

The `--debug` flag on `tp-cli` writes five GeoJSON files to a `debug/` subdirectory next to the output file (or to a custom directory specified via `--debug-output-dir`).

All files use **EPSG:4326 (WGS84)** coordinates and are compatible with any GeoJSON viewer such as QGIS, geojson.io, or VS Code with a GeoJSON extension.

---

## Quick Start

```bash
tp-cli calculate-path \
  --gnss positions.csv \
  --crs EPSG:4326 \
  --network network.geojson \
  --output result/path.geojson \
  --debug
# Debug files are written to result/debug/
```

To write debug files to a custom directory:

```bash
tp-cli calculate-path ... --output result/path.geojson --debug --debug-output-dir my-debug-dir
```

---

## Output Files

Files are numbered to reflect the order in which the HMM algorithm processes data.
They are only written when the corresponding algorithm phase produced data — in a degenerate case (e.g. no candidates found at all) one or more files may be absent.

| File | HMM phase | What it shows |
|------|-----------|---------------|
| `01_emission_probabilities.geojson` | Observation model | Per-candidate emission probabilities for every GNSS position |
| `02_transition_probabilities.geojson` | Transition model | All feasible candidate-pair links across consecutive GNSS steps, with probability scores |
| `03_viterbi_trace.geojson` | Viterbi decoding | Link from each GNSS point to the projected location on the chosen netelement |
| `04_candidate_netelements.geojson` | State space | All netelements that were ever a candidate, scored and flagged |
| `05_selected_path.geojson` | Final result | Only the netelements that form the decoded path |

---

## File Descriptions

### `01_emission_probabilities.geojson` — Emission probabilities

**Geometry**: `LineString` from the raw GNSS point to its projected location on the candidate netelement.

Each feature represents one GNSS position × netelement candidate pair.

**FeatureCollection properties**

| Property | Value |
|----------|-------|
| `phase` | `1` |
| `description` | `"HMM emission probabilities: links from each GNSS position to its candidate netelements"` |

**Feature properties**

| Property | Type | Description |
|----------|------|-------------|
| `step` | integer | GNSS position index (0-based, matches row order in input CSV) |
| `netelement_id` | string | Candidate netelement identifier |
| `emission_probability` | float [0–1] | Combined emission probability (distance × heading) |
| `distance_probability` | float [0–1] | Distance component only — exponential decay of `distance_m` |
| `distance_m` | float | Absolute distance from GNSS point to projected point (metres) |
| `heading_probability` | float [0–1] | Heading component (omitted when heading data is unavailable) |
| `heading_difference_deg` | float | Absolute difference between GNSS bearing and track bearing (degrees; omitted when unavailable) |
| `status` | string | `"selected"` — chosen by Viterbi at this step; `"candidate"` — within cutoff but not selected; `"rejected"` — outside probability threshold |

**How to interpret**

- Short, nearly-vertical lines with high `emission_probability` are the best-fitting candidates.
- A position with many `"rejected"` features and no `"selected"` feature indicates the algorithm had no viable candidate at that step — look for a gap in network coverage or a large `distance_m`.
- Compare `distance_probability` and `heading_probability` to understand whether distance or heading is dominating the score.

---

### `02_transition_probabilities.geojson` — Transition probabilities

**Geometry**: `LineString` from the projected location of the preceding candidate to the projected location of the succeeding candidate. Features where either candidate's geometry is unavailable have `null` geometry.

Each feature represents one feasible (from-candidate, to-candidate) pair across two consecutive GNSS steps — every pair for which the Viterbi algorithm computed a non-zero transition probability.

**FeatureCollection properties**

| Property | Value |
|----------|---------|
| `phase` | `2` |
| `description` | `"HMM transition probabilities: feasible candidate-pair links across consecutive GNSS steps"` |

**Feature properties**

| Property | Type | Description |
|----------|------|-------------|
| `from_step` | integer | Index of the earlier GNSS position (0-based) |
| `to_step` | integer | Index of the later GNSS position (always `from_step + 1`) |
| `from_netelement_id` | string | Candidate netelement at the earlier step |
| `to_netelement_id` | string | Candidate netelement at the later step |
| `transition_probability` | float [0–1] | Linear-scale transition probability for this pair (`exp(-|route_dist - gc_dist| / beta)`); `1.0` when both candidates are on the same netelement |
| `is_viterbi_chosen` | boolean | `true` if this pair is on the decoded Viterbi path |

**How to interpret**

- Pairs with `is_viterbi_chosen = true` form the backbone of the selected path; all other features show the alternatives that were considered but discarded.
- High `transition_probability` combined with `is_viterbi_chosen = false` means the pair had a good transition score but a weaker combined score (emission × transition) than the winning pair.
- Low probabilities across all pairs at a given step transition indicate a topology bottleneck or a large detour in network distance relative to great-circle distance — consider lowering `--beta`.
- Pairs where `from_netelement_id == to_netelement_id` have `transition_probability = 1.0`; the train stayed on the same netelement between the two observations.

---

### `03_viterbi_trace.geojson` — Viterbi decoding trace

**Geometry**: `LineString` from the raw GNSS point to its projected location on the netelement chosen by the Viterbi algorithm. Features with no matching candidate have `null` geometry and still appear in attribute tables.

Each feature represents one step of the Viterbi decoding — one per GNSS observation.

**FeatureCollection properties**

| Property | Value |
|----------|-------|
| `phase` | `3` |
| `description` | `"HMM Viterbi decoding trace: links from each GNSS position to the chosen netelement"` |

**Feature properties**

| Property | Type | Description |
|----------|------|-------------|
| `step` | integer | Observation index (0-based, aligns with `step` in file 01) |
| `netelement_id` | string | Netelement chosen at this step |
| `decision_type` | string | Type of Viterbi event: `"viterbi_init"` or `"viterbi_transition"` |
| `selected_probability` | float | Emission probability of the chosen candidate (omitted when unavailable) |
| `alternatives_count` | integer | Total number of candidate states considered at this step |
| `reason` | string | Human-readable rationale for the selection |

**Decision types (`decision_type`)**

| Value | When it appears |
|-------|----------------|
| `viterbi_init` | The first state of a Viterbi subsequence — the algorithm is starting (or re-starting after a gap) and selects the best initial state from the candidates at that position. |
| `viterbi_transition` | All subsequent states — the algorithm is extending the path from the previous step, picking the state that maximises the joint probability of the path so far (emission × transition). |

**How to interpret**

- Steps where the same netelement appears for many consecutive observations indicate a stable, confident stretch of path.
- A sudden jump to a different netelement may point to a topology gap, a network inaccuracy, or a tunnelling event.
- `alternatives_count = 1` means there was no real choice — the algorithm could only pick one candidate.
- Steps with `null` geometry are still shown in attribute tables; filter on `netelement_id` to cross-reference with the network.

---

### `04_candidate_netelements.geojson` — Candidate netelements

**Geometry**: `LineString` along each netelement's track centreline.

Each feature represents one netelement that was in the candidate pool for at least one GNSS position. This is the full HMM state space.

**FeatureCollection properties**

| Property | Value |
|----------|-------|
| `phase` | `4` |
| `description` | `"HMM candidate netelements: all states considered during Viterbi decoding"` |

**Feature properties**

| Property | Type | Description |
|----------|------|-------------|
| `netelement_id` | string | Netelement identifier |
| `avg_emission_probability` | float [0–1] | Average emission probability across all GNSS positions for which this netelement was a candidate |
| `position_count` | integer | Number of GNSS positions for which this netelement appeared as a candidate |
| `in_viterbi_path` | boolean | `true` if this netelement is part of the decoded path |
| `is_bridge` | boolean | `true` if this segment was inserted as a topological bridge (no direct GNSS evidence) |

**How to interpret**

- Use `in_viterbi_path` to visually compare the selected path against the full candidate set.
- Netelements with high `avg_emission_probability` but `in_viterbi_path = false` are strong candidates that were ruled out by transition probabilities — a sign the topology favoured a different route.
- `is_bridge = true` netelements connect disjoint sections of the path through the topology graph. They carry `avg_emission_probability = 0` and `position_count = 0`.

---

### `05_selected_path.geojson` — Selected path

**Geometry**: `LineString` along each netelement's track centreline.

This is a filtered subset of file 04 — only the netelements where `in_viterbi_path = true`. It is the spatial representation of the final decoded train path.

**FeatureCollection properties**

| Property | Value |
|----------|-------|
| `phase` | `5` |
| `description` | `"HMM selected path: netelements in the final Viterbi-decoded path"` |

**Feature properties**

| Property | Type | Description |
|----------|------|-------------|
| `netelement_id` | string | Netelement identifier |
| `avg_emission_probability` | float [0–1] | Average emission probability (0 for bridge segments) |
| `position_count` | integer | Number of GNSS positions associated with this netelement (0 for bridges) |
| `is_bridge` | boolean | `true` for topological bridge segments |

**How to interpret**

- This is the primary output to verify in QGIS or a similar tool.
- Bridge segments (`is_bridge = true`) indicate the route passed through netelements not covered by any GNSS position. Check whether these correspond to known tunnels, flyovers, or network gaps.
- A path with many bridge hops or a low overall `avg_emission_probability` may indicate a poor match — reconsider `--cutoff-distance`, `--distance-scale`, or `--heading-scale`.

---

## Typical Debugging Workflow

1. **Start with `05_selected_path.geojson`** — load it alongside the GNSS trace in QGIS to confirm the path follows the train route visually.
2. **If the path diverges**, open `04_candidate_netelements.geojson` and filter on `in_viterbi_path = false` to see which netelements were available but not selected.
3. **For a specific problematic position**, open `01_emission_probabilities.geojson` and filter on the `step` value. Check whether the correct netelement has a competitive `emission_probability`. If all probabilities are very low, the GNSS point may be too far from the network.
4. **Step through `03_viterbi_trace.geojson`** ordered by `step` to follow the algorithm's decision sequence. A sudden netelement change accompanied by a low `selected_probability` signals a weak transition. The LineString geometry connects each raw GNSS point directly to its projected location, making snap errors immediately visible.
5. **Inspect `02_transition_probabilities.geojson`** at the problematic step transition. Filter on `from_step` / `to_step` and compare `transition_probability` values across all candidate pairs. If the chosen pair has a noticeably lower probability than the discarded alternatives, the transition model may be penalising the correct route — try increasing `--beta`.

---

## Algorithm Background

The path calculation uses a **Hidden Markov Model (HMM)** following the map-matching approach of Newson & Krumm (2009):

- **States**: candidate netelements within `--cutoff-distance` of each GNSS position
- **Observations**: GNSS positions 
- **Emission probability**: product of distance probability and heading probability, both modelled as exponential decays
- **Transition probability**: based on the ratio of great-circle distance to network shortest-path distance (parameterised by `--beta`)
- **Decoding**: log-space Viterbi algorithm for global optimality

Key tuning parameters that affect the debug output:

| Parameter | Effect on debug files |
|-----------|----------------------|
| `--cutoff-distance` | Controls how many candidates appear in files 01 and 04 |
| `--distance-scale` | Affects `distance_probability` in file 01 |
| `--heading-scale` | Affects `heading_probability` in file 01 |
| `--probability-threshold` | Candidates below this threshold appear as `"rejected"` in file 01 |
| `--max-candidates` | Caps `alternatives_count` visible in file 03 |
| `--beta` | Transition probability scale; influences which path Viterbi picks (visible in files 02–05) |
