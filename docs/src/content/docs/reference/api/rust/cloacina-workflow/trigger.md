---
title: "cloacina-workflow::trigger"
description: "Trigger types for workflow authoring."
sidebar:
  order: 54
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Trigger types for workflow authoring.

T-0552 (I-0102 follow-up) relocated the `Trigger` trait from `cloacina`
(engine-only) into this leaf crate so packaged cdylibs can collect
`TriggerEntry` inventory entries (which hold `Arc<dyn Trigger>`) at
link time, and the unified `cloacina::package!()` shell macro can walk
them at FFI call time. Engine paths re-export `cloacina_workflow::Trigger`.

## Enums

### <span id="enum.TriggerResult"></span>`enum TriggerResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Result of a trigger poll operation.

#### Variants

- **`Skip`** - Do not fire the workflow, continue polling on the next interval.
- **`Fire`** - Fire the workflow with an optional context.



### <span id="enum.TriggerError"></span>`enum TriggerError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors that can occur during trigger polling.

#### Variants

- **`PollError`** - Error during trigger polling
- **`ContextError`** - Context creation error



