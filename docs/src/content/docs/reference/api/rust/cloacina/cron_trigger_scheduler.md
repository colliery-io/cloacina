---
title: "cloacina::cron_trigger_scheduler"
description: "Unified scheduler for both cron and trigger-based workflow execution."
editUrl: false
sidebar:
  order: 57
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Unified scheduler for both cron and trigger-based workflow execution.

This module provides a single `Scheduler` that replaces the separate
`CronScheduler` and `TriggerScheduler`, driving both cron and trigger
schedules from one run loop backed by the unified `schedules` and
`schedule_executions` tables.

**Key Features**

- **Single Run Loop**: One tick drives both cron and trigger checks
- **Atomic Claiming**: Prevents duplicate cron executions across instances
- **Per-trigger Poll Intervals**: Each trigger retains its own polling frequency
- **Context-based Deduplication**: Prevents duplicate trigger executions
- **Catchup Policies**: Configurable handling of missed cron executions
- **Audit Trail**: Records every handoff via `schedule_executions`
- **Saga Pattern**: Clean separation between scheduling and execution

**Architecture**

```text
┌───────────────┐    claim / fire   ┌──────────────────┐    execute    ┌─────────────┐
│   Scheduler   │   & hand off      │ WorkflowExecutor │  workflows   │   Tasks     │
│               │ ─────────────────▶│                  │ ────────────▶│             │
│ • Poll cron   │                   │ • Execute        │              │ • Business  │
│ • Poll trigs  │                   │ • Retry          │              │   Logic     │
│ • Deduplicate │                   │ • Recovery       │              │ • Context   │
│ • Audit log   │                   │                  │              │             │
└───────────────┘                   └──────────────────┘              └─────────────┘
```

## Structs

### <span id="struct.SchedulerConfig"></span><code style="font-size: 0.85em">struct SchedulerConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Configuration for the unified scheduler.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `cron_poll_interval` | `Duration` | Backstop for the timer-driven cron loop (CLOACI-T-0743). The scheduler sleeps until the next due schedule's exact instant; this caps that sleep so the loop still re-checks the DB at least this often even absent a change notification — a safety net for missed notifies and the multi-instance case (an in-process notify only wakes the local replica). It is NOT a fixed poll: when the next fire is sooner than this, the loop wakes exactly at the fire time, not on this interval. |
| `max_catchup_executions` | `usize` | Maximum number of missed executions to run in catchup mode. |
| `max_acceptable_delay` | `Duration` | Maximum acceptable delay for cron (used for observability / alerting). |
| `trigger_base_poll_interval` | `Duration` | Base poll interval — the tick rate of the run loop. |
| `trigger_poll_timeout` | `Duration` | Maximum time to wait for a single trigger poll operation. |
| `reactor_poll_interval` | `Duration` | How often to poll reactor subscriptions for new firings (CLOACI-I-0100 / T-0599). Defaults to the base tick interval. |
| `reactor_poll_batch_limit` | `i64` | Maximum number of unconsumed firings to drain per subscription per tick. Caps unbounded backlog work on a single tick. |
| `reactor_firings_prune_interval` | `Duration` | How often to prune old `reactor_firings` rows (CLOACI-I-0100 / T-0601). Defaults to 1 hour. |
| `reactor_firings_retention` | `Duration` | Retention window for `reactor_firings` rows. Anything with `fired_at < now - retention` is deleted on each prune sweep. Defaults to 7 days. Subscriptions whose watermark predates the retention window will miss firings — documented gotcha. |

### <span id="struct.Scheduler"></span><code style="font-size: 0.85em">struct Scheduler</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Unified scheduler for both cron and trigger-based workflow execution.

The scheduler runs a single polling loop that:
1. Ticks at `trigger_base_poll_interval` (default 1 s)
2. Every `cron_poll_interval`, queries due cron schedules and processes them
3. Every tick, checks enabled triggers respecting per-trigger poll intervals
4. Records audit trail for every handoff via `schedule_executions`

**Responsibilities**

**What Scheduler Does:**
- Poll database for due cron schedules and enabled triggers
- Atomically claim cron schedules
- Calculate missed execution times (catchup)
- Poll trigger functions and deduplicate
- Hand off workflow executions to the workflow executor
- Record execution audit trail
- Move on immediately (no waiting for completion)

**What Scheduler Does NOT Do:**
- Execute workflows directly
- Handle task retries or failures
- Wait for workflow completion
- Manage workflow state or recovery

#### Methods

