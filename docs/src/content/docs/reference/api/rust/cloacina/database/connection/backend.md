---
title: "cloacina::database::connection::backend"
description: "Database backend types and runtime backend selection."
sidebar:
  order: 116
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Database backend types and runtime backend selection.

## Enums

### <span id="enum.BackendType"></span>`enum BackendType`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Represents the database backend type, detected at runtime from the connection URL.

#### Variants

- **`Postgres`** - PostgreSQL backend
- **`Sqlite`** - SQLite backend



### <span id="enum.AnyConnection"></span>`enum AnyConnection`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Multi-connection enum that wraps both PostgreSQL and SQLite connections.

This enum enables runtime database backend selection using Diesel's
`MultiConnection` derive macro. The actual connection type is determined
at runtime based on the connection URL.

#### Variants

- **`Postgres`** - PostgreSQL connection variant
- **`Sqlite`** - SQLite connection variant



### <span id="enum.AnyPool"></span>`enum AnyPool`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Pool enum that wraps both PostgreSQL and SQLite connection pools.

This enum enables runtime pool selection based on the detected backend.

#### Variants

- **`Postgres`** - PostgreSQL connection pool
- **`Sqlite`** - SQLite connection pool



