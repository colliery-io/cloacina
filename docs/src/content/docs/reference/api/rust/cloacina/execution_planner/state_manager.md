---
title: "cloacina::execution_planner::state_manager"
description: "Task state management and dependency checking."
sidebar:
  order: 132
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Task state management and dependency checking.

This module handles checking task dependencies and updating task readiness
based on dependency states and trigger rules. Dispatch of Ready tasks is
handled separately by the scheduler loop.

## Structs

### <span id="struct.StateManager"></span>`struct StateManager<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


State management operations for the scheduler.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
| `runtime` | `Arc < Runtime >` |  |



