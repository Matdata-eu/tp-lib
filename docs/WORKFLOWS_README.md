# GitHub Workflows - Quick Reference

## Workflows Created

This project has 4 automated workflows:

### 1. 🔄 CI (Continuous Integration)
**File:** `.github/workflows/ci.yml`  
**Trigger:** Push or PR to `main`/`develop`  
**Jobs:**
- ✅ Test Suite (Rust)
- 🏃 Benchmarks
- 🐍 Python Tests
- 🧹 Linting (rustfmt + clippy)
- 🔒 License & Security Check (cargo-deny)

### 2. 📦 Publish to crates.io
**File:** `.github/workflows/publish-crates.yml`  
**Trigger:** GitHub Release with tag `v*.*.*`  
**Publishes:**
1. tp-core (core library)
2. tp-cli (command-line tool)
3. tp-py (Python bindings)

**Authentication:** OIDC Trusted Publishing (no secrets needed)

### 3. 🐍 Publish to PyPI
**File:** `.github/workflows/publish-pypi.yml`  
**Trigger:** GitHub Release with tag `v*.*.*`  
**Builds:**
- Wheels for Linux, Windows, macOS
- Python 3.9, 3.10, 3.11, 3.12
- Source distribution (sdist)

**Authentication:** OIDC Trusted Publishing (no secrets needed)

### 4. 📚 Deploy Documentation
**File:** `.github/workflows/docs.yml`  
**Trigger:** Push to `main` or manual dispatch  
**Deploys:**
- Rust API documentation to GitHub Pages
- Includes index.html with navigation
- Available at: https://matdata-eu.github.io/tp-lib/

## Quick Commands

### Run tests locally
```bash
cargo test --workspace --all-features
cd tp-py && pytest
```

### Check everything before push
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features
cargo deny check
cargo test --workspace --all-features
```

### Create a release
```bash
# 1. Update versions in Cargo.toml and pyproject.toml
# 2. Commit and tag
git commit -am "chore: release v1.0.0"
git tag v1.0.0
git push origin main
git push origin v1.0.0

# 3. Create GitHub Release at:
# https://github.com/matdata-eu/tp-lib/releases/new
```

### Build docs locally
```bash
cargo doc --workspace --no-deps
# Open target/doc/index.html
```

## Status Badges

Add to README:
```markdown
[![CI](https://github.com/matdata-eu/tp-lib/actions/workflows/ci.yml/badge.svg)](https://github.com/matdata-eu/tp-lib/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/tp-core.svg)](https://crates.io/crates/tp-core)
[![PyPI](https://img.shields.io/pypi/v/tp-lib.svg)](https://pypi.org/project/tp-lib/)
[![Documentation](https://img.shields.io/badge/docs-github.io-blue)](https://matdata-eu.github.io/tp-lib/)
```

## Files Created

```
.github/workflows/
├── ci.yml                 # Continuous Integration (updated)
├── publish-crates.yml     # Publish to crates.io
├── publish-pypi.yml       # Publish to PyPI
└── docs.yml               # Deploy documentation

docs/
├── WORKFLOWS.md           # Detailed workflow documentation
└── WORKFLOWS_SETUP.md     # Setup guide with secrets configuration
```

## Documentation

- **Full Guide:** [docs/WORKFLOWS.md](WORKFLOWS.md)
- **Setup Instructions:** [docs/WORKFLOWS_SETUP.md](WORKFLOWS_SETUP.md)
- **Security Policy:** [SECURITY.md](../SECURITY.md)

## Next Steps

1. ✅ Read [WORKFLOWS_SETUP.md](WORKFLOWS_SETUP.md) for setup instructions
2. ✅ Configure OIDC trusted publishers on crates.io and PyPI
3. ✅ Enable GitHub Pages (Settings → Pages → Source: GitHub Actions)
4. ✅ Test workflows by pushing to a test branch
5. ✅ Update `matdata-eu` placeholders in documentation and workflows
