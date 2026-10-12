---
title: "cloacina::dal::unified::models"
description: "Unified database models using custom SQL types"
sidebar:
  order: 93
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Unified database models using custom SQL types

These models use the unified schema with DbUuid, DbTimestamp, DbBool custom
SQL types that work with both PostgreSQL and SQLite backends.

## Structs

### <span id="struct.UnifiedDbContext"></span>`struct UnifiedDbContext`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

Unified context model that works with both PostgreSQL and SQLite.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `value` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.NewUnifiedDbContext"></span>`struct NewUnifiedDbContext`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

Insertable context with explicit ID and timestamps (for SQLite compatibility).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `value` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedWorkflowExecution"></span>`struct UnifiedWorkflowExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `workflow_name` | `String` |  |
| `workflow_version` | `String` |  |
| `status` | `String` |  |
| `context_id` | `Option < UniversalUuid >` |  |
| `started_at` | `UniversalTimestamp` |  |
| `completed_at` | `Option < UniversalTimestamp >` |  |
| `error_details` | `Option < String >` |  |
| `recovery_attempts` | `i32` |  |
| `last_recovery_at` | `Option < UniversalTimestamp >` |  |
| `paused_at` | `Option < UniversalTimestamp >` |  |
| `pause_reason` | `Option < String >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `trigger_origin` | `Option < String >` | How this run was triggered (CLOACI-T-0776); `Some("manual")` for an
operator REST run, `None` otherwise. |



### <span id="struct.NewUnifiedWorkflowExecution"></span>`struct NewUnifiedWorkflowExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `workflow_name` | `String` |  |
| `workflow_version` | `String` |  |
| `status` | `String` |  |
| `context_id` | `Option < UniversalUuid >` |  |
| `started_at` | `UniversalTimestamp` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedTaskExecution"></span>`struct UnifiedTaskExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_name` | `String` |  |
| `status` | `String` |  |
| `started_at` | `Option < UniversalTimestamp >` |  |
| `completed_at` | `Option < UniversalTimestamp >` |  |
| `attempt` | `i32` |  |
| `max_attempts` | `i32` |  |
| `error_details` | `Option < String >` |  |
| `trigger_rules` | `String` |  |
| `task_configuration` | `String` |  |
| `retry_at` | `Option < UniversalTimestamp >` |  |
| `last_error` | `Option < String >` |  |
| `recovery_attempts` | `i32` |  |
| `last_recovery_at` | `Option < UniversalTimestamp >` |  |
| `sub_status` | `Option < String >` |  |
| `claimed_by` | `Option < UniversalUuid >` |  |
| `heartbeat_at` | `Option < UniversalTimestamp >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.NewUnifiedTaskExecution"></span>`struct NewUnifiedTaskExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_name` | `String` |  |
| `status` | `String` |  |
| `attempt` | `i32` |  |
| `max_attempts` | `i32` |  |
| `trigger_rules` | `String` |  |
| `task_configuration` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedTaskExecutionMetadata"></span>`struct UnifiedTaskExecutionMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `task_execution_id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_name` | `String` |  |
| `context_id` | `Option < UniversalUuid >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.NewUnifiedTaskExecutionMetadata"></span>`struct NewUnifiedTaskExecutionMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `task_execution_id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_name` | `String` |  |
| `context_id` | `Option < UniversalUuid >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedRecoveryEvent"></span>`struct UnifiedRecoveryEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_execution_id` | `Option < UniversalUuid >` |  |
| `recovery_type` | `String` |  |
| `recovered_at` | `UniversalTimestamp` |  |
| `details` | `Option < String >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.NewUnifiedRecoveryEvent"></span>`struct NewUnifiedRecoveryEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_execution_id` | `Option < UniversalUuid >` |  |
| `recovery_type` | `String` |  |
| `recovered_at` | `UniversalTimestamp` |  |
| `details` | `Option < String >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedExecutionEvent"></span>`struct UnifiedExecutionEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

Unified execution event model for audit trail of state transitions. Append-only: events are never updated after creation.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_execution_id` | `Option < UniversalUuid >` |  |
| `event_type` | `String` |  |
| `event_data` | `Option < String >` |  |
| `worker_id` | `Option < String >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `sequence_num` | `i64` |  |
| `request_id` | `Option < UniversalUuid >` |  |
| `runner_id` | `Option < UniversalUuid >` |  |
| `tenant_id` | `Option < String >` |  |



