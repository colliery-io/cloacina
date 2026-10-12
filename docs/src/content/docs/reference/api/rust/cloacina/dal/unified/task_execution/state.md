---
title: "cloacina::dal::unified::task_execution::state"
description: "State transition operations for task executions."
sidebar:
  order: 107
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


State transition operations for task executions.

All state transitions are transactional: the status update and execution event
are written atomically. If either fails, both are rolled back.

