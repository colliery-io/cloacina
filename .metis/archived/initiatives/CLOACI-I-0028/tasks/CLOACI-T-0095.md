---
id: c4-level-3-component-diagrams
level: task
title: "C4 Level 3 — Component Diagrams: Macro Subsystem"
short_code: "CLOACI-T-0095"
created_at: 2026-03-13T14:30:00.738150+00:00
updated_at: 2026-03-13T18:01:36.169773+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# C4 Level 3 — Component Diagrams: Macro Subsystem

**Phase:** 2 — C4 Architecture Documentation
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Create the C4 Level 3 (Component) diagram and documentation for the Macro Subsystem — the `#[task]` and `workflow!` proc macros, compile-time registry, dependency validation, cycle detection, handle detection, and fingerprinting.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Mermaid C4 Component diagram for the Macro Subsystem within the `cloacina-macros` container
- [ ] Components: `#[task]` proc macro, `workflow!` macro, compile-time registry, dependency graph, cycle detector
- [ ] Task macro expansion flow documented: function → struct generation → registry integration → handle detection → fingerprinting
- [ ] Workflow macro expansion flow documented: task references → graph validation → topological sort → version calculation
- [ ] Handle detection mechanism documented (parameter name inspection)
- [ ] All components verified against source in `crates/cloacina-macros/src/`

## Implementation Notes

### Components to Document
- **`#[task]` proc macro** (`crates/cloacina-macros/src/tasks.rs`) — struct generation, handle detection, fingerprinting
- **`workflow!` macro** (`crates/cloacina-macros/src/workflow.rs`) — graph validation, version calculation
- **Compile-time registry** — global singleton for tracking tasks during compilation
- **Dependency graph** — DAG of task dependencies
- **Cycle detector** — Tarjan's algorithm implementation
- **Fingerprinting** — content-based version hashing

## Status Updates

### Completed 2026-03-13

**Created:** `docs/content/explanation/architecture/c4-macro-subsystem.md`

**Components:** #[task] macro, workflow! macro, #[packaged_workflow], CompileTimeTaskRegistry, Cycle Detector, Fingerprinting
- Task macro 3-phase expansion documented (parse → register → generate)
- TaskHandle detection mechanism documented
- Cycle detection: DFS with recursion stack (NOT Tarjan's SCC — corrected in docs)
- Compile-time registry: once_cell + Mutex global singleton
- Fingerprinting: hash of signature + body → 16-char hex
- #[packaged_workflow] FFI generation documented
- Compilation flow sequence diagram
- Complete macro attributes reference tables

**Build:** 100 pages, clean
