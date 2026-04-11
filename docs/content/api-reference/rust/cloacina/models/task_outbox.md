# cloacina::models::task_outbox <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task Outbox Model

This module defines domain structures for the task outbox, which is used for
work distribution. The outbox is a transient table - rows are deleted immediately
upon claiming by workers.
The outbox pattern provides:
- Reliable work distribution signaling
- Push notifications (Postgres LISTEN/NOTIFY) without polling
- Atomic task ready state + notification (single transaction)

## Structs

### `cloacina::models::task_outbox::TaskOutbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a task outbox entry (domain type).

The outbox is transient: entries are created when tasks become ready and
deleted when workers claim them. This provides a reliable work queue that
can be used with push notifications (LISTEN/NOTIFY on Postgres) or polling.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `i64` | Auto-incrementing primary key (BIGSERIAL) |
| `task_execution_id` | `UniversalUuid` | The task execution that is ready for processing |
| `created_at` | `UniversalTimestamp` | When the outbox entry was created |



### `cloacina::models::task_outbox::NewTaskOutbox`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new task outbox entries (domain type).

Only the task_execution_id is required; created_at is set automatically.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_execution_id` | `UniversalUuid` | The task execution that is ready for processing |
