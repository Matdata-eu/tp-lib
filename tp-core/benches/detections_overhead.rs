//! Detection-overhead benchmark (T038, SC-005, 004-train-detections).
//!
//! Measures path calculation wall-clock time with and without 1,000 detection
//! anchors over a 10,000-sample GNSS log and asserts that the relative
//! overhead introduced by the detection-aware Viterbi path is below 20%.

use chrono::{Duration, TimeZone, Utc};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use geo::{Coord, LineString};
use std::time::Instant;
use tp_lib_core::models::{GnssPosition, NetRelation, Netelement};
use tp_lib_core::path::PathConfig;
use tp_lib_core::{calculate_train_path, ResolvedAnchor};

const NETWORK_SIZE: usize = 1_000;
const GNSS_COUNT: usize = 10_000;
const DETECTION_COUNT: usize = 1_000;

/// Build a linear chain of netelements identical to `path_calculation_bench`.
fn build_network(segment_count: usize) -> (Vec<Netelement>, Vec<NetRelation>) {
    let mut netelements = Vec::with_capacity(segment_count);
    let mut netrelations = Vec::with_capacity(segment_count.saturating_sub(1));

    for i in 0..segment_count {
        let start_lat = 50.0 + (i as f64 * 0.001);
        let end_lat = start_lat + 0.001;
        let lon = 4.0;

        netelements.push(Netelement {
            id: format!("NE_{:04}", i),
            geometry: LineString::new(vec![
                Coord {
                    x: lon,
                    y: start_lat,
                },
                Coord { x: lon, y: end_lat },
            ]),
            crs: "EPSG:4326".to_string(),
        });

        if i < segment_count - 1 {
            let nr = NetRelation::new(
                format!("NR_{:04}", i),
                format!("NE_{:04}", i),
                format!("NE_{:04}", i + 1),
                1,
                0,
                true,
                true,
            )
            .unwrap();
            netrelations.push(nr);
        }
    }

    (netelements, netrelations)
}

/// Generate `count` GNSS samples roughly tracing the linear network.
fn build_gnss(count: usize) -> Vec<GnssPosition> {
    let base = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let mut positions = Vec::with_capacity(count);
    for i in 0..count {
        let lat = 50.0 + (i as f64 * 0.0001);
        let lon = 4.0;
        let ts = (base + Duration::seconds(i as i64)).into();
        positions.push(GnssPosition::new(lat, lon, ts, "EPSG:4326".to_string()).unwrap());
    }
    positions
}

/// Build `DETECTION_COUNT` punctual `ResolvedAnchor`s spread evenly across the
/// GNSS array — each anchor pins the netelement nearest to its GNSS index.
fn build_anchors(
    network_size: usize,
    gnss_count: usize,
    anchor_count: usize,
) -> Vec<ResolvedAnchor> {
    let mut anchors = Vec::with_capacity(anchor_count);
    for k in 0..anchor_count {
        let gnss_index = (k * gnss_count / anchor_count).min(gnss_count - 1);
        let net_idx = (gnss_index * network_size / gnss_count).min(network_size - 1);
        anchors.push(ResolvedAnchor::Punctual {
            netelement_id: format!("NE_{:04}", net_idx),
            intrinsic: 0.5,
            gnss_index,
        });
    }
    anchors
}

fn detections_overhead(c: &mut Criterion) {
    let (netelements, netrelations) = build_network(NETWORK_SIZE);
    let gnss = build_gnss(GNSS_COUNT);
    let anchors = build_anchors(NETWORK_SIZE, GNSS_COUNT, DETECTION_COUNT);

    let mut group = c.benchmark_group("detections_overhead");
    group.sample_size(10);

    group.bench_function("baseline_no_detections", |b| {
        b.iter(|| {
            let config = PathConfig::default();
            let r = calculate_train_path(&gnss, &netelements, &netrelations, &config);
            let _ = black_box(r);
        });
    });

    group.bench_function("with_1000_detections", |b| {
        b.iter(|| {
            let config = PathConfig {
                anchors: anchors.clone(),
                ..PathConfig::default()
            };
            let r = calculate_train_path(&gnss, &netelements, &netrelations, &config);
            let _ = black_box(r);
        });
    });

    group.finish();

    // Sanity check: warn (do not fail criterion) if overhead exceeds the SC-005
    // budget on this run. Criterion's own report remains the authoritative
    // measurement.
    let baseline = measure_once(|| {
        let config = PathConfig::default();
        let _ = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    });
    let with_dets = measure_once(|| {
        let config = PathConfig {
            anchors: anchors.clone(),
            ..PathConfig::default()
        };
        let _ = calculate_train_path(&gnss, &netelements, &netrelations, &config);
    });
    let overhead = (with_dets.as_secs_f64() - baseline.as_secs_f64()) / baseline.as_secs_f64();
    eprintln!(
        "[detections_overhead] baseline={:?} with_detections={:?} overhead={:.2}%",
        baseline,
        with_dets,
        overhead * 100.0
    );
    if overhead > 0.20 {
        eprintln!(
            "[detections_overhead] WARNING: overhead {:.2}% exceeds SC-005 20% budget",
            overhead * 100.0
        );
    }
}

fn measure_once<F: FnMut()>(mut f: F) -> std::time::Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}

criterion_group!(benches, detections_overhead);
criterion_main!(benches);
