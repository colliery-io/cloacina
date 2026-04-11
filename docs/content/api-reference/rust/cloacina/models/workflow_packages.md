# cloacina::models::workflow_packages <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Database models for workflow package metadata.

This module defines domain structures for workflow package metadata.
These are API-level types; backend-specific models handle database storage.

## Structs

### `cloacina::models::workflow_packages::WorkflowPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Domain model for workflow package metadata.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `registry_id` | `UniversalUuid` |  |
| `package_name` | `String` |  |
| `version` | `String` |  |
| `description` | `Option < String >` |  |
| `author` | `Option < String >` |  |
| `metadata` | `String` |  |
| `storage_type` | `StorageType` |  |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |
| `tenant_id` | `Option < String >` |  |



### `cloacina::models::workflow_packages::NewWorkflowPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating new workflow package metadata entries (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `registry_id` | `UniversalUuid` |  |
| `package_name` | `String` |  |
| `version` | `String` |  |
| `description` | `Option < String >` |  |
| `author` | `Option < String >` |  |
| `metadata` | `String` |  |
| `storage_type` | `StorageType` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (registry_id : UniversalUuid , package_name : String , version : String , description : Option < String > , author : Option < String > , metadata : String , storage_type : StorageType ,) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(
        registry_id: UniversalUuid,
        package_name: String,
        version: String,
        description: Option<String>,
        author: Option<String>,
        metadata: String,
        storage_type: StorageType,
    ) -> Self {
        Self {
            registry_id,
            package_name,
            version,
            description,
            author,
            metadata,
            storage_type,
        }
    }
```

</details>





## Enums

### `cloacina::models::workflow_packages::StorageType` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Storage type for workflow binary data.

#### Variants

- **`Database`** - Binary stored in workflow_registry database table
- **`Filesystem`** - Binary stored on filesystem at {storage_dir}/{registry_id}.so
