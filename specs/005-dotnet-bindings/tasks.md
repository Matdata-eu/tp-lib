# Tasks: C#/.NET Bindings (tp-net)

**Feature**: `005-dotnet-bindings`
**Input**: `specs/005-dotnet-bindings/`
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, contracts/api.md ✓, quickstart.md ✓

---

## Format: `[ID] [P?] [Story] Description with file path`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: User story label — US1, US2, US3, US5 (setup/foundational/polish phases carry no label)

---

## Phase 1: Setup (Scaffolding)

**Purpose**: Register `tp-net` in the Cargo workspace and create all project files as stubs.
No logic — empty/placeholder content only. After this phase `cargo check --workspace` and
`dotnet build tp-net/csharp/TpLib.csproj` must both succeed without errors.

- [X] T001 Add `tp-net` to `members` in root `Cargo.toml` workspace
- [X] T002 [P] Create `tp-net/Cargo.toml` — `crate-type = ["cdylib","rlib"]`; deps: `tp-lib-core` (path), `serde`, `serde_json`; build-deps: `csbindgen`
- [X] T003 [P] Create stub `tp-net/src/lib.rs`, `tp-net/src/ffi.rs`, `tp-net/src/marshal.rs` (empty modules with `mod` declarations)
- [X] T004 [P] Create `tp-net/build.rs` — csbindgen invocation that generates `tp-net/csharp/NativeMethods.g.cs` from `src/lib.rs`
- [X] T005 [P] Create `tp-net/csharp/TpLib.csproj` — `<TargetFramework>net8.0</TargetFramework>`, `<AssemblyName>TpLib</AssemblyName>`, allow unsafe blocks, NuGet packaging metadata placeholders (`<GeneratePackageOnBuild>`, `<Version>`, `<PackageId>TpLib</PackageId>`)
- [X] T006 [P] Create `tp-net/csharp/Tests/TpLib.Tests.csproj` — `net8.0`, xUnit 2 + `Microsoft.NET.Test.Sdk`, project reference to `TpLib.csproj`
- [X] T007 [P] Create stub C# source files: `tp-net/csharp/Exceptions.cs`, `tp-net/csharp/Enums.cs`, `tp-net/csharp/Models.cs`, `tp-net/csharp/TpLib.cs`, `tp-net/csharp/TpLibNative.cs` — each with `namespace TpLib;` and empty type stubs only

**Checkpoint**: `cargo check --workspace` passes with `tp-net` visible; `dotnet build tp-net/csharp/TpLib.csproj` compiles the stubs.

---

## Phase 2: Foundational (FFI Infrastructure + Core Types)

**Purpose**: Implement the complete FFI layer and all public C# types that every user story depends on.
No user story can begin until this phase is complete.

**⚠️ CRITICAL**: Complete before any user story implementation.

### FFI Layer (Rust)

