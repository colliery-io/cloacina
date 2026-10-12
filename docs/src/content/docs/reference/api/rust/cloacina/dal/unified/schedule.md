---
title: "cloacina::dal::unified::schedule"
description: "Unified Schedule DAL with runtime backend selection"
sidebar:
  order: 99
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Unified Schedule DAL with runtime backend selection

This module provides operations for the unified `schedules` table that
replaces the separate `cron_schedules` and `trigger_schedules` tables.
Works with both PostgreSQL and SQLite backends, selecting the appropriate
implementation at runtime based on the database connection type.

## Structs

### <span id="struct.ScheduleDAL"></span>`struct ScheduleDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Data access layer for unified schedule operations with runtime backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



