---
title: "cloacina::computation_graph::triggerless"
description: "Trigger-less computation graph registration."
editUrl: false
sidebar:
  order: 52
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Trigger-less computation graph registration.

T-0552 (I-0102 follow-up) relocated `TriggerlessGraphFn`,
`TriggerlessGraphRegistration`, and the `TriggerlessGraph` trait into
`cloacina-workflow-plugin` so packaged cdylibs can collect
`TriggerlessGraphEntry` inventory entries at link time. Engine paths
re-export.

## Re-exports

- `TriggerlessGraph` (`cloacina_workflow_plugin::TriggerlessGraph`)
- `TriggerlessGraphFn` (`cloacina_workflow_plugin::TriggerlessGraphFn`)
- `TriggerlessGraphRegistration` (`cloacina_workflow_plugin::TriggerlessGraphRegistration`)
