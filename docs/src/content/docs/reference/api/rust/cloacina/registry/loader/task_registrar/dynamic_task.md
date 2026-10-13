---
title: "cloacina::registry::loader::task_registrar::dynamic_task"
description: "Dynamic library task implementation using fidius-host for task execution."
editUrl: false
sidebar:
  order: 164
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Dynamic library task implementation using fidius-host for task execution.

The plugin library is loaded once during package registration and the handle
is shared across all task instances from that package. No per-execution
temp files or dlopen/dlclose cycles.
