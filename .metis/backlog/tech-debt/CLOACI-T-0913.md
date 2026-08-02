---
id: python-cron-scaffold-template-for
level: task
title: "Python cron scaffold template for cloacinactl package new"
short_code: "CLOACI-T-0913"
created_at: 2026-08-02T15:02:16.233117+00:00
updated_at: 2026-08-02T15:21:01.387529+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


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

- [x] `cloacinactl package new my-cron --language python --kind cron` produces an uploadable package (mirrors python-cron example: bare `@cloaca.task` + `@cloaca.trigger(on=..., cron=...)`, minimal manifest — resolver infers language/entry from layout, so no explicit PythonRuntime keys needed, same as the example)
- [x] `ScaffoldKind::Cron => unreachable!(...)` python arm replaced by scaffold_python_cron; refusal removed
- [x] Scaffold unit test alongside the existing rust_cron/graph scaffold tests
- [x] Docs touch-up: reference/cli.md kind table (also fixed its stale cannot-declare-cron claim), reference/python-api/trigger.md hint (now describes the scaffold), embed/how-to/running-the-daemon.md note — all updated post-merge

## Status Updates

- 2026-08-02: Filed per user request while executing CLOACI-T-0912 (which only corrected the misleading error text).
- 2026-08-02 (later): Rebased onto main (post #225/#226 squash merges) via rebase --onto, resolving the expected new.rs test conflict in favor of the scaffold-success test. Docs touch-ups applied (three files incl. a stale cli.md claim that predated the adjudication). PR opened; merge on green.
- 2026-08-02: Implemented on feat/t-0913-python-cron-scaffold (stacked on the T-0912 branch since it replaces that branch's corrected refusal). scaffold_python_cron mirrors examples/features/workflows/python-cron: minimal manifest (workflow_name = module), empty __init__.py, tasks.py with one bare @cloaca.task + @cloaca.trigger(on=module, cron="* * * * *") and body-unused comment. Module doc kind list now language-neutral. Old python_cron_is_rejected test replaced with python_cron_scaffold_binds_via_on_with_cron_schedule (asserts on=/cron= binding, no poll_interval, no WorkflowBuilder, layout). REMAINING before PR: rebase onto main after #225/#226 merge, then the docs touch-ups on this branch.