- [X] T008 Implement `ByteBuffer` struct (`ptr: *mut u8`, `len: i32`, `cap: i32`) and `#[no_mangle] extern "C" fn tp_net_free_byte_buffer(buf: ByteBuffer)` in `tp-net/src/ffi.rs`; implement `#[repr(C)] ProjectionConfigFfi` and `#[repr(C)] PathConfigFfi` flat structs mirroring all scalar fields from data-model.md
- [X] T009 [P] Implement JSON serialization helpers in `tp-net/src/marshal.rs` — `fn to_json_bytes<T: Serialize>(val: &T) -> ByteBuffer` (allocates heap bytes, caller must free via `tp_net_free_byte_buffer`); `fn from_json_bytes<'a, T: Deserialize<'a>>(ptr: *const u8, len: i32) -> Result<T, serde_json::Error>` (reads C# buffer without taking ownership); use `serde_json::to_vec` / `serde_json::from_slice`

### Exception Hierarchy (C#)

- [X] T010 [P] Implement `TpLibException` (base, `public class`), `TpLibParseException`, `TpLibProjectionException`, `TpLibPathException`, `TpLibDetectionException` (all sealed, each with `(string message)` and `(string message, Exception inner)` constructors) in `tp-net/csharp/Exceptions.cs`

### Enumerations (C#)

- [X] T011 [P] Implement all enums in `tp-net/csharp/Enums.cs`: `DetectionKind` (Punctual, Linear), `Navigability` (Both, Forward, Backward, None), `PathCalculationMode` (TopologyBased, FallbackIndependent), `PathOrigin` (Algorithm, UserAdded, UserEdited), `DetectionStatus` (Applied, Resolved, Discarded)

### Public Record Types (C#)

- [X] T012 [P] Implement output record types in `tp-net/csharp/Models.cs`: `ProjectedPosition` (all 10 properties from data-model.md; `Intrinsic` is `double?`), `AssociatedNetElement` (7 properties), `TrainPath` (Segments, OverallProbability, CalculatedAt), `PathResult` (Path, Mode, ProjectedPositions, Warnings, DetectionProvenance, HasPath computed), `PreparedDetections` (Records, Warnings)
- [X] T013 [P] Implement `DetectionTimestamp` (abstract sealed base with `Single` and `Range` sealed subclasses) and `DetectionRecord` (all fields from data-model.md; `Metadata` as `IReadOnlyDictionary<string, string>`) in `tp-net/csharp/Models.cs`
- [X] T014 [P] Implement `NetworkSegment` sealed record (`Id`, `Coordinates` as `IReadOnlyList<(double Longitude, double Latitude)>`, `Crs = "EPSG:4326"`) and `NetworkRelation` sealed record (all 6 fields) in `tp-net/csharp/Models.cs`
- [X] T015 [P] Implement `GnssRecord` record (`Latitude`, `Longitude`, `Timestamp` as `DateTimeOffset`) in `tp-net/csharp/Models.cs`

### Input Wrapper Types (C#)

- [X] T016 Implement `NetworkInput` sealed class in `tp-net/csharp/Models.cs` — private constructor holding internal JSON string; `public static NetworkInput FromGeoJson(string geoJson)` (validates non-null/empty, stores as-is); `public static NetworkInput FromRecords(IEnumerable<NetworkSegment> segments, IEnumerable<NetworkRelation> relations)` (serializes both to a merged GeoJSON FeatureCollection internally using `System.Text.Json`; mixed feature types matching tp-lib format); `internal string AsJson()` accessor
- [X] T017 Implement `GnssInput` sealed class in `tp-net/csharp/Models.cs` — private constructor holding internal JSON string; `public static GnssInput FromGeoJson(string geoJson)`; `public static GnssInput FromCsv(string csv)` (wraps CSV string directly — Rust core parses it); `public static GnssInput FromRecords(IEnumerable<GnssRecord> records)` (serializes to GeoJSON FeatureCollection of Points with `latitude`, `longitude`, `timestamp` properties); `internal string AsJson()` accessor

### Native Library Resolver (C#)

- [X] T018 Implement `TpLibNative` internal static class in `tp-net/csharp/TpLibNative.cs` — registers `NativeLibrary.SetDllImportResolver` in a static constructor that resolves `tp_lib_net` (Windows: `.dll`, Linux: `lib*.so`, macOS: `lib*.dylib`) from the `runtimes/{rid}/native/` path relative to the managed assembly; add `internal static void FreeByteBuffer(ByteBufferNative buf)` P/Invoke stub calling `tp_net_free_byte_buffer`; expose `internal const string LibName = "tp_lib_net"` used by all P/Invoke `[DllImport]` attributes in `NativeMethods.g.cs`

**Checkpoint**: `cargo build -p tp-net` produces a native library; `dotnet build tp-net/csharp/TpLib.csproj` compiles all foundational types without errors.

---

## Phase 3: User Story 1 — GNSS Projection in C# (Priority: P1) 🎯 MVP

**Goal**: A C# consumer calls `Projection.ProjectGnss(networkGeoJson, gnssGeoJson)` and receives a
typed `IReadOnlyList<ProjectedPosition>`. Also `Projection.ProjectOntoPath(network, gnss, path)` for
re-projection onto a pre-calculated path with `Intrinsic` populated on every position.

**Independent Test**: With `test-data/sample_network.geojson` and `test-data/sample_gnss.geojson`:
run `Projection.ProjectGnss(networkGeoJson, gnssGeoJson)` → list is non-empty, every element has
`NetelementId != null`, `MeasureMeters >= 0`, `ProjectionDistanceMeters >= 0`, `Intrinsic == null`.
For `ProjectOntoPath`: same data, every element has `Intrinsic` in `[0, 1]`.

### Implementation

- [X] T019 [US1] Implement `extern "C" fn tp_net_project_gnss(network_json_ptr, network_json_len, gnss_json_ptr, gnss_json_len, config: ProjectionConfigFfi, out_len: *mut i32) -> *mut u8` in `tp-net/src/lib.rs` — deserialize inputs via `marshal.rs`, call `tp_lib_core` projection, serialize `Vec<ProjectedPosition>` to JSON bytes via `marshal::to_json_bytes`, return pointer; errors return null with `out_len = -1`
- [X] T020 [US1] Implement `extern "C" fn tp_net_project_onto_path(network_json_ptr, network_json_len, gnss_json_ptr, gnss_json_len, path_json_ptr, path_json_len, config: PathConfigFfi, out_len: *mut i32) -> *mut u8` in `tp-net/src/lib.rs` — same pattern; pass pre-calculated `TrainPath` deserialized from JSON
- [X] T021 [US1] Implement `public static class Projection` in `tp-net/csharp/TpLib.cs` — `ProjectGnss(NetworkInput, GnssInput, ProjectionConfig?)` calls the P/Invoke stub from `NativeMethods.g.cs`, reads returned bytes, deserializes via `System.Text.Json` to `List<ProjectedPosition>`, throws `TpLibProjectionException` on null return, frees native buffer via `TpLibNative.FreeByteBuffer`; add all convenience overloads per `contracts/api.md` (`string, string, …`)
- [X] T022 [US1] Implement `ProjectOntoPath(NetworkInput, GnssInput, TrainPath, PathConfig?)` and its convenience overloads in `tp-net/csharp/TpLib.cs` — serialize `TrainPath` to JSON for the FFI call; deserialize result to `IReadOnlyList<ProjectedPosition>`

### Tests

- [X] T023 [US1] Write `ProjectionTests` in `tp-net/csharp/Tests/ProjectionTests.cs` — cover: `ProjectGnss` with `test-data/sample_network.geojson` + `test-data/sample_gnss.geojson` returns non-empty list with valid fields; custom `ProjectionConfig` respected; `ProjectGnss` with `string, string` convenience overload works; `ProjectOntoPath` with pre-calculated path returns positions with `Intrinsic` in `[0,1]`; null network throws `ArgumentNullException`; malformed GeoJSON throws `TpLibParseException`

**Checkpoint**: `dotnet test tp-net/csharp/Tests/` — `ProjectionTests` all green. MVP deliverable functional.

---

## Phase 4: User Story 2 — Train Path Calculation in C# (Priority: P2)

**Goal**: A C# consumer calls `PathCalculation.CalculateTrainPath(networkGeoJson, gnssGeoJson)` and
receives a `PathResult` with a `TrainPath` (when found), projected positions, and diagnostics.
`result.HasPath`, `result.Mode`, and `result.Warnings` are all accessible.

**Independent Test**: With `test-data/sample_network.geojson` and `test-data/sample_gnss.geojson`:
`result.HasPath == true`, `result.Path!.Segments.Count > 0`,
`result.Path!.OverallProbability` is in `[0, 1]`, `result.ProjectedPositions` is non-empty.

### Implementation

- [X] T024 [US2] Implement `extern "C" fn tp_net_calculate_train_path(network_json_ptr, network_json_len, gnss_json_ptr, gnss_json_len, config: PathConfigFfi, detections_json_ptr, detections_json_len, out_len: *mut i32) -> *mut u8` in `tp-net/src/lib.rs` — pass `detections_json_ptr` as optional (null pointer = no detections); deserialize optional `PreparedDetections`; serialize `PathResult` to JSON bytes
- [X] T025 [US2] Implement `public static class PathCalculation` in `tp-net/csharp/TpLib.cs` — `CalculateTrainPath(NetworkInput, GnssInput, PathConfig?, PreparedDetections?)` P/Invoke call, deserialize `PathResult`, throw `TpLibPathException` on error, free native buffer; add all convenience overloads per `contracts/api.md` (`string, string, …`); serialize optional `PreparedDetections` to JSON before FFI call (null if not provided)

### Tests

- [X] T026 [US2] Write `PathCalculationTests` in `tp-net/csharp/Tests/PathCalculationTests.cs` — cover: `CalculateTrainPath` with `test-data/sample_network.geojson` + `test-data/sample_gnss.geojson` returns `HasPath == true` with non-empty segments; `result.Mode` is `TopologyBased` or `FallbackIndependent` (not null); `result.ProjectedPositions` non-empty when `PathConfig.PathOnly == false`; `result.ProjectedPositions` empty when `PathConfig.PathOnly == true`; `string, string` convenience overload works; custom `PathConfig` respected; malformed GeoJSON throws `TpLibParseException`; `ProjectOntoPath` round-trip: calculate path → re-project → positions have `Intrinsic` populated

**Checkpoint**: `dotnet test tp-net/csharp/Tests/` — `PathCalculationTests` all green.

---

## Phase 5: User Story 5 — Database-Backed Service Without Disk I/O (Priority: P2)

**Goal**: A C# backend service that reads network and GNSS data from a database (as typed record
objects) calls `NetworkInput.FromRecords()` and `GnssInput.FromRecords()` with no temporary files
written to disk, and the projection/path calculation results are identical to the file-based path.
FR-012 compliance: no disk I/O anywhere in the code path.

