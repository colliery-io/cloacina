---
title: "cloacina::workflow::registry"
description: "Workflow registry types."
editUrl: false
sidebar:
  order: 206
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Workflow registry types.

The process-global workflow registry was removed in CLOACI-T-0509. Workflow
constructors are now owned by [`crate::Runtime`](../../../../rust/cloacina/runtime/#struct.Runtime), which is seeded from the
`inventory` entries emitted by the `#[workflow]` macro and then mutated
dynamically by the reconciler when packages are loaded/unloaded.

## Type aliases

| Name | Type | Description |
|------|------|-------------|
| <span id="type.WorkflowConstructor"></span>`WorkflowConstructor` | `Box<dyn Fn() -> Workflow + Send + Sync>` | Type alias for the workflow constructor function. |
