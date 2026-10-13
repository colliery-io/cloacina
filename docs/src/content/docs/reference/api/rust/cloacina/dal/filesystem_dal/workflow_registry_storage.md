---
title: "cloacina::dal::filesystem_dal::workflow_registry_storage"
description: "Filesystem DAL for workflow registry storage operations."
editUrl: false
sidebar:
  order: 64
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Filesystem DAL for workflow registry storage operations.

This module provides filesystem-based data access operations for workflow
registry binary data storage, following the established DAL patterns for
non-database storage backends.

## Structs

### <span id="struct.FilesystemRegistryStorage"></span><code style="font-size: 0.85em">struct FilesystemRegistryStorage</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Filesystem-based DAL for workflow registry storage operations.

This DAL implementation handles binary workflow data storage as individual
files on the local filesystem. Files are named using UUIDs to ensure
uniqueness and avoid conflicts.

**Directory Structure**

```text
/var/lib/cloacina/registry/
├── a1b2c3d4-e5f6-7890-abcd-ef1234567890.so
├── b2c3d4e5-f6g7-8901-bcde-f12345678901.so
└── ...
```

**Examples:**

```rust
use cloacina::dal::filesystem_dal::FilesystemRegistryStorage;
use cloacina::registry::RegistryStorage;

let mut storage = FilesystemRegistryStorage::new("/var/lib/cloacina/registry")?;

// Store binary workflow data
let workflow_data = std::fs::read("my_workflow.so")?;
let id = storage.store_binary(workflow_data).await?;

// Retrieve it later
if let Some(data) = storage.retrieve_binary(&id).await? {
    println!("Retrieved {} bytes", data.len());
}
```

#### Methods

##### <span id="FilesystemRegistryStorage.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new<P: AsRef<Path>>(storage_dir: P) -> Result<Self, std::io::Error>
```

Create a new filesystem workflow registry DAL.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `storage_dir` | `P` | Directory path where workflow files will be stored |

**Returns:**

* `Ok(FilesystemWorkflowRegistryDAL)` - Successfully created DAL
* `Err(std::io::Error)` - If directory creation fails

**Examples:**

```rust
use cloacina::dal::filesystem_dal::FilesystemRegistryStorage;

let dal = FilesystemRegistryStorage::new("/var/lib/cloacina/registry")?;
```

<details>
<summary>Source</summary>

```rust
    pub fn new<P: AsRef<Path>>(storage_dir: P) -> Result<Self, std::io::Error> {
        let storage_dir = storage_dir.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&storage_dir)?;

        // Verify we can write to the directory
        let test_file = storage_dir.join(".write_test");
        std::fs::write(&test_file, b"test")?;
        std::fs::remove_file(&test_file)?;

        Ok(Self { storage_dir })
    }
```

</details>

##### <span id="FilesystemRegistryStorage.storage_dir"></span><code>storage_​dir</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn storage_dir(&self) -> &Path
```

Get the storage directory path.

<details>
<summary>Source</summary>

```rust
    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }
```

</details>

##### <span id="FilesystemRegistryStorage.check_disk_space"></span><code>check_​disk_​space</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn check_disk_space(&self) -> Result<u64, StorageError>
```

Check available disk space and validate against a threshold.

<details>
<summary>Source</summary>

```rust
    pub async fn check_disk_space(&self) -> Result<u64, StorageError> {
        // Note: This is a simplified implementation
        // In production, you might want to use statvfs or similar
        match fs::metadata(&self.storage_dir).await {
            Ok(_) => {
                // For now, we'll assume space is available
                // A full implementation would check actual disk space
                Ok(u64::MAX)
            }
            Err(e) => Err(StorageError::Backend(format!(
                "Failed to check disk space: {}",
                e
            ))),
        }
    }
```

</details>
