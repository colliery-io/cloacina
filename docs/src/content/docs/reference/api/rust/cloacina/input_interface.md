---
title: "cloacina::input_interface"
description: "Injectable input interface (CLOACI-I-0128) — re-export of the canonical helpers."
editUrl: false
sidebar:
  order: 130
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Injectable input interface (CLOACI-I-0128) — re-export of the canonical helpers.

The implementation lives in `cloacina-workflow` (not here) because the
`#[workflow(params(...))]` macro emits calls to `schema_for` into **packaged
cdylibs**, which depend on `cloacina-workflow`, not core `cloacina`. Core +
server code reach the same helpers through this re-export.

Spec: [CLOACI-S-0013]; descriptor decision: [CLOACI-A-0007].

## Re-exports

- `InputSlot` (`cloacina_workflow::input_interface::InputSlot`)
- [`ProbeFallback`](../../../rust/cloacina-workflow/input_interface/#trait.ProbeFallback)
- [`ProbeTyped`](../../../rust/cloacina-workflow/input_interface/#trait.ProbeTyped)
- [`SchemaProbe`](../../../rust/cloacina-workflow/input_interface/#struct.SchemaProbe)
- [`default_json`](../../../rust/cloacina-workflow/input_interface/#fn.default_json)
- [`schema_for`](../../../rust/cloacina-workflow/input_interface/#fn.schema_for)
- [`slots_to_json`](../../../rust/cloacina-workflow/input_interface/#fn.slots_to_json)
