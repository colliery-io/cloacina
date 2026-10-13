---
title: "cloacina::models::workflow_execution"
description: "Workflow Execution Models"
editUrl: false
sidebar:
  order: 145
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Workflow Execution Models

This module defines domain structures for tracking workflow executions.
These are API-level types; backend-specific models handle database storage.

## Structs

### <span id="struct.WorkflowExecutionRecord"></span><code style="font-size: 0.85em">struct WorkflowExecutionRecord</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a workflow execution record (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `workflow_name` | `String` |  |
| `workflow_version` | `String` |  |
| `status` | `String` |  |
| `context_id` | `Option<UniversalUuid>` |  |
| `started_at` | `UniversalTimestamp` |  |
| `completed_at` | `Option<UniversalTimestamp>` |  |
| `error_details` | `Option<String>` |  |
| `recovery_attempts` | `i32` |  |
| `last_recovery_at` | `Option<UniversalTimestamp>` |  |
| `paused_at` | `Option<UniversalTimestamp>` |  |
| `pause_reason` | `Option<String>` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `trigger_origin` | `Option<String>` | How this run was triggered (CLOACI-T-0776). `Some("manual")` for an operator run via the REST execute endpoint; `None` for cron/trigger/ reactor-driven runs (and rows created before the column existed). |

### <span id="struct.NewWorkflowExecution"></span><code style="font-size: 0.85em">struct NewWorkflowExecution</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new workflow executions (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `String` |  |
| `workflow_version` | `String` |  |
| `status` | `String` |  |
| `context_id` | `Option<UniversalUuid>` |  |
