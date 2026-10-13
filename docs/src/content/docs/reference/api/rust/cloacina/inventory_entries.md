---
title: "cloacina::inventory_entries"
description: "Inventory entry types for linker-collected registry seeding."
editUrl: false
sidebar:
  order: 131
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

## Re-exports

- `ComputationGraphEntry` (`cloacina_workflow_plugin::ComputationGraphEntry`)
- `ReactorEntry` (`cloacina_workflow_plugin::ReactorEntry`)
- `TaskEntry` (`cloacina_workflow_plugin::TaskEntry`)
- `TriggerEntry` (`cloacina_workflow_plugin::TriggerEntry`)
- `TriggerlessGraphEntry` (`cloacina_workflow_plugin::TriggerlessGraphEntry`)

## Structs

### <span id="struct.WorkflowEntry"></span><code style="font-size: 0.85em">struct WorkflowEntry</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Workflow entry emitted by `#[workflow]`. Stays in cloacina because `Workflow` is an engine-only runtime type.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `&'static str` |  |
| `constructor` | `fn() -> Workflow` |  |

### <span id="struct.StreamBackendEntry"></span><code style="font-size: 0.85em">struct StreamBackendEntry</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `type_name` | `&'static str` |  |
| `factory` | `StreamBackendFactoryFn` |  |

## Type aliases

| Name | Type | Description |
|------|------|-------------|
| <span id="type.StreamBackendFactoryFn"></span>`StreamBackendFactoryFn` | `fn(StreamConfig) -> StreamBackendFuture` | Stream-backend entry emitted by the stream-backend registration helper.<br>The factory is a function pointer that takes an owned `StreamConfig` and returns a heap-allocated future; at seed time the runtime wraps the pointer into a `Box<dyn Fn(StreamConfig) -> Pin<Box<Future<..>>> + Send + Sync>` to match the shape of dynamically-registered backends. |
