---
id: cloacinactl-package-pack-cannot
level: task
title: "cloacinactl package pack cannot pack Python packages (requires Cargo.toml + cargo build)"
short_code: "CLOACI-T-0665"
created_at: 2026-06-12T02:22:33.501781+00:00
updated_at: 2026-06-12T02:22:33.501781+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
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

## Acceptance Criteria **[REQUIRED]**

- [ ] **Pick ONE canonical Python `.cloacina` format and make everything agree**
      (docs, the example, the soak builders, the server reconciler). Today the
      reconciler is the source of truth: `package.toml` + module under
      `workflow/<entry_module_root>/`.
- [ ] `cloacinactl package pack` reads `package.toml` `[metadata].language`; for
      `language = "python"` it skips the `Cargo.toml`/`cargo build` requirement
      and emits the canonical layout (no hand-rolled tar). For `rust` (default):
      unchanged.
- [ ] Validate the Python layout at pack time (entry_module resolves under
      `workflow/`) so a mis-laid-out module fails at pack, not at server upload.
- [ ] **Fix the docs** `how-to-guides/packaging-python-workflows.md` (Step 5):
      it currently documents a `manifest.json` + top-level-module layout the
      server rejects.
- [ ] **Fix or repackage the example**
      `examples/features/computation-graphs/python-packaged-graph` so it is
      packageable as-is (module under `workflow/`).

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
