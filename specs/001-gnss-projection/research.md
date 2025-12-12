# Phase 0 Research: GNSS Track Axis Projection

**Date**: 2025-12-12 | **Branch**: `001-gnss-projection`

## Purpose

Resolve technical unknowns before designing data models and API contracts. This research validates technology choices, explores library capabilities, and identifies potential integration challenges.

---

## Research Questions

### 1. Spatial Indexing Strategy

**Question**: Which Rust R-tree implementation provides best performance for nearest-netelement queries?

**Options**:
- `rstar` crate: Robust spatial indexing with R*-tree algorithm
- `rtree` crate: Simpler implementation, less actively maintained
- `spade` crate: Spatial data structures including R-tree

**Research Needed**:
- [ ] Compare crate popularity (crates.io downloads, GitHub stars)
- [ ] Benchmark nearest-neighbor query performance (1000 points vs 500 netelements)
- [ ] Evaluate API ergonomics for bounding box queries
- [ ] Check compatibility with `geo` crate types (Point, LineString)

**Decision Criteria**: >100k downloads/month, active maintenance, native `geo` integration, O(log n) query guarantees

---

### 2. Arrow Interop: CSV → Arrow → Polars

**Question**: How to efficiently parse CSV GNSS data into Arrow columnar format, then process with Polars DataFrames?

**Flow**: `CSV file` → `arrow::csv::Reader` → `RecordBatch` → `polars::DataFrame` → `Vec<GnssPosition>`

**Research Needed**:
- [ ] Explore `arrow-csv` crate for CSV parsing with schema inference
- [ ] Test Arrow → Polars conversion with `polars::prelude::DataFrame::from_arrow`
- [ ] Measure memory overhead: CSV (text) vs Arrow (columnar) vs Rust structs
- [ ] Identify zero-copy opportunities (e.g., reading Arrow IPC instead of CSV)

**Decision Criteria**: <10% memory overhead vs naive CSV parsing, <100ms for 1000 records

---

### 3. GeoJSON CRS Handling

**Question**: How to handle CRS in GeoJSON railway network files?

**Standard**: RFC 7946 mandates WGS84 (EPSG:4326) for GeoJSON, prohibits `crs` property

**Ambiguity**: Legacy GeoJSON 2008 spec allowed custom `crs` property

**Research Needed**:
- [ ] Test `geojson` crate parsing with/without `crs` property
- [ ] Determine default CRS behavior (assume WGS84 per RFC 7946?)
- [ ] Investigate validation: reject non-WGS84 CRS or transform on-the-fly?
- [ ] Check if Belgian railway networks use legacy `crs` property

**Decision Criteria**: Strict RFC 7946 compliance (WGS84 only), reject legacy `crs`, clear error messages

---

### 4. Projection Algorithm: Point-to-LineString

**Question**: What is the geometric formula for projecting a point onto a LineString and calculating measure?

**Core Operations**:
1. **Projection**: Find closest point on linestring to GNSS position
2. **Measure**: Calculate distance along linestring from start to projected point

**Research Needed**:
- [ ] Review `geo` crate for built-in projection methods
  - Check `geo::algorithm::closest_point` trait
  - Validate output: closest point coordinates + distance
- [ ] Understand multi-segment LineStrings (find closest segment, then project)
- [ ] Test edge cases: point beyond linestring ends, point equidistant from multiple segments

**Example Scenario**:
```rust
use geo::{Point, LineString, ClosestPoint};

let point = Point::new(50.5, 4.2); // GNSS position
let track = LineString::from(vec![(50.0, 4.0), (51.0, 4.0), (51.0, 5.0)]);
let projected = track.closest_point(&point); // → ClosestPoint::SinglePoint(Point)
```

**Decision Criteria**: Leverage `geo` crate built-ins, avoid custom geometric math unless necessary

---

### 5. Timezone Parsing with Chrono

**Question**: How does `chrono` handle ISO 8601 timestamps with timezone offsets?

**Input Format Examples**:
- `2025-12-09T14:30:00+01:00` (Brussels, UTC+1)
- `2025-06-15T12:00:00+02:00` (Brussels, DST UTC+2)
- `2025-12-09T13:30:00Z` (UTC)

**Research Needed**:
- [ ] Test parsing with `DateTime::<FixedOffset>::parse_from_rfc3339`
- [ ] Verify timezone preservation (no silent conversion to UTC)
- [ ] Handle DST transitions: validate 02:30 on DST switch day
- [ ] Benchmark parsing performance (10,000 timestamps)

