# cloacina::dal::filesystem_dal::workflow_registry_storage <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Filesystem DAL for workflow registry storage operations.

This module provides filesystem-based data access operations for workflow
registry binary data storage, following the established DAL patterns for
non-database storage backends.

## Structs

### `cloacina::dal::filesystem_dal::workflow_registry_storage::FilesystemRegistryStorage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Filesystem-based DAL for workflow registry storage operations.

This DAL implementation handles binary workflow data storage as individual
files on the local filesystem. Files are named using UUIDs to ensure
uniqueness and avoid conflicts.

**Examples:**

```rust,ignore
use cloacina::dal::filesystem_dal::FilesystemRegistryStorage;
use cloacina::registry::RegistryStorage;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut storage = FilesystemRegistryStorage::new("/var/lib/cloacina/registry")?;

// Store binary workflow data
let workflow_data = std::fs::read("my_workflow.so")?;
let id = storage.store_binary(workflow_data).await?;

// Retrieve it later
if let Some(data) = storage.retrieve_binary(&id).await? {
    println!("Retrieved {} bytes", data.len());
}
# Ok(())
# }
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `storage_dir` | `PathBuf` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new < P : AsRef < Path > > (storage_dir : P) -> Result < Self , std :: io :: Error >
```

Create a new filesystem workflow registry DAL.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `storage_dir` | `-` | Directory path where workflow files will be stored |


**Returns:**

* `Ok(FilesystemWorkflowRegistryDAL)` - Successfully created DAL * `Err(std::io::Error)` - If directory creation fails

**Examples:**

```rust,ignore
use cloacina::dal::filesystem_dal::FilesystemRegistryStorage;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let dal = FilesystemRegistryStorage::new("/var/lib/cloacina/registry")?;
# Ok(())
# }
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



##### `storage_dir` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn storage_dir (& self) -> & Path
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



##### `file_path` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn file_path (& self , id : & str) -> PathBuf
```

Generate the file path for a given workflow ID.

<details>
<summary>Source</summary>

```rust
    fn file_path(&self, id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.so", id))
    }
```

</details>



##### `check_disk_space` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn check_disk_space (& self) -> Result < u64 , StorageError >
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
