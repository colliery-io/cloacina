---
title: "cloacina::dal::unified::schedule"
description: "Unified Schedule DAL with runtime backend selection"
editUrl: false
sidebar:
  order: 83
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified Schedule DAL with runtime backend selection

This module provides operations for the unified `schedules` table that
replaces the separate `cron_schedules` and `trigger_schedules` tables.
Works with both PostgreSQL and SQLite backends, selecting the appropriate
implementation at runtime based on the database connection type.

## Structs

### <span id="struct.ScheduleDAL"></span><code style="font-size: 0.85em">struct ScheduleDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Data access layer for unified schedule operations with runtime backend selection.

#### Methods

##### <span id="ScheduleDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

Creates a new ScheduleDAL instance.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="ScheduleDAL.create"></span><code>create</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create(
    &self,
    new_schedule: NewSchedule,
) -> Result<Schedule, ValidationError>
```

Creates a new schedule record in the database.

<details>
<summary>Source</summary>

```rust
    pub async fn create(&self, new_schedule: NewSchedule) -> Result<Schedule, ValidationError> {
        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedSchedule {
            id,
            schedule_type: new_schedule.schedule_type,
            workflow_name: new_schedule.workflow_name,
            enabled: new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true)),
            cron_expression: new_schedule.cron_expression,
            timezone: new_schedule.timezone,
            catchup_policy: new_schedule.catchup_policy,
            start_date: new_schedule.start_date,
            end_date: new_schedule.end_date,
            trigger_name: new_schedule.trigger_name,
            poll_interval_ms: new_schedule.poll_interval_ms,
            allow_concurrent: new_schedule.allow_concurrent,
            next_run_at: new_schedule.next_run_at,
            created_at: now,
            updated_at: now,
            params: new_schedule.params,
            instance_name: new_schedule.instance_name,
        };

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(schedules::table)
                .values(&new_unified)
                .execute(conn)
        })?;

        let result: UnifiedSchedule =
            crate::interact_on_backend!(self.dal, |conn| schedules::table.find(id).first(conn))?;

        Ok(result.into())
    }
```

</details>

##### <span id="ScheduleDAL.find_by_instance_name"></span><code>find_​by_​instance_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_by_instance_name(
    &self,
    workflow_name: &str,
    instance_name: &str,
) -> Result<Option<Schedule>, ValidationError>
```

Retrieves a schedule by its ID. Look up a NAMED instance schedule by `(workflow_name, instance_name)` (CLOACI-I-0116). `Ok(None)` when no such instance exists.

<details>
<summary>Source</summary>

```rust
    pub async fn find_by_instance_name(
        &self,
        workflow_name: &str,
        instance_name: &str,
    ) -> Result<Option<Schedule>, ValidationError> {
        let wf = workflow_name.to_string();
        let inst = instance_name.to_string();
        let result: Option<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::workflow_name.eq(wf))
                .filter(schedules::instance_name.eq(inst))
                .first(conn)
                .optional()
        })?;
        Ok(result.map(Into::into))
    }
```

</details>

##### <span id="ScheduleDAL.get_by_id"></span><code>get_​by_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_by_id(
    &self,
    id: UniversalUuid,
) -> Result<Schedule, ValidationError>
```

<details>
<summary>Source</summary>

```rust
    pub async fn get_by_id(&self, id: UniversalUuid) -> Result<Schedule, ValidationError> {
        let result: UnifiedSchedule =
            crate::interact_on_backend!(self.dal, |conn| schedules::table.find(id).first(conn))?;
        Ok(result.into())
    }
```

</details>

##### <span id="ScheduleDAL.list"></span><code>list</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list(
    &self,
    schedule_type: Option<&str>,
    enabled_only: bool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Schedule>, ValidationError>
```

Lists schedules with optional filtering by type and enabled status.

<details>
<summary>Source</summary>

```rust
    pub async fn list(
        &self,
        schedule_type: Option<&str>,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let schedule_type = schedule_type.map(|s| s.to_string());
        let enabled_true = UniversalBool::from(true);
        let results: Vec<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            let mut query = schedules::table.into_boxed();

            if let Some(ref stype) = schedule_type {
                query = query.filter(schedules::schedule_type.eq(stype));
            }

            if enabled_only {
                query = query.filter(schedules::enabled.eq(enabled_true));
            }

            query
                .order(schedules::created_at.desc())
                .limit(limit)
                .offset(offset)
                .load(conn)
        })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }
```

