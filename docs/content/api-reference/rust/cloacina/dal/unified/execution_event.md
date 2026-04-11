# cloacina::dal::unified::execution_event <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Execution Event DAL with runtime backend selection

This module provides CRUD operations for ExecutionEvent entities that work with
both PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.
Execution events form an append-only audit trail of all task and pipeline
state transitions for debugging, compliance, and replay capability.

## Structs

### `cloacina::dal::unified::execution_event::ExecutionEventDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for execution event operations with runtime backend selection.

This DAL provides methods for creating and querying execution events,
which track all state transitions for tasks and pipelines.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
