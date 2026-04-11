# cloacina::dal::unified::task_outbox <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Task Outbox DAL with runtime backend selection

This module provides operations for the task outbox, which is used for
work distribution. The outbox is transient - entries are deleted immediately
when workers claim tasks.
Note: The primary outbox insertion happens in `mark_ready()` within the same
transaction as the status update. This DAL provides additional operations
for claiming and cleanup.

## Structs

### `cloacina::dal::unified::task_outbox::TaskOutboxDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for task outbox operations with runtime backend selection.

The outbox provides reliable work distribution by:
1. Inserting entries atomically with task status updates
2. Enabling push notifications (Postgres LISTEN/NOTIFY)
3. Supporting polling for SQLite
4. Deleting entries when tasks are claimed

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