##### <span id="Scheduler.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    dal: Arc<DAL>,
    executor: Arc<dyn WorkflowExecutor>,
    config: SchedulerConfig,
    shutdown: watch::Receiver<bool>,
    runtime: Arc<Runtime>,
    cron_change: Arc<Notify>,
) -> Self
```

Creates a new unified scheduler.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `dal` | `Arc<DAL>` | Data access layer for database operations |
| `executor` | `Arc<dyn WorkflowExecutor>` | Workflow executor for workflow execution |
| `config` | `SchedulerConfig` | Scheduler configuration |
| `shutdown` | `watch::Receiver<bool>` | Shutdown signal receiver |

<details>
<summary>Source</summary>

```rust
    pub fn new(
        dal: Arc<DAL>,
        executor: Arc<dyn WorkflowExecutor>,
        config: SchedulerConfig,
        shutdown: watch::Receiver<bool>,
        runtime: Arc<Runtime>,
        cron_change: Arc<Notify>,
    ) -> Self {
        Self {
            dal,
            executor,
            config,
            shutdown,
            runtime,
            cron_change,
            last_poll_times: HashMap::new(),
            last_reactor_poll: None,
            last_reactor_prune: None,
            predicate_cache: Arc::new(parking_lot::Mutex::new(HashMap::new())),
        }
    }
```

</details>

##### <span id="Scheduler.with_defaults"></span><code>with_​defaults</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_defaults(
    dal: Arc<DAL>,
    executor: Arc<dyn WorkflowExecutor>,
    shutdown: watch::Receiver<bool>,
    runtime: Arc<Runtime>,
) -> Self
```

Creates a new unified scheduler with default configuration.

<details>
<summary>Source</summary>

```rust
    pub fn with_defaults(
        dal: Arc<DAL>,
        executor: Arc<dyn WorkflowExecutor>,
        shutdown: watch::Receiver<bool>,
        runtime: Arc<Runtime>,
    ) -> Self {
        Self::new(
            dal,
            executor,
            SchedulerConfig::default(),
            shutdown,
            runtime,
            Arc::new(Notify::new()),
        )
    }
```

</details>

##### <span id="Scheduler.run_polling_loop"></span><code>run_​polling_​loop</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn run_polling_loop(
    &mut self) -> Result<(,
) -> Result<(), WorkflowExecutionError>
```

Runs the main polling loop.

Ticks at `trigger_base_poll_interval`. On each tick it:
- Checks cron schedules if `cron_poll_interval` has elapsed since the
  last cron check.
- Checks all enabled triggers, respecting per-trigger poll intervals.

The loop continues until a shutdown signal is received.

<details>
<summary>Source</summary>

```rust
    pub async fn run_polling_loop(&mut self) -> Result<(), WorkflowExecutionError> {
        info!(
            "Starting unified scheduler (cron interval: {:?}, trigger base interval: {:?})",
            self.config.cron_poll_interval, self.config.trigger_base_poll_interval,
        );

        let mut interval = tokio::time::interval(self.config.trigger_base_poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Timer-driven cron (CLOACI-T-0743): instead of sweeping every
        // `cron_poll_interval`, cache the next due instant and sleep until it.
        // Recomputed only after a fire or a `cron_change` notification, so the
        // 1 s trigger/reactor tick does NOT re-query the DB for cron.
        //
        // CLOACI-T-0928: the sleep must be created ONCE and live across loop
        // iterations. The earlier shape rebuilt it from zero every iteration,
        // and the 1 s trigger tick always won the select first — so a sleep
        // longer than ~1 s never accumulated progress and the backstop could
        // never elapse. Now only the branches that consume or invalidate the
        // deadline reset it.
        let mut next_cron_due = self.query_next_cron_due().await;
        let cron_sleep = tokio::time::sleep(self.cron_sleep_delay(next_cron_due));
        tokio::pin!(cron_sleep);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // --- Triggers ---
                    if let Err(e) = self.check_and_process_triggers().await {
                        error!("Error processing triggers: {}", e);
                    }

                    // --- Reactor subscriptions (CLOACI-I-0100 / T-0599) ---
                    let now = Instant::now();
                    let should_poll_reactors = match self.last_reactor_poll {
                        Some(last) => now.duration_since(last) >= self.config.reactor_poll_interval,
                        None => true,
                    };
                    if should_poll_reactors {
                        self.last_reactor_poll = Some(now);
                        if let Err(e) = self.check_and_process_reactor_subscriptions().await {
                            error!("Error processing reactor subscriptions: {}", e);
                        }
                    }

                    // --- Reactor firings TTL prune (CLOACI-I-0100 / T-0601) ---
                    let should_prune = match self.last_reactor_prune {
                        Some(last) => {
                            now.duration_since(last) >= self.config.reactor_firings_prune_interval
                        }
                        None => true,
                    };
                    if should_prune {
                        self.last_reactor_prune = Some(now);
                        self.prune_reactor_firings().await;
                    }
                }
                // --- Cron: wake exactly at the next due instant (or backstop) ---
                _ = &mut cron_sleep => {
                    if let Err(e) = self.check_and_execute_cron_schedules().await {
                        error!("Error processing cron schedules: {}", e);
                    }
                    next_cron_due = self.query_next_cron_due().await;
                    // An elapsed Sleep stays ready forever — re-arm it, or this
                    // branch would win every subsequent select (busy loop).
                    cron_sleep.as_mut().reset(
                        tokio::time::Instant::now() + self.cron_sleep_delay(next_cron_due),
                    );
                }
                // --- Cron schedule changed (registered/enabled/deleted): re-arm ---
                _ = self.cron_change.notified() => {
                    debug!("Cron change notification — recomputing next due time");
                    next_cron_due = self.query_next_cron_due().await;
                    cron_sleep.as_mut().reset(
                        tokio::time::Instant::now() + self.cron_sleep_delay(next_cron_due),
                    );
                }
                _ = self.shutdown.changed() => {
                    if *self.shutdown.borrow() {
                        info!("Unified scheduler received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Unified scheduler polling loop stopped");
        Ok(())
    }
```

</details>

##### <span id="Scheduler.poll_reactor_subscriptions_once"></span><code>poll_​reactor_​subscriptions_​once</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn poll_reactor_subscriptions_once(
    &self) -> Result<(,
) -> Result<(), WorkflowExecutionError>
```

Polls the `reactor_trigger_subscriptions` table and dispatches one workflow execution per unconsumed `reactor_firings` row.

Watermark advance happens after dispatch — at-least-once on crash.
Workflow idempotency is the user's concern (same as cron-triggered
workflows).
Run one pass over enabled reactor subscriptions, draining new
firings and dispatching workflows. Exposed publicly so that
integration tests can drive the loop deterministically without
waiting on the background tick, and so operators can trigger
an immediate poll in ad-hoc scripts.

<details>
<summary>Source</summary>

```rust
    pub async fn poll_reactor_subscriptions_once(&self) -> Result<(), WorkflowExecutionError> {
        self.check_and_process_reactor_subscriptions().await
    }
```

</details>

##### <span id="Scheduler.register_trigger"></span><code>register_​trigger</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn register_trigger(
    &self,
    trigger: &dyn Trigger,
    workflow_name: &str,
) -> Result<Schedule, ValidationError>
```

Registers a trigger with the scheduler.

Persists the trigger configuration to the database for recovery across
restarts. The trigger must also be registered in the global trigger
registry for the actual polling function.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger` | `&dyn Trigger` | The trigger instance to register |
| `workflow_name` | `&str` | Name of the workflow to fire when trigger activates |

