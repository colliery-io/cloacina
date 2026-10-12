---
title: "cloacina::dal::unified::workflow_execution"
description: "Unified Workflow Execution DAL with compile-time backend selection"
sidebar:
  order: 109
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Unified Workflow Execution DAL with compile-time backend selection

All state transitions are transactional: the status update and execution event
are written atomically. If either fails, both are rolled back.

## Structs

### <span id="struct.ExecutionListFilter"></span>`struct ExecutionListFilter`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Default`

Filter for `WorkflowExecutionDAL::list_filtered`. CLOACI-T-0594 / API-02: closes the silent-filter-drop bug where the REST route's `--status` / `--workflow_name` query params were discarded.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `status` | `Option < String >` | Filter by status (e.g. `Pending`, `Running`, `Completed`, `Failed`).
`None` means no status filter. |
| `workflow_name` | `Option < String >` | Filter by exact workflow name. `None` means no name filter. |
| `limit` | `i64` | SQL `LIMIT`. Caller is responsible for bounding (route validates
`limit <= 1000`); the DAL trusts whatever is passed. |
| `offset` | `i64` | SQL `OFFSET`. |



### <span id="struct.WorkflowExecutionDAL"></span>`struct WorkflowExecutionDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Data access layer for workflow execution operations with compile-time backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



