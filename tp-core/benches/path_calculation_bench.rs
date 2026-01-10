//! Performance benchmarks for path calculation
//!
//! Benchmarks to validate performance goals:
//! - Process 10,000 GNSS positions in <2 minutes
//! - Support networks with 50,000+ track segments
//! - Memory efficient: <500MB for typical datasets

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use geo::{Coord, LineString};
use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
use tp_lib_core::path::PathConfig;
use tp_lib_core::calculate_train_path;

/// Create a simple linear network for benchmarking
fn create_benchmark_network(segment_count: usize) -> (Vec<Netelement>, Vec<NetRelation>) {
    let mut netelements = Vec::with_capacity(segment_count);
    let mut netrelations = Vec::with_capacity(segment_count - 1);

    // Create linear sequence of netelements
    for i in 0..segment_count {
        let start_lat = 50.0 + (i as f64 * 0.001);
        let end_lat = start_lat + 0.001;
        let lon = 4.0;

        let netelement = Netelement {
            id: format!("NE_{:04}", i),
            geometry: LineString::new(vec![
                Coord {
                    x: lon,
                    y: start_lat,
                },
                Coord { x: lon, y: end_lat },
            ]),
            crs: "EPSG:4326".to_string(),
        };
        netelements.push(netelement);

        // Create netrelation connecting this netelement to the next
        if i < segment_count - 1 {
            let netrelation = NetRelation::new(
                format!("NR_{:04}", i),
                format!("NE_{:04}", i),
                format!("NE_{:04}", i + 1),
                1, // end of current
                0, // start of next
                true,
                true,
            )
            .unwrap();
            netrelations.push(netrelation);
        }
    }

    (netelements, netrelations)
}

/// Create GNSS positions along the linear network
fn create_gnss_positions(count: usize, spacing_meters: f64) -> Vec<GnssPosition> {
    let mut positions = Vec::with_capacity(count);
    let timestamp = Utc::now().into();

    for i in 0..count {
        let lat = 50.0 + (i as f64 * spacing_meters / 111_000.0); // ~111km per degree
        let lon = 4.0;

        let position =
            GnssPosition::new(lat, lon, timestamp, "EPSG:4326".to_string()).unwrap();
        positions.push(position);
    }

    positions
}

// T126: Performance benchmark comparing resampled vs full processing
fn resampling_performance_comparison(c: &mut Criterion) {
    let (netelements, netrelations) = create_benchmark_network(1000);

    // Test with different position densities
    let position_counts = vec![1000, 5000, 10000];

    let mut group = c.benchmark_group("resampling_comparison");

    for &pos_count in &position_counts {
        let gnss_positions = create_gnss_positions(pos_count, 1.0); // 1m spacing

        // Benchmark without resampling (full processing)
        group.bench_with_input(
            BenchmarkId::new("full_processing", pos_count),
            &pos_count,
            |b, _| {
                b.iter(|| {
                    let config = PathConfig::default(); // No resampling
                    let result = calculate_train_path(
                        &gnss_positions,
                        &netelements,
                        &netrelations,
                        &config,
                    );
                    let _ = black_box(result);
                });
            },
        );

        // Benchmark with 10m resampling
        group.bench_with_input(
            BenchmarkId::new("resampled_10m", pos_count),
            &pos_count,
            |b, _| {
                b.iter(|| {
                    let mut config = PathConfig::default();
                    config.resampling_distance = Some(10.0);
                    let result = calculate_train_path(
                        &gnss_positions,
                        &netelements,
                        &netrelations,
                        &config,
                    );
                    let _ = black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn path_calculation_benchmark(c: &mut Criterion) {
    // T186: Performance benchmarks
    let (netelements, netrelations) = create_benchmark_network(1000);

    c.bench_function("path_calc_1k_positions", |b| {
        let gnss_positions = create_gnss_positions(1000, 5.0);
        let config = PathConfig::default();

        b.iter(|| {
            let result =
                calculate_train_path(&gnss_positions, &netelements, &netrelations, &config);
            let _ = black_box(result);
        })
    });
}

criterion_group!(
    benches,
    path_calculation_benchmark,
    resampling_performance_comparison
);
criterion_main!(benches);
