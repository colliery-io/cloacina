---
title: "cloacina::workflow::metadata"
description: "Workflow metadata and versioning."
sidebar:
  order: 223
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Workflow metadata and versioning.

This module contains the `WorkflowMetadata` struct for managing
workflow versioning, timestamps, and organizational tags.

## Structs

### <span id="struct.WorkflowMetadata"></span>`struct WorkflowMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
metadata.description = Some("Production ETL workflow".to_string());
metadata.tags.insert("team".to_string(), "data-engineering".to_string());
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `created_at` | `DateTime < Utc >` | When the workflow was created |
| `version` | `String` | Content-based version hash |
| `description` | `Option < String >` | Optional human-readable description |
| `tags` | `HashMap < String , String >` | Arbitrary key-value tags for organization |



