---
id: gate-pyo3-behind-a-python-feature
level: task
title: "Gate pyo3 behind a `python` feature in cloacina core"
short_code: "CLOACI-T-0529"
created_at: 2026-04-18T16:50:00+00:00
updated_at: 2026-04-18T16:50:00+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Gate pyo3 behind a `python` feature in cloacina core

## Objective

`cloacina` has an unconditional dependency on `pyo3`. Any binary that
depends on cloacina — `cloacinactl`, `cloacina-server`,
`cloacina-compiler`, future services — inherits a hard link to
`Python3.framework` even when it does not execute a single line of
Python. This adds a LC_RPATH requirement, a `build.rs` in every
downstream crate, a `@rpath/Python3.framework` reference at runtime,
and a whole class of "it ran locally but not in the container"
failures.

The compiler service made this visible: it has no business touching
Python yet `cloacina-compiler` fails to start without
`Python3.framework` on the dyld rpath. Every other downstream crate
papers over this with a `build.rs` that calls
`cloacina_build::configure()`.

## Technical Debt Impact

- **Current problems**: Every downstream crate needs a `build.rs`
  and a build-dep on `cloacina-build` just to set rpath for a
  Python framework 90% of them never use. Deployment images bake
  Python in when they shouldn't. LC_RPATH points at
  `/Applications/Xcode.app/...` on developer machines, which is
  fragile when Xcode moves.
- **Benefits of fixing**: `cloacina-compiler` drops its `build.rs`,
  the binary links no Python, the Dockerfile gets smaller, the
  surface area for macOS framework-loader bugs shrinks dramatically.
  New services stop silently inheriting Python.
- **Risk of deferring**: Every new crate that depends on `cloacina`
  compounds the workaround. Future platform ports (Windows,
  musl-static Linux, embedded) will trip over this first.

## Acceptance Criteria

- [ ] `cloacina/Cargo.toml`: add a `python` feature. Move `pyo3`,
  `pyo3-async-runtimes` (if any), and any Python-only transitive
  deps behind it.
- [ ] Every module under `crates/cloacina/src/python/` and the
  Python-specific loader path gated via `#[cfg(feature = "python")]`.
  Call sites in `registry/reconciler/loading.rs` dispatch to a stub
  that errors cleanly ("python feature not enabled") when the
  feature is off.
- [ ] `cloacina-compiler`: drop `build.rs` + `cloacina-build`
  build-dep. Ensure the binary links no Python (verify with
  `otool -L` / `ldd`).
- [ ] `cloacina-server`: keep the `python` feature enabled (server
  still runs Python workflows).
- [ ] `cloacinactl`: re-evaluate — it probably doesn't need Python
  either; drop the feature if so.
- [ ] Python unit tests still pass under
  `cargo test -p cloacina --features python`.
- [ ] `angreal cloacina unit` still green with the default feature
  set (which should exclude python).
- [ ] Deployment images shrink: `deploy/docker/server.Dockerfile`
  keeps Python; `deploy/docker/compiler.Dockerfile` drops it.

## Implementation Notes

### Feature boundary

The natural seam is already in the code — everything Python-related
lives in a few well-named modules:

- `crates/cloacina/src/python/` (computation_graph, loader, etc.)
- `crates/cloacina/src/registry/loader/python_loader.rs`
- Any `computation_graph::python_*` submodules.

Gating those behind `#[cfg(feature = "python")]` and making the
reconciler's Python branch conditional is the bulk of the work. No
runtime logic changes; just conditional compilation.

### Downstream crate updates

After the gate lands:

- `cloacina-compiler/Cargo.toml` — drop `cloacina-build` from
  `[build-dependencies]`, delete `build.rs`.
- `cloacinactl/Cargo.toml` — drop if confirmed unused. Verify
  empirically; the CLI does some workflow-package introspection that
  may pull in Python bits.
- `cloacina-server/Cargo.toml` — explicitly enable `python` (was
  implicit before; make it explicit now).

### Don't forget

- Tests that import from `cloacina::python::*` or exercise the
  Python loader must be gated too (`#[cfg(feature = "python")]`).
- CI matrix should cover both `--features python` and default
  feature set — catches feature-flag rot early.

### Dependencies

None on other tasks, but pairs well with **CLOACI-T-0528** (reactor /
computation_graph naming drift) — both are core cleanups on the same
module. Can be done in either order.

## Status Updates

*To be added during implementation*
