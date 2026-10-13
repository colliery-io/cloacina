---
title: "cloacina::dal::unified::schedule_execution"
description: "Unified Schedule Execution DAL with runtime backend selection"
editUrl: false
sidebar:
  order: 85
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified Schedule Execution DAL with runtime backend selection

This module provides operations for the unified `schedule_executions` table
that replaces the separate `cron_executions` and `trigger_executions` tables.
Works with both PostgreSQL and SQLite backends, selecting the appropriate
implementation at runtime based on the database connection type.

## Structs

### <span id="struct.ScheduleExecutionStats"></span><code style="font-size: 0.85em">struct ScheduleExecutionStats</code>

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

### <span id="struct.ScheduleExecutionDAL"></span><code style="font-size: 0.85em">struct ScheduleExecutionDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for unified schedule execution operations with runtime backend selection.

#### Methods

##### <span id="ScheduleExecutionDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

Creates a new ScheduleExecutionDAL instance.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="ScheduleExecutionDAL.create"></span><code>create</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create(
    &self,
    new_execution: NewScheduleExecution,
) -> Result<ScheduleExecution, ValidationError>
```

Creates a new schedule execution record in the database.

<details>
<summary>Source</summary>

```rust
    pub async fn create(
        &self,
        new_execution: NewScheduleExecution,
    ) -> Result<ScheduleExecution, ValidationError> {
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedScheduleExecution {
            id,
            schedule_id: new_execution.schedule_id,
            workflow_execution_id: new_execution.workflow_execution_id,
            scheduled_time: new_execution.scheduled_time,
            claimed_at: new_execution.claimed_at,
            context_hash: new_execution.context_hash,
            started_at: now,
            created_at: now,
            updated_at: now,
        };

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(schedule_executions::table)
                .values(&new_unified)
                .execute(conn)
        })?;

        let result: UnifiedScheduleExecution = crate::interact_on_backend!(self.dal, |conn| {
            schedule_executions::table.find(id).first(conn)
        })?;

        Ok(result.into())
    }
```

</details>

##### <span id="ScheduleExecutionDAL.get_by_id"></span><code>get_​by_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_by_id(
    &self,
    id: UniversalUuid,
) -> Result<ScheduleExecution, ValidationError>
```

Retrieves a schedule execution by its ID.

<details>
<summary>Source</summary>

```rust
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<ScheduleExecution, ValidationError> {
        let result: UnifiedScheduleExecution = crate::interact_on_backend!(self.dal, |conn| {
            schedule_executions::table.find(id).first(conn)
        })?;

        Ok(result.into())
    }
```

</details>

##### <span id="ScheduleExecutionDAL.list_by_schedule"></span><code>list_​by_​schedule</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_by_schedule(
    &self,
    schedule_id: UniversalUuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<ScheduleExecution>, ValidationError>
