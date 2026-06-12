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
- [x] P2 - Medium (nice to have)

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: Python authors get a first-class `pack` command instead of
  hand-rolling a bzip2 tar (which is what the soak tests and the UI seed harness
  currently do to produce Python `.cloacina` archives).
- **Effort Estimate**: S — branch on `package.toml` `[metadata].language`.

## Acceptance Criteria **[REQUIRED]**

- [ ] `cloacinactl package pack` reads `package.toml` `[metadata].language`.
- [ ] For `language = "python"`: skip the `Cargo.toml`/`cargo build` requirement;
      archive `package.toml` + the Python source tree (the entry module + its
      package dir) into the `.cloacina` bzip2 tar. Server-side build stays the
      compiler's job (it already handles `language = "python"`).
- [ ] For `language = "rust"` (default): unchanged behavior.
- [ ] Validates the Python layout minimally (entry_module resolves to a file in
      the archive) so a mis-packaged module fails at pack time, not upload time.

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
