---
title: "cloacina::runner::default_runner::service_manager"
description: "Background service lifecycle management for `DefaultRunner`."
editUrl: false
sidebar:
  order: 182
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Background service lifecycle management for `DefaultRunner`.

Each long-running background loop owned by the runner is wrapped as a
`BackgroundService`. The `ServiceManager` owns the collection,
orchestrates `start_all()`/`shutdown_all()`, and provides typed slots so
external callers can still reach individual service Arcs (registry,
graph scheduler, unified scheduler, etc).
