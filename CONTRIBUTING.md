# Contributing to TP-Lib

Thank you for your interest in contributing to TP-Lib! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful and professional in all interactions. We're all here to build better software together.

## Development Setup

### Prerequisites

- **Rust**: 1.91.1 or later ([Install rustup](https://rustup.rs/))
- **Python**: 3.12 or later (for Python bindings)
- **Git**: For version control
- **Docker**: Optional, for containerized testing

### Local Setup

```bash
# Clone the repository
git clone https://github.com/matdata-eu/tp-lib.git
cd tp-lib

# Build all workspace crates
cargo build --workspace

# Run tests
cargo test --workspace --all-features

# Build Python bindings
cd tp-py
pip install maturin pytest
maturin develop
pytest python/tests/
```

## Development Workflow

TP-Lib follows **Test-Driven Development (TDD)**:

1. **RED**: Write a failing test first
2. **GREEN**: Write minimum code to make it pass
3. **REFACTOR**: Improve code while keeping tests green

### Before Committing

Run these checks locally:

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all --check

# Run linter
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run security/license check
cargo deny check

# Run all tests
cargo test --workspace --all-features

# Test Python bindings
cd tp-py
maturin develop
pytest
```

### Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new CRS transformation
fix: correct haversine distance calculation
docs: update API documentation
test: add integration tests for projection
chore: update dependencies
refactor: simplify spatial indexing
```

## Pull Request Process

1. **Fork** the repository
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make changes** following TDD workflow
4. **Run all checks** (see "Before Committing")
5. **Push** to your fork
6. **Open a Pull Request** against `main`

### PR Requirements

Your PR must:

- ✅ Pass all CI checks (tests, linting, security)
- ✅ Include tests for new functionality
- ✅ Update documentation if needed
- ✅ Follow code style (enforced by rustfmt)
- ✅ Have clear commit messages
- ✅ Not introduce security vulnerabilities

**CI Checks:**
- Test Suite (Rust tests)
- Python Tests
- Linting (rustfmt + clippy)
- License & Security Check (cargo-deny)

See [docs/WORKFLOWS.md](docs/WORKFLOWS.md) for details on CI/CD automation.

## Testing Guidelines

### Test Types

1. **Unit Tests**: Test individual functions/methods
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       
       #[test]
       fn test_haversine_distance() {
           // Arrange
           let pos1 = Point::new(50.0, 4.0);
           let pos2 = Point::new(50.1, 4.1);
           
           // Act
           let distance = haversine_distance(&pos1, &pos2);
           
           // Assert
           assert!((distance - 13545.0).abs() < 1.0);
       }
   }
   ```

2. **Integration Tests**: Test component interactions
   - Located in `tests/` directory
   - Test complete workflows

3. **Contract Tests**: Verify CLI behavior
   - Located in `tests/contract/`
   - Test exit codes, output formats

4. **Doc Tests**: Examples in documentation
   ```rust
   /// Calculate distance between two points
   /// 
   /// ```
   /// use tp_lib_core::haversine_distance;
   /// let dist = haversine_distance(50.0, 4.0, 50.1, 4.1);
   /// assert!(dist > 0.0);
   /// ```
   pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
       // implementation
   }
   ```

### Running Tests

```bash
# All tests
cargo test --workspace --all-features

# Specific test
cargo test test_haversine_distance

# With output
cargo test -- --nocapture

# Doc tests only
cargo test --doc

# Integration tests only
cargo test --test integration

# Benchmarks
cargo bench --workspace
```

## Documentation

### Code Documentation

- Add rustdoc comments to all public APIs
- Include examples in doc comments
- Explain complex algorithms
- Document panics, errors, and edge cases

Example:
```rust
/// Projects a GNSS position onto the nearest railway netelement.
///
/// # Arguments
///
/// * `position` - The GNSS position to project
/// * `network` - The railway network with spatial index
///
/// # Returns
///
/// Returns `Ok(ProjectedPosition)` on success, or `Err(ProjectionError)`
/// if projection fails.
///
/// # Example
///
/// ```
/// use tp_lib_core::{GnssPosition, RailwayNetwork, project_position};
///
/// let position = GnssPosition::new(50.8503, 4.3517, "2024-01-01T12:00:00+00:00");
/// let network = RailwayNetwork::from_geojson("network.geojson")?;
/// let projected = project_position(&position, &network)?;
/// ```
pub fn project_position(
    position: &GnssPosition,
    network: &RailwayNetwork,
) -> Result<ProjectedPosition, ProjectionError> {
    // implementation
}
```

### Building Documentation

```bash
# Generate docs
cargo doc --workspace --no-deps

# Open in browser
cargo doc --workspace --no-deps --open
```

Documentation is automatically deployed to GitHub Pages on every push to `main`.

## Release Process

Releases are automated via GitHub Actions. See [docs/WORKFLOWS.md](docs/WORKFLOWS.md) for details.

### Creating a Release

**For Maintainers Only:**

1. **Update versions** in all `Cargo.toml` and `pyproject.toml` files
2. **Update CHANGELOG.md** with release notes
3. **Commit and tag:**
   ```bash
   git commit -am "chore: release v1.0.0"
   git tag v1.0.0
   git push origin main
   git push origin v1.0.0
   ```
4. **Create GitHub Release** at: https://github.com/matdata-eu/tp-lib/releases/new
   - Tag: `v1.0.0`
   - Title: `Release 1.0.0`
   - Description: Copy from CHANGELOG.md
   - Click "Publish release"

**Automated Actions:**
- ✅ Publishes `tp-core`, `tp-cli`, `tp-py` to crates.io
- ✅ Publishes `tp-lib` Python package to PyPI
- ✅ Builds wheels for Linux, Windows, macOS
- ✅ Updates documentation on GitHub Pages

## Project Structure

```
tp-lib/
├── .github/workflows/     # CI/CD automation
├── tp-core/               # Core library
│   ├── src/
│   │   ├── models/        # Data models
│   │   ├── projection/    # Projection algorithms
│   │   ├── io/            # Input/output parsers
│   │   ├── crs/           # Coordinate transformations
│   │   └── temporal/      # Timezone utilities
│   ├── tests/             # Integration tests
│   └── benches/           # Performance benchmarks
├── tp-cli/                # Command-line interface
└── tp-py/                 # Python bindings
    └── python/tests/      # Python tests
```

## Constitution Principles

TP-Lib follows strict architectural principles (see Constitution v1.1.0):

1. **Library-First**: Core functionality in library, not CLI
2. **CLI Mandatory**: All features accessible via command-line
3. **High Performance**: Use efficient data structures (R-tree, Arrow)
4. **TDD**: Test-driven development, write tests first
5. **Full Coverage**: Comprehensive test suite
6. **Timezone Awareness**: All timestamps with explicit timezone
7. **CRS Explicit**: All coordinates include CRS specification
8. **Error Handling**: Typed errors, fail-fast validation
9. **Data Provenance**: Preserve original data, enable auditing
10. **Integration Flexibility**: Rust API + CLI + Python bindings

## Getting Help

- **Documentation**: https://matdata-eu.github.io/tp-lib/
- **Issues**: https://github.com/matdata-eu/tp-lib/issues
- **Discussions**: https://github.com/matdata-eu/tp-lib/discussions

## License

By contributing, you agree that your contributions will be licensed under the Apache License 2.0.

---

Thank you for contributing to TP-Lib! 🚄