**Example Code**:
```rust
use chrono::{DateTime, FixedOffset};

let ts = "2025-12-09T14:30:00+01:00";
let dt = DateTime::parse_from_rfc3339(ts)?; // → DateTime<FixedOffset>
assert_eq!(dt.timezone(), FixedOffset::east_opt(3600));
```

**Decision Criteria**: No timezone information loss, DST-aware, <1ms per 1000 timestamps

---

### 6. PyO3 Error Handling: Rust → Python

**Question**: How to map Rust `Result<T, E>` to Python exceptions for FFI bindings?

**Challenge**: Rust uses `Result` enum, Python uses exception raising

**Research Needed**:
- [ ] Explore `PyO3` error conversion with `#[pyclass]` and `From<E> for PyErr`
- [ ] Test custom error types: `ProjectionError` → `ValueError` in Python
- [ ] Verify stack trace preservation for debugging
- [ ] Measure FFI overhead: Rust call vs Python wrapper

**Example Pattern**:
```rust
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

#[pyfunction]
fn project_gnss(data: Vec<f64>) -> PyResult<Vec<f64>> {
    rust_lib::project(data)
        .map_err(|e| PyValueError::new_err(format!("Projection failed: {}", e)))
}
```

**Decision Criteria**: Idiomatic Python exceptions, preserve error context, <5% FFI overhead

---

### 7. Performance Baseline: Naive Nearest-Netelement Search

**Question**: What is the baseline performance of O(n*m) brute-force search?

**Purpose**: Quantify R-tree speedup potential

**Benchmark Setup**:
- Input: 1000 GNSS points, 50 netelements
- Algorithm: For each point, compute distance to all netelements, select minimum
- Metrics: Total runtime (ms), queries/sec

**Research Needed**:
- [ ] Implement naive double-loop in Rust
- [ ] Use `criterion` crate for microbenchmarking
- [ ] Establish baseline: ??? ms for 1000×50 operations
- [ ] Extrapolate to 10,000 points: acceptable? (<10s per SC-001)

**Decision Criteria**: If naive <5s for 1000 points, R-tree optional for MVP; if >5s, R-tree mandatory

---

## Investigation Tasks

### Task 1: Crate Evaluation Matrix
Create comparison table for all candidate crates:

| Crate | Purpose | Downloads/mo | Last Updated | Pros | Cons |
|-------|---------|--------------|--------------|------|------|
| `rstar` | Spatial index | ??? | ??? | ??? | ??? |
| `arrow-csv` | CSV → Arrow | ??? | ??? | ??? | ??? |
| `geojson` | GeoJSON parse | ??? | ??? | ??? | ??? |
| `geo` | Geospatial ops | ??? | ??? | ??? | ??? |
| `proj` | CRS transform | ??? | ??? | ??? | ??? |
| `chrono` | Timezone time | ??? | ??? | ??? | ??? |
| `polars` | DataFrames | ??? | ??? | ??? | ??? |
| `pyo3` | Python FFI | ??? | ??? | ??? | ??? |

### Task 2: Environment Setup Test
**CRITICAL FIRST IMPLEMENTATION STEP** (per user requirement)

Create minimal test to validate dependencies are correctly integrated:

**File**: `tp-core/tests/unit/projection_basic_test.rs`

```rust
#[cfg(test)]
mod basic_projection_test {
    use geo::{Point, LineString, ClosestPoint};
    
    #[test]
    fn test_project_point_on_linestring() {
        // Hardcoded test data (no file I/O)
        let point = Point::new(50.0, 4.0);
        let linestring = LineString::from(vec![
            (50.0, 4.0),
            (51.0, 4.0),
        ]);
        
        // Validate projection works
        let result = linestring.closest_point(&point);
        match result {
            ClosestPoint::SinglePoint(p) => {
                assert_eq!(p.x(), 50.0);
                assert_eq!(p.y(), 4.0);
            }
            _ => panic!("Expected SinglePoint, got {:?}", result),
        }
    }
}
```

**Purpose**: Verify `geo` crate is correctly linked, projection algorithm compiles, test framework runs

**Success Criteria**: `cargo test` passes, outputs "test basic_projection_test ... ok"

### Task 3: Arrow Columnar Memory Spike
Prototype CSV → Arrow → Rust conversion:

