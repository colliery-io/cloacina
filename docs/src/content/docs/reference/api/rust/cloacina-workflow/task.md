---
title: "cloacina-workflow::task"
description: "API reference of the Rust module cloacina-workflow::task."
sidebar:
  order: 53
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>




## Enums

### <span id="enum.TaskState"></span>`enum TaskState`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Represents the execution state of a task throughout its lifecycle.

Tasks progress through these states during execution, providing visibility
into the current status and enabling proper error handling and recovery.

#### Variants

- **`Pending`** - Task is registered but not yet started
- **`Running`** - Task is currently executing
- **`Completed`** - Task finished successfully
- **`Failed`** - Task encountered an error
- **`Skipped`** - Task was skipped (e.g., trigger rule not satisfied)



