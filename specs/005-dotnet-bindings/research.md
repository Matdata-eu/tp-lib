# Research: C#/.NET Bindings (tp-net)

**Phase**: Phase 0 — Outline & Research  
**Date**: 2026-05-13  
**Feature**: `005-dotnet-bindings`

---

## Decision 1: Rust → .NET FFI Mechanism

**Decision**: Use **csbindgen** (Cysharp) for FFI layer generation, with a hand-written C# wrapper layer on top.

**Rationale**:
- Production-proven: actively used by Cysharp in NativeCompressions, and by other projects wrapping Bullet Physics, Quiche, and SQLite.
- Aligns with the existing project philosophy: "Rust-first, minimal ceremony" (same as pyo3 for Python).
- Simpler build pipeline — a single `build.rs` invocation generates `NativeMethods.g.cs`; no extra UDL step.
- The public API types (`ProjectedPosition`, `TrainPath`, etc.) are I/O-boundary types; they can be efficiently represented as flat `#[repr(C)]` structs or serialized over a `ByteBuffer` pattern, keeping the FFI surface thin.
- Lower adoption risk than the community-maintained `uniffi-bindgen-cs` backend.

**How it works**:
1. `tp-net/` Rust crate exposes `extern "C"` functions with `#[no_mangle]`, using `#[repr(C)]` structs for simple types and a `ByteBuffer`/JSON-over-bytes pattern for `Vec<T>` and complex types.
2. `csbindgen` runs in `build.rs` and emits `NativeMethods.g.cs` (unsafe P/Invoke stubs).
3. A thin, hand-written `TpLib.cs` (public managed API) wraps the stubs and provides: safe types, XML docs, exception mapping, and `IDisposable` lifecycle management.

**Alternatives considered**:

| Alternative | Why rejected |
|---|---|
| **uniffi + uniffi-bindgen-cs** (NordSecurity) | Community-maintained C# backend (v0.29.4); versioning tightly coupled to uniffi-rs, less proven in production. Would provide better automatic type marshalling but +2-3 weeks learning curve. |
| **Manual cbindgen + P/Invoke** | Higher maintenance burden (dual codebases that can drift). Acceptable for small surfaces; tp-net's surface is large enough that automation is worthwhile. |
| **dotnet-bindgen** | Unmaintained as of 2023; not viable. |

---

## Decision 2: Complex Type Marshalling Strategy

