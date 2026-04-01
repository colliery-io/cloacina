---
id: fidius-source-packages-replace
level: initiative
title: "Fidius source packages — replace binary archives with build-on-host model"
short_code: "CLOACI-I-0065"
created_at: 2026-04-01T12:32:55.452386+00:00
updated_at: 2026-04-01T22:54:37.305981+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: fidius-source-packages-replace
---

# Fidius source packages — replace binary archives with build-on-host model Initiative

## Context

I-0060 replaced the FFI calling convention with fidius but the packaging format is still the old model: gzip tar archives containing a pre-compiled `.so`/`.dylib` + `manifest.json`. Cross-architecture deployment is impossible and we duplicate what fidius already provides.

Fidius 0.0.5 has a complete source package model: `package.toml` manifest, `pack_package()`/`unpack_package()` (bzip2 tar), content digests, custom extensions. Both Rust and Python packages will use this unified format. No backward compatibility with the old format — clean break.

## Goals & Non-Goals

**Goals:**
- Replace gzip tar + `manifest.json` + compiled dylib with fidius bzip2 tar + `package.toml` + source
- Host builds Rust cdylib locally on demand — cross-architecture deployment works
- Use `fidius_core::package::{pack_package, unpack_package, load_manifest}` directly
- Unified format for both Rust and Python — `language` field in `CloacinaMetadata` distinguishes them
- Reconciler compiles Rust source packages on first load
- All packaged examples produce valid fidius source packages
- Soak test uses real compilable source packages

**Non-Goals:**
- Changing the FFI plugin interface (done in I-0060)
- Pre-compiled binary distribution
- Build server / remote compilation
- Package signing

## Detailed Design

### Package format

`.cloacina` archives are bzip2 tar via `fidius_core::package::pack_package()`:
- `package.toml` (fidius manifest with `[metadata]` = `CloacinaMetadata`)
- Rust source (`Cargo.toml` + `src/`) or Python source (`workflow/` + `vendor/`)

### Loading flow

```
.cloacina archive -> unpack_package() -> load_manifest::<CloacinaMetadata>()
  -> if Rust: cargo build -> fidius_host::load_library() -> register
  -> if Python: validate source, add to sys.path -> register
```

### What gets deleted

`archive.rs`, `compile.rs` (pre-compilation), `manifest.json`/ManifestV2 schema, `is_cloacina_archive()`, `extract_library_from_archive()`, `peek_manifest()`, `is_cloacina_package()`/`extract_so_from_cloacina()`, `PackageLanguage`/`RustRuntime`/`PythonRuntime`, flate2 dependency

## Alternatives Considered

**Keep cloacina's archive format:** Duplicates fidius. Rejected.
**Separate Rust/Python formats:** More complexity. Rejected.
**Backward compat:** More detection code. Rejected — clean break.

## Implementation Plan

1. Update CloacinaMetadata with language + Python fields
2. Add package.toml to examples, replace package_workflow() with fidius packing
3. Replace archive loading with fidius unpack + load_manifest
4. Add compilation to reconciler for Rust packages
5. Update Python loader for fidius format
6. Delete old archive code
7. Update FilesystemWorkflowRegistry
8. Update tests + soak test
9. Update docs
