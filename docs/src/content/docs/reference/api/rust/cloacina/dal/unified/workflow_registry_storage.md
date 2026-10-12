---
title: "cloacina::dal::unified::workflow_registry_storage"
description: "Unified workflow registry storage with runtime backend selection"
sidebar:
  order: 111
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Unified workflow registry storage with runtime backend selection

This module provides binary storage operations that work with both
PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

## Structs

### <span id="struct.UnifiedRegistryStorage"></span>`struct UnifiedRegistryStorage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Unified registry storage that works with both PostgreSQL and SQLite.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` |  |

#### Methods

##### <span id="UnifiedRegistryStorage.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (database : Database) -> Self
```

Creates a new UnifiedRegistryStorage instance.

<details>
<summary>Source</summary>

```rust
    pub fn new(database: Database) -> Self {
        Self { database }
    }
```

</details>



##### <span id="UnifiedRegistryStorage.database"></span>`database`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn database (& self) -> & Database
```

Returns a reference to the underlying database.

<details>
<summary>Source</summary>

```rust
    pub fn database(&self) -> &Database {
        &self.database
    }
```

</details>