**Independent Test**: Using `test-data/sample_network.geojson` and `test-data/sample_gnss.geojson`
as source: parse them manually in C# test code into `IEnumerable<NetworkSegment>` +
`IEnumerable<NetworkRelation>` and `IEnumerable<GnssRecord>`; call
`Projection.ProjectGnss(NetworkInput.FromRecords(…), GnssInput.FromRecords(…))` and compare the
result to the file-based call — projected netelement IDs and measures must match.

### Tests

- [X] T027 [US5] Write `InMemoryInputTests` in `tp-net/csharp/Tests/InMemoryInputTests.cs` — cover: `NetworkInput.FromRecords(segments, relations)` + `GnssInput.FromRecords(records)` round-trip via `ProjectGnss` produces identical `NetelementId` and `MeasureMeters` values as the GeoJSON file path; `GnssInput.FromCsv(csvString)` matches `GnssInput.FromGeoJson(geoJsonString)` on the same positions; `CalculateTrainPath` via `FromRecords` path returns `HasPath == true`; `DetectionPreparation.PrepareDetections` via `FromRecords` path succeeds with empty detection list; zero files on disk created during any call (verify using temp-directory isolation)
- [X] T028 [US5] Write `DetectionRecordSerializationTests` in `tp-net/csharp/Tests/InMemoryInputTests.cs` — cover: `DetectionTimestamp.Single` round-trips through `System.Text.Json` with timezone preserved; `DetectionTimestamp.Range` (From, To) round-trips; `DetectionRecord.Metadata` dictionary survives serialization; `DetectionKind.Punctual` and `DetectionKind.Linear` serialize/deserialize correctly

