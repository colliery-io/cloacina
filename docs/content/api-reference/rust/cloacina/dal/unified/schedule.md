# cloacina::dal::unified::schedule <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Schedule DAL with runtime backend selection

This module provides operations for the unified `schedules` table that
replaces the separate `cron_schedules` and `trigger_schedules` tables.
Works with both PostgreSQL and SQLite backends, selecting the appropriate
implementation at runtime based on the database connection type.

## Structs

### `cloacina::dal::unified::schedule::ScheduleDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for unified schedule operations with runtime backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
