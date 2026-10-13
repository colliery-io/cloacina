---
title: "cloacina::dal::unified::task_execution::claiming"
description: "Task claiming and retry scheduling operations."
editUrl: false
sidebar:
  order: 87
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Task claiming and retry scheduling operations.

All operations are transactional: state changes and execution events
are written atomically. If either fails, both are rolled back.
