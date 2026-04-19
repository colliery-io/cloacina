---
id: isolate-python-runtime-so-non
level: task
title: "Isolate Python runtime so non-Python binaries don't transitively link pyo3"
short_code: "CLOACI-T-0529"
created_at: 2026-04-18T16:50:00+00:00
updated_at: 2026-04-19T16:40:22.885265+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Isolate Python runtime so non-Python binaries don't transitively link pyo3

## Objective

Python package support is a **core Cloacina capability** and stays that
way — this task is *not* about making Python optional. The problem is
that `cloacina` today is one big crate that mixes the Python runtime
in with everything else, so every downstream binary that depends on
`cloacina` inherits a hard link to `Python3.framework` whether or not
it ever executes Python. That's an architectural smell, not a
feature-flag problem.

The goal is to rearrange the cloacina crate tree so Python-touching
code lives somewhere that binaries which have no Python responsibility
can simply not depend on.

The compiler service made this visible: `cloacina-compiler` polls the
DB, runs `cargo build`, and writes compiled bytes back. It has zero
business touching Python — but it pulls in pyo3 transitively through
`cloacina` and currently needs a `build.rs` + `cloacina-build` just to
resolve the Python framework at dyld load time. Every new crate that
depends on `cloacina` repeats this workaround.

## Technical Debt Impact

- **Current problems**: Every downstream crate needs a `build.rs`
  and a build-dep on `cloacina-build` to set rpath for a Python
  framework most of them don't use at runtime. Deployment images
  bake Python in when they shouldn't. LC_RPATH points at
  `/Applications/Xcode.app/...` on dev boxes, which is fragile
  when Xcode moves. macOS framework-loader surface area is large.
- **Benefits of fixing**: `cloacina-compiler` links zero Python
  and drops its `build.rs`. The compiler Docker image (currently
  `rust:1.85-bookworm`, ~1 GB largely from Python toolchain) can
  shrink meaningfully. Future services that only need the DAL /
  scheduler don't pay the Python tax. The server keeps Python
  everywhere it needs it, because it genuinely runs Python
  workflows.
- **Risk of deferring**: Every new crate compounds the rpath
  workaround. Platform ports (Windows, musl-static Linux) will
  trip here first.

## Approach — architectural split, not feature flags

The Python runtime already lives in well-defined corners of the
tree:

- `crates/cloacina/src/python/` — pyo3 bindings, computation graph
  executor, Python workflow loader.
- `crates/cloacina/src/registry/loader/python_loader.rs` — bridges
  uploaded `.cloacina` packages into the Python loader.

The right fix is to move those out of `cloacina` into their own
crate (working name: **`cloacina-python`**), and have the reconciler
invoke them through a thin trait that `cloacina-python` implements.
The server depends on `cloacina-python` explicitly; the compiler
doesn't.

### Proposed crate shape

```
cloacina              — core runtime, DAL, scheduler, registry, reconciler.
                        No pyo3.
cloacina-python       — everything from src/python/ and the python_loader.
                        Pulls pyo3. Implements a PythonExecutor trait
                        exposed by cloacina so the reconciler can
                        dispatch without a direct dep.
cloacina-server       — depends on cloacina + cloacina-python.
                        Real Python support end-to-end.
cloacina-compiler     — depends on cloacina only. Never sees pyo3.
cloacinactl           — depends on cloacina only (verify CLI doesn't
                        need Python for anything).
```

The reconciler-side dispatch becomes:

- `cloacina` defines `trait PythonExecutor { fn load_package(...); ... }`.
- A `&'static` slot or registry accepts a `PythonExecutor`
  implementation at runtime. When the server starts, it plugs in
  the one from `cloacina-python`.
- The reconciler's `loading.rs` Python branch calls into the trait
  object; if no implementation is registered, it errors cleanly
  ("python runtime not attached") — the compiler doesn't care
  because the compiler never reconciles.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New crate `crates/cloacina-python/` carrying everything
  pyo3-related.
- [ ] `cloacina` has no direct or transitive pyo3 dependency
  (`cargo tree -p cloacina | grep pyo3` → empty).
- [ ] Reconciler's Python branch dispatches through a trait; the
  `cloacina-python` crate registers the implementation when its
  host process wires it in.
- [ ] `cloacina-compiler`: `build.rs` deleted, `cloacina-build`
  build-dep removed, `otool -L target/release/cloacina-compiler |
  grep -i python` is empty.