### <span id="struct.NewUnifiedExecutionEvent"></span>`struct NewUnifiedExecutionEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_execution_id` | `Option < UniversalUuid >` |  |
| `event_type` | `String` |  |
| `event_data` | `Option < String >` |  |
| `worker_id` | `Option < String >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `request_id` | `Option < UniversalUuid >` | CLOACI-T-0583: id of the originating request (from the tracing span,
after T-0578 lands). `None` on transitional paths. |
| `runner_id` | `Option < UniversalUuid >` | CLOACI-T-0583: id of the runner instance that emitted the event.
Populated for per-tenant runner emissions (after T-0580), `None` for
the single-runner daemon path. |
| `tenant_id` | `Option < String >` | CLOACI-T-0583: tenant scope. Populated from `AuthenticatedKey`
(server) or the current tenant context. `None` on the daemon path
and on background-scheduler emissions that don't have a tenant. |



### <span id="struct.UnifiedDeliveryOutbox"></span>`struct UnifiedDeliveryOutbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

Unified delivery-outbox row: durable, ack-tracked, recipient-addressed push delivery for the interservice communication substrate.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `i64` |  |
| `recipient` | `String` |  |
| `kind` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `payload` | `UniversalBinary` |  |
| `delivery_state` | `String` |  |
| `delivery_attempts` | `i32` |  |
| `created_at` | `UniversalTimestamp` |  |
| `delivered_at` | `Option < UniversalTimestamp >` |  |
| `acked_at` | `Option < UniversalTimestamp >` |  |



### <span id="struct.NewUnifiedDeliveryOutbox"></span>`struct NewUnifiedDeliveryOutbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `recipient` | `String` |  |
| `kind` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `payload` | `UniversalBinary` |  |
| `delivery_state` | `String` |  |
| `delivery_attempts` | `i32` |  |
| `created_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedWsTicket"></span>`struct UnifiedWsTicket`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

Single-use WebSocket auth ticket row. DB-backed so a ticket minted on one server replica redeems on any other (no session affinity required).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `ticket` | `String` |  |
| `key_id` | `UniversalUuid` |  |
| `key_name` | `String` |  |
| `permissions` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `is_admin` | `UniversalBool` |  |
| `created_at` | `UniversalTimestamp` |  |
| `expires_at` | `UniversalTimestamp` |  |
| `redeemed_at` | `Option < UniversalTimestamp >` |  |



### <span id="struct.NewUnifiedWsTicket"></span>`struct NewUnifiedWsTicket`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `ticket` | `String` |  |
| `key_id` | `UniversalUuid` |  |
| `key_name` | `String` |  |
| `permissions` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `is_admin` | `UniversalBool` |  |
| `created_at` | `UniversalTimestamp` |  |
| `expires_at` | `UniversalTimestamp` |  |
| `redeemed_at` | `Option < UniversalTimestamp >` |  |



### <span id="struct.UnifiedLoginThrottle"></span>`struct UnifiedLoginThrottle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

One failed-login counter. `throttle_key` is opaque and carries its own scope prefix (`u:<tenant-or-_>/<username>` or `ip:<source>`) so the two scopes share a table without colliding.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `throttle_key` | `String` |  |
| `scope` | `String` |  |
| `failure_count` | `i32` |  |
| `first_failure_at` | `UniversalTimestamp` |  |
| `last_failure_at` | `UniversalTimestamp` |  |
| `locked_until` | `Option < UniversalTimestamp >` |  |



### <span id="struct.NewUnifiedLoginThrottle"></span>`struct NewUnifiedLoginThrottle`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Insertable`, `AsChangeset`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `throttle_key` | `String` |  |
| `scope` | `String` |  |
| `failure_count` | `i32` |  |
| `first_failure_at` | `UniversalTimestamp` |  |
| `last_failure_at` | `UniversalTimestamp` |  |
| `locked_until` | `Option < UniversalTimestamp >` |  |



### <span id="struct.UnifiedFleetAgent"></span>`struct UnifiedFleetAgent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

Execution-agent fleet roster row: register/heartbeat state persisted so selection, capacity views and dead-agent reclaim are multi-replica correct. `capabilities` is a JSON string array.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `agent_id` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `target_triple` | `String` |  |
| `capabilities` | `String` |  |
| `max_concurrency` | `i32` |  |
| `in_flight` | `i32` |  |
| `available_capacity` | `i32` |  |
| `registered_at` | `UniversalTimestamp` |  |
| `last_heartbeat_at` | `UniversalTimestamp` |  |



