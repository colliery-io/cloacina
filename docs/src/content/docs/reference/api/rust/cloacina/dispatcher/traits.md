---
title: "cloacina::dispatcher::traits"
description: "Trait definitions for dispatcher and executor abstractions."
sidebar:
  order: 125
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Trait definitions for dispatcher and executor abstractions.

This module defines the core traits that enable pluggable executor backends.
Implementors can create custom executors (Kubernetes, serverless, message queues)
that integrate seamlessly with the scheduler.

