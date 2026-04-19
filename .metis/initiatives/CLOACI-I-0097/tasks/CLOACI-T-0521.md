---
id: t3-compiler-build-loop-claim
level: task
title: "T3: Compiler build loop — claim, dispatch-by-language, persist artifact"
short_code: "CLOACI-T-0521"
created_at: 2026-04-18T01:50:00+00:00
updated_at: 2026-04-19T00:23:01.672554+00:00
parent: CLOACI-I-0097
blocked_by: [CLOACI-T-0520]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0097
---

# T3: Compiler build loop — claim, dispatch-by-language, persist artifact

## Parent Initiative

CLOACI-I-0097 — Compiler Service

## Objective

Replace T2's stub build loop with the real thing: claim a row, fetch source bytes, unpack, run language-appropriate build step, persist compiled artifact, mark success (or capture error + mark failed).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] On each tick (every `poll_interval_ms`), compiler calls `claim_next_build()` until it either claims a row or exhausts the queue. Only one row claimed per iteration; loop continues on next tick.
- [ ] Build pipeline per claimed row:
  1. `storage.retrieve_binary(registry_id)` → source archive bytes.
  2. Unpack via `fidius_core::package::unpack_package` into `$tmp_root/<uuid>`.
  3. Load manifest from `package.toml`.
  4. Dispatch on `manifest.package.language` (default `"rust"`):
     - **Rust** / mixed: run `cargo build --release --lib` (flags configurable via `compiler.cargo_flags`). Read produced cdylib bytes.
     - **Python** (pure): no build; `compiled_data` = empty `Vec<u8>` (reconciler skips FFI load for pure-Python).
  5. On success: `mark_build_success(id, &cdylib_bytes)`.
  6. On failure: collect stderr tail (max 64KB), `mark_build_failed(id, &error)`.
- [ ] Temp dir is always dropped (success and failure) using `tempfile::TempDir` guard.
- [ ] Tracing spans: one span per build with `package_id`, `package_name`, `language` fields.
- [ ] Integration test: upload a minimal Rust package fixture, run one compiler iteration, assert `build_status = success` and `compiled_data` is non-empty and FFI-loadable.
- [ ] Negative-path integration test: upload a package with intentionally broken Rust, run compiler, assert `build_status = failed` with non-empty `build_error`.

## Implementation Notes

### Cargo invocation

```rust
let output = Command::new("cargo")
    .args(&config.cargo_flags)  // default ["build", "--release", "--lib"]
    .current_dir(&unpacked)
    .env("CARGO_TARGET_DIR", build_target_dir)  // isolate per-build cache
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
let out = output.wait_with_output().await?;
```

### Locating the cdylib

After a successful cargo build, read `target/<profile>/<lib_name>.{so,dylib,dll}`. `lib_name` comes from `manifest.package.name` with cargo's underscore normalization.

### Manifest language field

New field `manifest.package.language` with default `"rust"`. If absent, behave as Rust for back-compat with existing packages.

### Stderr truncation

`build_error` is capped at 64KB. Keep the tail (last 64KB) because Rust compile errors put the useful info at the end.

### Python packages

For v1, Python packages write an empty `compiled_data` blob. The reconciler (T6) inspects the manifest language and skips FFI loading for pure-Python, registering tasks through the existing Python task loader path.

### Temp-dir policy

`config.tmp_root` (default `$home/build-tmp`) is the base. Each build gets a `TempDir` under it; dropped on return. The whole `tmp_root` can be safely wiped between compiler restarts.

## Status Updates

*To be added during implementation*
