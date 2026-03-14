---
id: c4-level-1-system-context-diagram
level: task
title: "C4 Level 1 — System Context Diagram & Documentation"
short_code: "CLOACI-T-0088"
created_at: 2026-03-13T14:29:51.053754+00:00
updated_at: 2026-03-13T15:29:22.121434+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# C4 Level 1 — System Context Diagram & Documentation

**Phase:** 2 — C4 Architecture Documentation
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Create the C4 Level 1 (System Context) documentation page showing Cloacina's position within its broader ecosystem — external actors, systems, and the boundaries of the system itself. This is the highest-level architectural view.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Create `docs/content/explanation/architecture/c4-system-context.md` with proper Hugo frontmatter
- [ ] Mermaid C4 System Context diagram showing Cloacina as the central system
- [ ] All external actors documented: Workflow Developer, Operator, Host Application, Python Application
- [ ] All external systems documented: PostgreSQL, SQLite, External Data Sources, Package Storage
- [ ] Prose description of each actor/system and their relationship to Cloacina
- [ ] Diagram uses standard C4 notation (person, system, database shapes)
- [ ] Page renders correctly in `angreal docs build`
- [ ] Verified against actual codebase — no fictitious actors or systems

## Deliverable

File: `docs/content/explanation/architecture/c4-system-context.md`

## Implementation Notes

### Actors to Document
- **Workflow Developer** — writes task functions (Rust `#[task]` or Python `@task`), defines workflows, configures scheduling
- **Operator** — uses `cloacinactl` CLI for key management, package signing/verification, admin operations
- **Host Application** — Rust binary that embeds `cloacina` as a library dependency, runs the engine
- **Python Application** — Python script that uses `cloaca` bindings to define and execute workflows

### External Systems to Document
- **PostgreSQL** — production database backend (schema-based multi-tenancy)
- **SQLite** — lightweight/embedded database backend (file-based)
- **External Data Sources** — APIs, files, queues, etc. that tasks interact with during execution
- **Package Storage** — filesystem location where `.cloacina` archives are stored for the registry

### Source Files to Verify Against
- `crates/cloacina/src/lib.rs` — core library public API
- `crates/cloacinactl/src/main.rs` — operator CLI commands
- `bindings/cloaca-backend/python/cloaca/__init__.py` — Python public API
- `crates/cloacina/src/dal/` — database backend implementations

## Status Updates

### Completed 2026-03-13

**Created files:**
- `docs/content/explanation/architecture/_index.md` — section index with C4 model overview
- `docs/content/explanation/architecture/c4-system-context.md` — L1 System Context

**Content:**
- Mermaid C4Context diagram with all actors and external systems
- 4 actors documented: Workflow Developer, Operator, Host Application, Python Application
- 4 external systems documented: PostgreSQL, SQLite, External Data Sources, Package Storage
- All actors/systems verified against actual codebase (lib.rs, main.rs, cloaca bindings)
- Forward ref to L2 left as plain text (page not yet created — will be linked in T-0089)

**Build:** `angreal docs build` passes — 93 pages (3 new)
