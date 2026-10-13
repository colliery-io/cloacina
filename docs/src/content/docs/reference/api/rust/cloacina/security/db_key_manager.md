---
title: "cloacina::security::db_key_manager"
description: "Database-backed key manager implementation."
editUrl: false
sidebar:
  order: 190
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Database-backed key manager implementation.

This module provides a `KeyManager` implementation that stores keys
in the database using Diesel. Private keys are encrypted at rest using
AES-256-GCM.

## Structs

### <span id="struct.DbKeyManager"></span><code style="font-size: 0.85em">struct DbKeyManager</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Database-backed implementation of the `KeyManager` trait.

This implementation:
- Stores signing keys with AES-256-GCM encrypted private keys
- Supports trust relationships between organizations via ACLs
- Does NOT cache any data to ensure immediate effect of revocations

#### Methods

##### <span id="DbKeyManager.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: DAL) -> Self
```

Creates a new database-backed key manager.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: DAL) -> Self {
        Self { dal }
    }
```

</details>
