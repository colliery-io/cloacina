---
title: "cloacina-python::gil"
description: "The one GIL-safe async→sync bridge for the Python bindings (CLOACI-I-0136)."
editUrl: false
sidebar:
  order: 15
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

The one GIL-safe async→sync bridge for the Python bindings (CLOACI-I-0136).

Every sync PyO3 method that must drive an async Rust future to completion
calls `py_block_on` — the single auditable place the GIL-deadlock invariant
lives. Do NOT hand-roll `Runtime::new().block_on(...)` or `Handle::current()
.block_on(...)` at a call site: doing so risks holding the GIL across the
await, the exact footgun behind the scenario 30/32/33 PyO3↔tokio deadlocks.

NOTE ON SCOPE: this is for a ONE-SHOT future resolved from a GIL-holding sync
method. `bindings/runner.rs` is deliberately NOT routed through here — its
`run_event_loop` is a long-running actor loop on a dedicated OS thread that
does not hold the GIL, so the "release the GIL then block" contract is
meaningless there.
