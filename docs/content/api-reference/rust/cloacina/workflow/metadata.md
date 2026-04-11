# cloacina::workflow::metadata <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Workflow metadata and versioning.

This module contains the `WorkflowMetadata` struct for managing
workflow versioning, timestamps, and organizational tags.

## Structs

### `cloacina::workflow::metadata::WorkflowMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Metadata information for a Workflow.

Contains versioning, creation timestamps, and arbitrary tags for
organizing and managing workflow instances.

**Examples:**

```rust
use cloacina::WorkflowMetadata;
use std::collections::HashMap;

let mut metadata = WorkflowMetadata::default();
metadata.version = "a1b2c3d4".to_string();
metadata.description = Some("Production ETL pipeline".to_string());
metadata.tags.insert("team".to_string(), "data-engineering".to_string());
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `created_at` | `DateTime < Utc >` | When the workflow was created |
| `version` | `String` | Content-based version hash |
| `description` | `Option < String >` | Optional human-readable description |
| `tags` | `HashMap < String , String >` | Arbitrary key-value tags for organization |
