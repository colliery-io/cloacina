---
id: audit-t4-drop-zero-caller-dal
level: task
title: "Audit T4: drop zero-caller DAL/Runtime/Executor/Registry methods + dead struct fields"
short_code: "CLOACI-T-0558"
created_at: 2026-05-04T16:10:23.399330+00:00
updated_at: 2026-05-04T20:19:09.024752+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Audit T4: drop zero-caller DAL/Runtime/Executor/Registry methods + dead struct fields

Methods, types, fields, and constants with confirmed zero non-test, non-self callers. Per-item LOC is small but the count is large (~30 items); cumulative grep noise is high.

## Objective

Drop or `#[cfg(test)]`-gate every confirmed orphan. Every item below was verified by the audit via workspace-wide grep across `crates/`, `examples/`, and `tests/`.

## Backlog Item Details

### Type
- [x] Tech Debt — surface-area trim.

### Priority
- [x] P3 — Low. Each item is independent and tiny; bundle into a single sweep PR.

### Technical Debt Impact
- **Current Problems**: 13+ DAL methods, 6+ Runtime methods, 7+ executor methods, 8+ registry methods, 5+ wire-format dead constants. Future audit agents have to re-prove "these are dead" every time. `#[allow(dead_code)]` annotations leave landmines.
- **Benefits of Fixing**: smaller public surface; easier "what's used" answers.
- **Risk Assessment**: Low — every item below has been confirmed via grep. Tests pass.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### DAL methods (zero non-test callers)

- [ ] `RecoveryEventDAL`: `get_by_workflow` (line 143), `get_by_task` (205), `get_by_type` (267), `get_workflow_unavailable_events` (331), `get_recent` (339).
- [ ] `ExecutionEventDAL`: `list_by_task` (210), `list_by_type` (272), `get_recent` (341), `delete_older_than` (400), `count_by_workflow` (462), `count_older_than` (526).
- [ ] `TaskOutboxDAL::delete_older_than` (`dal/unified/task_outbox.rs:308`).
- [ ] `ContextDAL`: `list` (327), `update` (213).
- [ ] `WorkflowExecutionDAL::increment_recovery_attempts` (line 687) — orphaned by T-0502 RecoveryManager removal.

### Runtime methods (zero non-test callers)

- [ ] `Runtime::all_workflows` (`runtime.rs:242`).
- [ ] `Runtime::all_triggers` (`runtime.rs:282`).
- [ ] `Runtime::unregister_reactor` (`runtime.rs:394`) — reconciler unregisters tasks/workflows/triggers/CGs/triggerless but never reactors.
- [ ] `Runtime::has_stream_backend` (`runtime.rs:430`), `Runtime::stream_backend_names` (`:448`), `Runtime::has_task` (`:200`) — only own-tests reference.
- [ ] `Runtime::Debug` impl (`runtime.rs:459-474`) include `triggerless_graphs` and `reactors` (currently omitted — minor).
- [ ] Type aliases `TriggerlessGraphConstructor` (`:56`), `TaskConstructorFn` (`:59`), `WorkflowConstructorFn` (`:62`), `TriggerConstructorFn` (`:65`) — confirm none are referenced from outside `runtime.rs`; downgrade to `pub(crate)` or inline.

### Executor / scheduler / dispatcher

- [ ] `executor/thread_task_executor.rs:427-588` `handle_task_result` + `mark_task_failed` (both `#[allow(dead_code)]`, only call each other).
- [ ] `executor/task_handle.rs:132 with_dal`, `:304 into_slot_token`, `:121 TaskHandle::new` — all zero callers (production uses `with_dal_and_cancel`).
- [ ] `execution_planner/scheduler_loop.rs:62 SchedulerLoop::new` — `#[allow(dead_code)]`, no callers; audit whether the whole `SchedulerLoop` type is orphan and if so delete the file.
- [ ] `dispatcher/work_distributor.rs::PostgresDistributor` — only the file's own factory at L339 references it (and that factory errors out telling callers to "use directly"). Confirm and delete if truly unused.
- [ ] `executor/workflow_executor.rs:510 WorkflowStatus::from_str` — only used by tests in the same file.
- [ ] `computation_graph/scheduler.rs:251 RunningGraph.manual_tx` — written but never read (restart at L928 mints a fresh channel). Delete the field; correct the doc comment at L248.

### Registry / loader

- [ ] `WorkflowRegistryImpl`: `with_strict_validation` (`:88`), `loaded_package_count` (`:104`), `total_registered_tasks` (`:109`).
- [ ] `WorkflowRegistryImpl` fields: `loader` (`:50`), `validator` (`:55`) — both `#[allow(dead_code)]`. The `validator` removal couples to T-0555 (T1).
- [ ] `WorkflowRegistryImpl` convenience methods with zero non-test callers: `unregister_workflow_package_by_id`, `exists_by_id`, `exists_by_name`, `get_workflow_package_by_id`, `get_workflow_package_by_name`, `list_packages` (cloacina-server uses `inspect_package_by_id` + `list_all_packages` instead).
- [ ] `RegistryReconciler::with_graph_scheduler` (`mod.rs:286`) — only tests use it; production goes through `set_graph_scheduler_slot`. Decide: gate behind `#[cfg(test)]` or remove the dual API.
- [ ] `package_loader::temp_dir()` (`:543`), `TaskRegistrar::new()` (`:61`).

