---
id: fidius-source-packages-replace
level: initiative
title: "Fidius source packages — replace binary archives with build-on-host model"
short_code: "CLOACI-I-0064"
created_at: 2026-04-01T12:26:43.876040+00:00
updated_at: 2026-04-01T12:26:43.876040+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: fidius-source-packages-replace
---

# Fidius source packages — replace binary archives with build-on-host model Initiative

## Context

I-0060 replaced the FFI calling convention with fidius but the packaging format is still the old model: gzip tar archives containing a pre-compiled `.so`/`.dylib` + `manifest.json`. This means cross-architecture deployment is impossible and we duplicate what fidius already provides.

Fidius 0.0.5 has a complete source package model: `package.toml` manifest, `pack_package()`/`unpack_package()` (bzip2 tar), content digests, custom file extensions. Both Rust and Python packages will use this unified format. No backward compatibility with the old format.

## Goals & Non-Goals

**Goals:**
- Replace gzip tar + `manifest.json` + compiled dylib with fidius bzip2 tar + `package.toml` + source code
- Host builds Rust cdylib locally on demand — cross-architecture deployment works
- Use `fidius_core::package::{pack_package, unpack_package, load_manifest}` directly
- Unified format for both Rust and Python — `language` field in `CloacinaMetadata` distinguishes them
- Python packages migrate from gzip tar + `manifest.json` to fidius bzip2 tar + `package.toml`
- Reconciler compiles Rust source packages on first load, caches compiled artifacts
- All packaged examples produce valid fidius source packages
- Soak test uses real compilable source packages
- No backward compatibility — clean break from old format

**Non-Goals:**
- Changing the FFI plugin interface (already done in I-0060)
- Pre-compiled binary distribution (source-only for now)
- Build server / remote compilation
- Package signing (fidius has digest support but signing is future)

## Detailed Design

### Package format (unified for Rust and Python)

A `.cloacina` archive is a bzip2 tar (via `fidius_core::package::pack_package()`).

**Rust:**
```
my-workflow-1.0.0/
  package.toml
  Cargo.toml
  src/lib.rs
```

**Python:**
```
my-workflow-1.0.0/
  package.toml
  workflow/tasks.py
  vendor/           (optional)
```

**package.toml:**
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
description = "..."
author = "..."
requires_python = ">=3.11"   # python only
entry_module = "workflow.tasks"  # python only
```

### Loading flow

```
.cloacina archive (bzip2 tar)
  -> unpack_package() -> temp dir with source
  -> load_manifest::<CloacinaMetadata>() -> validate metadata
  -> if Rust: cargo build -> cdylib -> fidius_host::load_library()
  -> if Python: validate source, add to sys.path
  -> register tasks/workflows
```

### What gets deleted

- `archive.rs` — gzip tar creation
- `compile.rs` — pre-compilation (moves to reconciler)
- `manifest.json` / ManifestV2 schema types
- `is_cloacina_archive()` — gzip magic byte check
- `extract_library_from_archive()` — dylib extraction from tar
- `peek_manifest()` — manifest.json reader from gzip tar
- `is_cloacina_package()` / `extract_so_from_cloacina()`
- `PackageLanguage`, `RustRuntime`, `PythonRuntime` schema types
- flate2 dependency (gzip no longer needed)

## Alternatives Considered

**Keep cloacina's archive format, change contents:** Duplicates fidius packaging. Rejected.
**Separate formats for Rust/Python:** More complexity. Rejected — unified `package.toml` with language field.
**Backward compatibility with old format:** More detection code. Rejected — clean break.

## Implementation Plan

1. Update `CloacinaMetadata` with language field and Python fields
2. Add `package.toml` to packaged examples, replace `package_workflow()` with fidius packing
3. Replace archive loading with fidius `unpack_package()` + `load_manifest()`
4. Add compilation step to reconciler for Rust packages
5. Update Python loader for fidius format
6. Delete old archive code (gzip tar, manifest.json, dylib extraction, ManifestV2)
7. Update FilesystemWorkflowRegistry to detect fidius source packages
8. Update tests and soak test with real source packages
9. Update docs
