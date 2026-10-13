---
title: "cloacina::executor::host_impl"
description: "The engine's implementation of the `CloacinaHost` callback interface (CLOACI-T-0897) — what a packaged task's `defer_until` actually drives."
editUrl: false
sidebar:
  order: 120
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

The engine's implementation of the `CloacinaHost` callback interface (CLOACI-T-0897) — what a packaged task's `defer_until` actually drives.

**Why blocking here is safe**

These methods are synchronous (fidius's boundary is sync by design) while
the operations behind them are async, so each bridges with
`Handle::block_on`. That is safe *specifically* because of where the call
lands: the host invokes packaged tasks through
`tokio::task::spawn_blocking` (`registry/loader/task_registrar/dynamic_task.rs`),
so a callback runs on the BLOCKING pool, never on a runtime worker. Runtime
workers keep draining, other tasks keep completing, and the slot this call
is waiting for keeps being freed.

An earlier reading of this had it backwards and predicted a self-deadlock;
see the ticket for the retraction. The property to preserve is the one that
makes it safe: **never call these from a runtime worker.**

Residual cost, accepted and documented: a deferred task holds its
blocking-pool thread for the whole wait. It has released its concurrency
slot, so it is not blocking other work — it is just parked.

## Structs

### <span id="struct.EngineHost"></span><code style="font-size: 0.85em">struct EngineHost</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Services `defer_until` callbacks from packaged tasks.

Stateless: everything it needs is looked up per call from the deferral
registry, keyed by the task-execution id the plugin sends. That keeps one
bound instance correct for every tenant and every concurrently running task.
