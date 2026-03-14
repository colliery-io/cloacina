---
id: semantic-accuracy-audit-macro
level: task
title: "Semantic Accuracy Audit — Macro System, Versioning & Multi-Tenancy Docs"
short_code: "CLOACI-T-0106"
created_at: 2026-03-13T14:30:19.057457+00:00
updated_at: 2026-03-14T02:18:34.146437+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# Semantic Accuracy Audit — Macro System, Versioning & Multi-Tenancy Docs

**Phase:** 5 — Semantic Accuracy Audit (Pass 4)
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Read each macro system, workflow versioning, and multi-tenancy explanation doc alongside corresponding source code. Verify every claim is accurate.

## Scope

- `docs/content/explanation/macro-system.md`
- `docs/content/explanation/workflow-versioning.md`
- `docs/content/explanation/multi-tenancy.md` (if exists)
- Any other explanation docs covering macros, versioning, or tenancy

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Macro system claims verified: task struct generation, registry integration, ctor usage, handle detection
- [ ] "Required Dependencies" section verified — are `async-trait`, `ctor`, `serde_json`, `chrono` still required?
- [ ] Workflow versioning algorithm description verified against fingerprinting source
- [ ] Multi-tenancy claims verified against PostgreSQL schema isolation and SQLite file isolation
- [ ] Registry system descriptions verified against compile-time and runtime registry implementations
- [ ] All inaccuracies corrected in-place

## Implementation Notes

### Key Verification Points
- `macro-system.md` claims "uses `once_cell` and `Mutex` for compile-time registry" — verify
- `macro-system.md` claims "uses Tarjan's algorithm" — find and verify the algorithm
- Dependencies section claims these are "required for macro expansion" — verify they can't be made transitive
- Workflow versioning claims "content-based version" — verify hashing mechanism
- Handle detection claims "based purely on parameter name" — already verified in T-0078, but re-verify

## Status Updates

### Completed
Audited 2 docs: macro-system.md and multi-tenancy.md (no workflow-versioning.md exists).

**macro-system.md** (4 fixes):
- Fixed `ctor` attribution: `#[task]` macro registers during macro expansion, not via `ctor`. `ctor` is only used by `workflow!` macro for runtime workflow registration.
- Fixed cycle detection algorithm: was "Tarjan's algorithm", actually DFS with recursion stack
- Clarified Required Dependencies: `ctor` only needed for `workflow!` macro, not `#[task]`
- Removed specific version pin `ctor = "0.2"` (managed via workspace)

**multi-tenancy.md** (6 fixes):
- Removed fabricated `SchemaCustomizer`/`CustomizeConnection` code example (codebase uses deadpool-diesel, not r2d2)
- Added `.await` to all `create_tenant()` calls (5 locations) — it's an async fn
- Added `.await` to `remove_tenant()` call — it's an async fn
- Fixed password charset from "94 chars with symbols" to "62 alphanumeric chars" (source uses only a-zA-Z0-9 to avoid URL issues)
- Fixed entropy from ~202 bits to ~190 bits
- Fixed builder pattern: `max_concurrent_tasks()` and `task_timeout()` are on `DefaultRunnerConfig::builder()`, not `DefaultRunner::builder()`

All fixes verified with docs build.