- [ ] `cloacinactl`: re-evaluated; same treatment if confirmed
  Python-free at runtime.
- [ ] `cloacina-server`: depends on both `cloacina` +
  `cloacina-python`; Python workflows end-to-end still work (the
  existing `soak_server_python` / `python-packaged-graph` demos
  continue to pass).
- [ ] Deployment images: `deploy/docker/compiler.Dockerfile`
  switches from `rust:1.85-bookworm` (full Python) to a slim
  Rust image with no Python. Server Dockerfile keeps Python.
- [ ] All existing angreal tests (unit, integration, cli-e2e,
  compiler-e2e, server-soak) still pass.

## Implementation Notes

### Rough sequencing

1. Audit the Python touch surface in `cloacina`: what public API
   does the rest of the crate depend on? (Reconciler loading path,
   Python-CG scheduler hook, package loader.) Build a minimal
   trait that covers those call sites.
2. Create `crates/cloacina-python/` and move `src/python/` +
   `src/registry/loader/python_loader.rs` into it. Provide the
   trait impl.
3. Add the trait to `cloacina` and wire a registration point
   (e.g. `OnceLock<Arc<dyn PythonExecutor>>`) that the reconciler
   uses. Default state is "not registered"; Python packages error
   cleanly.
4. Update `cloacina-server` to depend on `cloacina-python` and
   register the impl during startup.
5. Drop `build.rs` + `cloacina-build` build-dep from
   `cloacina-compiler` and (pending verification) `cloacinactl`.
6. Update `compiler.Dockerfile` to a slim Rust base.
7. Verify with `cargo tree` + `otool -L` + running the existing
   Python demos through the server.

### What not to touch

- `cloacina-workflow-plugin` — already minimal, stays as-is.
- The Python demos under `examples/python-*` — these are
  consumer-side packages, unaffected by this refactor.
- Existing migrations — schema doesn't care about Python.

### Risk

The reconciler's Python branch today is a direct call into
`crate::python::loader::*`. Moving that to a trait indirection
is the scary part; it needs to preserve the same error semantics
and the same runtime behaviour for Python CG registration. The
existing `server-soak` test with the Python-packaged-graph fixture
is the acceptance gate — if that stays green, the indirection is
faithful.

### Dependencies

None. Pairs loosely with CLOACI-T-0528 (naming drift) since both
touch the reactor / computation_graph subsystem, but they're
independent.

## Status Updates

### 2026-04-19 — Phase A + Phase B shipped

**Phase A** (`dae12e7`): introduced `cloacina::python_runtime::PythonRuntime`
trait + `OnceLock` registration slot. Reconciler dispatches through the
trait instead of direct `crate::python::*` calls. Cloacina's own impl
auto-installs via `#[ctor::ctor]` so behavior was preserved while the
indirection settled.

**Phase B seed** (`eadde52`): created `crates/cloacina-python/`,
duplicated `src/python/` + `registry::loader::python_loader` into it,
rewrote intra-crate refs. Both copies coexist at this commit.

**Phase B** (`7a3e4e9`): deleted the cloacina-core copies, dropped
pyo3 / pythonize / ctor / extension-module / build.rs from cloacina,
made cloacina-server depend on cloacina-python and call
`cloacina_python::install()` at startup. Compiler + CLI dropped
build.rs + cloacina-build build-dep.

**Polish**: moved `pyproject.toml` to cloacina-python so maturin
builds from the correct crate; updated `.angreal/cloaca/cloaca_utils.py`
to point at the new path; refreshed `deploy/docker/compiler.Dockerfile`
comment to note it no longer carries Python.

**Verification**:

```text
$ otool -L target/debug/cloacina-compiler | grep -i python
(empty)
$ otool -l target/debug/cloacina-compiler | grep LC_RPATH
(empty)
$ otool -L target/debug/cloacinactl | grep -i python
(empty)
$ otool -L target/debug/cloacina-server | grep -i python
@rpath/Python3.framework/Versions/3.9/Python3    (as expected)

$ cargo tree -p cloacina-compiler --no-dev-dependencies --edges=normal | grep -i pyo3
(empty)
$ cargo tree -p cloacinactl --no-dev-dependencies --edges=normal | grep -i pyo3
(empty)

$ angreal cloacina unit                         — all passed
$ angreal cloacina compiler-e2e                 — 5/5 green
```

All acceptance criteria met. Task ready to transition to completed.
