---
title: "cloacina::models::workflow_packages"
description: "Database models for workflow package metadata."
sidebar:
  order: 163
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Database models for workflow package metadata.

This module defines domain structures for workflow package metadata.
These are API-level types; backend-specific models handle database storage.

## Structs

### <span id="struct.WorkflowPackage"></span>`struct WorkflowPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
| `content_hash` | `String` |  |
| `superseded` | `bool` |  |
| `compiled_data` | `Option < Vec < u8 > >` |  |
| `build_status` | `String` |  |
| `build_error` | `Option < String >` |  |
| `build_claimed_at` | `Option < UniversalTimestamp >` |  |
| `compiled_at` | `Option < UniversalTimestamp >` |  |



### <span id="struct.NewWorkflowPackage"></span>`struct NewWorkflowPackage`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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

##### <span id="NewWorkflowPackage.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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

### <span id="enum.StorageType"></span>`enum StorageType`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Storage type for workflow binary data.

#### Variants

- **`Database`** - Binary stored in workflow_registry database table
- **`Filesystem`** - Binary stored on filesystem at {storage_dir}/{registry_id}.so



