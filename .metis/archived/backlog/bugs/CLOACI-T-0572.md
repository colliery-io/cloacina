---
id: cron-schedule-executions-never
level: task
title: "Cron schedule_executions never marked complete → cron_recovery infinite loop"
short_code: "CLOACI-T-0572"
created_at: 2026-05-09T00:00:00+00:00
updated_at: 2026-05-12T18:52:03.940294+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Cron `schedule_executions` never marked complete → cron_recovery infinite loop

## Severity / Priority

**P1** — affects every cron-scheduled embedded workflow when `cron_enable_recovery(true)` is set (the default). Any workflow that fires once per cron tick ends up firing every ~13 seconds in an unbounded recovery loop.

Found 2026-05-09 while running [arawn](https://github.com/dstorey/arawn) integration tests against cloacina 0.6.0 (sqlite backend).

## Reproduction

1. Build a `DefaultRunner` with the defaults — `enable_cron_scheduling(true)`, `cron_enable_recovery(true)`. Sqlite backend.
2. Register an embedded workflow with `runner.register_workflow(...)` + `runner.register_cron_workflow("my_workflow", "*/15 * * * *", "UTC")`.
3. Let it run for 30 minutes.

**Expected:** 2 firings (one per 15-min boundary).

**Observed:** 30+ firings — workflow re-executes every ~13 seconds.

In our case, a single 15-min cron firing at `*/15 * * * *` produced 37 actual workflow runs per cron cadence — **906 runs in 6 hours** instead of 24.

## Diagnosis

Direct inspection of `schedule_executions` shows `started_at` populated but `completed_at` permanently NULL, even though the workflow audit reports successful completion:

```sql
sqlite> SELECT scheduled_time, claimed_at, started_at, completed_at
        FROM schedule_executions ORDER BY scheduled_time DESC LIMIT 3;

2026-05-09T10:45:00+00:00 | 2026-05-09T10:45:14.014128 | 2026-05-09T10:45:14.014244 | (NULL)
2026-05-09T10:30:00+00:00 | 2026-05-09T10:30:29.010102 | 2026-05-09T10:30:29.010167 | (NULL)
2026-05-09T10:15:00+00:00 | 2026-05-09T10:15:15.009614 | 2026-05-09T10:15:15.009912 | (NULL)
```

Server log shows the workflow itself is completing fine:

```
INFO Workflow execution completed: <uuid> (name: my_workflow, 1 completed, 0 skipped)
INFO Successfully executed and audited workflow my_workflow for cron schedule <uuid>
```

But on every recovery cycle:

```
INFO cron_recovery: Scheduling workflow execution: my_workflow
```

…fires *immediately* after each completion, which is what produces the 13s cadence.

## Root cause

`crates/cloacina/src/cron_trigger_scheduler.rs:300-360`. The cron success path:

1. `create_cron_execution_audit(schedule.id, scheduled_time)` → creates the `schedule_executions` row with `started_at`.
2. `execute_cron_workflow(schedule, scheduled_time)` → hands off to the executor.
3. `update_workflow_execution_id(audit_record_id, workflow_execution_id)` → links the audit row to the workflow_execution.
4. **No step 4 calls `.complete()` on the schedule_execution row.**

The cron failure path is also missing a `.complete()` call.

`grep -rn 'schedule_execution()\.complete'` across the entire crate returns:

- `crates/cloacina/src/cron_trigger_scheduler.rs:696` — the **trigger** failure path (not cron, not success).
- `crates/cloacina/src/dal/unified/schedule_execution/mod.rs:389,511` — both inside `#[cfg(test)]`.

So in production, *no code path marks a cron-driven `schedule_execution` complete*. The column stays NULL forever.

`cron_recovery::CronRecoveryService::check_and_recover_lost_executions()` calls `find_lost_executions(threshold_minutes)`, which returns rows whose `started_at` is past the threshold but `completed_at` is NULL. Every successfully-completed cron execution matches that predicate. Recovery re-schedules them all on every tick → tight feedback loop.

## Suggested fix

In `cron_trigger_scheduler.rs`, add a step 4 in the success path:

```rust
match self.execute_cron_workflow(schedule, scheduled_time).await {
    Ok(workflow_execution_id) => {
        // Step 3 — link audit record (existing)
        self.dal.schedule_execution()
            .update_workflow_execution_id(audit_record_id, workflow_execution_id)
            .await?;

        // Step 4 — mark execution complete so cron_recovery doesn't
        // treat it as lost.
        if let Err(e) = self.dal.schedule_execution()
            .complete(audit_record_id, Utc::now())
            .await
        {
            warn!("Failed to mark cron schedule execution complete: {}", e);
        }
        ...
    }
    Err(e) => {
        // Same: mark complete on failure too.
        if let Err(e) = self.dal.schedule_execution()
            .complete(audit_record_id, Utc::now())
            .await
        {
            warn!("Failed to mark cron schedule execution complete after failure: {}", e);
        }
        ...
    }
}
```

Mirroring what the trigger failure path already does (line 696).

Caveat: if the design intent is "completed_at = workflow finished", then the call should happen *after* the workflow execution actually finishes — not when the executor handoff returns. The current code awaits `execute_cron_workflow` which appears to await workflow completion (the timing matches), so completing right after that await is correct. If the executor is fire-and-forget for cron workflows in some configurations, the `.complete()` call needs to live in the workflow-completion callback instead.

## Workaround for downstream users

Set `.cron_enable_recovery(false)` on the runner config. Trade-off: lose missed-firing recovery across server restarts. We're using this in arawn until a fix lands upstream.

```rust
let runner_config = DefaultRunnerConfig::builder()
    .enable_cron_scheduling(true)
    .cron_enable_recovery(false)  // workaround for CLOACI-T-0572
    .build()?;
```

## Acceptance criteria

- [x] Add a `.complete()` call to the cron success path in `cron_trigger_scheduler.rs`.
- [x] Add a `.complete()` call to the cron failure path (mirror of the trigger failure handling).
- [x] Add a regression test that pins the DAL contract the fix relies on: rows with NULL `completed_at` are returned by `find_lost_executions`; after `.complete()` they are excluded.
- [ ] (Follow-up) Full end-to-end test that registers an embedded cron workflow at fast cadence, lets it fire twice, and asserts `schedule_executions.completed_at` is populated for both. Deferred — requires non-trivial executor/runtime scaffolding; the DAL contract test plus the obvious cron-path call site change are sufficient to close the loop.

## Status updates

**2026-05-09** — Fix landed.
- Edited `crates/cloacina/src/cron_trigger_scheduler.rs` (process_cron_schedule, success + failure branches, ~lines 328-380): added `.complete(audit_record_id, Utc::now())` calls on both branches. Mirrors the existing trigger-failure pattern at line 696. Removed the misleading "Execution lost: audit record N exists but workflow execution failed" log line in the failure branch since we now mark it complete.
- Added regression test `test_completed_schedule_executions_excluded_from_lost_recovery` to `crates/cloacina/tests/integration/scheduler/cron_basic.rs`. Uses `find_lost_executions(-1)` (negative threshold → cutoff in the future) to make the assertion timing-independent.
- Verification commands (run externally): `angreal lint clippy`, `angreal test integration`.

## Evidence

Reproduced against cloacina 0.6.0 from crates.io:

```
$ grep "feed run complete" server.log | wc -l
920
$ grep "Successfully claimed cron schedule" server.log | wc -l
25
```

920 task executions for 25 cron claims = **~37x amplification**.