```

Lists schedule executions for a given schedule.

<details>
<summary>Source</summary>

```rust
    pub async fn list_by_schedule(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ScheduleExecution>, ValidationError> {
        let results: Vec<UnifiedScheduleExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(schedule_id))
                    .order(schedule_executions::created_at.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }
```

</details>

##### <span id="ScheduleExecutionDAL.complete"></span><code>complete</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn complete(
    &self,
    id: UniversalUuid,
    completed_at: DateTime<Utc>,
) -> Result<(), ValidationError>
```

Marks a schedule execution as completed.

<details>
<summary>Source</summary>

```rust
    pub async fn complete(
        &self,
        id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let completed_ts = UniversalTimestamp::from(completed_at);
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedule_executions::table.find(id))
                .set((
                    schedule_executions::completed_at.eq(Some(completed_ts)),
                    schedule_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleExecutionDAL.has_active_execution"></span><code>has_​active_​execution</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn has_active_execution(
    &self,
    schedule_id: UniversalUuid,
    context_hash: &str,
) -> Result<bool, ValidationError>
```

Checks if there is an active (uncompleted) execution for a schedule with the given context hash.

<details>
<summary>Source</summary>

```rust
    pub async fn has_active_execution(
        &self,
        schedule_id: UniversalUuid,
        context_hash: &str,
    ) -> Result<bool, ValidationError> {
        let context_hash_owned = context_hash.to_string();

        let count: i64 = crate::interact_on_backend!(self.dal, |conn| {
            schedule_executions::table
                .filter(schedule_executions::schedule_id.eq(schedule_id))
                .filter(schedule_executions::context_hash.eq(context_hash_owned))
                .filter(schedule_executions::completed_at.is_null())
                .count()
                .get_result(conn)
        })?;

        Ok(count > 0)
    }
```

</details>

##### <span id="ScheduleExecutionDAL.update_workflow_execution_id"></span><code>update_​workflow_​execution_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn update_workflow_execution_id(
    &self,
    id: UniversalUuid,
    workflow_execution_id: UniversalUuid,
) -> Result<(), ValidationError>
```

Updates the workflow execution ID for a schedule execution.

<details>
<summary>Source</summary>

```rust
    pub async fn update_workflow_execution_id(
        &self,
        id: UniversalUuid,
        workflow_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedule_executions::table.find(id))
                .set((
                    schedule_executions::workflow_execution_id.eq(Some(workflow_execution_id)),
                    schedule_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleExecutionDAL.find_lost_executions"></span><code>find_​lost_​executions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_lost_executions(
    &self,
    older_than_minutes: i32,
) -> Result<Vec<ScheduleExecution>, ValidationError>
```

Finds lost executions (started but not completed) older than the specified minutes.

<details>
<summary>Source</summary>

```rust
    pub async fn find_lost_executions(
        &self,
        older_than_minutes: i32,
    ) -> Result<Vec<ScheduleExecution>, ValidationError> {
        let cutoff = Utc::now() - Duration::minutes(older_than_minutes as i64);
        let cutoff_ts = UniversalTimestamp::from(cutoff);

        let results: Vec<UnifiedScheduleExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                schedule_executions::table
                    .filter(schedule_executions::completed_at.is_null())
                    .filter(schedule_executions::started_at.lt(cutoff_ts))
                    .order(schedule_executions::started_at.asc())
                    .load(conn)
            })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }
```

</details>

##### <span id="ScheduleExecutionDAL.claim_for_recovery"></span><code>claim_​for_​recovery</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn claim_for_recovery(
    &self,
    id: UniversalUuid,
    owner_id: UniversalUuid,
    stale_after: std::time::Duration,
) -> Result<RecoveryClaimResult, ValidationError>
```

Atomically claim a lost handoff for recovery by `owner_id`.

`find_lost_executions` is a plain SELECT, so every replica's recovery
service sees the same lost rows. This compare-and-set is what makes
exactly one of them re-fire, in the same shape as
`TaskExecutionDAL::claim_for_runner`: one conditional UPDATE,
`rows_updated == 1` wins.

The claim succeeds when the row is still open AND either unclaimed or
held by an owner whose heartbeat went stale. Two guards matter:

* `completed_at IS NULL` is part of the CAS, not a pre-check. A loser
  that selected the row before the winner finished still holds a stale
  snapshot; without this predicate it would claim the (now completed)
  row after the winner released it and re-fire — the duplicate this is
  here to prevent.
* the staleness arm uses the heartbeat, not the claim time. A re-fire
  blocks for the workflow's full (unbounded) duration, so a fixed
  claim-age window would expire mid-execution and hand the row to a
  second service. The owner beats while it works, so a stale beat is a
  true death signal and a crashed service never locks a handoff
  permanently.

<details>
<summary>Source</summary>

