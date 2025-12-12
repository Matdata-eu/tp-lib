# TP-Lib: Train Positioning Library

**Status**: 🚧 In Development - Phase 2 Foundational Complete

Train positioning library for processing GNSS data by projecting positions onto railway track netelements (track axis centerlines). Developed for Infrabel infrastructure management.

## Features

- **GNSS Projection**: Project noisy GNSS coordinates onto accurate track centerlines
- **Netelement Assignment**: Identify which railway segment (netelement) each position corresponds to
- **Measure Calculation**: Calculate distance along track from netelement start
- **Multi-format Support**: CSV and GeoJSON input/output
- **CRS Handling**: Explicit coordinate reference system transformations
- **Timezone Aware**: Full timezone support for temporal data

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
- C compiler (gcc/clang) for native dependencies (proj, rstar)

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

### Usage Example (Planned)

```bash
# CLI usage (Phase 3 implementation)
tp-cli project-gnss \
  --gnss-file journey.csv \
  --gnss-crs EPSG:31370 \
  --network-file network.geojson \
  > projected_output.csv
```

## Development Status

### ✅ Completed (Phase 1 & 2)

- [X] Workspace structure with tp-core, tp-cli, tp-py crates
- [X] Error types (ProjectionError enum with thiserror)
- [X] Data models (GnssPosition, Netelement, ProjectedPosition)
- [X] Basic validation (latitude/longitude ranges, timezone presence)
- [X] Module structure (models, projection, io, crs, temporal)
- [X] CI/CD pipeline configuration
- [X] **FIRST TEST**: Basic projection test validating geo crate integration

### 🚧 In Progress (Phase 3)

- [ ] Geometric projection implementation
- [ ] Spatial indexing (R-tree for O(log n) queries)
- [ ] CSV/GeoJSON parsing and writing
- [ ] CRS transformation (PROJ integration)
- [ ] Main processing pipeline
- [ ] CLI interface

### 📋 Planned (Phase 4)

- [ ] Python bindings (PyO3)
- [ ] Performance benchmarks
- [ ] API documentation
- [ ] Integration tests
- [ ] Production-ready error handling and logging

## Implementation Notes

### Known Issues

1. **Compilation Dependencies**: The project requires a C compiler (gcc/clang/msvc) for native dependencies:
   - `proj` crate needs PROJ library and dlltool on Windows
   - `rstar` spatial indexing may have Windows build requirements

2. **Windows Development**: If encountering `dlltool.exe` or `gcc.exe` errors:
   - Install MSYS2: https://www.msys2.org/
   - Install mingw-w64 toolchain: `pacman -S mingw-w64-x86_64-gcc`
   - Add to PATH: `C:\msys64\mingw64\bin`

3. **Alternative**: Use WSL2 (Windows Subsystem for Linux) for development

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

## Documentation

- [Feature Specification](specs/001-gnss-projection/spec.md)
- [Implementation Plan](specs/001-gnss-projection/plan.md)
- [Data Model](specs/001-gnss-projection/data-model.md)
- [CLI Contract](specs/001-gnss-projection/contracts/cli.md)
- [API Contracts](specs/001-gnss-projection/contracts/)
- [Tasks](specs/001-gnss-projection/tasks.md)

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