</details>

##### <span id="ScheduleDAL.enable"></span><code>enable</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn enable(&self, id: UniversalUuid) -> Result<(), ValidationError>
```

Enables a schedule.

<details>
<summary>Source</summary>

```rust
    pub async fn enable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let enabled_true = UniversalBool::from(true);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::enabled.eq(enabled_true),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleDAL.disable"></span><code>disable</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn disable(
    &self) -> Result<(,
    id: UniversalUuid) -> Result<(,
) -> Result<(), ValidationError>
```

Disables a schedule.

<details>
<summary>Source</summary>

```rust
    pub async fn disable(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let enabled_false = UniversalBool::from(false);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::enabled.eq(enabled_false),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleDAL.pause"></span><code>pause</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn pause(&self, id: UniversalUuid) -> Result<(), ValidationError>
```

Pauses a schedule (CLOACI-T-0749): the scheduler stops firing it until resumed. Distinct from `disable` — pause is transient operator state.

<details>
<summary>Source</summary>

```rust
    pub async fn pause(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let paused_true = UniversalBool::from(true);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::paused.eq(paused_true),
                    schedules::paused_at.eq(Some(now)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleDAL.resume"></span><code>resume</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn resume(&self, id: UniversalUuid) -> Result<(), ValidationError>
```

Resumes a paused schedule (CLOACI-T-0749): clears the paused flag so the scheduler fires it again on its normal schedule. Missed fires are not caught up (skip policy).

<details>
<summary>Source</summary>

```rust
    pub async fn resume(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        let now = UniversalTimestamp::now();
        let paused_false = UniversalBool::from(false);

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::paused.eq(paused_false),
                    schedules::paused_at.eq(None::<UniversalTimestamp>),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleDAL.delete"></span><code>delete</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn delete(&self, id: UniversalUuid) -> Result<(), ValidationError>
```

Deletes a schedule from the database.

<details>
<summary>Source</summary>

```rust
    pub async fn delete(&self, id: UniversalUuid) -> Result<(), ValidationError> {
        crate::interact_on_backend!(self.dal, |conn| diesel::delete(schedules::table.find(id))
            .execute(conn))?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleDAL.get_due_cron_schedules"></span><code>get_​due_​cron_​schedules</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_due_cron_schedules(
    &self,
    now: DateTime<Utc>,
) -> Result<Vec<Schedule>, ValidationError>
```

Retrieves all enabled cron schedules that are due for execution.

<details>
<summary>Source</summary>

```rust
    pub async fn get_due_cron_schedules(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let now_ts = UniversalTimestamp::from(now);
        let enabled_true = UniversalBool::from(true);

        let results: Vec<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::schedule_type.eq("cron"))
                .filter(schedules::enabled.eq(enabled_true))
                .filter(schedules::paused.eq(UniversalBool::from(false)))
                .filter(schedules::next_run_at.le(Some(now_ts)))
                .order(schedules::next_run_at.asc())
                .load(conn)
        })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }
```

</details>

##### <span id="ScheduleDAL.next_cron_due_time"></span><code>next_​cron_​due_​time</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn next_cron_due_time(
    &self,
) -> Result<Option<DateTime<Utc>>, ValidationError>
```

Earliest `next_run_at` over all enabled cron schedules, or `None` when there are none. Powers the timer-driven scheduler's sleep-until-next-due loop (CLOACI-T-0743) — the scheduler sleeps until this instant instead of polling on a fixed interval.

<details>
<summary>Source</summary>

```rust
    pub async fn next_cron_due_time(&self) -> Result<Option<DateTime<Utc>>, ValidationError> {
        let enabled_true = UniversalBool::from(true);

        let earliest: Option<UniversalTimestamp> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::schedule_type.eq("cron"))
                .filter(schedules::enabled.eq(enabled_true))
                .filter(schedules::paused.eq(UniversalBool::from(false)))
                .filter(schedules::next_run_at.is_not_null())
                .order(schedules::next_run_at.asc())
                .select(schedules::next_run_at)
                .first::<Option<UniversalTimestamp>>(conn)
                .optional()
        })?
        .flatten();

        Ok(earliest.map(DateTime::<Utc>::from))
    }
```

</details>

##### <span id="ScheduleDAL.claim_and_update_cron"></span><code>claim_​and_​update_​cron</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn claim_and_update_cron(
    &self,
    id: UniversalUuid,
    current_time: DateTime<Utc>,
    last_run: DateTime<Utc>,
    next_run: DateTime<Utc>,
) -> Result<bool, ValidationError>
```

Atomically claims and updates a cron schedule's timing.

<details>
<summary>Source</summary>

```rust
    pub async fn claim_and_update_cron(
        &self,
        id: UniversalUuid,
        current_time: DateTime<Utc>,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<bool, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.claim_and_update_cron_postgres(id, current_time, last_run, next_run)
                .await,
            self.claim_and_update_cron_sqlite(id, current_time, last_run, next_run)
                .await
        )
    }
