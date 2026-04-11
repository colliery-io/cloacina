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
| `pipeline_name` | `String` |  |
| `pipeline_version` | `String` |  |
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



### `cloacina::dal::unified::models::NewUnifiedWorkflowExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `pipeline_name` | `String` |  |
| `pipeline_version` | `String` |  |
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
| `pipeline_execution_id` | `UniversalUuid` |  |
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
| `pipeline_execution_id` | `UniversalUuid` |  |
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
| `pipeline_execution_id` | `UniversalUuid` |  |
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
| `pipeline_execution_id` | `UniversalUuid` |  |
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
| `pipeline_execution_id` | `UniversalUuid` |  |
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
| `pipeline_execution_id` | `UniversalUuid` |  |
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
| `pipeline_execution_id` | `UniversalUuid` |  |
| `task_execution_id` | `Option < UniversalUuid >` |  |
| `event_type` | `String` |  |
| `event_data` | `Option < String >` |  |
| `worker_id` | `Option < String >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `sequence_num` | `i64` |  |



### `cloacina::dal::unified::models::NewUnifiedExecutionEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `pipeline_execution_id` | `UniversalUuid` |  |
| `task_execution_id` | `Option < UniversalUuid >` |  |
| `event_type` | `String` |  |
| `event_data` | `Option < String >` |  |
| `worker_id` | `Option < String >` |  |
| `created_at` | `UniversalTimestamp` |  |



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



### `cloacina::dal::unified::models::UnifiedScheduleExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Queryable`, `Selectable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `schedule_id` | `UniversalUuid` |  |
| `pipeline_execution_id` | `Option < UniversalUuid >` |  |
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
| `pipeline_execution_id` | `Option < UniversalUuid >` |  |
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
