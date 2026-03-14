---
id: implement-testrunner-with
level: task
title: "Implement TestRunner with topological execution"
short_code: "CLOACI-T-0112"
created_at: 2026-03-14T02:59:44.320196+00:00
updated_at: 2026-03-14T03:19:07.535840+00:00
parent: CLOACI-I-0027
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0027
---

# Implement TestRunner with topological execution

## Parent Initiative

[[CLOACI-I-0027]]

## Objective

Implement the `TestRunner` struct — the core no-DB, in-process task executor. It accepts registered `Task` implementations, builds a dependency graph, topologically sorts them, and executes sequentially with context propagation. On failure, dependents are skipped.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `TestRunner::new()` creates an empty runner
- [ ] `TestRunner::register(task)` accepts `Arc<dyn Task>` with builder-pattern chaining
- [ ] `TestRunner::run(context)` executes tasks in topological order
- [ ] Context is propagated from task to task (output of task N becomes input of task N+1)
- [ ] On task failure: record `TaskOutcome::Failed`, mark all dependents as `TaskOutcome::Skipped`
- [ ] Dependency cycle detection produces a clear error (reuse existing `DependencyGraph`)
- [ ] No retries, no timeouts, no concurrency — deterministic sequential execution
- [ ] Works with tasks created via `#[task]` macro without modification

## Implementation Notes

### Technical Approach

1. `register()` stores tasks in an `IndexMap<String, Arc<dyn Task>>` keyed by `task.id()`
2. `run()`:
   - Build `DependencyGraph` from each task's `dependencies()` return value
   - Call `topological_sort()` to get execution order
   - Iterate in order: call `task.execute(context).await`
   - On `Ok(new_context)` → update context, record `TaskOutcome::Completed`
   - On `Err(e)` → record `TaskOutcome::Failed(e)`, collect all transitive dependents, mark as `Skipped`
   - Return `TestResult` with final context and all outcomes

### Key Source References
- `DependencyGraph`: `crates/cloacina/src/workflow/graph.rs` — has `add_node()`, `add_edge()`, `topological_sort()`, `has_cycles()`
- `Task` trait: `crates/cloacina-workflow/src/task.rs` — `execute()`, `id()`, `dependencies()`
- Existing `MockTask` pattern: `crates/cloacina/tests/integration/scheduler/dependency_resolution.rs`

### Dependencies
- Depends on CLOACI-T-0111 (crate scaffold)

## Status Updates

- Implemented full `TestRunner::run()` with topological execution
- Uses `petgraph` directly (not `cloacina::DependencyGraph`) to avoid heavy dependency
- Dependency matching works by `task_id` field of `TaskNamespace` — matches how `#[task]` macro generates dependencies
- Unregistered dependencies silently skipped (allows testing workflow subsets)
- Context uses `clone_data()` not `clone()` (Context doesn't impl Clone)
- `cargo check -p cloacina-testing` passes clean
