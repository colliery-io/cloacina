---
title: "cloacina::dal"
description: "Data Access Layer with runtime backend selection"
editUrl: false
sidebar:
  order: 62
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Data Access Layer with runtime backend selection

This module provides storage-specific DAL implementations:
- unified: Runtime backend selection (PostgreSQL or SQLite)
- filesystem_dal: For filesystem-based storage operations

**Architecture**

The unified DAL uses custom Diesel SQL types (DbUuid, DbTimestamp, DbBool,
DbBinary) that work with both PostgreSQL and SQLite backends. Backend
selection happens at runtime based on the database connection URL.

## Re-exports

- [`DAL`](../../../rust/cloacina/dal/unified/#struct.DAL)
- [`ScheduleExecutionStats`](../../../rust/cloacina/dal/unified/schedule_execution/#struct.ScheduleExecutionStats)
- [`FilesystemRegistryStorage`](../../../rust/cloacina/dal/filesystem_dal/workflow_registry_storage/#struct.FilesystemRegistryStorage)
- [`UnifiedDAL`](../../../rust/cloacina/dal/unified/#struct.DAL) (`unified::DAL`)
- [`UnifiedRegistryStorage`](../../../rust/cloacina/dal/unified/workflow_registry_storage/#struct.UnifiedRegistryStorage)
