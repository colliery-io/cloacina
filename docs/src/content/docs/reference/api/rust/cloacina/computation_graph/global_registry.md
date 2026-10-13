---
title: "cloacina::computation_graph::global_registry"
description: "Computation graph constructor types."
editUrl: false
sidebar:
  order: 42
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Computation graph constructor types.

The process-global computation-graph registry was removed in
CLOACI-T-0509. Constructors are now owned by [`crate::Runtime`](../../../../rust/cloacina/runtime/#struct.Runtime).

## Re-exports

- [`CompiledGraphFn`](../../../../rust/cloacina-computation-graph/#type.CompiledGraphFn)
- [`ComputationGraphConstructor`](../../../../rust/cloacina-computation-graph/#type.ComputationGraphConstructor)
- [`ComputationGraphRegistration`](../../../../rust/cloacina-computation-graph/#struct.ComputationGraphRegistration)
