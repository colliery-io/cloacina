---
id: cloacinactl-package-pack-cannot
level: task
title: "cloacinactl package pack cannot pack Python packages (requires Cargo.toml + cargo build)"
short_code: "CLOACI-T-0665"
created_at: 2026-06-12T02:22:33.501781+00:00
updated_at: 2026-06-14T16:14:02.397888+00:00
parent: CLOACI-I-0119
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0119
---

# cloacinactl package pack cannot pack Python packages (requires Cargo.toml + cargo build)

## Objective **[REQUIRED]**

`cloacinactl package pack <dir>` only handles Rust packages: it hard-requires a
`Cargo.toml` and unconditionally runs `cargo build`. A Python `.cloacina` source
package has a `package.toml` (`language = "python"`) + a Python module tree and
**no** `Cargo.toml`, so `pack` rejects it (`"<dir> has no Cargo.toml"`). Yet the
server + compiler fully support Python packages — they build them via the
language-appropriate path. The packer should support Python sources so authors
can `cloacinactl package pack` a Python workflow/graph like a Rust one.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (important for user experience)

Re-prioritised P1: there is **no** working tool *or* consistent format for
packaging Python — see "Format inconsistency" below. Authors and the demo
harness hand-roll bzip2 tars, and the one documented procedure produces an
archive the server **rejects**.

### Format inconsistency (the deeper problem)

Three different Python `.cloacina` layouts exist in-tree, and they disagree:

| Source | Manifest | Module location |
| --- | --- | --- |
| **Docs** — `docs/.../how-to-guides/packaging-python-workflows.md` (Step 5) | `manifest.json` (`format_version: "2"`) | **top level** (`data_pipeline/`) |
| **Server reconciler** (what actually loads) | `package.toml` (`[metadata] language="python"`) | under **`workflow/`** |
| **Soak tests** — `.angreal/test/soak/server.py` | `package.toml` | under **`workflow/`** |
| **Example** — `examples/features/computation-graphs/python-packaged-graph` | `package.toml` | **top level** (`market_maker/`) |

Consequences observed (2026-06-12, demo harness against a live server):
- The committed **example** `python-packaged-graph` is **unpackageable as-is** —
  packing it and uploading fails to load: `Registration failed: Failed to
  extract Python CG package: Missing workflow source directory in Python
  package` (reconciler wants `workflow/`, the example is top-level).
- The **documented** how-to (`manifest.json` + top-level module) does **not**
  match what the reconciler accepts (`package.toml` + `workflow/`). Following
  the docs produces a package the server won't load.
- A re-laid-out fixture (`examples/fixtures/demo-py-graph`, module under
  `workflow/`) is what the server accepts.

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: Python authors get a first-class `pack` command instead of
  hand-rolling a bzip2 tar (which is what the soak tests and the UI seed harness
  currently do to produce Python `.cloacina` archives).
- **Effort Estimate**: S — branch on `package.toml` `[metadata].language`.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] **Pick ONE canonical Python `.cloacina` format and make everything agree**
      (docs, the example, the soak builders, the server reconciler). Canonical:
      `package.toml` + module under `workflow/<entry_module_root>/` (T-0677).
- [x] `cloacinactl package pack` reads `package.toml` `[metadata].language`; for
      `language = "python"` it skips the `Cargo.toml`/`cargo build` requirement
      and emits the canonical layout (no hand-rolled tar). For `rust` (default):
      unchanged. (`build` no-ops for Python; `publish` works for both.)
- [x] Validate the Python layout at pack time (entry_module resolves under
      `workflow/`) so a mis-laid-out module fails at pack, not at server upload.
- [x] **Fix the docs** `how-to-guides/packaging-python-workflows.md` (T-0677 +
      Step 5 now leads with `cloacinactl package pack`).
- [x] **Fix or repackage the example**
      `examples/features/computation-graphs/python-packaged-graph` (T-0677 — module
      moved under `workflow/`, verified loading on a live server).

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
`crates/cloacinactl/src/nouns/package/build.rs::run` currently: assert
`Cargo.toml` + `package.toml`, then `cargo build`. Branch on the manifest
language: rust → today's path; python → archive the source tree directly (no
cargo). The bzip2-tar archive format is identical to what the soak harness
builds (`.angreal/test/soak/daemon.py::create_python_test_package`) — reuse that
layout (`<name>-<version>/package.toml`, `<name>-<version>/workflow/<mod>/…`).

### Dependencies
Independent. Unblocks tidier Python fixtures in the UI seed harness
(CLOACI-T-0664), which currently builds Python `.cloacina` archives by hand.

### Risk Considerations
Low — additive language branch. Note: `[[metadata.triggers]]` in `package.toml`
is **rejected** by the server manifest schema (allowed metadata fields:
`workflow_name, graph_name, language, description, author, requires_python,
entry_module, reaction_mode, input_strategy, accumulators`), so triggers are
code-declared, not manifest-declared — don't add a triggers passthrough here.

## Status Updates **[REQUIRED]**

**2026-06-11 — Filed.** Found while extending the UI seed harness to include
Python packages (CLOACI-T-0664): `cloacinactl package pack` errors on a Python
package dir for lack of `Cargo.toml`. Confirmed the server accepts Python
`.cloacina` uploads (a hand-tarred Python computation-graph package uploaded
`201`), so the only gap is the packer's Rust-only assumption.

**2026-06-14 — Implemented (T2 of I-0119); reparented under CLOACI-I-0119.**
Reassessed against current code: the "requires Cargo.toml" symptom was already
resolved by the `package build` / `package pack` noun split — `pack.rs` only
required `package.toml` and delegated to `fidius_core::package::pack_package`,
which archives source (no cargo) and handles the bzip2-tar `<name>-<version>/`
layout for any language. Criteria 1/4/5 (canonical format + docs + example) were
delivered in CLOACI-T-0677. Two real gaps remained, now fixed:
- **`publish` was broken for Python** — it unconditionally ran `build::run`
  (cargo build). `build` now reads `[metadata].language` and no-ops for Python.
- **No pack-time layout validation** — added. New
  `crates/cloacinactl/src/nouns/package/manifest.rs` reads `[metadata]` through
  `CloacinaMetadata` (so `package_type` / `[[metadata.triggers]]` are rejected at
  pack via `deny_unknown_fields`) and, for Python, validates that `workflow/`
  exists and `entry_module` resolves under it. `pack` and `publish` share a
  `pack::pack_to` helper that runs this before archiving. A mis-laid-out package
  now fails at pack, not at upload.
- Unit tests added for language parsing + the layout validator (module file,
  package `__init__`, missing `workflow/`, missing/unresolvable `entry_module`).
- Docs updated: how-to Step 5 + `package-format.md` now lead with
  `cloacinactl package pack` for Python (hand-tar recipe demoted to a fallback).

Files: `nouns/package/{manifest.rs (new), build.rs, pack.rs, publish.rs, mod.rs}`;
`docs/.../packaging-python-workflows.md`; `docs/.../platform/explanation/package-format.md`.

**2026-06-14 — Verified + completed.** `angreal check crate crates/cloacinactl`,
`angreal test e2e cli`, and the end-to-end `package pack` of `demo-py-workflow`
(+ live-server upload) all pass. All acceptance criteria met (1/4/5 via T-0677;
2/3 via this change). Committed `bbc5b1b6` on `i0119-authoring-dx`.