<details>
<summary>Source</summary>

```rust
    pub async fn register_trigger(
        &self,
        trigger: &dyn Trigger,
        workflow_name: &str,
    ) -> Result<Schedule, ValidationError> {
        let mut new_schedule =
            NewSchedule::trigger(trigger.name(), workflow_name, trigger.poll_interval());
        new_schedule.allow_concurrent = Some(crate::database::universal_types::UniversalBool::new(
            trigger.allow_concurrent(),
        ));

        // Upsert to handle re-registration
        self.dal.schedule().upsert_trigger(new_schedule).await
    }
```

</details>

##### <span id="Scheduler.disable_trigger"></span><code>disable_​trigger</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn disable_trigger(
    &self) -> Result<(,
    trigger_name: &str) -> Result<(,
) -> Result<(), ValidationError>
```

Disables a trigger by name.

<details>
<summary>Source</summary>

```rust
    pub async fn disable_trigger(&self, trigger_name: &str) -> Result<(), ValidationError> {
        if let Some(schedule) = self
            .dal
            .schedule()
            .get_by_trigger_name(trigger_name)
            .await?
        {
            self.dal.schedule().disable(schedule.id).await?;
            info!("Disabled trigger '{}'", trigger_name);
        }
        Ok(())
    }
```

</details>

##### <span id="Scheduler.enable_trigger"></span><code>enable_​trigger</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn enable_trigger(
    &self) -> Result<(,
    trigger_name: &str) -> Result<(,
) -> Result<(), ValidationError>
```

Enables a trigger by name.

<details>
<summary>Source</summary>

```rust
    pub async fn enable_trigger(&self, trigger_name: &str) -> Result<(), ValidationError> {
        if let Some(schedule) = self
            .dal
            .schedule()
            .get_by_trigger_name(trigger_name)
            .await?
        {
            self.dal.schedule().enable(schedule.id).await?;
            info!("Enabled trigger '{}'", trigger_name);
        }
        Ok(())
    }
```

</details>

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.MAX_CONSECUTIVE_PREDICATE_ERRORS"></span>`MAX_CONSECUTIVE_PREDICATE_ERRORS` | `i32` | `5` | CLOACI-T-0922 — how many consecutive CEL evaluation errors on the SAME reactor firing are tolerated before that firing is dead-lettered and the watermark force-advances past it.<br>The bound exists because holding the watermark forever would wedge the subscription: a firing whose payload shape genuinely breaks the predicate never becomes evaluable, and every later firing would queue behind it. Five poll ticks is enough to ride out a transient fault (a momentarily missing payload key, a blip in payload decode) while still resuming promptly on a structural one. |
