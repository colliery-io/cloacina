---
id: fidius-source-packages-replace
level: initiative
title: "Fidius source packages — replace binary archives with build-on-host model"
short_code: "CLOACI-I-0062"
created_at: 2026-04-01T11:49:17.787365+00:00
updated_at: 2026-04-01T11:49:17.787365+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: fidius-source-packages-replace
---

# Fidius source packages — replace binary archives with build-on-host model

## Context

I-0060 replaced the FFI calling convention (how methods are called on loaded dylibs) with fidius. But the packaging format — how workflows are distributed — is still the old model: gzip tar archives containing a pre-compiled `.so`/`.dylib` + `manifest.json`.

This creates two problems:
1. **Cross-architecture deployment is impossible** — a dylib compiled on macOS ARM can't run on Linux x86_64
2. **Format mismatch** — we use fidius for FFI but cloacina's own archive format for packaging, duplicating what fidius already provides

Fidius 0.0.5 has a complete source package model:
- `package.toml` manifest with host-defined metadata schema
- `pack_package()` / `unpack_package()` — bzip2 tar archives
- `package_digest()` — SHA-256 content digests
- Custom file extensions (`extension = "cloacina"`)
- `PackageManifest<M>` generic over host metadata schema

## Goals & Non-Goals

**Goals:**
- Replace gzip tar + `manifest.json` + compiled dylib with fidius bzip2 tar + `package.toml` + Rust source
- Host builds cdylib locally on demand — cross-architecture deployment works
- Use `fidius_core::package::{pack_package, unpack_package, load_manifest}` directly
- `.cloacina` archives contain source code, not binaries
- `CloacinaMetadata` schema (from `cloacina-workflow-plugin`) validates `[metadata]` section
- Reconciler compiles source packages on first load, caches compiled artifacts
- All packaged examples produce valid fidius source packages
- Soak test uses real compilable source packages

- Unified format for both Rust and Python packages — `package.toml` language field distinguishes them
- Python packages migrate from gzip tar + `manifest.json` to fidius bzip2 tar + `package.toml`
- No backward compatibility — clean break from old format

**Non-Goals:**
- Changing the FFI plugin interface (already done in I-0060)
- Pre-compiled binary distribution (source-only for now)
- Build server / remote compilation
- Package signing (fidius has digest support but signing is future)

## Detailed Design

### Package format (new)

A `.cloacina` archive is a bzip2 tar (via `fidius_core::package::pack_package()`) containing:

**Rust package:**
```
my-workflow-1.0.0/
  package.toml          # fidius manifest with [metadata] = CloacinaMetadata
  Cargo.toml            # standard Rust project
  src/
    lib.rs              # #[workflow] + #[task] macros
```

**Python package:**
```
my-workflow-1.0.0/
  package.toml          # fidius manifest with [metadata] = CloacinaMetadata
  workflow/
    __init__.py
    tasks.py            # workflow task definitions
  vendor/               # vendored dependencies (optional)
```

`package.toml` (unified for both languages):
```toml
[package]
name = "my-workflow"
version = "1.0.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "data_processing"
language = "rust"       # or "python"
description = "Data processing pipeline"
author = "Team"

# Python-only fields
requires_python = ">=3.11"
entry_module = "workflow.tasks"
```

### Loading flow (new)

```
.cloacina archive
  → fidius_core::package::unpack_package() → temp dir with source
  → fidius_core::package::load_manifest::<CloacinaMetadata>() → validate metadata
  → cargo build --lib (compile source to cdylib)
  → fidius_host::load_library() → validate ABI, load plugin
  → handle.call_method(0, &()) → PackageTasksMetadata
  → register tasks/workflows in host registry
```

### What gets deleted

- `archive.rs` — `create_package_archive()` (gzip tar creation)
- `compile.rs` — `execute_cargo_build()`, `find_compiled_library()` (moved to reconciler)
- `manifest.json` format — replaced by `package.toml`
- `ManifestV2` / `Manifest` / `RustRuntime` schema types
- `is_cloacina_archive()` — gzip magic byte check
- `extract_library_from_archive()` — dylib extraction from tar
- `peek_manifest()` — `manifest.json` reader from tar
- `is_cloacina_package()` / `extract_so_from_cloacina()` in registry

### What stays

- `validation.rs` — Cargo.toml structure checks (still needed for source validation)
- `PackageMetadata` — internal type for extracted metadata (fields change)
- `TaskRegistrar` — registers tasks from loaded plugin (unchanged)
- `DynamicLibraryTask` — executes tasks via fidius PluginHandle (unchanged)
- `FilesystemWorkflowRegistry` — scans directories (updated to detect source packages)
- Python packaging path — unchanged for now

## Alternatives Considered

**Keep cloacina's archive format, change contents to source:** More independent from fidius but duplicates fidius's packaging infrastructure. Rejected — less code to maintain with fidius.

**Pre-compiled binary packages with platform tags:** Avoids build-on-host but can't cross-compile. Rejected — source packages solve cross-arch deployment.

## Implementation Plan

1. Add `package.toml` to packaged examples + update `package_workflow()` to create fidius source packages
2. Replace archive loading with fidius `unpack_package()` + `load_manifest::<CloacinaMetadata>()`
3. Add compilation step to reconciler — build source packages on demand
4. Delete old archive format code (gzip tar, manifest.json, dylib extraction)
5. Update `FilesystemWorkflowRegistry` to detect source packages
6. Update integration tests + soak test with real compilable source packages
7. Update docs and examples