### <span id="struct.NewUnifiedFleetAgent"></span>`struct NewUnifiedFleetAgent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Insertable`, `AsChangeset`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `agent_id` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `target_triple` | `String` |  |
| `capabilities` | `String` |  |
| `max_concurrency` | `i32` |  |
| `in_flight` | `i32` |  |
| `available_capacity` | `i32` |  |
| `registered_at` | `UniversalTimestamp` |  |
| `last_heartbeat_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedSchedule"></span>`struct UnifiedSchedule`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `schedule_type` | `String` |  |
| `workflow_name` | `String` |  |
| `enabled` | `UniversalBool` |  |
| `cron_expression` | `Option < String >` |  |
| `timezone` | `Option < String >` |  |
| `catchup_policy` | `Option < String >` |  |
| `start_date` | `Option < UniversalTimestamp >` |  |
| `end_date` | `Option < UniversalTimestamp >` |  |
| `trigger_name` | `Option < String >` |  |
| `poll_interval_ms` | `Option < i32 >` |  |
| `allow_concurrent` | `Option < UniversalBool >` |  |
| `next_run_at` | `Option < UniversalTimestamp >` |  |
| `last_run_at` | `Option < UniversalTimestamp >` |  |
| `last_poll_at` | `Option < UniversalTimestamp >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `paused` | `UniversalBool` | Transient operator pause (CLOACI-T-0749). Distinct from `enabled`: a
paused schedule is not fired by the scheduler but is otherwise intact. |
| `paused_at` | `Option < UniversalTimestamp >` |  |
| `params` | `Option < String >` | CLOACI-I-0116: fully-resolved bound instance params (JSON object) +
human instance name; both None for anonymous schedules. |
| `instance_name` | `Option < String >` |  |



### <span id="struct.NewUnifiedSchedule"></span>`struct NewUnifiedSchedule`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `schedule_type` | `String` |  |
| `workflow_name` | `String` |  |
| `enabled` | `UniversalBool` |  |
| `cron_expression` | `Option < String >` |  |
| `timezone` | `Option < String >` |  |
| `catchup_policy` | `Option < String >` |  |
| `start_date` | `Option < UniversalTimestamp >` |  |
| `end_date` | `Option < UniversalTimestamp >` |  |
| `trigger_name` | `Option < String >` |  |
| `poll_interval_ms` | `Option < i32 >` |  |
| `allow_concurrent` | `Option < UniversalBool >` |  |
| `next_run_at` | `Option < UniversalTimestamp >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `params` | `Option < String >` |  |
| `instance_name` | `Option < String >` |  |



### <span id="struct.UnifiedScheduleExecution"></span>`struct UnifiedScheduleExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `schedule_id` | `UniversalUuid` |  |
| `workflow_execution_id` | `Option < UniversalUuid >` |  |
| `scheduled_time` | `Option < UniversalTimestamp >` |  |
| `claimed_at` | `Option < UniversalTimestamp >` |  |
| `context_hash` | `Option < String >` |  |
| `started_at` | `UniversalTimestamp` |  |
| `completed_at` | `Option < UniversalTimestamp >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `recovery_attempts` | `i32` | CLOACI-T-0926: durable recovery attempt count for this handoff. |
| `recovery_claimed_by` | `Option < UniversalUuid >` | CLOACI-T-0926: recovery service that owns the re-fire of this handoff. |
| `recovery_heartbeat_at` | `Option < UniversalTimestamp >` | CLOACI-T-0926: last liveness beat from the claim owner. |



### <span id="struct.NewUnifiedScheduleExecution"></span>`struct NewUnifiedScheduleExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `schedule_id` | `UniversalUuid` |  |
| `workflow_execution_id` | `Option < UniversalUuid >` |  |
| `scheduled_time` | `Option < UniversalTimestamp >` |  |
| `claimed_at` | `Option < UniversalTimestamp >` |  |
| `context_hash` | `Option < String >` |  |
| `started_at` | `UniversalTimestamp` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedWorkflowRegistryEntry"></span>`struct UnifiedWorkflowRegistryEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `created_at` | `UniversalTimestamp` |  |
| `data` | `UniversalBinary` |  |



### <span id="struct.NewUnifiedWorkflowRegistryEntry"></span>`struct NewUnifiedWorkflowRegistryEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `created_at` | `UniversalTimestamp` |  |
| `data` | `UniversalBinary` |  |



