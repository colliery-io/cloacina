---
title: "cloacina::dal::unified::schedule_execution"
description: "Unified Schedule Execution DAL with runtime backend selection"
sidebar:
  order: 101
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Unified Schedule Execution DAL with runtime backend selection

This module provides operations for the unified `schedule_executions` table
that replaces the separate `cron_executions` and `trigger_executions` tables.
Works with both PostgreSQL and SQLite backends, selecting the appropriate
implementation at runtime based on the database connection type.

## Structs

### <span id="struct.ScheduleExecutionStats"></span>`struct ScheduleExecutionStats`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`

Statistics about schedule execution performance

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `total_executions` | `i64` | Total number of executions attempted |
| `successful_executions` | `i64` | Number of executions that successfully handed off to workflow executor |
| `lost_executions` | `i64` | Number of executions that were lost (started but never completed within expected time) |
| `success_rate` | `f64` | Success rate as a percentage |



### <span id="struct.ScheduleExecutionDAL"></span>`struct ScheduleExecutionDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Data access layer for unified schedule execution operations with runtime backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Enums

### <span id="enum.RecoveryClaimResult"></span>`enum RecoveryClaimResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Outcome of a compare-and-set attempt to own the recovery of a lost handoff.

CLOACI-T-0926. Mirrors `RunnerClaimResult` in the task-claiming DAL: the
claim is a single conditional UPDATE and `rows_updated == 1` is the winner.

#### Variants

- **`Claimed`** - This caller now owns the re-fire of the handoff.
- **`NotClaimed`** - Another recovery service owns it (or the row left the lost set).



### <span id="enum.RecoveryHeartbeatResult"></span>`enum RecoveryHeartbeatResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Outcome of a recovery claim heartbeat.

#### Variants

- **`Ok`** - The claim is still ours.
- **`ClaimLost`** - Another recovery service took the claim over (ours went stale).