**Checkpoint**: `dotnet test tp-net/csharp/Tests/` — `InMemoryInputTests` all green. FR-012 compliance confirmed.

---

## Phase 6: User Story 3 — Train Detection Preparation in C# (Priority: P3)

**Goal**: A C# consumer calls `DetectionPreparation.PrepareDetections(network, gnss, detections)` and
receives a `PreparedDetections` where every input record has a non-null `Status`
(Applied / Resolved / Discarded). The result can be passed directly to `CalculateTrainPath`.

**Independent Test**: Load `test-data/sample_network.geojson` and one of the sample detection files.
Call `PrepareDetections` → `result.Records.Count` equals the number of input detections; no exception
thrown; passing the result to `CalculateTrainPath` succeeds and `result.DetectionProvenance` is
non-empty.

### Implementation

- [X] T029 [US3] Implement `extern "C" fn tp_net_prepare_detections(network_json_ptr, network_json_len, gnss_json_ptr, gnss_json_len, detections_json_ptr, detections_json_len, cutoff_distance_meters: f64, out_len: *mut i32) -> *mut u8` in `tp-net/src/lib.rs` — deserialize `Vec<DetectionRecord>` from JSON; call `tp_lib_core` detection preparation; serialize `PreparedDetections` to JSON bytes
- [X] T030 [US3] Implement `public static class DetectionPreparation` in `tp-net/csharp/TpLib.cs` — `PrepareDetections(NetworkInput, GnssInput, IEnumerable<DetectionRecord>, double cutoffDistanceMeters = 2.5)` P/Invoke call, serialize detections list to JSON, deserialize `PreparedDetections`, throw `TpLibDetectionException` on error; add `string networkGeoJson` convenience overload per `contracts/api.md`

