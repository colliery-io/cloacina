---
title: "cloacina::models::context"
description: "Context management module for storing and retrieving context data."
editUrl: false
sidebar:
  order: 134
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Context management module for storing and retrieving context data.

This module provides domain structures for working with context data.
These are API-level types used in business logic; backend-specific
models handle actual database interaction.

## Structs

### <span id="struct.DbContext"></span><code style="font-size: 0.85em">struct DbContext</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents a context record (domain type).

This is used at the API boundary and in business logic.
Backend-specific models (PgDbContext, SqliteDbContext) handle database storage.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `value` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |

### <span id="struct.NewDbContext"></span><code style="font-size: 0.85em">struct NewDbContext</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new context records (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `value` | `String` |  |