**Decision**: Use a **two-layer marshalling** approach.
- **Layer 1 (FFI)**: Flat `#[repr(C)]` structs for scalar-only types (e.g., `ProjectionConfig`). For collection types (`Vec<ProjectedPosition>`, `Vec<AssociatedNetElement>`) and types with associated-data enums (discard reasons), serialize to **JSON bytes** across the FFI boundary using `serde_json`.
- **Layer 2 (C# wrapper)**: Deserialize JSON into idiomatic C# records/classes using `System.Text.Json`.

**Rationale**:
- JSON-over-FFI is a well-established pattern for complex types (used in several Rust→.NET integrations). The overhead is negligible for tp-lib's batch workloads (not hot-loop).
- Avoids the complexity of manually bridging `geo::Point<f64>`, `chrono::DateTime<FixedOffset>`, `HashMap<String,String>`, and enum variants with embedded payloads.
- The Python bindings already convert these types to Python-native dicts/lists at the FFI boundary; JSON is the equivalent idiom for .NET.

**Alternatives considered**:
- **FlatBuffers/Protocol Buffers**: More efficient but adds two schema files and code-gen steps. Not justified for this batch-oriented API.
- **Pure `#[repr(C)]` structs everywhere**: Would require duplicating every Rust type as a C-compatible struct (no `Vec`, no `Option`, no `String`), leading to manual memory management on the C# side. Higher risk of memory leaks and unsafety.

---

## Decision 3: NuGet Packaging Layout

**Decision**: Publish a **single `TpLib` NuGet package** with RID-specific native binaries embedded in `runtimes/{rid}/native/`. The managed C# assembly goes in `lib/net8.0/`.

**Package layout**:
```
TpLib.{version}.nupkg
├── lib/
│   └── net8.0/
│       └── TpLib.dll           # Managed API + P/Invoke stubs
├── runtimes/
│   ├── win-x64/native/         tp_lib_net.dll
│   ├── linux-x64/native/       libtp_lib_net.so
│   ├── osx-x64/native/         libtp_lib_net.dylib
│   └── osx-arm64/native/       libtp_lib_net.dylib
└── TpLib.nuspec
```

**Rationale**:
- This is the standard Microsoft-documented layout for native NuGet packages (used by SQLitePCLRaw, SkiaSharp, etc.).
- At runtime, the .NET host resolves the correct native binary via the RID graph automatically — zero consumer configuration required.
- `NativeLibrary.SetDllImportResolver` in the C# assembly ensures the correct `runtimes/{rid}/native/` path is used regardless of working directory.

**Alternatives considered**:
- **Separate `TpLib.runtime.{rid}` packages**: Standard pattern for large binaries. For tp-lib, the native binary is small; a single package is simpler for consumers.
- **Embedded resources + runtime extraction**: More portable but adds startup latency and temp-file management. Not needed since NuGet RID support is universal.

---

## Decision 4: CI/CD Build Pipeline

**Decision**: Extend the existing **GitHub Actions** workflow (modeled after `publish-pypi.yml`) with a matrix build across 4 RIDs, then a NuGet pack+publish step.

**Build targets**:
| RID | Cargo target | Runner |
|---|---|---|
| `win-x64` | `x86_64-pc-windows-msvc` | `windows-latest` |
| `linux-x64` | `x86_64-unknown-linux-gnu` | `ubuntu-latest` |
| `osx-x64` | `x86_64-apple-darwin` | `macos-latest` |
| `osx-arm64` | `aarch64-apple-darwin` | `macos-latest` (cross-compile) |

**Rationale**: Follows the same pattern already established for Python wheels. Leverages existing Rust toolchain installation steps.

---

## Decision 5: .NET Target Framework and Package Naming

**Decision**: Target **net8.0** as minimum TFM. Package name: **`TpLib`** (NuGet) / assembly name: `TpLib`.

**Rationale**:
- FR-011 mandates .NET 8+; net8.0 is the current LTS release (supported until Nov 2026) and the most widely deployed modern .NET version.
- `TpLib` matches the existing naming convention (the Python package is `tp-lib-py`; the .NET equivalent follows the namespace convention `TpLib`).

**Alternatives considered**:
- `net6.0`: LTS but end-of-life May 2024. Not worth supporting.
- `netstandard2.0`: Would maximize compatibility but lacks modern C# features (records, nullable reference types, `NativeLibrary`). Excluded per spec requirement for .NET 8+.

---

## Decision 6: C# Project Structure

**Decision**: Create `tp-net/` at workspace root as a **single Rust crate** (cdylib + rlib) plus a **single .NET SDK project** (`tp-net/csharp/TpLib.csproj`) inside it. The Rust crate builds the native library; the .NET project wraps it.

**Source layout**:
```
tp-net/
├── Cargo.toml                  # cdylib + rlib
├── build.rs                    # csbindgen invocation
├── src/
│   └── lib.rs                  # FFI functions (extern "C")
└── csharp/
    ├── TpLib.csproj            # net8.0 SDK project
    ├── TpLib.cs                # Public managed API
    ├── NativeMethods.g.cs      # (generated by csbindgen)
    ├── Exceptions.cs           # Typed exception hierarchy
    └── Tests/
        ├── TpLib.Tests.csproj
        └── ProjectionTests.cs
```

**Rationale**: Mirrors `tp-py/` structure (Rust src + language-specific wrapper in subfolder). Keeps the workspace consistent.

---

## Resolved NEEDS CLARIFICATION

All items marked "NEEDS CLARIFICATION" in the Technical Context are resolved:

| Unknown | Resolution |
|---|---|
| FFI mechanism | csbindgen (see Decision 1) |
| Complex type marshalling | JSON-over-FFI + System.Text.Json (see Decision 2) |
| NuGet packaging | Single package with RID runtimes/ layout (see Decision 3) |
| CI/CD pipeline | GitHub Actions matrix (see Decision 4) |
| Target framework | net8.0 minimum (see Decision 5) |
| Project structure | tp-net/ with csharp/ subdirectory (see Decision 6) |
