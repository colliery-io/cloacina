---
title: "cloacina::dal::unified::checkpoint"
description: "Unified Checkpoint DAL for computation graph state persistence"
sidebar:
  order: 86
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Unified Checkpoint DAL for computation graph state persistence

Provides save/load operations for accumulator checkpoints, boundaries,
reactor state, and state accumulator buffers. All operations use upsert
semantics keyed by (graph_name, accumulator_name) or (graph_name).

## Structs

### <span id="struct.CheckpointDAL"></span>`struct CheckpointDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Data access layer for computation graph checkpoint operations.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



