---
id: recovery-sweeper-service-periodic
level: task
title: "Recovery sweeper service — periodic orphan detection with startup grace period"
short_code: "CLOACI-T-0234"
created_at: 2026-03-23T23:34:18.126679+00:00
updated_at: 2026-03-24T00:16:31.988677+00:00
parent: CLOACI-I-0043
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0043
---

# Recovery sweeper service — periodic orphan detection with startup grace period

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0043]]

## Objective

Build the Recovery Sweeper background service that periodically scans for orphaned tasks (stale heartbeats) and resets them for re-execution. Also wire the executor to call `claim_task()` before executing and heartbeat during execution.

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `RecoverySweepService` runs as background service with shutdown signal
- [ ] Sweeper runs every 30s (configurable), startup mode uses `sweeper_start_time - orphan_threshold` as cutoff
- [ ] Orphaned tasks reset to Ready via `mark_ready()` (which auto-inserts task_outbox)
- [ ] Tasks exceeding `max_recovery_attempts` marked Failed with "Abandoned" reason
- [ ] `ThreadTaskExecutor` calls `claim_task(task_id, instance_id)` before executing
- [ ] `ThreadTaskExecutor` spawns heartbeat tokio task (every 10s) during execution
- [ ] Heartbeat aborted on task completion
- [ ] Config options added to `DefaultRunnerConfig` (sweep_interval, orphan_threshold, max_attempts, startup_grace)
- [ ] Wired into `services.rs` using standard spawn pattern with broadcast shutdown
- [ ] `recovery_sweeper_handle` added to `RuntimeHandles`
- [ ] All existing tests pass
- [ ] `test_dal()` integration test: claim → heartbeat → find_stale_heartbeats roundtrip

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

### 2026-03-23 — Exploration complete

**Service pattern** (from `services.rs` + `cron_recovery.rs`):
- watch::channel for shutdown, broadcast for coordinated stop
- `tokio::select!` loop: interval.tick() OR shutdown.changed()
- Service gets `Arc<DAL>`, config struct, shutdown receiver
- Spawned with `tokio::spawn` + `instrument(span)`
- Handle stored in `RuntimeHandles`

**Key finding — sweeper is simpler than cron_recovery:**
- Cron recovery re-executes workflows via `executor.execute()` directly
- Sweeper just calls `mark_ready(task_id)` which auto-inserts task_outbox
- The normal dispatcher flow picks up the re-queued task — no direct executor call needed

**Executor integration** (from `thread_task_executor.rs`):
- Already has `instance_id: UniversalUuid` and `dal: DAL`
- Receives `TaskReadyEvent` via dispatcher — has `task_execution_id`
- Need to add: `dal.task_execution().claim_task(event.task_execution_id, instance_id)` before execution
- Need to add: spawn heartbeat tokio task alongside execution, abort on completion
- If claim fails (0 rows = someone else claimed): return success without executing

**Task outbox re-insertion:**
- `mark_ready(task_id)` handles outbox + event atomically
- Sweeper just calls this — dispatcher picks it up automatically

**Config additions needed:**
- `enable_recovery_sweep: bool` (default true)
- `recovery_sweep_interval: Duration` (default 30s)
- `recovery_orphan_threshold: Duration` (default 60s)
- `recovery_startup_grace: Duration` (default 120s)
- `recovery_max_attempts: usize` (default 3)
- `task_heartbeat_interval: Duration` (default 10s)

**Files to create/modify:**
1. NEW: `crates/cloacina/src/recovery_sweep.rs`
2. MODIFY: `runner/default_runner/config.rs` — add config fields
3. MODIFY: `runner/default_runner/services.rs` — spawn sweeper
4. MODIFY: `runner/default_runner/mod.rs` — add handle to RuntimeHandles + shutdown
5. MODIFY: `executor/thread_task_executor.rs` — claim + heartbeat
6. MODIFY: `lib.rs` — export

### Implementation complete

**New file:** `crates/cloacina/src/recovery_sweep.rs`
- `RecoverySweepService` with startup/normal mode orphan detection
- Startup grace: first 120s only recovers tasks stale before this instance started
- Normal mode: real-time detection of stale heartbeats
- Respects max_recovery_attempts (3), marks Failed with "ABANDONED:" on exceed

**Wired into runner:**
- `services.rs`: `start_recovery_sweeper()` following standard spawn pattern
- `mod.rs`: `recovery_sweeper_handle` in RuntimeHandles + shutdown sequence
- `config.rs`: handle field in second constructor site

**Executor integration** (`thread_task_executor.rs`):
- `claim_task()` called before execution — atomic WHERE status='Ready'
- If claim fails (another executor claimed): returns success without executing
- Heartbeat tokio task spawned (10s interval) alongside execution
- Heartbeat aborted when execution completes (success or failure)
- Backward compatible: claim failure logged as warning, doesn't block tests

495 tests pass.
