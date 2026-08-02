---
id: python-cron-scaffold-template-for
level: task
title: "Python cron scaffold template for cloacinactl package new"
short_code: "CLOACI-T-0913"
created_at: 2026-08-02T15:02:16.233117+00:00
updated_at: 2026-08-02T15:02:16.233117+00:00
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

# Python cron scaffold template for cloacinactl package new

## Objective

Add a Python template for `cloacinactl package new --kind cron --language python`, removing the current refusal. The capability already exists — packaged Python cron triggers work end to end via `@cloaca.trigger(name=..., on=<workflow>, cron="<expr>")` (crates/cloacina-python/src/trigger.rs; working CI-wired example: examples/features/workflows/python-cron) — only the scaffold template is missing (crates/cloacinactl/src/nouns/package/new.rs rejects the combination; error text corrected to say template-missing by CLOACI-T-0912).

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

### Technical Debt Impact
- **Current Problems**: Python users scaffolding a cron package are bounced to hand-assembly; asymmetry with the Rust path for a capability that is fully supported.
- **Benefits of Fixing**: `package new` covers the full kind x language matrix; one less doc caveat (reference/python-api/trigger.md hint + running-the-daemon.md note can be simplified).
- **Risk Assessment**: Low; scaffold-only change.

## Acceptance Criteria

- [ ] `cloacinactl package new my-cron --language python --kind cron` produces an uploadable package (mirror examples/features/workflows/python-cron: bare `@cloaca.task` tasks + `@cloaca.trigger(on=..., cron=...)`, package.toml with PythonRuntime fields)
- [ ] `ScaffoldKind::Cron => unreachable!(...)` python arm in new.rs replaced by the real template writer; refusal + corrected error text removed
- [ ] Scaffold unit test alongside the existing rust_cron/graph scaffold tests
- [ ] Docs touch-up: reference/cli.md package-new kind table + reference/python-api/trigger.md hint updated to drop the template-missing caveat

## Status Updates

- 2026-08-02: Filed per user request while executing CLOACI-T-0912 (which only corrected the misleading error text).