### Tests

- [X] T031 [P] [US3] Write `DetectionPreparationTests` in `tp-net/csharp/Tests/DetectionPreparationTests.cs` — cover: `PrepareDetections` with `test-data/sample_network.geojson` + `test-data/sample_gnss.geojson` + empty detections list returns `PreparedDetections` with empty `Records`; with `test-data/sample_detections_punctual.csv` rows parsed to `DetectionRecord` list → `result.Records.Count` equals input count, each record has non-null status; out-of-window detections produce `DetectionStatus.Discarded`; `PreparedDetections` passed to `CalculateTrainPath` succeeds and `PathResult.DetectionProvenance` is non-empty; `string` network convenience overload works
- [X] T032 [P] [US3] Write end-to-end detection-anchored path test in `tp-net/csharp/Tests/DetectionPreparationTests.cs` — cover: prepare punctual detections → calculate path with detections → verify `result.DetectionProvenance` length matches input detection count; detections with `DetectionStatus.Applied` have non-null netelement IDs

**Checkpoint**: `dotnet test tp-net/csharp/Tests/` — `DetectionPreparationTests` all green.

---

## Phase 7: User Story 4 — NuGet Package Distribution (Priority: P4)

**Goal**: A .NET developer runs `dotnet add package TpLib` and can call `Projection.ProjectGnss(…)`
without installing Rust toolchain, copying native binaries, or setting `LD_LIBRARY_PATH`.
The correct native binary is resolved automatically via the NuGet RID graph.

**Independent Test**: Build the NuGet package locally; `dotnet pack`; install into a fresh test
project with `dotnet add package TpLib --source ./nupkg`; call `Projection.ProjectGnss(…)` —
succeeds without any native-library configuration. Repeat on each supported platform.

### NuGet Packaging Configuration

- [X] T033 Configure NuGet package layout in `tp-net/csharp/TpLib.csproj` — set packaging props: `<PackageId>TpLib</PackageId>`, `<IsPackable>true</IsPackable>`, `<IncludeBuildOutput>true</IncludeBuildOutput>`; add `<Content>` items that copy the locally-built native library (`tp_lib_net.dll` / `.so` / `.dylib`) into `runtimes/{rid}/native/` inside the package; add `<files>` section in `.nuspec` stub or equivalent MSBuild targets to embed all 4 RID variants when building on CI
- [X] T034 [P] Create `tp-net/csharp/TpLib.targets` MSBuild targets file — on `dotnet restore`, copy `runtimes/{rid}/native/tp_lib_net{ext}` to the output directory so `dotnet run` works without installing the package from NuGet; referenced via `<None Include="TpLib.targets" Pack="true" PackagePath="build/TpLib.targets" />`

### CI/CD Pipeline

- [X] T035 Create `.github/workflows/publish-nuget.yml` — matrix build with 4 jobs (win-x64 / linux-x64 / osx-x64 / osx-arm64) mirroring `publish-pypi.yml`; each job: checkout + Rust toolchain setup + `cargo build --release -p tp-net --target {cargo_target}` + upload native artifact; final job: download all 4 artifacts + `dotnet pack` + `dotnet nuget push` to NuGet.org using `NUGET_API_KEY` secret
- [X] T036 [P] Add `tp-net` to existing `.github/workflows/ci.yml` — add `cargo test -p tp-net` step; add `dotnet test tp-net/csharp/Tests/TpLib.Tests.csproj` step (linux only; native library built from workspace); gate both steps on the `tp-net` path filter

**Checkpoint**: `dotnet pack tp-net/csharp/TpLib.csproj` produces a `.nupkg` with `lib/net8.0/TpLib.dll`; a `dotnet add package TpLib --source ./nupkg` + simple console app calling `Projection.ProjectGnss` runs without errors locally.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, XML doc comments, and README updates that affect multiple user stories
and must be done after the implementation is stable.

