---
title: "cloacina::inventory_entries"
description: "Inventory entry types for linker-collected registry seeding."
sidebar:
  order: 148
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Inventory entry types for linker-collected registry seeding.

The macros (`#[task]`, `#[workflow]`, `#[trigger]`, `#[computation_graph]`,
and the stream-backend registration helper) emit
`inventory::submit!` statements of these types instead of `#[ctor]`
constructors. The runtime reads them post-`main()` via `inventory::iter`,
eliminating the initialization-ordering bug that sank I-0095.
Function pointers — not `Box<dyn Fn>` — are used because `inventory` stores
entries in a linker section with `'static` + `Sized` bounds. Zero-capture
closures at the macro call site coerce to `fn` pointers automatically, so
the ergonomics stay identical.
Nothing in this file reads inventory yet. That wiring lands in T-0506
together with the removal of the global static registries.

## Structs

### <span id="struct.WorkflowEntry"></span>`struct WorkflowEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Workflow entry emitted by `#[workflow]`. Stays in cloacina because `Workflow` is an engine-only runtime type.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `& 'static str` |  |
| `constructor` | `fn () -> Workflow` |  |



### <span id="struct.StreamBackendEntry"></span>`struct StreamBackendEntry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


#### Fields

| Name | Type | Description |
|------|------|-------------|
| `type_name` | `& 'static str` |  |
| `factory` | `StreamBackendFactoryFn` |  |



