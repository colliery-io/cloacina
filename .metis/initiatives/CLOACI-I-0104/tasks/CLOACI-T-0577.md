---
id: t-05-production-deployment-md
level: task
title: "T-05: production-deployment.md — Phase 1 compiler posture"
short_code: "CLOACI-T-0577"
created_at: 2026-05-13T12:43:34.487699+00:00
updated_at: 2026-05-13T17:22:40.555771+00:00
parent: CLOACI-I-0104
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0104
---

# T-05: production-deployment.md — Phase 1 compiler posture

## Parent Initiative

[[CLOACI-I-0104]]

## Objective

Document the Phase 1 compiler deployment posture in `docs/content/platform/how-to-guides/` — cover the threat model (`build.rs` is RCE on the host), the Phase 1 mitigations operators are responsible for configuring, the `cargo vendor` workflow for adding in-house crates, and how to read compiler audit events. Cross-link to I-0105 as the successor that brings the kernel-enforced sandbox. Adds a `[Unreleased]` CHANGELOG entry calling out the breaking defaults change.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] New operator-facing page at `docs/content/platform/how-to-guides/running-the-compiler.md` (sibling to `running-the-daemon.md` / `use-cloacina-compiler-locally.md`). Hugo's `{{< toc-tree >}}` auto-lists it — no manual `_index.md` edit needed. Cross-linked from `use-cloacina-compiler-locally.md`.
- [x] Threat-model section direct, no hedging: "a malicious `build.rs` is code execution on the compiler host." Phase 1 bounds *what* and *how much*; Phase 2 closes the gap.
- [x] Operator-responsibilities enumerated with concrete commands: dedicated UID + restricted DB role, no outbound network beyond vendor dir, `--build-timeout-s` tuning notes, `--build-rlimit-*` table with kernel-resource mappings + tuning notes for mem/procs.
- [x] `cargo vendor` workflow with concrete commands (clone source tree, `cargo vendor --locked` into the operator's vendor path, copy the emitted `.cargo/config.toml`, point `--vendor-dir` at it, restart). Plus the in-house-crate addition path.
- [x] Audit log section reflects Path B (tracing-only per T-0576, not SQL): field reference for both event kinds + three grep recipes (by build_claim_id, rlimit-like kills filtered by exit_signal, per-package version).
- [x] Phase 2 cross-reference: short section pointing at CLOACI-I-0105 (bubblewrap + landlock).
- [x] CHANGELOG `[Unreleased]` `### Security` entry above the existing `### Changed (breaking)`. Covers the offline-by-default flip, rlimits, audit events, and an explicit "operator action required" subsection for existing deployments (`cargo vendor`, unprivileged UID + restricted DB role, mem-rlimit tuning).
- [ ] `angreal docs build` clean — **run externally** per user convention.

## Test Cases

- **TC-1 (doc completeness):** an operator reading only this page can deploy the compiler in the Phase 1 posture without referring to source code. Walk the doc end-to-end against a fresh deployment.
- **TC-2 (cargo vendor recipe works):** follow the recipe verbatim with a real in-house crate; build succeeds.
- **TC-3 (audit query works):** the SELECT example returns rows after a few builds.

## Implementation Notes

### Technical Approach

- Read `docs/content/platform/how-to-guides/use-cloacina-compiler-locally.md` first. Decide whether to extend it or create a sibling page. If extending: add a "Production deployment posture" section. If creating: name it something like `production-compiler-posture.md` and link it from both the platform how-to index and the existing local-compiler doc.
- Pull baseline rlimit recommendations from T-0575's implementation notes; cross-reference the actual defaults shipped.
- For the audit-log SELECT example, copy the event payload shape from T-0576's acceptance criteria.
- Keep the threat-model section short and direct — operators reading it should be left without illusions. Don't soften the language ("malicious build scripts can do X") into hedges ("build scripts may, under some conditions, possibly do X").

### Dependencies

- **T-0573, T-0574, T-0575, T-0576** — all behavior this doc describes must be shipped. Lands last in the initiative.

### Risk Considerations

- **Out-of-date the moment it ships:** documentation drift is real. Make sure flag names, defaults, and audit event names all reference the *single* canonical source (the compiler's `--help` output and the audit event-kind enum). If a future change renames a flag, this doc breaks — acceptable because the docs build will fail if the example commands no longer resolve.
- **Operator pushback on offline-default:** the breaking change from "compiler fetches deps automatically" to "compiler rejects un-vendored deps" will surprise existing deployments. The CHANGELOG entry should be prominent. Add a migration paragraph in the doc itself.

## Status Updates

**2026-05-13** — Docs + CHANGELOG landed locally; ready for external `angreal docs build`.

### What changed

- **New page**: `docs/content/platform/how-to-guides/running-the-compiler.md`. Sections: threat model, five operator responsibilities, `cargo vendor` workflow, flag reference table, audit-event reference (`compiler.build.started` / `_finished` with all fields documented), grep recipes, Phase 2 forward-pointer.
- **Cross-link**: `use-cloacina-compiler-locally.md` now points at the new page from its "When to Run cloacina-compiler Instead" section.
- **CHANGELOG**: new `### Security` section under `[Unreleased]` above the existing `### Changed (breaking)`. Operator-action-required subsection lists `cargo vendor`, restricted DB role, mem-rlimit tuning.

### Design decisions

- **New sibling page, not extension.** Scout found `use-cloacina-compiler-locally.md` is scoped to developer/CI; `production-deployment.md` is server-only (TLS/reverse-proxy). Putting compiler-service threat model into either muddles audience. New page parallels `running-the-daemon.md`'s pattern.
- **Plain language in the threat model.** "A malicious `build.rs` is code execution on the compiler host." No softening. Phase 1 mitigations bound what/how-much, not whether.
- **Single compact flag-reference table.** Operators reading in a hurry shouldn't need to crawl four code snippets to find cpu/mem/files/procs.
- **Audit recipes via grep on tracing output** (Path B). Reflects T-0576's tracing-only emit decision — no SQL recipes for an audit_events table that doesn't exist.

### Verification (2026-05-13)

External run: `angreal docs build` clean.
