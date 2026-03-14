---
id: c4-level-4-code-contracts-core
level: task
title: "C4 Level 4 — Code Contracts: Core Trait Hierarchies & Key Abstractions"
short_code: "CLOACI-T-0096"
created_at: 2026-03-13T14:30:02.060368+00:00
updated_at: 2026-03-13T18:05:59.078151+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# C4 Level 4 — Code Contracts: Core Trait Hierarchies & Key Abstractions

**Phase:** 2 — C4 Architecture Documentation
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Create the C4 Level 4 (Code) documentation page focused on Cloacina's core trait hierarchies, key abstractions, and type contracts. These are the interfaces that define the system's extension points and behavioral contracts.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Create `docs/content/explanation/architecture/c4-code-contracts.md` with proper Hugo frontmatter
- [ ] `Task` trait hierarchy documented with all methods and their contracts
- [ ] `Context<T>` type documented with generic parameter, key methods, and merge semantics
- [ ] `TaskError` enum variants documented
- [ ] `RetryPolicy` / `BackoffStrategy` documented
- [ ] `TaskNamespace` hierarchical addressing documented
- [ ] `DAL` facade composition pattern documented
- [ ] `RegistryStorage` trait documented (filesystem vs DB-backed)
- [ ] `TaskExecutorTrait` and `DispatcherTrait` extension points documented
- [ ] All trait/type signatures verified against actual source code
- [ ] Mermaid class diagrams for key trait hierarchies

## Deliverable

File: `docs/content/explanation/architecture/c4-code-contracts.md`

## Implementation Notes

### Key Abstractions to Document
- **Task trait** (`crates/cloacina/src/task.rs`) — `execute()`, `id()`, `dependencies()`, `retry_policy()`, `trigger_rules()`, `requires_handle()`, `code_fingerprint()`
- **Context<T>** (`crates/cloacina/src/context.rs`) — generic data container
- **TaskError** (`crates/cloacina/src/error.rs`) — error enum
- **RetryPolicy/BackoffStrategy** (`crates/cloacina/src/retry.rs`)
- **TaskNamespace** (`crates/cloacina/src/namespace.rs`) — `tenant.package.workflow.task_id`
- **DAL** (`crates/cloacina/src/dal/mod.rs`) — composed repositories
- **RegistryStorage** (`crates/cloacina/src/registry/storage.rs`)
- **TaskExecutorTrait** (`crates/cloacina/src/executor/mod.rs`)
- **DispatcherTrait** (`crates/cloacina/src/executor/dispatch/mod.rs`)

### Dependencies
- Should reference and be consistent with all L3 component diagrams (T-0090 through T-0095)

## Status Updates

- Created `docs/content/explanation/architecture/c4-code-contracts.md` with full trait hierarchy documentation
- Documented all core traits: Task, Context<T>, TaskError, RetryPolicy, BackoffStrategy, TaskNamespace, DAL, RegistryStorage, Dispatcher, TaskExecutor
- Mermaid class diagram showing trait relationships
- All signatures verified against actual source code via Explore subagent
- Hugo build passes cleanly (100+ pages)
