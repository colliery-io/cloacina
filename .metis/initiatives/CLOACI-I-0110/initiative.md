---
id: partial-failure-correctness-sweep
level: initiative
title: "Partial-failure correctness sweep — eliminate log-and-continue at data boundaries"
short_code: "CLOACI-I-0110"
created_at: 2026-05-06T11:05:38.946209+00:00
updated_at: 2026-05-06T11:05:38.946209+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: partial-failure-correctness-sweep
---

# Partial-failure correctness sweep — eliminate log-and-continue at data boundaries Initiative

## Context

The May 2026 correctness review found a recurring pattern across the executor and CG runtime: production code prefers "log and continue" over "stop and surface" at the boundaries where data loss happens silently. None of these are individually catastrophic, but collectively they let runs fail in ways operators cannot easily diagnose.

Each is a small, bounded patch; together they remove a class of partial-failure invisibility.

## Goals & Non-Goals

**Goals:**
- Replace `chrono::Duration::from_std(...).unwrap()` panic risk with a fallback default (COR-06).
- Atomic `complete_task_transaction`: save context first, then `mark_completed` only if it succeeded — or single Diesel transaction for both writes (COR-10).
- Promote silent JSON parse + context-merge failures to typed errors with counter (COR-11).
- Add deterministic tiebreaker on `final_context` selection by completion timestamp (COR-14).
- Add instance/build-claim guard column to `workflow_packages`; filter on it in `mark_build_success`/`mark_build_failed` (COR-16).
- Replace wildcard `_ => WorkflowStatus::Failed` arm in `get_execution_status` with fallible parse (COR-18).
- Restructure heartbeat shutdown so it closes synchronously (COR-08).

**Non-Goals:**
- Reworking the executor model.
- Adding new metrics beyond the merge-failure counter from COR-11.
- Tackling all of REC-17's adjacent items; this is the bounded set.

## Source Findings (May 2026 review)

- **COR-06 (Minor)** — `chrono::Duration::from_std(...).unwrap()` panic risk in `cron_recovery.rs:212`.
- **COR-08 (Minor)** — Heartbeat-handle await missing after shutdown.
- **COR-10 (Major)** — Context-save-after-mark-completed asymmetry.
- **COR-11 (Major)** — Silent JSON parse and context-merge failures.
- **COR-14 (Observation)** — Non-deterministic `final_context` selection on tied completion timestamp.
- **COR-16 (Minor)** — `mark_build_*` not guarded by claim ownership; clobber race.
- **COR-18 (Minor)** — `get_execution_status` wildcard fallback to `Failed`.

## Discovery Questions

- **COR-10 ordering vs single transaction**: which is cheaper / safer / more idiomatic in the existing diesel patterns?
- **COR-11 typed error**: introduce `ExecutorError::ContextLoadFailed` or extend an existing variant? What's the propagation pattern downstream — is the failure recoverable?
- **COR-16 column type**: instance-id (UUID/string) vs build-claim id? Mirror the task-claim pattern.
- **COR-18 round-trip test**: enumerate every status variant; preserve enum→string→enum for `get_execution_status`.

## Initial Sketch

These are all small individual patches. Bundle them into one initiative for a single review/PR rather than spreading across multiple short tasks.

## Acceptance Criteria

- Each finding's "Acceptance" line from `review/02-correctness.md` is satisfied.
- A new test exercises `complete_task_transaction` with an injected post-mark context-save failure; the resulting state is consistent (either both writes or neither).
- A new test round-trips every `WorkflowStatus` variant through `get_execution_status`.
- A new test confirms that two concurrent build claims for the same package row cannot both `mark_build_success`.
- `cloacina_context_merge_failures_total{kind}` counter exists and is exercised in CI.

## References

- `review/02-correctness.md` — COR-06, COR-08, COR-10, COR-11, COR-14, COR-16, COR-18
- `review/10-recommendations.md` — REC-17