```rust
    pub async fn claim_for_recovery(
        &self,
        id: UniversalUuid,
        owner_id: UniversalUuid,
        stale_after: std::time::Duration,
    ) -> Result<RecoveryClaimResult, ValidationError> {
        let now = UniversalTimestamp::now();
        let cutoff = UniversalTimestamp(
            Utc::now() - Duration::from_std(stale_after).unwrap_or_else(|_| Duration::seconds(120)),
        );

        let rows_updated: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                schedule_executions::table
                    .find(id)
                    .filter(schedule_executions::completed_at.is_null())
                    .filter(
                        schedule_executions::recovery_claimed_by
                            .is_null()
                            .or(schedule_executions::recovery_heartbeat_at.lt(Some(cutoff))),
                    ),
            )
            .set((
                schedule_executions::recovery_claimed_by.eq(Some(owner_id)),
                schedule_executions::recovery_heartbeat_at.eq(Some(now)),
                schedule_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;

        Ok(if rows_updated > 0 {
            RecoveryClaimResult::Claimed
        } else {
            RecoveryClaimResult::NotClaimed
        })
    }
```

</details>

##### <span id="ScheduleExecutionDAL.recovery_heartbeat"></span><code>recovery_​heartbeat</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn recovery_heartbeat(
    &self,
    id: UniversalUuid,
    owner_id: UniversalUuid,
) -> Result<RecoveryHeartbeatResult, ValidationError>
```

Refresh the recovery claim heartbeat, only while we still own it.

<details>
<summary>Source</summary>

```rust
    pub async fn recovery_heartbeat(
        &self,
        id: UniversalUuid,
        owner_id: UniversalUuid,
    ) -> Result<RecoveryHeartbeatResult, ValidationError> {
        let now = UniversalTimestamp::now();
        let rows_updated: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                schedule_executions::table
                    .find(id)
                    .filter(schedule_executions::recovery_claimed_by.eq(Some(owner_id))),
            )
            .set((
                schedule_executions::recovery_heartbeat_at.eq(Some(now)),
                schedule_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;

        Ok(if rows_updated > 0 {
            RecoveryHeartbeatResult::Ok
        } else {
            RecoveryHeartbeatResult::ClaimLost
        })
    }
```

</details>

##### <span id="ScheduleExecutionDAL.release_recovery_claim"></span><code>release_​recovery_​claim</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn release_recovery_claim(
    &self,
    id: UniversalUuid,
    owner_id: UniversalUuid,
) -> Result<(), ValidationError>
```

Release a recovery claim we hold. No-op if someone else owns it now.

<details>
<summary>Source</summary>

```rust
    pub async fn release_recovery_claim(
        &self,
        id: UniversalUuid,
        owner_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                schedule_executions::table
                    .find(id)
                    .filter(schedule_executions::recovery_claimed_by.eq(Some(owner_id))),
            )
            .set((
                schedule_executions::recovery_claimed_by.eq(None::<UniversalUuid>),
                schedule_executions::recovery_heartbeat_at.eq(None::<UniversalTimestamp>),
                schedule_executions::updated_at.eq(now),
            ))
            .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleExecutionDAL.increment_recovery_attempts"></span><code>increment_​recovery_​attempts</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn increment_recovery_attempts(
    &self,
    id: UniversalUuid,
) -> Result<i32, ValidationError>
```

Increment the durable recovery attempt count and return the new value.

The count lives on the audit row (not in the recovery service's memory)
so a schedule that reliably fails recovery cannot earn a fresh budget by
bouncing the process. Callers hold the recovery claim, so the read-back
is not racing another attempt; the increment itself is still done
in-SQL and inside a transaction so the value can never regress.

<details>
<summary>Source</summary>

```rust
    pub async fn increment_recovery_attempts(
        &self,
        id: UniversalUuid,
    ) -> Result<i32, ValidationError> {
        use diesel::connection::Connection;

        let attempts: i32 = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                let now = UniversalTimestamp::now();
                diesel::update(schedule_executions::table.find(id))
                    .set((
                        schedule_executions::recovery_attempts
                            .eq(schedule_executions::recovery_attempts + 1),
                        schedule_executions::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                schedule_executions::table
                    .find(id)
                    .select(schedule_executions::recovery_attempts)
                    .first(conn)
            })
        })?;

        Ok(attempts)
    }
```

</details>

##### <span id="ScheduleExecutionDAL.reset_recovery_attempts"></span><code>reset_​recovery_​attempts</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn reset_recovery_attempts(
    &self,
    id: UniversalUuid,
) -> Result<(), ValidationError>
```

Reset the durable recovery attempt count (used after a successful re-fire, the durable equivalent of clearing the old in-memory entry).

<details>
<summary>Source</summary>

```rust
    pub async fn reset_recovery_attempts(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedule_executions::table.find(id))
                .set((
                    schedule_executions::recovery_attempts.eq(0),
                    schedule_executions::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleExecutionDAL.get_latest_by_schedule"></span><code>get_​latest_​by_​schedule</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_latest_by_schedule(
    &self,
    schedule_id: UniversalUuid,
) -> Result<Option<ScheduleExecution>, ValidationError>
```

Gets the latest execution for a given schedule.

<details>
<summary>Source</summary>

```rust
    pub async fn get_latest_by_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<Option<ScheduleExecution>, ValidationError> {
        let result: Option<UnifiedScheduleExecution> =
            crate::interact_on_backend!(self.dal, |conn| {
                schedule_executions::table
                    .filter(schedule_executions::schedule_id.eq(schedule_id))
                    .order(schedule_executions::created_at.desc())
                    .first(conn)
                    .optional()
            })?;

        Ok(result.map(|r| r.into()))
    }
```

</details>

##### <span id="ScheduleExecutionDAL.get_execution_stats"></span><code>get_​execution_​stats</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_execution_stats(
    &self,
    since: DateTime<Utc>,
) -> Result<ScheduleExecutionStats, ValidationError>
```

Gets execution statistics for monitoring and alerting.

<details>
<summary>Source</summary>

```rust
    pub async fn get_execution_stats(
        &self,
        since: DateTime<Utc>,
    ) -> Result<ScheduleExecutionStats, ValidationError> {
        use crate::database::schema::unified::workflow_executions;

        let since_ts = UniversalTimestamp::from(since);
        let lost_cutoff = UniversalTimestamp::from(Utc::now() - Duration::minutes(10));

        let (total_executions, successful_executions, lost_executions) =
            crate::interact_on_backend!(self.dal, |conn| {
                let total_executions: i64 = schedule_executions::table
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .count()
                    .first(conn)?;

                let successful_executions: i64 = schedule_executions::table
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .filter(schedule_executions::workflow_execution_id.is_not_null())
                    .count()
                    .first(conn)?;

                let lost_executions: i64 = schedule_executions::table
                    .left_join(
                        workflow_executions::table.on(schedule_executions::workflow_execution_id
                            .eq(workflow_executions::id.nullable())),
                    )
                    .filter(workflow_executions::id.is_null())
                    .filter(schedule_executions::started_at.ge(since_ts))
                    .filter(schedule_executions::started_at.lt(lost_cutoff))
                    .count()
                    .first(conn)?;

                Ok::<(i64, i64, i64), diesel::result::Error>((
                    total_executions,
                    successful_executions,
                    lost_executions,
                ))
            })?;

        Ok(ScheduleExecutionStats {
            total_executions,
            successful_executions,
            lost_executions,
            success_rate: if total_executions > 0 {
                (successful_executions as f64 / total_executions as f64) * 100.0
            } else {
                0.0
            },
        })
    }
```

</details>

## Enums

### <span id="enum.RecoveryClaimResult"></span><code style="font-size: 0.85em">enum RecoveryClaimResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Outcome of a compare-and-set attempt to own the recovery of a lost handoff.

CLOACI-T-0926. Mirrors `RunnerClaimResult` in the task-claiming DAL: the
claim is a single conditional UPDATE and `rows_updated == 1` is the winner.

#### Variants

- **`Claimed`** - This caller now owns the re-fire of the handoff.
- **`NotClaimed`** - Another recovery service owns it (or the row left the lost set).

### <span id="enum.RecoveryHeartbeatResult"></span><code style="font-size: 0.85em">enum RecoveryHeartbeatResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Outcome of a recovery claim heartbeat.

#### Variants

- **`Ok`** - The claim is still ours.
- **`ClaimLost`** - Another recovery service took the claim over (ours went stale).
