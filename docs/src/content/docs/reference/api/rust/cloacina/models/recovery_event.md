---
title: "cloacina::models::recovery_event"
description: "Recovery Event Model"
sidebar:
  order: 156
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Recovery Event Model

This module defines domain structures and types for tracking recovery events.
These are API-level types; backend-specific models handle database storage.

## Structs

### <span id="struct.RecoveryEvent"></span>`struct RecoveryEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a recovery event record (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_execution_id` | `Option < UniversalUuid >` |  |
| `recovery_type` | `String` |  |
| `recovered_at` | `UniversalTimestamp` |  |
| `details` | `Option < String >` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |



### <span id="struct.NewRecoveryEvent"></span>`struct NewRecoveryEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new recovery event records (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `workflow_execution_id` | `UniversalUuid` |  |
| `task_execution_id` | `Option < UniversalUuid >` |  |
| `recovery_type` | `String` |  |
| `details` | `Option < String >` |  |



## Enums

### <span id="enum.RecoveryType"></span>`enum RecoveryType`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Enumeration of possible recovery types in the system.

#### Variants

- **`TaskReset`**
- **`TaskAbandoned`**
- **`WorkflowFailed`**
- **`WorkflowUnavailable`**



