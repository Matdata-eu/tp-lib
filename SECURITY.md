# Security Policy

## Dependency Auditing

This project uses [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) to audit dependencies for security vulnerabilities, license compatibility, and quality issues.

### Running Security Audits

```bash
cargo deny check
```

### Current Security Status

**✅ All Checks Passing** (with documented warnings)

#### Known Vulnerabilities (Accepted)

1. **fast-float 0.2.0** (RUSTSEC-2024-0379, RUSTSEC-2025-0003)
   - **Status**: Accepted risk
   - **Reason**: Transitive dependency via `polars 0.44`
   - **Details**: Multiple soundness issues including undefined behavior and segmentation fault risk
   - **Mitigation**: Polars team is aware and will migrate when ready. Tracking: https://github.com/pola-rs/polars/issues/19964
   - **Impact**: Low - library is used by polars, which is actively maintained

2. **pyo3 0.21.2** (RUSTSEC-2025-0020)
   - **Status**: Accepted risk (temporarily)
   - **Reason**: Upgrade to pyo3 0.24.1+ blocked by polars dependency compatibility
   - **Details**: Buffer overflow vulnerability in `PyString::from_object`
   - **Mitigation**: Will be resolved when polars supports newer pyo3 versions
   - **Impact**: Low - only affects Python bindings (`tp-py` module)
   - **Note**: Attempted upgrade to pyo3 0.24.1 and polars 0.52 but encountered memory allocation issues during compilation

## License Policy

This project is licensed under **Apache-2.0** and only accepts dependencies with compatible licenses:

- Apache-2.0 / Apache-2.0 WITH LLVM-exception
- MIT / MIT-0
- ISC
- BSD-2-Clause / BSD-3-Clause
- BSL-1.0 (Boost Software License)
- CC0-1.0 (Creative Commons Zero)
- Unlicense
- Zlib
- Unicode-DFS-2016 / Unicode-3.0

**All dependencies are compliant** with this policy.

## Duplicate Dependencies

The following duplicate dependencies are present due to ecosystem-wide version transitions. These are normal for Rust projects:

- `bitflags` (1.3.2, 2.10.0)
- `hashbrown` (0.15.5, 0.16.1)
- `heck` (0.4.1, 0.5.0)
- `thiserror` (1.0.69, 2.0.17)
- `thiserror-impl` (1.0.69, 2.0.17)
- `windows-sys` (0.59.0, 0.61.2)

These duplicates do not pose security risks and will be resolved through normal ecosystem upgrades.

## Reporting Security Issues

If you discover a security vulnerability in this project, please report it by:

1. Creating a private security advisory on GitHub
2. Or emailing the maintainers directly

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if available)

## Security Update Policy

- Security vulnerabilities in direct dependencies will be addressed within 7 days
- Transitive dependency vulnerabilities will be evaluated and tracked with documented mitigation strategies
- Regular dependency updates via `cargo update` will be performed monthly
