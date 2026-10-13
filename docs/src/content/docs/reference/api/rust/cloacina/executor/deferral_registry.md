---
title: "cloacina::executor::deferral_registry"
description: "Slot access for packaged tasks that defer (CLOACI-T-0897)."
editUrl: false
sidebar:
  order: 119
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Slot access for packaged tasks that defer (CLOACI-T-0897).

An embedded task reaches its concurrency slot through the [`TaskHandle`](../../../../rust/cloacina/executor/task_handle/#struct.TaskHandle)
the executor installs in a task-local. A PACKAGED task cannot: its
`defer_until` runs inside a cdylib and comes back through the
`CloacinaHost` callback channel, which lands on a tokio blocking-pool
thread (the host invokes packaged tasks via `spawn_blocking`) where that
task-local is invisible.

So the executor also registers the handle's slot here, keyed by the
task-execution UUID the callback carries. It is the SAME `Arc`, not a copy
— there is exactly one `SlotToken` per running task, whichever door reaches
it.

Entries are removed when the task finishes. A packaged task that never
defers simply never looks its entry up.