### <span id="struct.UnifiedWorkflowPackage"></span>`struct UnifiedWorkflowPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `registry_id` | `UniversalUuid` |  |
| `package_name` | `String` |  |
| `version` | `String` |  |
| `description` | `Option < String >` |  |
| `author` | `Option < String >` |  |
| `metadata` | `String` |  |
| `storage_type` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `tenant_id` | `Option < String >` |  |
| `content_hash` | `String` |  |
| `superseded` | `UniversalBool` |  |
| `compiled_data` | `Option < UniversalBinary >` |  |
| `build_status` | `String` |  |
| `build_error` | `Option < String >` |  |
| `build_claimed_at` | `Option < UniversalTimestamp >` |  |
| `compiled_at` | `Option < UniversalTimestamp >` |  |
| `paused` | `UniversalBool` | Transient operator pause (CLOACI-T-0749): blocks new executions of this
workflow regardless of source. In-flight executions are unaffected. |
| `paused_at` | `Option < UniversalTimestamp >` |  |



### <span id="struct.NewUnifiedWorkflowPackage"></span>`struct NewUnifiedWorkflowPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `registry_id` | `UniversalUuid` |  |
| `package_name` | `String` |  |
| `version` | `String` |  |
| `description` | `Option < String >` |  |
| `author` | `Option < String >` |  |
| `metadata` | `String` |  |
| `storage_type` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `tenant_id` | `Option < String >` |  |
| `content_hash` | `String` |  |
| `superseded` | `UniversalBool` |  |
| `compiled_data` | `Option < UniversalBinary >` |  |
| `build_status` | `String` |  |
| `build_error` | `Option < String >` |  |
| `build_claimed_at` | `Option < UniversalTimestamp >` |  |
| `compiled_at` | `Option < UniversalTimestamp >` |  |



### <span id="struct.PackageArtifact"></span>`struct PackageArtifact`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `package_name` | `String` |  |
| `version` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `target_triple` | `String` |  |
| `content_hash` | `String` |  |
| `compiled_data` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |



### <span id="struct.NewPackageArtifact"></span>`struct NewPackageArtifact`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `package_name` | `String` |  |
| `version` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `target_triple` | `String` |  |
| `content_hash` | `String` |  |
| `compiled_data` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |



