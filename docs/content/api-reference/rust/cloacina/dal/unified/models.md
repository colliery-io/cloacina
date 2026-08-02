# cloacina::dal::unified::models <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified database models using custom SQL types

These models use the unified schema with DbUuid, DbTimestamp, DbBool custom
SQL types that work with both PostgreSQL and SQLite backends.

## Structs

### `cloacina::dal::unified::models::UnifiedDbContext`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

Unified context model that works with both PostgreSQL and SQLite.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `value` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### `cloacina::dal::unified::models::NewUnifiedDbContext`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Insertable`

Insertable context with explicit ID and timestamps (for SQLite compatibility).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `value` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### `cloacina::dal::unified::models::UnifiedWorkflowExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedWorkflowExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedTaskExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedTaskExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedTaskExecutionMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedTaskExecutionMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedRecoveryEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedRecoveryEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedExecutionEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedExecutionEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedTaskOutbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

Unified task outbox model for work distribution. Transient: rows are deleted immediately upon claiming.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `i64` |  |
| `task_execution_id` | `UniversalUuid` |  |
| `created_at` | `UniversalTimestamp` |  |



### `cloacina::dal::unified::models::NewUnifiedTaskOutbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_execution_id` | `UniversalUuid` |  |
| `created_at` | `UniversalTimestamp` |  |



### `cloacina::dal::unified::models::UnifiedDeliveryOutbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedDeliveryOutbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedSchedule`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedSchedule`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedScheduleExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedScheduleExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedWorkflowRegistryEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `created_at` | `UniversalTimestamp` |  |
| `data` | `UniversalBinary` |  |



### `cloacina::dal::unified::models::NewUnifiedWorkflowRegistryEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `created_at` | `UniversalTimestamp` |  |
| `data` | `UniversalBinary` |  |



### `cloacina::dal::unified::models::UnifiedWorkflowPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedWorkflowPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::PackageArtifact`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewPackageArtifact`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::PackageProvider`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewPackageProvider`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedSigningKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedSigningKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::TenantDataKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewTenantDataKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `wrapped_dek` | `UniversalBinary` |  |
| `created_at` | `UniversalTimestamp` |  |



### `cloacina::dal::unified::models::Secret`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewSecret`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedTrustedKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedTrustedKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedKeyTrustAcl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `parent_org_id` | `UniversalUuid` |  |
| `child_org_id` | `UniversalUuid` |  |
| `granted_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option < UniversalTimestamp >` |  |



### `cloacina::dal::unified::models::NewUnifiedKeyTrustAcl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `parent_org_id` | `UniversalUuid` |  |
| `child_org_id` | `UniversalUuid` |  |
| `granted_at` | `UniversalTimestamp` |  |



### `cloacina::dal::unified::models::UnifiedPackageSignature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedPackageSignature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedAccumulatorCheckpoint`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedAccumulatorCheckpoint`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedAccumulatorBoundary`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedAccumulatorBoundary`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedReactorState`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedReactorState`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::UnifiedStateAccumulatorBuffer`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::models::NewUnifiedStateAccumulatorBuffer`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
