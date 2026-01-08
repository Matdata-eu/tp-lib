<!--
SYNC IMPACT REPORT
==================
Version Change: 1.1.0 → 1.2.0
Amendment Type: MINOR - Added new principle for module organization
Modified Principles:
  - NEW Principle XI: Modern Module Organization (Rust) → mandates Rust 1.30+ naming convention (directory.rs, not directory/mod.rs)
  - Context: Enforces modern Rust ecosystem standards for module file naming
Templates Status:
  ✅ plan-template.md - no changes needed
  ✅ spec-template.md - no changes needed
  ✅ tasks-template.md - no changes needed
  ✅ checklist-template.md - no changes needed
  ⚠ commands/*.md - directory not present, will need creation when commands are added
Follow-up TODOs: None
-->

# TP-Lib Constitution

## Core Principles

### I. Library-First Architecture

This project develops **ONE** unified library for train positioning data post-processing with sensor fusion. The library MUST be:
- Built on quality external dependencies rather than reinventing solutions (prefer established libraries for geospatial operations, data processing, numerical computation)
- Independently testable without requiring application context
- Documented with comprehensive API contracts and usage examples
- Designed for reusability across different contexts and environments

**ALL** features are implemented as modules within this single library, maintaining clear internal boundaries and cohesion.

**Rationale**: Leveraging proven external libraries (e.g., for CRS transformations, geometric operations, data fusion algorithms) accelerates development and improves reliability. A unified library provides consistent APIs for fusing GNSS, punctual train registrations, odometry, and topology-based positioning into accurate train location estimates.

### II. CLI Interface Mandatory

**ALL** library functionality MUST be exposed through a command-line interface. CLI implementations MUST:
- Accept input via stdin, command-line arguments, or file paths
- Emit results to stdout in machine-readable formats (JSON primary, human-readable optional)
- Write errors and diagnostics exclusively to stderr
- Return appropriate exit codes (0 = success, non-zero = failure)
- Support --help and --version flags

**Rationale**: CLI interfaces enable batch processing of positioning data, integration with ETL pipelines, and automation of post-processing workflows. They provide a universal contract for infrastructure managers' business processes.

### III. High Performance

Performance MUST be a first-class design concern, not an afterthought. All implementations MUST:
- Minimize memory allocations and copies
- Use appropriate data structures for access patterns
- Avoid unnecessary computational complexity
- Profile critical paths and optimize hot spots
- Document performance characteristics (time/space complexity)
- Establish performance benchmarks for regression detection

**Rationale**: Post-processing large volumes of positioning data from multiple sources (GNSS tracks, registration events, odometer readings) requires efficient algorithms and data structures. While not real-time safety-critical, performance directly impacts business process turnaround times for infrastructure managers analyzing train movements.

### IV. Test-Driven Development (NON-NEGOTIABLE)

**MANDATORY TDD workflow**: Tests written → User/stakeholder approval → Tests FAIL → Implementation → Tests PASS.

**ALL** code MUST follow the Red-Green-Refactor cycle:
1. **Red**: Write a failing test that defines desired behavior
2. **Green**: Write minimal code to make the test pass
3. **Refactor**: Improve code quality while keeping tests green

**NO** implementation code may be written without a corresponding failing test first. Test-first is non-negotiable for:
- New features (unit + integration tests)
- Bug fixes (regression tests)
- Refactoring (behavior preservation tests)

**Rationale**: TDD ensures specification-driven development, prevents regression, and provides living documentation. For positioning algorithms that fuse multiple sensor sources, tests define expected behavior for edge cases (sparse data, conflicting measurements, topology constraints). This rigor is essential for business-critical processes and provides a foundation if patterns are adapted for safety-critical systems.

### V. Full Test Coverage

**100%** test coverage is the target. Every code path MUST be exercised by tests:
- **Unit tests**: All functions, methods, branches, and edge cases
- **Integration tests**: Component interactions and API contracts
- **Contract tests**: Interface stability and backward compatibility
- **Property-based tests**: Where applicable for complex logic
- **Performance tests**: Benchmarks for critical operations

Coverage reports MUST be generated and reviewed. Any uncovered code MUST be either:
- Tested immediately, or
- Justified in writing with explicit approval and tracked as technical debt

**Rationale**: Comprehensive testing provides safety for refactoring, confidence in deployments, and documentation of expected behavior. For business-critical positioning post-processing, coverage gaps risk incorrect location estimates that impact operational decisions. High coverage also demonstrates development discipline valuable if these patterns inspire safety-critical implementations.

### VI. Time with Timezone Awareness

**ALL** temporal data MUST include timezone information. Implementations MUST:
- Store times with explicit timezone (prefer UTC internally)
- Parse input times with timezone validation
- Convert between timezones accurately considering DST and historical changes
- Never assume local timezone or use naive datetime objects
- Document timezone expectations in APIs and data schemas

**Rationale**: Train positioning data from GNSS, registration systems, and odometers arrives with timestamps that may span timezone boundaries. Accurate temporal correlation is essential for sensor fusion—timezone-naive handling causes misalignment between data sources, leading to incorrect position estimates and faulty business intelligence.

### VII. Positions with Coordinate Reference System

**ALL** spatial data MUST specify its Coordinate Reference System (CRS). Implementations MUST:
- Store positions with explicit CRS identifier (e.g., EPSG codes)
- Validate CRS compatibility before coordinate operations
- Transform between CRS when required with documented accuracy
- Never assume a default CRS (WGS84, local grid, etc.)
- Document CRS requirements in APIs and data schemas

**Rationale**: Train positioning sources use different coordinate systems: GNSS provides WGS84, infrastructure databases use local/national grids, topology networks have their own reference frames. Fusing these sources without explicit CRS handling produces incorrect positions, undermining business processes that depend on accurate location data. While not safety-critical, this library's rigor can inform future safety system development.

### VIII. Thorough Error Handling

**EVERY** failure mode MUST be anticipated and handled explicitly. Error handling MUST:
- Use typed errors/exceptions with specific error codes
- Provide actionable error messages with context
- Distinguish between recoverable and non-recoverable errors
- Log errors with sufficient diagnostic information
- Never silently swallow errors or use bare except/catch blocks
- Validate all external inputs and fail fast on invalid data
- Document expected error conditions in API contracts

**Rationale**: Positioning data post-processing encounters numerous error conditions: missing GNSS signals, conflicting sensor readings, invalid odometer data, topology mismatches. Explicit error handling ensures graceful degradation and clear diagnostics for infrastructure managers troubleshooting data quality issues. While this library supports business processes (not safety operations), robust error patterns provide valuable reference for safety-critical development.

### IX. Data Provenance and Audit Trail

**ALL** data transformations and state changes MUST be traceable. Implementations MUST:
- Record data source and lineage for all derived data
- Log all modifications with timestamp, actor, and reason
- Maintain immutable audit logs that cannot be altered retroactively
- Version data with change history
- Enable reconstruction of system state at any point in time
- Document data flows and transformation logic

**Rationale**: Infrastructure managers need to trace positioning results back to source data: which GNSS measurements, registration events, and odometer readings contributed to each position estimate? Data provenance enables quality assurance, algorithm validation, and troubleshooting when fused positions differ from expectations. Audit trails support regulatory reporting and provide accountability for business-critical location data.

### X. Integration Flexibility

The library MUST support diverse integration patterns. Implementations MUST:
- Provide library API as primary interface with clear entry points
- Expose CLI for command-line automation and toolchain integration
- Use standard data formats (JSON, CSV, binary formats) for interoperability
- Support both synchronous and asynchronous operations where applicable
- Enable embedding in other languages via FFI or language-specific bindings
- Document integration examples for common use cases
- Maintain stable interfaces with semantic versioning

**Rationale**: Infrastructure managers operate diverse IT environments: data warehouses, ETL pipelines, analysis notebooks, legacy positioning systems. The library must integrate via native APIs, command-line batch processing, and language bindings to support varied post-processing workflows and enable gradual adoption alongside existing tools.

### XI. Modern Module Organization (Rust)

**ALL** Rust module definitions MUST use the modern naming convention (Rust 1.30+):

- Use `directory_name.rs` files to declare modules with submodules
- Place submodule files inside `directory_name/` subdirectory
- Use `#[path = "directory_name/submodule.rs"]` attributes when needed
- NEVER use `directory_name/mod.rs` pattern (deprecated pre-1.30 convention)

**Structure example**:

```
src/
  lib.rs
  models.rs              # Module declaration with re-exports
  models/                # Submodules directory
    gnss.rs
    netelement.rs
    result.rs
```

In `models.rs`:

```rust
pub mod gnss;
pub mod netelement;
pub mod result;
```

**Rationale**: The `mod.rs` pattern was deprecated in Rust 1.30 in favor of cleaner directory-level module files. The modern convention improves code navigation, reduces confusion with multiple files named `mod.rs`, and follows current Rust ecosystem best practices. Consistent adherence ensures maintainability and aligns with community standards.

## Licensing and Legal Compliance

### Apache License 2.0 Absolute Compatibility

**ALL** code, dependencies, and incorporated materials MUST be fully compatible with Apache License 2.0.

**PROHIBITED** licenses and materials:
- GPL, LGPL, AGPL (any version) - incompatible copyleft terms
- Creative Commons Non-Commercial (CC BY-NC) - conflicts with commercial use
- Any license restricting commercial use, modification, or distribution
- Proprietary or closed-source components
- Code without explicit license (legally ambiguous)

**PERMITTED** licenses:
- Apache 2.0, MIT, BSD (2-clause, 3-clause)
- ISC, Unlicense, Public Domain (CC0)
- LGPL ONLY via dynamic linking with clear separation

**MANDATORY** compliance checks:
- Every dependency MUST have documented license review
- License scanning tools MUST run in CI/CD pipeline
- New dependencies require explicit license approval
- NOTICE file MUST be maintained with all third-party attributions

**Rationale**: Apache 2.0 enables commercial use, modification, and distribution while providing patent protection. Incompatible licenses create legal liability and restrict project adoption.

## Quality Standards

### Code Quality Gates

**ALL** contributions MUST pass:
- Linter checks with zero errors (warnings documented and justified)
- Type checking (where language supports static typing)
- Test suite execution (all tests green)
- Coverage threshold (minimum 90%, target 100%)
- Performance benchmarks (no regressions without justification)
- Security scanning (no high/critical vulnerabilities)

### Documentation Requirements

**EVERY** public interface MUST include:
- Purpose and usage examples
- Parameter descriptions and types
- Return value specifications
- Error conditions and handling
- Performance characteristics where relevant

### Review Process

**ALL** changes MUST receive peer review verifying:
- Constitution compliance (explicit confirmation)
- Test coverage and TDD adherence
- Error handling completeness
- Documentation accuracy
- Performance impact assessment

## Governance

### Authority and Precedence

This Constitution supersedes all other development practices, style guides, or conventions. In any conflict, Constitutional principles take precedence.

### Amendment Process

Constitution amendments require:
1. Written proposal with rationale and impact analysis
2. Review period (minimum 7 days for community feedback)
3. Approval from project maintainers
4. Migration plan for affected code
5. Version bump according to semantic rules

### Version Semantics

Constitution versions follow MAJOR.MINOR.PATCH:
- **MAJOR**: Breaking changes to core principles or removal of guarantees
- **MINOR**: New principles, sections, or material expansions
- **PATCH**: Clarifications, wording improvements, non-semantic fixes

### Compliance Verification

All pull requests and reviews MUST explicitly verify Constitutional compliance. Complexity that violates principles MUST be justified in writing with explicit approval.

### Living Document

This Constitution is maintained as a living document in `.specify/memory/constitution.md`. Runtime development guidance and tactical practices should reference but not duplicate Constitutional principles.

**Version**: 1.2.0 | **Ratified**: 2025-12-09 | **Last Amended**: 2025-01-20