- [X] T037 Add XML documentation comments to all public API surface in `tp-net/csharp/TpLib.cs`, `tp-net/csharp/Models.cs`, `tp-net/csharp/Enums.cs`, `tp-net/csharp/Exceptions.cs` — verify `<GenerateDocumentationFile>true</GenerateDocumentationFile>` is set in `TpLib.csproj` so no CS1591 warnings remain
- [X] T038 [P] Write `tp-net/README.md` — cover: prerequisites (.NET 8 SDK), `dotnet add package TpLib`, quick example (ProjectGnss + CalculateTrainPath), supported platforms table (win-x64, linux-x64, osx-x64, osx-arm64), link to quickstart.md and contracts/api.md
- [X] T039 Update root `README.md` — add NuGet badge (`[![NuGet](https://img.shields.io/nuget/v/TpLib.svg)](https://www.nuget.org/packages/TpLib/)`); add `.NET` to the "It exposes…" sentence in the features list; add `tp-net/` entry to the Project Structure tree
- [X] T040 Update `docs/OpenRail-onboarding.md` — update the tech stack section to include `.NET bindings: csbindgen + System.Text.Json, published as TpLib NuGet package (net8.0)`; update the "It exposes…" sentence from `a .NET API (...)` to include the actual package name and NuGet install command

**Checkpoint**: `dotnet build tp-net/csharp/TpLib.csproj /warnaserror:CS1591` succeeds; `README.md` NuGet badge renders; onboarding doc reflects the .NET binding.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 completion — **BLOCKS all user stories**
- **US1 (Phase 3)**: Depends on Phase 2 completion — MVP increment; no dependency on US2/US3/US5
- **US2 (Phase 4)**: Depends on Phase 2 completion — no dependency on US1 (path calc is independent)
- **US5 (Phase 5)**: Depends on Phase 3 + Phase 4 completion — validates the full in-memory path end-to-end
- **US3 (Phase 6)**: Depends on Phase 2 completion — no dependency on US1/US2/US5
- **US4 (Phase 7)**: Depends on Phase 3 + Phase 4 + Phase 6 completion — packages the completed API
- **Polish (Phase 8)**: Depends on all user story phases completing

### User Story Dependencies

- **US1 (P1)**: Can start after Phase 2 — no dependency on other stories
- **US2 (P2)**: Can start after Phase 2 — no dependency on other stories
- **US5 (P2)**: Requires US1 + US2 to be complete (validates both code paths)
- **US3 (P3)**: Can start after Phase 2 — no dependency on US1/US2
- **US4 (P4)**: Requires US1 + US2 + US3 to be complete (packages everything)

### Within Each User Story

- Rust FFI function (`tp-net/src/lib.rs`) before C# wrapper class
- C# wrapper before tests
- Tests written after implementation (this feature does not use TDD)

### Parallel Opportunities

- All `[P]` tasks within Phase 1 can run in parallel (different files)
- All `[P]` tasks within Phase 2 can run in parallel (different files; Rust FFI is independent of C# types)
- US1 and US2 can be worked in parallel once Phase 2 is complete (entirely separate static classes)
- US3 and US2/US1 can be worked in parallel (no dependency between detection and projection)
- Within US1: T019 and T020 can run in parallel (two separate FFI functions)

---

## Summary

| Phase | Tasks | User Story | Priority |
|---|---|---|---|
| Phase 1 — Setup | T001–T007 | — | — |
| Phase 2 — Foundational | T008–T018 | — | — |
| Phase 3 — GNSS Projection | T019–T023 | US1 | P1 🎯 MVP |
| Phase 4 — Train Path Calculation | T024–T026 | US2 | P2 |
| Phase 5 — Database-Backed Service | T027–T028 | US5 | P2 |
| Phase 6 — Detection Preparation | T029–T032 | US3 | P3 |
| Phase 7 — NuGet Packaging | T033–T036 | US4 | P4 |
| Phase 8 — Polish | T037–T040 | — | — |

**Total**: 40 tasks across 8 phases.

**Parallel opportunities**: 19 tasks marked `[P]`.

**Independent test criteria**:
- US1: `Projection.ProjectGnss` with sample GeoJSON files → non-empty typed list ✓
- US2: `PathCalculation.CalculateTrainPath` → `HasPath == true`, `Segments.Count > 0` ✓
- US5: `FromRecords` path produces identical results to `FromGeoJson` path ✓
- US3: `DetectionPreparation.PrepareDetections` → `Records.Count == input count` ✓
- US4: `dotnet add package TpLib` → `ProjectGnss` works without native setup ✓

**Suggested MVP scope**: Phase 1 + Phase 2 + Phase 3 (T001–T023)