### Wire / packaging constants

- [ ] `packaging_bridge.rs:114-117` constants `METHOD_GET_TASK_METADATA`, `METHOD_EXECUTE_TASK`, `METHOD_GET_GRAPH_METADATA`, `METHOD_EXECUTE_GRAPH` — call sites still use bare numeric literals (0/1/2/3/7). Either consume the constants everywhere OR delete them.
- [ ] `cloacina-computation-graph::ComputationGraphRegistration::entry_accumulators` field (`lib.rs:293`) — written but no production reader. Reconciler/packaging/scheduler all consume `accumulator_names` instead. Delete `entry_accumulators` OR migrate callers and delete `accumulator_names`.
- [ ] `cloacina-computation-graph::GraphResult::completed_empty` (`:223`) — zero callers.
- [ ] `cloacina-computation-graph::json_to_wire` (`:93`) — zero callers.

### Server / auth

- [ ] `routes/auth.rs:107 KeyCache::evict()` (`#[allow(dead_code)]`) — revocation uses `key_cache.clear()`.
- [ ] `routes/auth.rs:66 KeyCache::new(capacity, ttl)` — only `default_cache()` calls; could be private.
- [ ] `routes/error.rs:47 ApiError::new` — never called outside the file.
- [ ] `lib.rs:354 RequestId` extension — set on requests but no handler reads it. Either drop the struct or wire a consumer.

### Python

- [ ] `cloacina-python/src/lib.rs:52,58` aliases `task_decorator`, `trigger_decorator` — zero external consumers.
- [ ] `cloacina-python/src/computation_graph.rs`: `stream_accumulator_decorator` (177), `polling_accumulator_decorator` (226), `batch_accumulator_decorator` (263) — wired into wheel + loader but zero Python tests/tutorials/examples consume them. Either land tutorials/tests OR remove until they have callers.

### Triggers

- [ ] `cloacina/src/trigger/mod.rs:121-161` `TriggerConfig` + builder methods `with_allow_concurrent` / `with_enabled` / `new` — zero non-test callers; superseded by macro-driven `Trigger` impls.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

One sweep PR per subsystem (DAL, Runtime, Executor, Registry, Wire, Server, Python, Triggers). Each PR small and focused; reviewer can verify "did anything that grep didn't catch break?" by running the test gates.

### Dependencies

- The `WorkflowRegistryImpl::validator` field couples to T-0555.
- Python decorator removal couples to T-0555 if the bindings module itself goes away.

### Risk Considerations

- **Public-API surface**: some methods are `pub` and may have out-of-tree consumers. Where uncertain, downgrade to `pub(crate)` + 2-week soak before deletion.

## Resolution: Split (2026-05-04)

After review, this ticket bundled three different shapes of work plus several misclassified items. Splitting into focused replacements:

- **T-0563 (replaces T-0558a)** — Drop confirmed orphan code. Only items with verified zero callers AND no public-API or admin-tooling rationale: `RunningGraph.manual_tx`, `WorkflowExecutionDAL::increment_recovery_attempts`, dead executor pair (`handle_task_result`/`mark_task_failed`), `TaskHandle::new`/`with_dal`/`into_slot_token`, `TaskRegistrar::new()`, `package_loader::temp_dir()`, `ApiError::new`, dead-code island in `executor/scheduler_loop.rs`. ~200-300 LOC.

- **T-0564 (replaces T-0558b)** — Reconciler reactor-unload gap + method-index constant adoption. The `Runtime::unregister_reactor` finding isn't dead code — it's a missing reconciler unload arm (reactors leak on package unload). The `packaging_bridge::METHOD_*` constants finding isn't dead code — it's "constants exist, call sites use bare numeric literals" (consume them everywhere).

- **T-0565 (replaces T-0558c)** — DAL/Runtime/Registry visibility downgrades. Items that look like deliberate API surface (DAL `get_recent`, `count_by_workflow`, `delete_older_than`; Runtime introspection `all_workflows`, `all_triggers`; `WorkflowRegistryImpl` convenience methods). Treat as `pub(crate)` downgrade rather than deletion — preserves the option of re-promoting later without code-archaeology.

**Dropped from scope entirely** (misclassified):
- Python `stream_accumulator_decorator` / `polling_accumulator_decorator` / `batch_accumulator_decorator` — accumulators are a core feature; lack of tutorials ≠ dead code. T-0388 already lands a tutorial that exercises them.
- `RegistryReconciler::with_graph_scheduler` dual API — `#[cfg(test)]` gate or leave; not deletion-shaped.
- `Runtime::Debug` impl gap (missing fields in formatter) — chore, not orphan code.

Closing as split. See T-0563 / T-0564 / T-0565.

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*
