# Quickstart Guide: Train Path Calculation

**Feature**: 002-train-path-calculation  
**Date**: January 9, 2026  
**Phase**: 1 - Design & Contracts

- [Quickstart Guide: Train Path Calculation](#quickstart-guide-train-path-calculation)
  - [Overview](#overview)
  - [Table of Contents](#table-of-contents)
  - [CLI Usage Examples](#cli-usage-examples)
    - [Example 1: Complete Workflow (Calculate + Project)](#example-1-complete-workflow-calculate--project)
    - [Example 2: Path Calculation Only (For Inspection/Editing)](#example-2-path-calculation-only-for-inspectionediting)
    - [Example 3: Tuned Parameters for Urban Rail](#example-3-tuned-parameters-for-urban-rail)
    - [Example 4: High-Frequency GNSS with Resampling](#example-4-high-frequency-gnss-with-resampling)
    - [Example 5: Two-Step Workflow (Path Then Project)](#example-5-two-step-workflow-path-then-project)
    - [Example 6: Legacy Independent Projection (No Path Calculation)](#example-6-legacy-independent-projection-no-path-calculation)
  - [Rust Library API Examples](#rust-library-api-examples)
    - [Example 1: Basic Path Calculation](#example-1-basic-path-calculation)
    - [Example 2: Custom Configuration with Builder](#example-2-custom-configuration-with-builder)
    - [Example 3: Path-Only Mode (No Projection)](#example-3-path-only-mode-no-projection)
    - [Example 4: Using Pre-Calculated Path](#example-4-using-pre-calculated-path)
    - [Example 5: Error Handling and Fallback](#example-5-error-handling-and-fallback)
  - [Python API Examples](#python-api-examples)
    - [Example 1: Basic Usage (via PyO3 bindings)](#example-1-basic-usage-via-pyo3-bindings)
    - [Example 2: Custom Configuration](#example-2-custom-configuration)
  - [Common Workflows](#common-workflows)
    - [Workflow 1: Batch Processing Multiple Journeys](#workflow-1-batch-processing-multiple-journeys)
    - [Workflow 2: Parameter Tuning Pipeline](#workflow-2-parameter-tuning-pipeline)
    - [Workflow 3: Validation Against Ground Truth](#workflow-3-validation-against-ground-truth)
  - [Troubleshooting](#troubleshooting)
    - [Issue: "No navigable path found"](#issue-no-navigable-path-found)
    - [Issue: Low path probability (\<0.5)](#issue-low-path-probability-05)
    - [Issue: Performance slow on high-frequency GNSS](#issue-performance-slow-on-high-frequency-gnss)
    - [Issue: Path jumps between parallel tracks](#issue-path-jumps-between-parallel-tracks)
  - [Next Steps](#next-steps)


## Overview

This guide provides practical examples for using the train path calculation feature in tp-lib. It covers both library API usage (Rust) and command-line interface (CLI) usage.

---

## Table of Contents

1. [CLI Usage Examples](#cli-usage-examples)
2. [Rust Library API Examples](#rust-library-api-examples)
3. [Python API Examples](#python-api-examples)
4. [Common Workflows](#common-workflows)
5. [Troubleshooting](#troubleshooting)

---

## CLI Usage Examples

### Example 1: Complete Workflow (Calculate + Project)

Calculate train path and project coordinates in one step:

```bash
tp-cli \
  --gnss data/train_journey.csv \
  --network data/rail_network.geojson \
  --output results/projected.csv
```

**Input files:**

`data/train_journey.csv`:
```csv
timestamp,latitude,longitude,crs,heading,distance
2026-01-09T08:00:00+01:00,50.8503,4.3517,EPSG:4326,45.3,
2026-01-09T08:00:01+01:00,50.8504,4.3518,EPSG:4326,47.1,12.5
2026-01-09T08:00:02+01:00,50.8505,4.3519,EPSG:4326,46.8,11.9
```

`data/rail_network.geojson`:
```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {
        "type": "netelement",
        "id": "NE_001"
      },
      "geometry": {
        "type": "LineString",
        "coordinates": [[4.3515, 50.8502], [4.3520, 50.8506]]
      }
    },
    {
      "type": "Feature",
      "properties": {
        "type": "netrelation",
        "id": "NR_001",
        "netelementA": "NE_001",
        "netelementB": "NE_002",
        "positionOnA": 1,
        "positionOnB": 0,
        "navigability": "both"
      },
      "geometry": {
        "type": "Point",
        "coordinates": [4.3520, 50.8506]
      }
    }
  ]
}
```

**Output:** `results/projected.csv` contains projected GNSS coordinates.

**Optional:** Save the calculated path alongside projection:
```bash
tp-cli \
  --gnss data/train_journey.csv \
  --network data/rail_network.geojson \
  --output results/projected.csv \
  --save-path results/path.json
```

---

### Example 2: Path Calculation Only (For Inspection/Editing)

Calculate path without projecting coordinates:

```bash
tp-cli calculate-path \
  --gnss data/train_journey.csv \
  --network data/rail_network.geojson \
  --output results/path.json
```

**Output:** `results/path.json` contains TrainPath with segment sequence and probabilities.

**Use cases:**
- Inspect calculated path before projecting coordinates
- Edit path manually and reuse for projection
- Validate path calculation algorithm

---

### Example 3: Tuned Parameters for Urban Rail

Adjust parameters for dense urban network with tight curves:

```bash
tp-cli \
  --gnss data/metro_journey.csv \
  --network data/metro_network.geojson \
  --output results/metro_projected.csv \
  --distance-scale 15.0 \
  --heading-cutoff 10.0 \
  --probability-threshold 0.20 \
  --verbose
```

**Parameter rationale:**
- `--distance-scale 15.0`: Tighter distance tolerance for urban rail
- `--heading-cutoff 10.0`: Allow sharper turns typical in metros
- `--probability-threshold 0.20`: Lower threshold to handle complex junctions
- `--verbose`: See detailed progress for debugging

---

### Example 4: High-Frequency GNSS with Resampling

Process 1Hz GNSS data (1 position per second) with resampling:

```bash
tp-cli \
  --gnss data/high_freq_gnss.csv \
  --network data/rail_network.geojson \
  --output results/projected.csv \
  --resampling-distance 10.0 \
  --verbose
```

**Effect:**
- If GNSS positions are ~1m apart → resample to every 10th position
- Path calculation uses 10% of data (significant speedup)
- Final projection uses ALL original positions (maintains accuracy)

**Performance improvement:** 5-10x faster for path calculation phase.

---

### Example 5: Two-Step Workflow (Path Then Project)

Separate path calculation from projection:

**Step 1: Calculate path only**
```bash
tp-cli calculate-path \
  --gnss data/train.csv \
  --network data/network.geojson \
  --output cache/path.json
```

**Step 2: Manually edit path if needed** (optional)

**Step 3: Project coordinates using cached/edited path**
```bash
tp-cli \
  --gnss data/train.csv \
  --network data/network.geojson \
  --train-path cache/path.json \
  --output results/projected.csv
```

**Use cases:**
- Edit path manually between calculation and projection
- Reuse path for multiple GNSS datasets from same route
- Cache path calculation results for analysis pipeline
- Debug projection separately from path calculation

---

### Example 6: Legacy Independent Projection (No Path Calculation)

Project each GNSS position independently to nearest netelement (feature 001 behavior):

```bash
tp-cli simple-projection \
  --gnss data/train.csv \
  --network data/network.geojson \
  --output results/independent_projected.csv
```

**Output:** `results/independent_projected.csv` contains projected coordinates without path continuity.

**Use case:**
- Quick projection without topology analysis
- Backward compatibility with feature 001
- Scenarios where path continuity is not required

---

## Rust Library API Examples

### Example 1: Basic Path Calculation

```rust
use tp_lib_core::{
    calculate_train_path, PathConfig, GnssPosition, Netelement, NetRelation,
    parse_gnss_csv, parse_network_geojson, write_csv,
};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load input data
    let gnss_positions = parse_gnss_csv(Path::new("train.csv"))?;
    let (netelements, netrelations) = parse_network_geojson(Path::new("network.geojson"))?;
    
    // Configure algorithm (using defaults)
    let config = PathConfig::default();
    
    // Calculate path
    let result = calculate_train_path(
        &gnss_positions,
        &netelements,
        &netrelations,
        &config,
        false, // Project coordinates (not path-only)
    )?;
    
    // Check result
    if result.is_topology_based() {
        let path = result.path.unwrap();
        println!("✓ Path calculated successfully");
        println!("  Probability: {:.2}", path.overall_probability);
        println!("  Segments: {}", path.segments.len());
        println!("  Route: {:?}", path.netelement_ids());
        
        // Write projected coordinates
        write_csv(Path::new("output.csv"), &result.projected_positions)?;
    } else {
        println!("⚠ Fallback mode used: {}", result.warnings.join("; "));
        println!("  Projected positions: {}", result.projected_positions.len());
    }
    
    Ok(())
}
```

---

### Example 2: Custom Configuration with Builder

```rust
use tp_lib_core::{calculate_train_path, PathConfigBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load data (as before)
    let gnss_positions = load_gnss_data()?;
    let (netelements, netrelations) = load_network()?;
    
    // Build custom configuration
    let config = PathConfigBuilder::new()
        .distance_scale(15.0)           // Tighter distance tolerance
        .heading_scale(3.0)             // More lenient heading tolerance
        .cutoff_distance(75.0)          // Wider search radius
        .heading_cutoff(8.0)            // Allow up to 8° heading difference
        .probability_threshold(0.20)    // Lower acceptance threshold
        .resampling_distance(Some(10.0)) // Resample at 10m intervals
        .max_candidates(5)              // Consider top 5 candidates
        .build()?;
    
    // Calculate with custom config
    let result = calculate_train_path(
        &gnss_positions,
        &netelements,
        &netrelations,
        &config,
        false,
    )?;
    
    // Process result
    handle_result(result)?;
    
    Ok(())
}
```

---

### Example 3: Path-Only Mode (No Projection)

```rust
use tp_lib_core::{calculate_train_path, PathConfig, write_train_path_geojson};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gnss_positions = load_gnss_data()?;
    let (netelements, netrelations) = load_network()?;
    let config = PathConfig::default();
    
    // Calculate path only (skip coordinate projection)
    let result = calculate_train_path(
        &gnss_positions,
        &netelements,
        &netrelations,
        &config,
        true, // path_only = true
    )?;
    
    if let Some(path) = result.path {
        // Export path as GeoJSON
        write_train_path_geojson(Path::new("path.json"), &path)?;
        
        // Inspect path metadata
        if let Some(metadata) = &path.metadata {
            println!("Algorithm config:");
            println!("  distance_scale: {}", metadata.distance_scale);
            println!("  heading_scale: {}", metadata.heading_scale);
            println!("  Bidirectional: {}", metadata.bidirectional_path);
            println!("  Fallback used: {}", metadata.fallback_mode);
            println!("  Paths evaluated: {}", metadata.candidate_paths_evaluated);
        }
        
        // Analyze path segments
        for (i, segment) in path.segments.iter().enumerate() {
            println!(
                "Segment {}: {} (P={:.2}, positions {}-{})",
                i + 1,
                segment.netelement_id,
                segment.probability,
                segment.gnss_start_index,
                segment.gnss_end_index,
            );
        }
    }
    
    Ok(())
}
```

---

### Example 4: Using Pre-Calculated Path

```rust
use tp_lib_core::{
    project_onto_path, TrainPath, Netelement, GnssPosition,
    parse_train_path_geojson, parse_gnss_csv, parse_network_geojson,
};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load pre-calculated path
    let train_path: TrainPath = parse_train_path_geojson(Path::new("cached_path.json"))?;
    
    // Load GNSS data and network
    let gnss_positions = parse_gnss_csv(Path::new("train.csv"))?;
    let (netelements, _) = parse_network_geojson(Path::new("network.geojson"))?;
    
    // Project coordinates onto pre-calculated path
    let projected = project_onto_path(
        &gnss_positions,
        &train_path,
        &netelements,
    )?;
    
    println!("Projected {} positions onto path", projected.len());
    
    // Process projected positions
    for (gnss, proj) in gnss_positions.iter().zip(projected.iter()) {
        println!(
            "{}: projected to {} at intrinsic {:.3} (distance: {:.1}m)",
            gnss.timestamp,
            proj.netelement_id,
            proj.intrinsic_coord,
            proj.distance,
        );
    }
    
    Ok(())
}
```

---

### Example 5: Error Handling and Fallback

```rust
use tp_lib_core::{calculate_train_path, PathConfig, PathCalculationMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gnss_positions = load_gnss_data()?;
    let (netelements, netrelations) = load_network()?;
    let config = PathConfig::default();
    
    let result = calculate_train_path(
        &gnss_positions,
        &netelements,
        &netrelations,
        &config,
        false,
    )?;
    
    match result.mode {
        PathCalculationMode::TopologyBased => {
            // Success: topology-aware path calculated
            let path = result.path.unwrap();
            println!("✓ Topology-based path calculated");
            println!("  Probability: {:.2}", path.overall_probability);
            
            if path.overall_probability < 0.5 {
                println!("⚠ Warning: Low path probability, results may be unreliable");
            }
            
            // Use projected_positions for downstream analysis
            save_results(&result.projected_positions)?;
        }
        
        PathCalculationMode::FallbackIndependent => {
            // Fallback: topology constraints could not be satisfied
            println!("⚠ Fallback to independent projection");
            
            for warning in &result.warnings {
                eprintln!("Warning: {}", warning);
            }
            
            // Still have projected positions, but without topology constraints
            println!("  Projected {} positions independently", result.projected_positions.len());
            
            // Use fallback results with caution
            save_results_with_flag(&result.projected_positions, "fallback")?;
        }
    }
    
    Ok(())
}
```

---

## Python API Examples

### Example 1: Basic Usage (via PyO3 bindings)

```python
import tp_lib

# Load data
gnss_positions = tp_lib.load_gnss_csv("train.csv")
network = tp_lib.load_network_geojson("network.geojson")

# Calculate path with default config
result = tp_lib.calculate_train_path(
    gnss_positions=gnss_positions,
    netelements=network.netelements,
    netrelations=network.netrelations,
    config=tp_lib.PathConfig(),  # Use defaults
    path_only=False,
)

# Check result
if result.is_topology_based():
    path = result.path
    print(f"✓ Path probability: {path.overall_probability:.2f}")
    print(f"  Segments: {len(path.segments)}")
    print(f"  Route: {path.netelement_ids()}")
    
    # Save projected coordinates
    tp_lib.write_csv("output.csv", result.projected_positions)
else:
    print("⚠ Fallback mode used")
    print(f"  Warnings: {', '.join(result.warnings)}")
```

### Example 2: Custom Configuration

```python
import tp_lib

# Custom config for urban rail
config = tp_lib.PathConfig(
    distance_scale=15.0,
    heading_scale=3.0,
    cutoff_distance=75.0,
    heading_cutoff=8.0,
    probability_threshold=0.20,
    resampling_distance=10.0,
    max_candidates=5,
)

# Load and process
gnss = tp_lib.load_gnss_csv("metro.csv")
network = tp_lib.load_network_geojson("metro_network.geojson")

result = tp_lib.calculate_train_path(
    gnss, network.netelements, network.netrelations, config, False
)

# Export result
tp_lib.write_geojson("result.geojson", result.projected_positions)
```

---

## Common Workflows

### Workflow 1: Batch Processing Multiple Journeys

Process multiple train journeys through the same network:

```bash
#!/bin/bash

NETWORK="data/rail_network.geojson"
CONFIG="--distance-scale 10 --heading-cutoff 5"

for journey in data/journeys/*.csv; do
    output="results/$(basename "$journey" .csv)_path.csv"
    
    echo "Processing: $journey"
    tp-cli calculate-path \
        --gnss "$journey" \
        --network "$NETWORK" \
        --output "$output" \
        $CONFIG
    
    if [ $? -eq 0 ]; then
        echo "✓ Success: $output"
    else
        echo "✗ Failed: $journey"
    fi
done
```

---

### Workflow 2: Parameter Tuning Pipeline

Test different parameter combinations to find optimal configuration:

```rust
use tp_lib_core::{calculate_train_path, PathConfigBuilder};

fn tune_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let gnss = load_test_data()?;
    let (netelements, netrelations) = load_network()?;
    
    // Test grid: distance_scale × heading_scale
    let distance_scales = vec![8.0, 10.0, 12.0];
    let heading_scales = vec![1.5, 2.0, 2.5];
    
    for dist_scale in &distance_scales {
        for head_scale in &heading_scales {
            let config = PathConfigBuilder::new()
                .distance_scale(*dist_scale)
                .heading_scale(*head_scale)
                .build()?;
            
            let result = calculate_train_path(&gnss, &netelements, &netrelations, &config, false)?;
            
            if let Some(path) = result.path {
                println!(
                    "dist={:.1}, head={:.1} → P={:.3}, segments={}",
                    dist_scale, head_scale, path.overall_probability, path.segments.len()
                );
            }
        }
    }
    
    Ok(())
}
```

---

### Workflow 3: Validation Against Ground Truth

Compare calculated path with known ground truth:

```rust
use tp_lib_core::{calculate_train_path, PathConfig, TrainPath};

fn validate_against_ground_truth() -> Result<(), Box<dyn std::error::Error>> {
    let gnss = load_gnss_data()?;
    let (netelements, netrelations) = load_network()?;
    let ground_truth: TrainPath = load_ground_truth_path()?;
    
    let result = calculate_train_path(&gnss, &netelements, &netrelations, &PathConfig::default(), false)?;
    
    if let Some(calculated_path) = result.path {
        let calculated_ids = calculated_path.netelement_ids();
        let truth_ids = ground_truth.netelement_ids();
        
        // Calculate accuracy
        let matches: usize = calculated_ids.iter()
            .zip(truth_ids.iter())
            .filter(|(calc, truth)| calc == truth)
            .count();
        
        let accuracy = matches as f64 / truth_ids.len() as f64;
        
        println!("Validation Results:");
        println!("  Accuracy: {:.1}%", accuracy * 100.0);
        println!("  Matches: {}/{}", matches, truth_ids.len());
        println!("  Probability: {:.2}", calculated_path.overall_probability);
        
        // Report mismatches
        for (i, (calc, truth)) in calculated_ids.iter().zip(truth_ids.iter()).enumerate() {
            if calc != truth {
                println!("  Mismatch at position {}: {} vs {}", i, calc, truth);
            }
        }
    }
    
    Ok(())
}
```

---

## Troubleshooting

### Issue: "No navigable path found"

**Symptoms:** Fallback mode used, warning messages about missing connections.

**Possible causes:**
1. Network topology incomplete (missing netrelations)
2. GNSS data far from network (beyond cutoff distance)
3. Navigability constraints too strict

**Solutions:**
```bash
# Increase cutoff distance
tp-cli calculate-path ... --cutoff-distance 100.0

# Lower probability threshold
tp-cli calculate-path ... --probability-threshold 0.15

# Check network coverage
tp-cli validate-network data/network.geojson --verbose
```

---

### Issue: Low path probability (<0.5)

**Symptoms:** Path calculated but probability score low.

**Possible causes:**
1. GNSS data noisy (large distances from track)
2. Heading misalignment
3. Sparse GNSS coverage

**Solutions:**
```bash
# Relax distance and heading constraints
tp-cli calculate-path ... \
  --distance-scale 25.0 \
  --heading-cutoff 10.0 \
  --probability-threshold 0.20

# Inspect path visually
tp-cli calculate-path ... --format geojson --output path.geojson
# Open in QGIS or similar GIS tool
```

---

### Issue: Performance slow on high-frequency GNSS

**Symptoms:** Long processing time, high memory usage.

**Cause:** Too many GNSS positions (e.g., 10,000+ at 1m spacing).

**Solution:**
```bash
# Enable resampling
tp-cli calculate-path ... --resampling-distance 10.0 --verbose

# Check performance improvement in verbose output
# Expected: 5-10x speedup for path calculation phase
```

---

### Issue: Path jumps between parallel tracks

**Symptoms:** Calculated path alternates between parallel tracks.

**Possible causes:**
1. Distance scale too large (can't discriminate parallel tracks)
2. Missing or incorrect netrelations

**Solutions:**
```bash
# Reduce distance scale for tighter discrimination
tp-cli calculate-path ... --distance-scale 10.0

# Add heading data if not present
# Heading helps disambiguate parallel tracks with different directions
```

---

## Next Steps

1. **Read the algorithm documentation**: See [algorithm.md](algorithm.md) for detailed explanation of the 5-phase calculation process

2. **Review API contracts**: See [contracts/lib-api.md](contracts/lib-api.md) and [contracts/cli.md](contracts/cli.md) for complete API specifications

3. **Check data model**: See [data-model.md](data-model.md) for detailed data structure definitions

4. **Run tests**: See test suites in `tp-core/tests/` for additional examples

---

**Quickstart Version**: 1.0  
**Feature Version**: 002-train-path-calculation  
**Last Updated**: January 9, 2026