### <span id="struct.PackageProvider"></span>`struct PackageProvider`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `package_name` | `String` |  |
| `version` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `provider_name` | `String` |  |
| `provider_version` | `String` |  |
| `content_hash` | `String` |  |
| `provider_data` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |
| `target_triple` | `Option < String >` | CLOACI-T-0908: `None` = the primary build (arch-neutral for wasm, the
compiler host's arch for native); `Some(triple)` = a per-arch NATIVE build. |
| `runtime` | `String` | `"wasm"` (arch-neutral) or `"native"` (per-arch host cdylib). |



### <span id="struct.NewPackageProvider"></span>`struct NewPackageProvider`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `package_name` | `String` |  |
| `version` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `provider_name` | `String` |  |
| `provider_version` | `String` |  |
| `content_hash` | `String` |  |
| `provider_data` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |
| `target_triple` | `Option < String >` |  |
| `runtime` | `String` |  |



### <span id="struct.UnifiedSigningKey"></span>`struct UnifiedSigningKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `key_name` | `String` |  |
| `encrypted_private_key` | `UniversalBinary` |  |
| `public_key` | `UniversalBinary` |  |
| `key_fingerprint` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option < UniversalTimestamp >` |  |



### <span id="struct.NewUnifiedSigningKey"></span>`struct NewUnifiedSigningKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `key_name` | `String` |  |
| `encrypted_private_key` | `UniversalBinary` |  |
| `public_key` | `UniversalBinary` |  |
| `key_fingerprint` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |



### <span id="struct.TenantDataKey"></span>`struct TenantDataKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

Per-tenant data key (DEK) wrapped by the server KEK (envelope encryption, D-7).

`wrapped_dek` is the 32-byte DEK encrypted under the server KEK via
AES-256-GCM (`nonce || ciphertext || tag`). It is only ever unwrapped
server-side; the plaintext DEK never leaves memory.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `wrapped_dek` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |



### <span id="struct.NewTenantDataKey"></span>`struct NewTenantDataKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `wrapped_dek` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |



### <span id="struct.Secret"></span>`struct Secret`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

An encrypted, tenant-scoped named-field secret.

`field_names` is plaintext metadata (a JSON array of the field names only —
never the values). `encrypted_fields` is the `{field: value}` JSON encrypted
under the tenant DEK (`nonce || ciphertext || tag`). Plaintext field values
exist only transiently in memory during an internal resolve.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `name` | `String` |  |
| `field_names` | `String` |  |
| `encrypted_fields` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.NewSecret"></span>`struct NewSecret`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `name` | `String` |  |
| `field_names` | `String` |  |
| `encrypted_fields` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedTrustedKey"></span>`struct UnifiedTrustedKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `key_fingerprint` | `String` |  |
| `public_key` | `UniversalBinary` |  |
| `key_name` | `Option < String >` |  |
| `trusted_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option < UniversalTimestamp >` |  |



### <span id="struct.NewUnifiedTrustedKey"></span>`struct NewUnifiedTrustedKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `key_fingerprint` | `String` |  |
| `public_key` | `UniversalBinary` |  |
| `key_name` | `Option < String >` |  |
| `trusted_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedKeyTrustAcl"></span>`struct UnifiedKeyTrustAcl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `parent_org_id` | `UniversalUuid` |  |
| `child_org_id` | `UniversalUuid` |  |
| `granted_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option < UniversalTimestamp >` |  |



### <span id="struct.NewUnifiedKeyTrustAcl"></span>`struct NewUnifiedKeyTrustAcl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `parent_org_id` | `UniversalUuid` |  |
| `child_org_id` | `UniversalUuid` |  |
| `granted_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedPackageSignature"></span>`struct UnifiedPackageSignature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `package_hash` | `String` |  |
| `key_fingerprint` | `String` |  |
| `signature` | `UniversalBinary` |  |
| `signed_at` | `UniversalTimestamp` |  |
| `org_id` | `Option < UniversalUuid >` |  |



### <span id="struct.NewUnifiedPackageSignature"></span>`struct NewUnifiedPackageSignature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `package_hash` | `String` |  |
| `key_fingerprint` | `String` |  |
| `signature` | `UniversalBinary` |  |
| `signed_at` | `UniversalTimestamp` |  |
| `org_id` | `Option < UniversalUuid >` |  |



### <span id="struct.UnifiedAccumulatorCheckpoint"></span>`struct UnifiedAccumulatorCheckpoint`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `graph_name` | `String` |  |
| `accumulator_name` | `String` |  |
| `checkpoint_data` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.NewUnifiedAccumulatorCheckpoint"></span>`struct NewUnifiedAccumulatorCheckpoint`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `graph_name` | `String` |  |
| `accumulator_name` | `String` |  |
| `checkpoint_data` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedAccumulatorBoundary"></span>`struct UnifiedAccumulatorBoundary`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `graph_name` | `String` |  |
| `accumulator_name` | `String` |  |
| `boundary_data` | `UniversalBinary` |  |
| `sequence_number` | `i64` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.NewUnifiedAccumulatorBoundary"></span>`struct NewUnifiedAccumulatorBoundary`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `graph_name` | `String` |  |
| `accumulator_name` | `String` |  |
| `boundary_data` | `UniversalBinary` |  |
| `sequence_number` | `i64` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedReactorState"></span>`struct UnifiedReactorState`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `graph_name` | `String` |  |
| `cache_data` | `UniversalBinary` |  |
| `dirty_flags` | `UniversalBinary` |  |
| `sequential_queue` | `Option < UniversalBinary >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.NewUnifiedReactorState"></span>`struct NewUnifiedReactorState`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `graph_name` | `String` |  |
| `cache_data` | `UniversalBinary` |  |
| `dirty_flags` | `UniversalBinary` |  |
| `sequential_queue` | `Option < UniversalBinary >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedStateAccumulatorBuffer"></span>`struct UnifiedStateAccumulatorBuffer`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `graph_name` | `String` |  |
| `accumulator_name` | `String` |  |
| `buffer_data` | `UniversalBinary` |  |
| `capacity` | `i32` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.NewUnifiedStateAccumulatorBuffer"></span>`struct NewUnifiedStateAccumulatorBuffer`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `graph_name` | `String` |  |
| `accumulator_name` | `String` |  |
| `buffer_data` | `UniversalBinary` |  |
| `capacity` | `i32` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.UnifiedReactorOwnerAddress"></span>`struct UnifiedReactorOwnerAddress`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

Where to reach the replica that owns a reactor. A ROUTING HINT ONLY — the advisory lock in `computation_graph::reactor_lock_key` is the sole authority on ownership; a stale row here may cost a wasted redirect, never a wrong ownership decision.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option < String >` |  |
| `reactor_name` | `String` |  |
| `address` | `String` |  |
| `claimed_at` | `UniversalTimestamp` |  |



### <span id="struct.NewUnifiedReactorOwnerAddress"></span>`struct NewUnifiedReactorOwnerAddress`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option < String >` |  |
| `reactor_name` | `String` |  |
| `address` | `String` |  |
| `claimed_at` | `UniversalTimestamp` |  |



