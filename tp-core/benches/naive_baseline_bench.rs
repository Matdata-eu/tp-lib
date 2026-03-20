//! # Naive Baseline Benchmarks (O(n×m) Brute-Force)
//!
//! This benchmark suite provides a **performance baseline** using a naive O(n×m) algorithm
//! that scans all network elements for each position without spatial indexing.
//!
//! ## Why This Matters
//!
//! - **R-tree justification**: Quantifies the performance improvement from using R-tree indexing
//! - **Overhead analysis**: For small datasets, R-tree construction overhead may exceed its benefits
//! - **Algorithm choice**: Helps decide when to use spatial indexing vs. simple brute-force
//! - **Performance comparison**: Establishes a "worst-case" reference point
//!
//! ## Key Insight: When Naive Wins
//!
//! For very small workloads (e.g., 100 positions × 10 netelements), the naive approach
//! can actually be **faster** than R-tree because:
//! - No index construction overhead
//! - Simple linear scan is cache-friendly
//! - Modern CPUs optimize tight loops extremely well
//!
//! However, as dataset size grows, R-tree's O(log m) queries dominate brute-force's O(m) scans.
//!
//! ## Comparison with R-tree Benchmarks
//!
//! Run both benchmark suites to see the crossover point:
//! ```bash
//! cargo bench --no-default-features
//! ```
//!
//! Then compare results for the same dataset size (e.g., 1000×50) to see relative performance.

use chrono::DateTime;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use geo::{EuclideanDistance, LineString, Point};
use tp_lib_core::{GnssPosition, Netelement};

/// **Naive Nearest-Neighbor Search**: Brute-force O(m) scan without spatial indexing
///
/// This function implements the simplest possible nearest-neighbor algorithm:
/// for each position, scan through ALL network elements and track the minimum distance.
///
/// ## Algorithm Complexity
/// - **Time**: O(m) per query, where m = number of network elements
/// - **Space**: O(1) - no auxiliary data structures needed
/// - **Total cost for n positions**: O(n×m) - quadratic in worst case
///
/// ## Why This Exists
/// - **Performance baseline**: Establishes "worst-case" reference for comparison
/// - **Simplicity**: No index construction overhead or complexity
/// - **Small dataset optimization**: May outperform R-tree for tiny datasets
/// - **Correctness verification**: Simple implementation is easier to validate
///
/// ## When Naive Wins
/// - Very small networks (m < ~20 elements)
/// - One-off queries (no amortization of index construction)
/// - Sparse spatial distributions (R-tree provides little pruning)
///
/// ## When R-tree Wins
/// - Large networks (m > ~50 elements)
/// - Repeated queries (index construction cost is amortized)
/// - Dense spatial distributions (R-tree prunes >90% of candidates)
fn naive_nearest_netelement(point: &Point<f64>, netelements: &[Netelement]) -> Option<usize> {
    let mut min_distance = f64::MAX;
    let mut nearest_idx = None;

    for (idx, netelement) in netelements.iter().enumerate() {
        // Calculate distance to first point of LineString as approximation
        if let Some(coord) = netelement.geometry.coords().next() {
            let ne_point = Point::new(coord.x, coord.y);
            let distance = point.euclidean_distance(&ne_point);

            if distance < min_distance {
                min_distance = distance;
                nearest_idx = Some(idx);
            }
        }
    }

    nearest_idx
}

/// Generate synthetic railway network with N netelements
fn generate_network(n_netelements: usize) -> Vec<Netelement> {
    let mut netelements = Vec::with_capacity(n_netelements);

    for i in 0..n_netelements {
        let base_lon = 4.35 + (i as f64 * 0.01);
        let base_lat = 50.85 + (i as f64 * 0.005);

        let coords = vec![
            (base_lon, base_lat),
            (base_lon + 0.001, base_lat + 0.001),
            (base_lon + 0.002, base_lat + 0.002),
        ];

        netelements.push(Netelement {
            id: format!("NE{:04}", i),
            geometry: LineString::from(coords),
            crs: "EPSG:4326".to_string(),
        });
    }

    netelements
}

/// Generate synthetic GNSS positions
fn generate_gnss_positions(n_positions: usize) -> Vec<GnssPosition> {
    let mut positions = Vec::with_capacity(n_positions);
    let base_time = DateTime::parse_from_rfc3339("2025-12-09T14:30:00+01:00").unwrap();

    for i in 0..n_positions {
        let lat = 50.85 + (i as f64 * 0.0001);
        let lon = 4.35 + (i as f64 * 0.0001);

        positions.push(GnssPosition {
            latitude: lat,
            longitude: lon,
            timestamp: base_time + chrono::Duration::seconds(i as i64),
            crs: "EPSG:4326".to_string(),
            metadata: Default::default(),
            distance: None,
            heading: None,
        });
    }

    positions
}