```

</details>

##### <span id="ScheduleDAL.update_schedule_times"></span><code>update_​schedule_​times</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn update_schedule_times(
    &self,
    id: UniversalUuid,
    last_run: DateTime<Utc>,
    next_run: DateTime<Utc>,
) -> Result<(), ValidationError>
```

Updates the last run and next run times for a schedule.

<details>
<summary>Source</summary>

```rust
    pub async fn update_schedule_times(
        &self,
        id: UniversalUuid,
        last_run: DateTime<Utc>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let last_run_ts = UniversalTimestamp::from(last_run);
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::last_run_at.eq(Some(last_run_ts)),
                    schedules::next_run_at.eq(Some(next_run_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleDAL.get_enabled_triggers"></span><code>get_​enabled_​triggers</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_enabled_triggers(
    &self,
) -> Result<Vec<Schedule>, ValidationError>
```

Retrieves all enabled trigger schedules.

<details>
<summary>Source</summary>

```rust
    pub async fn get_enabled_triggers(&self) -> Result<Vec<Schedule>, ValidationError> {
        let enabled_true = UniversalBool::from(true);

        let results: Vec<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::schedule_type.eq("trigger"))
                .filter(schedules::enabled.eq(enabled_true))
                .filter(schedules::paused.eq(UniversalBool::from(false)))
                .order(schedules::created_at.asc())
                .load(conn)
        })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }
```

</details>

##### <span id="ScheduleDAL.update_last_poll"></span><code>update_​last_​poll</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn update_last_poll(
    &self,
    id: UniversalUuid,
    last_poll_at: DateTime<Utc>,
) -> Result<(), ValidationError>
```

Updates the last poll time for a trigger schedule.

<details>
<summary>Source</summary>

```rust
    pub async fn update_last_poll(
        &self,
        id: UniversalUuid,
        last_poll_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let last_poll_ts = UniversalTimestamp::from(last_poll_at);
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::last_poll_at.eq(Some(last_poll_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>

##### <span id="ScheduleDAL.upsert_trigger"></span><code>upsert_​trigger</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn upsert_trigger(
    &self,
    new_schedule: NewSchedule,
) -> Result<Schedule, ValidationError>
```

Upserts a trigger schedule by trigger_name.

<details>
<summary>Source</summary>

```rust
    pub async fn upsert_trigger(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        let trigger_name =
            new_schedule
                .trigger_name
                .clone()
                .ok_or_else(|| ValidationError::DatabaseQuery {
                    message: "trigger_name is required for upsert_trigger".to_string(),
                })?;

        let trigger_name_check = trigger_name.clone();

        // Check if a schedule with this trigger_name already exists
        let existing: Option<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::trigger_name.eq(trigger_name_check))
                .filter(schedules::schedule_type.eq("trigger"))
                .first(conn)
                .optional()
        })?;

        if let Some(existing) = existing {
            // Update existing schedule
            let now = UniversalTimestamp::now();
            let existing_id = existing.id;
            let workflow_name = new_schedule.workflow_name;
            let poll_interval_ms = new_schedule.poll_interval_ms;
            let allow_concurrent = new_schedule.allow_concurrent;
            let enabled = new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true));

            crate::interact_on_backend!(self.dal, |conn| {
                diesel::update(schedules::table.find(existing_id))
                    .set((
                        schedules::workflow_name.eq(workflow_name),
                        schedules::poll_interval_ms.eq(poll_interval_ms),
                        schedules::allow_concurrent.eq(allow_concurrent),
                        schedules::enabled.eq(enabled),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })?;

            let result: UnifiedSchedule = crate::interact_on_backend!(self.dal, |conn| {
                schedules::table.find(existing_id).first(conn)
            })?;

            Ok(result.into())
        } else {
            // Insert new schedule
            let id = UniversalUuid::new_v4();
            let now = UniversalTimestamp::now();

            let new_unified = NewUnifiedSchedule {
                id,
                schedule_type: new_schedule.schedule_type,
                workflow_name: new_schedule.workflow_name,
                enabled: new_schedule
                    .enabled
                    .unwrap_or_else(|| UniversalBool::from(true)),
                cron_expression: new_schedule.cron_expression,
                timezone: new_schedule.timezone,
                catchup_policy: new_schedule.catchup_policy,
                start_date: new_schedule.start_date,
                end_date: new_schedule.end_date,
                trigger_name: new_schedule.trigger_name,
                poll_interval_ms: new_schedule.poll_interval_ms,
                allow_concurrent: new_schedule.allow_concurrent,
                next_run_at: new_schedule.next_run_at,
                created_at: now,
                updated_at: now,
                params: new_schedule.params,
                instance_name: new_schedule.instance_name,
            };

            crate::interact_on_backend!(self.dal, |conn| {
                diesel::insert_into(schedules::table)
                    .values(&new_unified)
                    .execute(conn)
            })?;

            let result: UnifiedSchedule =
                crate::interact_on_backend!(self.dal, |conn| schedules::table
                    .find(id)
                    .first(conn))?;

            Ok(result.into())
        }
    }
