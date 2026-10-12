---
title: "cloacina::runner::default_runner::services"
description: "Background service construction and registration for the DefaultRunner."
sidebar:
  order: 201
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Background service construction and registration for the DefaultRunner.

Each service is built here, wrapped in a [`BackgroundService`] adapter,
and registered with the runner's [`ServiceManager`]. The manager owns the
lifecycle from that point on.

