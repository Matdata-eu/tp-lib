# TP-Lib: Train Positioning Library

[![CI](https://github.com/matdata-eu/tp-lib/actions/workflows/ci.yml/badge.svg)](https://github.com/matdata-eu/tp-lib/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/tp-core.svg)](https://crates.io/crates/tp-core)
[![PyPI](https://img.shields.io/pypi/v/tp-lib.svg)](https://pypi.org/project/tp-lib/)
[![Documentation](https://img.shields.io/badge/docs-github.io-blue)](https://matdata-eu.github.io/tp-lib/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

**Status**: ✅ Production Ready - All 66 Tasks Complete

Train positioning library for processing GNSS data by projecting positions onto railway track netelements (track axis centerlines). Developed for Infrabel infrastructure management.

## Features

- 🚄 **High Performance**: R-tree spatial indexing for O(log n) nearest-track search
- 📍 **Accurate Projection**: Haversine distance and geodesic calculations with geo-rs
- 🌍 **CRS Aware**: Explicit coordinate reference system handling (EPSG codes)
- ⏰ **Timezone Support**: RFC3339 timestamps with explicit timezone offsets
- 📊 **Multiple Formats**: CSV and GeoJSON input/output
- 🧪 **Well Tested**: 84 comprehensive tests (all passing) - unit, integration, contract, CLI, and doctests
- ⚡ **Production Ready**: Full CLI interface with validation and error handling

## Project Structure

```
tp-lib/                    # Rust workspace root
├── tp-core/               # Core Rust library
│   ├── src/
│   │   ├── models/        # Data models (GnssPosition, Netelement, ProjectedPosition)
│   │   ├── projection/    # Projection algorithms (geom, spatial indexing)
│   │   ├── io/            # Input/output (CSV, GeoJSON, Arrow)
│   │   ├── crs/           # Coordinate reference system transformations
│   │   ├── temporal/      # Timezone handling utilities
│   │   └── errors.rs      # Error types
│   ├── tests/
│   │   ├── unit/          # Unit tests
│   │   └── integration/   # Integration tests
│   └── benches/           # Performance benchmarks
├── tp-cli/                # Command-line interface
└── tp-py/                 # Python bindings (PyO3)
```

## Quick Start

### Prerequisites

- Rust 1.91.1+ (install from [rustup.rs](https://rustup.rs/))
- Python 3.12+ (for Python bindings)

### Build from Source

```bash
# Clone repository
git clone https://github.com/infrabel/tp-lib
cd tp-lib

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench --workspace
```

### Docker Usage

#### Production Deployment

Use Docker to run the CLI without installing Rust:

```bash
# Build production image
docker build -t tp-lib:latest .

# Run with mounted data directory
docker run --rm -v $(pwd)/data:/data tp-lib:latest \
  --gnss-file /data/gnss.csv \
  --crs EPSG:4326 \
  --network-file /data/network.geojson \
  --output-format csv > output.csv

# Or use docker-compose
docker-compose up tp-cli
```

#### Running Tests in Docker

Run the complete test suite including CRS transformation tests:

```bash
# Using docker-compose (recommended)
docker-compose run --rm test

# Or build and run test image directly
docker build -f Dockerfile.test -t tp-lib-test .
docker run --rm tp-lib-test

# Run specific tests
docker-compose run --rm test cargo test test_identity_transform

# Run only CRS transformation tests
docker-compose run --rm test cargo test crs_transform

# Interactive shell for debugging
docker run --rm -it tp-lib-test bash
```

**Why Docker for tests?**

- **Complete test coverage**: Runs all tests including CRS transformation tests
- **Consistent environment**: Same Rust version across all machines
- **No local setup needed**: No need to install Rust toolchain locally
- **CI/CD ready**: Use `Dockerfile.test` in GitHub Actions or other CI systems

### Usage Examples

```bash
# CLI usage - CSV input/output
tp-cli --gnss-file train_positions.csv \
       --gnss-crs EPSG:4326 \
       --network-file railway_network.geojson \
       --output-format csv > projected.csv

# GeoJSON output with custom warning threshold
tp-cli --gnss-file positions.csv \
       --gnss-crs EPSG:4326 \
       --network-file network.geojson \
       --output-format json \
       --warning-threshold 100.0 > projected.geojson

# Custom CSV column names
tp-cli --gnss-file data.csv \
       --gnss-crs EPSG:4326 \
       --network-file network.geojson \
       --lat-col lat --lon-col lon --time-col timestamp
```

### Library Usage

```rust
use tp_core::{parse_gnss_csv, parse_network_geojson, RailwayNetwork};
use tp_core::{project_gnss, ProjectionConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load railway network from GeoJSON
    let netelements = parse_network_geojson("network.geojson")?;
    let network = RailwayNetwork::new(netelements)?;

    // Load GNSS positions from CSV
    let positions = parse_gnss_csv(
        "gnss.csv",
        "EPSG:4326",
        "latitude",
        "longitude",
        "timestamp"
    )?;

    // Project onto network with default config (50m warning threshold)
    let config = ProjectionConfig::default();
    let projected = project_gnss(&positions, &network, &config)?;

    // Use results
    for pos in projected {
        println!(
            "Position at {}m on netelement {} (accuracy: {:.2}m)",
            pos.measure_meters,
            pos.netelement_id,
            pos.projection_distance_meters
        );
    }

    Ok(())
}
```

## Development Status

### ✅ Phase 1 Complete: Setup (T001-T015)

- [x] Workspace structure with tp-core, tp-cli, tp-py crates
- [x] Cargo.toml configuration for workspace and dependencies
- [x] Git repository initialization with .gitignore
- [x] Directory structure (models, projection, io, crs, temporal)
- [x] Error types (ProjectionError enum with thiserror)

### ✅ Phase 2 Complete: Foundational (T016-T025)

- [x] Data models (GnssPosition, Netelement, ProjectedPosition)
- [x] Basic validation (latitude/longitude ranges, timezone presence)
- [x] Module structure and public API exports
- [x] Unit tests for all models
- [x] Test fixtures and integration test framework

### ✅ Phase 3 Complete: User Story 1 MVP (T026-T049)

- [x] **Geometric Projection** (T026-T028): ClosestPoint algorithm, measure calculation, 8 unit tests
- [x] **Spatial Indexing** (T029-T031): R-tree implementation, O(log n) nearest-neighbor, 3 unit tests
- [x] **Input Parsing** (T032-T035): CSV/GeoJSON readers with Polars/geojson crates, 3 integration tests
- [x] **Main Pipeline** (T036-T040): RailwayNetwork struct, project_gnss() function, 1 end-to-end test
- [x] **Output Writers** (T041-T042): CSV/GeoJSON serialization, 2 integration tests
- [x] **CLI Interface** (T043-T047): clap argument parsing, validation, exit codes, help documentation
- [x] **Integration Tests** (T048): Full pipeline test with 3 GNSS positions × 2 netelements
- [x] **Configuration** (T049): ProjectionConfig with warning threshold and CRS transform flag

**Result**: Fully functional CLI and library with 28 passing tests

### ✅ Phase 4 Complete: Polish & Cross-Cutting (T050-T066)

- [x] **Documentation** (T050-T053): Rustdoc comments, README files
- [x] **Performance Benchmarks** (T054-T056): Criterion benchmarks, naive vs optimized
- [x] **Python Bindings** (T057-T060): PyO3 wrappers, error conversion, pytest tests
- [x] **Additional Testing** (T061-T064): Contract tests, GNSS validation, CRS transform tests, CLI integration tests
- [x] **Structured Logging** (T065-T066): Tracing instrumentation, subscriber configuration

## Implementation Notes

### Performance

- **Target**: < 10 seconds for 1000 GNSS positions × 50 netelements (SC-001)
- **Memory**: Handles 10,000+ positions without exhaustion (SC-006)
- **Accuracy**: 95% of positions within 2m projection distance (GPS quality dependent)
- **R-tree Complexity**: O(log n) nearest-neighbor search

### Input Data Requirements

**GNSS CSV:**

```csv
latitude,longitude,timestamp,altitude,hdop
50.8503,4.3517,2025-12-09T14:30:00+01:00,100.0,2.0
```

- RFC3339 timestamps with timezone (+HH:MM format required)
- CRS must be specified via `--gnss-crs` flag
- Column names configurable with `--lat-col`, `--lon-col`, `--time-col`

**Railway Network GeoJSON:**

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": { "id": "NE001", "crs": "EPSG:4326" },
      "geometry": {
        "type": "LineString",
        "coordinates": [
          [4.35, 50.85],
          [4.36, 50.86]
        ]
      }
    }
  ]
}
```

- LineString geometries (track centerlines)
- Unique `id` property per netelement
- `crs` property with EPSG code

### Troubleshooting

**"Large projection distance" warnings:****

Indicates GNSS position is far from nearest track (> threshold). Possible causes:

- GPS inaccuracy or poor signal quality
- Train on parallel track not in network
- Missing netelement in railway network
- CRS mismatch between GNSS and network
- Track geometry outdated or incorrect

Adjust threshold with `--warning-threshold` flag or investigate data quality.

**"No Python 3.x interpreter found" build error:**

Building with default features requires Python for PyO3 bindings. Disable with:

```bash
cargo build --no-default-features
```

Or install Python 3.12+ and ensure it's in your PATH.

### Known Issues

1. **Windows Build Dependencies**: Requires MSVC toolchain or mingw-w64 for some native dependencies
2. **Python Bindings**: Requires Python 3.12+ installed (excluded from tp-py crate builds by default)

## CRS Transformations

TP-Lib uses **proj4rs**, a pure Rust implementation of PROJ.4, for coordinate reference system transformations. This eliminates system dependencies and enables cross-platform compatibility.

**Key Features:**

- **Pure Rust**: No external C libraries required (libproj, sqlite3, etc.)
- **Zero system dependencies**: Works on Windows, Linux, macOS without installation
- **EPSG support**: Uses `crs-definitions` crate for EPSG code lookup
- **WASM compatible**: Can be used in browser environments
- **Always enabled**: CRS transformations are available by default

**Supported Transformations:**

TP-Lib has been tested with Belgian railway coordinate systems:

- EPSG:4326 (WGS84) ↔ EPSG:31370 (Belgian Lambert 72)
- EPSG:4326 (WGS84) ↔ EPSG:3812 (Belgian Lambert 2008)
- Any EPSG codes supported by [crs-definitions](https://docs.rs/crs-definitions/)

**Usage:**

```rust
use tp_core::crs::CrsTransformer;
use geo::Point;

// Create transformer (EPSG codes or PROJ strings)
let transformer = CrsTransformer::new(
    "EPSG:4326".to_string(),
    "EPSG:31370".to_string()
)?;

// Transform point (automatic degree/radian conversion)
let wgs84_point = Point::new(4.3517, 50.8503);
let lambert_point = transformer.transform(wgs84_point)?;
```

**Technical Details:**

- proj4rs automatically handles radian/degree conversions for geographic CRS
- EPSG codes are resolved to PROJ strings using the crs-definitions crate
- Custom PROJ strings can be used directly instead of EPSG codes
- Transformation accuracy matches PROJ for standard 2D transformations

**Limitations:**

- proj4rs implements PROJ.4 API (2D transformations only)
- No 3D/4D or orthometric transformations
- Grid shift support is experimental
- For complex geodetic requirements, consider using [PROJ](https://proj.org/) directly

## Documentation

### API Documentation

**Online:** https://matdata-eu.github.io/tp-lib/

The documentation is automatically built and deployed on every push to `main`. It includes:

- **tp-core**: Core library API with examples
- **tp-cli**: Command-line interface documentation
- **tp-py**: Python bindings API reference

**Build locally:**

```bash
# Generate documentation for all workspace crates
cargo doc --no-deps --workspace

# Open in browser (on Windows)
start target/doc/index.html

# Open in browser (on Linux/macOS)
open target/doc/index.html  # macOS
xdg-open target/doc/index.html  # Linux
```

### Specification Documents

- [Feature Specification](specs/001-gnss-projection/spec.md)
- [Implementation Plan](specs/001-gnss-projection/plan.md)
- [Data Model](specs/001-gnss-projection/data-model.md)
- [CLI Contract](specs/001-gnss-projection/contracts/cli.md)
- [API Contracts](specs/001-gnss-projection/contracts/)
- [Tasks](specs/001-gnss-projection/tasks.md)

### CI/CD & Workflows

This project uses automated workflows for continuous integration and deployment:

- 🔄 **Continuous Integration**: Automated testing, linting, and security checks on every push
- 📦 **crates.io Publishing**: Automatic release to Rust package registry
- 🐍 **PyPI Publishing**: Automatic release to Python package index
- 📚 **Documentation Deployment**: Auto-deployed to GitHub Pages

See **[CI/CD Workflows Documentation](docs/WORKFLOWS.md)** for details on:
- Build and test automation
- Release process and version management
- Security and license validation
- Publishing to crates.io and PyPI
- Documentation deployment

### Constitution Compliance

This project follows the TP-Lib Constitution v1.1.0 principles:

- ✅ **I. Library-First**: Single unified library with quality external dependencies
- ✅ **II. CLI Mandatory**: Command-line interface for all functionality
- ✅ **III. High Performance**: Apache Arrow, R-tree spatial indexing
- ✅ **IV. TDD**: Test-driven development with FIRST TEST validation
- ✅ **V. Full Coverage**: Comprehensive test suite (unit, integration, property-based)
- ✅ **VI. Timezone Awareness**: DateTime<FixedOffset> for all timestamps
- ✅ **VII. CRS Explicit**: All coordinates include CRS specification
- ✅ **VIII. Error Handling**: Typed errors with thiserror, fail-fast validation
- ✅ **IX. Data Provenance**: Preserve original GNSS data, audit logging
- ✅ **X. Integration Flexibility**: Rust API + CLI + Python bindings

## Contributing

This project follows strict TDD workflow:

1. Write test first (RED)
2. Implement minimum code to pass (GREEN)
3. Refactor while keeping tests green

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details

## Contact

TP-Lib Contributors - [GitHub Issues](https://github.com/infrabel/tp-lib/issues)
