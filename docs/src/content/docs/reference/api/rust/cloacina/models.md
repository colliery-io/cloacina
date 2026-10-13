---
title: "cloacina::models"
description: "API reference of the Rust module cloacina::models."
editUrl: false
sidebar:
  order: 133
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Database Models**

This module contains Diesel model definitions for database entities.
Models define the structure of database tables and provide type-safe
access to database records.

**Models**

- [`context`](../../../rust/cloacina/models/context/): Models for execution context storage
- [`schedule`](../../../rust/cloacina/models/schedule/): Models for unified cron and trigger workflow scheduling
- [`workflow_execution`](../../../rust/cloacina/models/workflow_execution/): Models for tracking workflow execution state and metadata
- [`recovery_event`](../../../rust/cloacina/models/recovery_event/): Models for recording system recovery events and state transitions
- [`task_execution`](../../../rust/cloacina/models/task_execution/): Models for managing individual task execution records
- [`task_execution_metadata`](../../../rust/cloacina/models/task_execution_metadata/): Models for storing task execution metadata and context references
- [`workflow_registry`](../../../rust/cloacina/models/workflow_registry/): Models for binary workflow package storage

**Usage**

Models are typically used with the Data Access Layer (DAL) to perform
database operations:

```rust
use cloacina::models::context::NewDbContext;
use cloacina::dal::DAL;

let new_context = NewDbContext {
    value: r#"{"key": "value"}"#.to_string(),
};

let inserted = dal.context().create(new_context)?;
```

**Module Structure**

Each module contains:
- Struct definitions for database tables
- New-type structs for creating new records
- Type-safe query builders
- Serialization/deserialization implementations

**Best Practices**

- Always use the provided new-type structs for creating records
- Utilize the DAL for database operations rather than direct Diesel queries
- Handle database errors appropriately using the Result type
- Keep model definitions in sync with database schema migrations