```rust
use arrow::csv::ReaderBuilder;
use arrow::datatypes::{Schema, Field, DataType};
use std::sync::Arc;

fn parse_gnss_csv(path: &str) -> arrow::error::Result<Vec<(f64, f64)>> {
    let schema = Schema::new(vec![
        Field::new("latitude", DataType::Float64, false),
        Field::new("longitude", DataType::Float64, false),
    ]);
    
    let file = std::fs::File::open(path)?;
    let reader = ReaderBuilder::new(Arc::new(schema))
        .build(file)?;
    
    // Process record batches
    // TODO: Convert to Vec<(lat, lon)>
    Ok(vec![])
}
```

Measure memory usage with `valgrind` or `heaptrack`.

### Task 4: CRS Transformation Accuracy Test
Validate `proj` crate transformations against known coordinates:

```rust
use proj::Proj;

#[test]
fn test_belgium_lambert_to_wgs84() {
    // Belgian Lambert 2008 (EPSG:3812) → WGS84 (EPSG:4326)
    let transform = Proj::new_known_crs("EPSG:3812", "EPSG:4326", None).unwrap();
    
    // Known Brussels coordinate
    let (x, y) = (649328.0, 665262.0); // Lambert 2008
    let (lon, lat) = transform.convert((x, y)).unwrap();
    
    // Expected WGS84 (approximate)
    assert!((lon - 4.3517).abs() < 0.0001);
    assert!((lat - 50.8503).abs() < 0.0001);
}
```

### Task 5: Performance Baseline Benchmark
Implement naive search and benchmark:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use geo::{Point, LineString, Distance};

fn naive_nearest(points: &[Point<f64>], tracks: &[LineString<f64>]) -> Vec<usize> {
    points.iter().map(|p| {
        tracks.iter()
            .enumerate()
            .min_by_key(|(_, track)| {
                track.euclidean_distance(p) as i64
            })
            .map(|(idx, _)| idx)
            .unwrap()
    }).collect()
}

fn bench_naive_search(c: &mut Criterion) {
    let points = vec![Point::new(50.0, 4.0); 1000];
    let tracks = vec![LineString::from(vec![(50.0, 4.0), (51.0, 4.0)]); 50];
    
    c.bench_function("naive_1000x50", |b| {
        b.iter(|| naive_nearest(black_box(&points), black_box(&tracks)))
    });
}

criterion_group!(benches, bench_naive_search);
criterion_main!(benches);
```

Run with `cargo bench`.

---

## Research Outcomes

> **Fill this section after completing investigation tasks**

### Selected Technologies

| Category | Choice | Rationale |
|----------|--------|-----------|
| Spatial Index | `rstar` | [TBD after benchmarks] |
| Arrow Parser | `arrow-csv` | [TBD after testing] |
| GeoJSON Parser | `geojson` | [TBD after validation] |
| Geospatial Ops | `geo` | [TBD after testing] |
| CRS Transforms | `proj` | [TBD after accuracy test] |
| Temporal | `chrono` | [TBD after timezone validation] |
| DataFrames | `polars` | [TBD after Arrow interop] |
| Python FFI | `pyo3` | [TBD after error handling test] |

### Performance Baselines

| Operation | Naive Performance | Optimized Target | Gap |
|-----------|-------------------|------------------|-----|
| Nearest netelement (1000×50) | [TBD] ms | [TBD] ms | [TBD]× |
| CSV parsing (1000 records) | [TBD] ms | [TBD] ms | [TBD]× |
| CRS transformation (1000 points) | [TBD] ms | [TBD] ms | [TBD]× |

### Architectural Decisions

1. **Spatial Indexing**: [TBD - mandatory if naive >5s, optional if <2s]
2. **Memory Strategy**: [TBD - Arrow columnar vs direct structs]
3. **GeoJSON CRS**: [TBD - strict RFC 7946 or legacy support]
4. **Error Handling**: [TBD - PyO3 pattern selected]

---

## Risks Identified

| Risk | Severity | Mitigation |
|------|----------|------------|
| [TBD] | High/Med/Low | [TBD] |

---

## Next Steps

After completing this research:
1. ✅ Validate all technology choices
2. → Proceed to Phase 1: Data Model Design (`data-model.md`)
3. → Design API Contracts (`contracts/`)
4. → Write User Guide (`quickstart.md`)
5. → Generate implementation tasks (`/speckit.tasks`)

**Estimated Research Duration**: 1-2 days
