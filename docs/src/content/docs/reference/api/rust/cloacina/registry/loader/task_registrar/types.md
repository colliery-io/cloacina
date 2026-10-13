---
title: "cloacina::registry::loader::task_registrar::types"
description: "Types for task metadata exchange with dynamic libraries."
editUrl: false
sidebar:
  order: 166
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Types for task metadata exchange with dynamic libraries.

The raw FFI structs have been removed — fidius-host handles all serialization
and FFI safety transparently. Only the owned (post-extraction) types remain.

## Structs

### <span id="struct.OwnedTaskMetadata"></span><code style="font-size: 0.85em">struct OwnedTaskMetadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Owned task metadata — safe to use after library is unloaded.

All fields are owned `String` values; no raw pointers are involved.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `local_id` | `String` | Local task ID (e.g., "collect_data") |
| `dependencies_json` | `String` | JSON string of task dependencies |
| `trigger_rules_json` | `String` | JSON string of the task's trigger rules (conditional execution). `{"type":"Always"}` when unconditional. (CLOACI-T-0721) |
| `requires_handle` | `bool` | Whether the task takes a `TaskHandle` parameter (CLOACI-T-0897). Gates the executor's entire handle path; `false` for packages built before the field existed, which is correct — they could not use one. |

### <span id="struct.OwnedTaskMetadataCollection"></span><code style="font-size: 0.85em">struct OwnedTaskMetadataCollection</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Owned collection of task metadata — safe to use after library is unloaded.

All fields are owned `String` values; no raw pointers are involved.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `String` | Name of the workflow (e.g., "data_processing") |
| `package_name` | `String` | Name of the package (e.g., "simple_demo") |
| `tasks` | `Vec<OwnedTaskMetadata>` | Owned task metadata for each task in the package |