```

</details>

##### <span id="ScheduleDAL.upsert_cron"></span><code>upsert_​cron</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn upsert_cron(
    &self,
    new_schedule: NewSchedule,
) -> Result<Schedule, ValidationError>
```

Upserts a cron schedule by its identity (workflow_name, cron_expression, timezone). Idempotent: re-registering the same packaged cron trigger (e.g. on every reconcile re-load) updates the existing row's next-run time instead of inserting a duplicate (CLOACI-T-0669). Returns the resulting schedule.

<details>
<summary>Source</summary>

```rust
    pub async fn upsert_cron(
        &self,
        new_schedule: NewSchedule,
    ) -> Result<Schedule, ValidationError> {
        // Identity of a cron schedule: (workflow_name, cron_expression,
        // timezone). Re-registering the same packaged cron on a reconcile
        // re-load updates the existing row instead of inserting a duplicate.
        let cron_expression =
            new_schedule
                .cron_expression
                .clone()
                .ok_or_else(|| ValidationError::DatabaseQuery {
                    message: "cron_expression is required for upsert_cron".to_string(),
                })?;
        let workflow_name = new_schedule.workflow_name.clone();
        let timezone = new_schedule.timezone.clone();

        let (wf, cron, tz) = (
            workflow_name.clone(),
            cron_expression.clone(),
            timezone.clone(),
        );
        let existing: Option<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            let mut q = schedules::table
                .filter(schedules::schedule_type.eq("cron"))
                .filter(schedules::workflow_name.eq(wf))
                .filter(schedules::cron_expression.eq(cron))
                .into_boxed();
            q = match tz {
                Some(t) => q.filter(schedules::timezone.eq(t)),
                None => q.filter(schedules::timezone.is_null()),
            };
            q.first(conn).optional()
        })?;

        if let Some(existing) = existing {
            let now = UniversalTimestamp::now();
            let existing_id = existing.id;
            let next_run_at = new_schedule.next_run_at;
            let enabled = new_schedule
                .enabled
                .unwrap_or_else(|| UniversalBool::from(true));

            crate::interact_on_backend!(self.dal, |conn| {
                diesel::update(schedules::table.find(existing_id))
                    .set((
                        schedules::next_run_at.eq(next_run_at),
                        schedules::enabled.eq(enabled),
                        schedules::updated_at.eq(now),
                    ))
                    .execute(conn)
            })?;

            let result: UnifiedSchedule = crate::interact_on_backend!(self.dal, |conn| {
                schedules::table.find(existing_id).first(conn)
            })?;
            Ok(result.into())
        } else {
            let id = UniversalUuid::new_v4();
            let now = UniversalTimestamp::now();
            let new_unified = NewUnifiedSchedule {
                id,
                schedule_type: new_schedule.schedule_type,
                workflow_name: new_schedule.workflow_name,
                enabled: new_schedule
                    .enabled
                    .unwrap_or_else(|| UniversalBool::from(true)),
                cron_expression: new_schedule.cron_expression,
                timezone: new_schedule.timezone,
                catchup_policy: new_schedule.catchup_policy,
                start_date: new_schedule.start_date,
                end_date: new_schedule.end_date,
                trigger_name: new_schedule.trigger_name,
                poll_interval_ms: new_schedule.poll_interval_ms,
                allow_concurrent: new_schedule.allow_concurrent,
                next_run_at: new_schedule.next_run_at,
                created_at: now,
                updated_at: now,
                params: new_schedule.params,
                instance_name: new_schedule.instance_name,
            };
            crate::interact_on_backend!(self.dal, |conn| {
                diesel::insert_into(schedules::table)
                    .values(&new_unified)
                    .execute(conn)
            })?;

            let result: UnifiedSchedule =
                crate::interact_on_backend!(self.dal, |conn| schedules::table
                    .find(id)
                    .first(conn))?;
            Ok(result.into())
        }
    }
```

