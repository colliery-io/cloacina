---
title: "cloacina::registry::storage"
description: "Storage backend implementations for the workflow registry."
editUrl: false
sidebar:
  order: 169
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Storage backend implementations for the workflow registry.

This module provides storage backends for persisting workflow binaries:
- `UnifiedRegistryStorage` - Database storage (PostgreSQL or SQLite)
- `FilesystemRegistryStorage` - Filesystem-based storage

**Usage Example**

```rust
use cloacina::dal::{UnifiedRegistryStorage, FilesystemRegistryStorage};
use cloacina::registry::RegistryStorage;
use cloacina::database::Database;

// Database storage (unified - works with both PostgreSQL and SQLite)
let database = Database::new("sqlite::memory:", "test", 10);
let db_storage = UnifiedRegistryStorage::new(database);

// Filesystem storage
let fs_storage = FilesystemRegistryStorage::new("/var/lib/cloacina/registry")?;

// Both implement RegistryStorage trait
let data = b"compiled workflow binary data";
// let id = db_storage.store_binary(data.to_vec()).await?;
```

## Re-exports

- [`FilesystemRegistryStorage`](../../../../rust/cloacina/dal/filesystem_dal/workflow_registry_storage/#struct.FilesystemRegistryStorage)
- [`UnifiedRegistryStorage`](../../../../rust/cloacina/dal/unified/workflow_registry_storage/#struct.UnifiedRegistryStorage)