/// **Naive Baseline Comparison**: Tests brute-force performance across network sizes
///
/// This benchmark varies network element count (10 → 100) while keeping position count
/// constant (1000), measuring the O(m) linear scaling of brute-force scanning.
///
/// ## What This Measures
/// - Brute-force performance WITHOUT R-tree spatial indexing
/// - Linear O(m) scaling as network size increases
/// - Cache effects and CPU optimization of tight scanning loops
///
/// ## Why This Matters
/// - **Performance baseline**: Establishes reference point for R-tree comparison
/// - **Crossover analysis**: Identifies dataset sizes where R-tree overhead is justified
/// - **Simple vs. complex trade-off**: Quantifies the cost of algorithmic complexity
/// - **Surprising insight**: Naive may win for small datasets due to simplicity
///
/// ## Expected Results
/// - Linear scaling: 100 elements should take ~10× longer than 10 elements
/// - For 1000×50 scenario: ~0.3-0.5 ms (faster than R-tree due to no construction overhead)
/// - Performance degrades linearly with network size (no logarithmic advantage)
fn bench_naive_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("naive_baseline");

    eprintln!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("🐌 NAIVE BASELINE COMPARISON (O(n×m) brute-force, 10 → 100 netelements)");
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!(
        "Tests brute-force performance WITHOUT R-tree spatial indexing (1000 positions fixed)"
    );
    eprintln!("Purpose: Establish baseline for R-tree comparison and identify crossover point");
    eprintln!("Expected: Linear O(m) scaling, may be faster than R-tree for small datasets");
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    group.sample_size(10); // Reduce sample size for naive O(n*m) algorithm

    // Test with different network sizes
    for n_netelements in [10, 50, 100].iter() {
        let netelements = generate_network(*n_netelements);
        let positions = generate_gnss_positions(1000);

        group.bench_with_input(
            BenchmarkId::new("nearest_search", format!("1000pos_{}ne", n_netelements)),
            &(&positions, &netelements),
            |b, (positions, netelements)| {
                b.iter(|| {
                    for pos in positions.iter() {
                        let point = Point::new(pos.longitude, pos.latitude);
                        black_box(naive_nearest_netelement(&point, netelements));
                    }
                });
            },
        );
    }

    group.finish();
}

/// **Naive Position Scaling**: Tests how brute-force performs as batch size grows
///
/// This benchmark varies position count (100 → 1000) while keeping network size
/// constant (50 netelements), measuring whether naive scaling remains truly linear.
///
/// ## What This Measures
/// - Impact of batch size on total O(n×m) brute-force cost
/// - Per-position overhead (should be constant ~m comparisons)
/// - Whether CPU cache effects improve or degrade at scale
///
/// ## Why This Matters
/// - **Linear verification**: Confirms naive algorithm has no hidden quadratic behavior
/// - **Comparison with R-tree**: At what point does R-tree's O(n log m) beat naive's O(n×m)?
/// - **Simple correctness**: Easy to verify that performance matches theoretical complexity
///
/// ## Expected Results
/// - Perfect linear scaling: 1000 positions = 10× slower than 100 positions
/// - For small datasets, naive total time may still beat R-tree (construction + queries)
/// - As position count increases, R-tree's advantage grows (amortizes construction cost)
fn bench_naive_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("naive_scalability");

    eprintln!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("🔢 NAIVE POSITION SCALING (100 → 1000 positions, O(n×m) brute-force)");
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("Tests how brute-force performs as batch size grows (50 netelements fixed)");
    eprintln!("Purpose: Verify linear complexity and compare with R-tree's amortized advantage");
    eprintln!("Expected: Perfect linear scaling (1000 positions = 10× slower than 100)");
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    group.sample_size(10); // Reduce sample size for expensive benchmarks

    // Demonstrate O(n*m) complexity
    let netelements_50 = generate_network(50);

    for n_positions in [100, 500, 1000].iter() {
        let positions = generate_gnss_positions(*n_positions);

        group.bench_with_input(
            BenchmarkId::new("varying_positions", format!("{}pos_50ne", n_positions)),
            &(&positions, &netelements_50),
            |b, (positions, netelements)| {
                b.iter(|| {
                    for pos in positions.iter() {
                        let point = Point::new(pos.longitude, pos.latitude);
                        black_box(naive_nearest_netelement(&point, netelements));
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_naive_baseline, bench_naive_scalability);
criterion_main!(benches);