</details>

##### <span id="ScheduleDAL.get_by_trigger_name"></span><code>get_​by_​trigger_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_by_trigger_name(
    &self,
    name: &str,
) -> Result<Option<Schedule>, ValidationError>
```

Retrieves a schedule by its trigger name.

<details>
<summary>Source</summary>

```rust
    pub async fn get_by_trigger_name(
        &self,
        name: &str,
    ) -> Result<Option<Schedule>, ValidationError> {
        let name_owned = name.to_string();
        let result: Option<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::trigger_name.eq(name_owned))
                .filter(schedules::schedule_type.eq("trigger"))
                .first(conn)
                .optional()
        })?;

        Ok(result.map(|r| r.into()))
    }
```

</details>

##### <span id="ScheduleDAL.find_by_workflow"></span><code>find_​by_​workflow</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_by_workflow(
    &self,
    workflow_name: &str,
) -> Result<Vec<Schedule>, ValidationError>
```

Finds schedules by workflow name.

<details>
<summary>Source</summary>

```rust
    pub async fn find_by_workflow(
        &self,
        workflow_name: &str,
    ) -> Result<Vec<Schedule>, ValidationError> {
        let name_owned = workflow_name.to_string();
        let results: Vec<UnifiedSchedule> = crate::interact_on_backend!(self.dal, |conn| {
            schedules::table
                .filter(schedules::workflow_name.eq(name_owned))
                .order(schedules::created_at.desc())
                .load(conn)
        })?;

        Ok(results.into_iter().map(|r| r.into()).collect())
    }
```

</details>

##### <span id="ScheduleDAL.update_cron_expression_and_timezone"></span><code>update_​cron_​expression_​and_​timezone</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn update_cron_expression_and_timezone(
    &self,
    id: UniversalUuid,
    cron_expression: Option<&str>,
    timezone: Option<&str>,
    next_run: DateTime<Utc>,
) -> Result<(), ValidationError>
```

Updates the cron expression and timezone for a cron schedule.

<details>
<summary>Source</summary>

```rust
    pub async fn update_cron_expression_and_timezone(
        &self,
        id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
        next_run: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let cron_expression_owned = cron_expression.map(|s| s.to_string());
        let timezone_owned = timezone.map(|s| s.to_string());
        let next_run_ts = UniversalTimestamp::from(next_run);
        let now = UniversalTimestamp::now();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(schedules::table.find(id))
                .set((
                    schedules::cron_expression.eq(cron_expression_owned),
                    schedules::timezone.eq(timezone_owned),
                    schedules::next_run_at.eq(Some(next_run_ts)),
                    schedules::updated_at.eq(now),
                ))
                .execute(conn)
        })?;

        Ok(())
    }
```

</details>
