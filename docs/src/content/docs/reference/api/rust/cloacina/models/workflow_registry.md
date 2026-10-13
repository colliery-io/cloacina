---
title: "cloacina::models::workflow_registry"
description: "Database models for workflow registry storage."
editUrl: false
sidebar:
  order: 147
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Database models for workflow registry storage.

This module defines domain structures for workflow registry storage.
These are API-level types; backend-specific models handle database storage.

## Structs

### <span id="struct.WorkflowRegistryEntry"></span><code style="font-size: 0.85em">struct WorkflowRegistryEntry</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Domain model for a workflow registry entry.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `created_at` | `UniversalTimestamp` |  |
| `data` | `Vec<u8>` |  |

### <span id="struct.NewWorkflowRegistryEntry"></span><code style="font-size: 0.85em">struct NewWorkflowRegistryEntry</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating new workflow registry entries (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `data` | `Vec<u8>` |  |

#### Methods

##### <span id="NewWorkflowRegistryEntry.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(data: Vec<u8>) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
```

</details>

### <span id="struct.NewWorkflowRegistryEntryWithId"></span><code style="font-size: 0.85em">struct NewWorkflowRegistryEntryWithId</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating new workflow registry entries with explicit ID and timestamp.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `created_at` | `UniversalTimestamp` |  |
| `data` | `Vec<u8>` |  |
