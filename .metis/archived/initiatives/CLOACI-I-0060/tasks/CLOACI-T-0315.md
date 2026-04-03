---
id: replace-host-side-loading-with
level: task
title: "Replace host-side loading with fidius-host — package_loader, extraction, manifest"
short_code: "CLOACI-T-0315"
created_at: 2026-03-31T23:39:30.535801+00:00
updated_at: 2026-04-01T03:44:45.111760+00:00
parent: CLOACI-I-0060
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0060
---

# Replace host-side loading with fidius-host — package_loader, extraction, manifest

## Parent Initiative

[[CLOACI-I-0060]]

## Objective

Replace the manual dlopen/dlsym/struct-casting code in `package_loader.rs`, `extraction.rs`, and `manifest.rs` with `fidius-host` loading. The host uses `PluginHost` to load dylibs, validate ABI hashes, and call `CloacinaPlugin` methods through a type-safe `PluginHandle`.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `cloacina` Cargo.toml depends on `fidius-host`
- [ ] Source package pipeline: `unpack_package()` → `load_manifest::<CloacinaMetadata>()` → `build_package()` → `load_library()` → `PluginHandle`
- [ ] `package_loader.rs` rewritten to use fidius source package flow instead of binary archive extraction
- [ ] `extraction.rs` calls `handle.call_method::<(), PackageTasksMetadata>()` instead of manual FFI
- [ ] Task execution in `DynamicLibraryTask` uses `handle.call_method::<TaskExecutionRequest, TaskExecutionResult>()`
- [ ] `.cloacina` archives are now source packages (created via `fidius_core::package::pack_package()` with `extension = "cloacina"`)
- [ ] Host builds cdylib locally from source on registration — enables cross-architecture deployment
- [ ] `PackageManifest<CloacinaMetadata>` validation at load time — reject packages with missing/invalid metadata
- [ ] ABI hash validation happens automatically at load time
- [ ] Wire format validation (debug vs release) checked at load time
- [ ] Cloacina's existing `create_package_archive`/`CompileResult`/`generate_manifest` replaced by fidius equivalents
- [ ] Registry storage stores the source `.cloacina` archive (not the compiled binary)
- [ ] All integration tests that load packaged workflows pass

## Implementation Notes

### Key files to modify/replace
- `crates/cloacina/src/registry/loader/package_loader.rs` — rewrite: unpack source archive, validate manifest, build cdylib, load via fidius
- `crates/cloacina/src/registry/loader/task_registrar/extraction.rs` — replace manual FFI with `PluginHandle::call_method`
- `crates/cloacina/src/registry/loader/task_registrar/types.rs` — delete (types now in `cloacina-plugin-api`)
- `crates/cloacina/src/packaging/` — replace binary packaging with fidius source packaging
- `crates/cloacina/src/registry/loader/dynamic_task.rs` — update task execution to use `PluginHandle`

### Source package flow
```
.cloacina archive (source + package.toml)
  → unpack_package() → temp dir with Rust source
  → load_manifest::<CloacinaMetadata>() → validate metadata
  → build_package() → cargo build → cdylib
  → load_library() → fidius validation → PluginHandle
  → handle.call_method() → type-safe calls
```

### Depends on
- T-0313 (interface crate with CloacinaMetadata)
- T-0314 (macro generates fidius plugins)

## Status Updates

*To be added during implementation*
