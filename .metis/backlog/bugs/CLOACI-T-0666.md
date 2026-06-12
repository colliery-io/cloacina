---
id: compiler-reads-package-language
level: task
title: "Compiler reads [package].language not [metadata].language — all packaged Python fails to build; no gating test catches it"
short_code: "CLOACI-T-0666"
created_at: 2026-06-12T11:52:37.862326+00:00
updated_at: 2026-06-12T11:52:37.862326+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Compiler reads [package].language not [metadata].language — all packaged Python fails to build; no gating test catches it

## Objective **[REQUIRED]**

`cloacina-compiler` decides whether to skip cargo (for `language = "python"`) by
reading the WRONG TOML table. `manifest_language()` reads `[package].language`,
but the canonical location — used by the **upload schema**, **every example**,
and the **soak test fixtures** — is `[metadata].language`. So for any real
Python package `manifest_language()` returns the default `"rust"`, the compiler
runs `cargo build`, and it fails with `could not find Cargo.toml`. **All
packaged Python through the server+compiler path is broken**, and no gating test
catches it.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

P1: it makes a whole advertised capability (packaged Python workflows/graphs via
the server) non-functional, and the test gap means regressions here are
invisible.

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: Anyone uploading a `language = "python"` package to
  `cloacina-server` (Python workflows AND Python computation graphs).
- **Reproduction**:
  1. Upload a Python `.cloacina` whose `package.toml` has `[metadata] language =
     "python"` (the canonical/only-accepted form — the upload schema lists
     `language` under `[metadata]`).
  2. Compiler claims the build and runs `cargo build` →
     `outcome=failed … error: could not find Cargo.toml in …/source/<pkg>`.
  3. build_status never reaches `success` → reconciler never loads it → workflow
     never registers → cannot execute.
- **Confirmed workaround**: adding `language = "python"` under **`[package]`**
  (in addition to `[metadata]`) makes the compiler skip cargo and the build
  succeeds (`outcome=success`, empty artifact, ~2ms). This pinpoints the table
  mismatch.

### Root cause
`crates/cloacina-compiler/src/build.rs`:
```rust
fn manifest_language(manifest: &toml::Value) -> String {
    manifest
        .get("package")            // <- WRONG: language lives under [metadata]
        .and_then(|p| p.get("language"))
        .and_then(|v| v.as_str())
        .unwrap_or("rust")
        .to_ascii_lowercase()
}
```
`load_manifest()` reads `package.toml`. The upload validator's allowed
`[metadata]` fields are exactly `workflow_name, graph_name, language,
description, author, requires_python, entry_module, reaction_mode,
input_strategy, accumulators` — i.e. `language` is a `[metadata]` field. The
examples confirm it:
`examples/features/computation-graphs/python-packaged-graph/package.toml` and
`.angreal/test/soak/server.py::create_python_source_package` both put
`language = "python"` under `[metadata]`.

## Why no test caught it

- **`tests/python/`** scenarios exercise the **in-process embedded `cloaca`
  runtime** (DefaultRunner), not the upload->compiler->reconciler server path.
- **`.angreal/test/soak/server.py` Step 8b** is the only test that uploads
  Python to the server, and it **does not assert the build/load succeeds**: it
  checks `201` on upload, then waits for a `"Python package loaded"` log line and
  on timeout merely prints `WARNING: Python package may not have loaded` and
  continues (no assertion failure). The execute step is then silently skipped.
- **Soak is not in any CI workflow** (`grep soak .github/workflows` -> none).

## Acceptance Criteria **[REQUIRED]**

- [ ] `manifest_language()` reads `[metadata].language` (canonical), ideally with
      a `[package].language` fallback for resilience; `language = "python"` under
      `[metadata]` makes the compiler skip cargo.
- [ ] A **gating** test asserts the full server path: upload a `[metadata]
      language="python"` package -> poll `build_status` to `success` -> poll
      `/workflows/{name}` registers -> execute returns 202 -> execution reaches
      `Complete`. Same for a Python computation graph asserting
      `/v1/health/graphs` + `/v1/health/accumulators` populate.
- [ ] Harden `soak/server.py` Step 8b to **assert** (not warn) Python load +
      execution, and/or add the above to a CI lane.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
One-line fix in `manifest_language` (read `[metadata].language`). Then add the
asserting test. Related: CLOACI-T-0665 (`cloacinactl package pack` can't pack
Python) and CLOACI-T-0664 (UI seed harness wants Python in the demo) — both
unblocked by this fix.

## Status Updates **[REQUIRED]**

**2026-06-12 — Filed.** Found while extending the UI seed/demo harness
(CLOACI-T-0664) to upload a Python workflow + a Python computation graph to a
live `cloacina-server` + `cloacina-compiler`. Rust packages (incl. `mixed-rust`
= reactor + accumulator + reactor-bound CG + trigger + workflow) built fine; both
Python packages failed with `could not find Cargo.toml`. Traced to
`manifest_language` reading `[package].language`; verified the `[package]`-table
workaround flips the build to success.
