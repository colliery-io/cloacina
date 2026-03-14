---
id: c4-level-2-container-diagram
level: task
title: "C4 Level 2 — Container Diagram & Documentation"
short_code: "CLOACI-T-0089"
created_at: 2026-03-13T14:29:51.962600+00:00
updated_at: 2026-03-13T15:33:52.928208+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# C4 Level 2 — Container Diagram & Documentation

**Phase:** 2 — C4 Architecture Documentation
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Create the C4 Level 2 (Container) documentation page showing the major deployable/runnable units in Cloacina and how they communicate. Maps directly to crate boundaries, binaries, and database backends.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Create `docs/content/explanation/architecture/c4-container.md` with proper Hugo frontmatter
- [ ] Mermaid C4 Container diagram showing all major containers and their relationships
- [ ] Each container documented with: name, technology, responsibility, and inter-container communication
- [ ] Dependency arrows: which crate depends on which, runtime vs compile-time dependencies
- [ ] Database communication paths clearly shown (PostgreSQL and SQLite as separate containers)
- [ ] Python bridge (PyO3) boundary clearly marked between `cloaca` and `cloacina`
- [ ] Page renders correctly in `angreal docs build`

## Deliverable

File: `docs/content/explanation/architecture/c4-container.md`

## Implementation Notes

### Containers to Document
- **cloacina** (Rust library crate) — core orchestration engine: executor, scheduler, DAL, registry, security
- **cloacina-workflow** (Rust library crate) — minimal types for authoring packaged workflows (`Context`, `Task`, `TaskError`, `RetryPolicy`)
- **cloacina-macros** (Rust proc-macro crate) — compile-time task/workflow validation and code generation
- **cloacinactl** (Rust binary) — operator CLI: `package`, `key`, `admin` command groups
- **cloaca** (Python package via PyO3) — Python bindings exposing `@task`, `WorkflowBuilder`, `DefaultRunner`, `Context`
- **PostgreSQL** — production backend with schema-based multi-tenancy
- **SQLite** — lightweight/embedded backend

### Communication Paths to Document
- Host app → `cloacina` (library embedding, Rust API)
- Python app → `cloaca` → `cloacina` (PyO3 FFI bridge)
- `cloacina` → PostgreSQL/SQLite (DAL abstraction)
- `cloacinactl` → `cloacina` (library dependency for packaging/admin)
- `cloacina-macros` → `cloacina` (compile-time registry check)
- `cloacina-workflow` ← workflow crates (compile-time dependency only)

### Source Files to Verify Against
- All `Cargo.toml` files for dependency relationships
- `crates/cloacina/src/lib.rs` — public re-exports
- `bindings/cloaca-backend/` — PyO3 bridge structure

### Dependencies
- Should align with T-0088 (System Context) — containers are the zoom-in of the central system box

## Status Updates

### Completed 2026-03-13

**Created:** `docs/content/explanation/architecture/c4-container.md`

**Content:**
- Mermaid C4Container diagram with all 5 containers + 2 DB backends + package storage
- Each container documented: type, location, technology, responsibility
- All dependency arrows verified against Cargo.toml files
- Dependency graph (ASCII) showing crate relationships
- L1→L2 cross-reference link added to system-context.md
- L3 forward references left as plain text (pages not yet created)

**Build:** `angreal docs build` passes — 94 pages
