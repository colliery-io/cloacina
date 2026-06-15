---
title: "Context System"
description: "A comprehensive guide to understanding and using Cloacina's context management system"
date: 2024-03-19
weight: 2
reviewer: "dstorey"
review_date: "2024-03-19"
---

# Mastering Cloacina's Context System

## Introduction

The Context system is a fundamental component of Cloacina that enables safe and efficient data sharing between tasks. This article explores how it works and how to use it effectively.

## Context Fundamentals

### What is the Context System?

The Context system provides:
- Type-safe data sharing between tasks
- Atomic updates with database persistence
- Automatic dependency resolution
- Thread-safe access patterns

### Core Components

The context system consists of several key components:

1. **Context Structure** {{< api-link path="cloacina::context::Context" type="struct" >}}
   - Generic type-safe container for task data
   - JSON serialization for persistence
   - Execution scope tracking
   - Dependency loading capabilities

2. **Database Integration** {{< api-link path="cloacina::models::context::DbContext" type="struct" >}}
   - Contexts are stored in the `contexts` table
   - Task execution metadata links contexts to tasks
   - Automatic timestamp management
   - UUID-based record identification

3. **Dependency Management** {{< api-link path="cloacina::executor::types::DependencyLoader" type="struct" >}}
   - Automatic loading of values from dependency tasks
   - Configurable dependency loading strategy
   - Caching of loaded values for performance

## System Architecture

### Database Schema

{{< mermaid >}}
erDiagram
    contexts {
        uuid id PK
        varchar value
        timestamp created_at
        timestamp updated_at
    }

    pipeline_executions {
        uuid id PK
        varchar pipeline_name
        varchar pipeline_version
        varchar status
        uuid context_id FK
        timestamp started_at
        timestamp completed_at
        text error_details
        int4 recovery_attempts
        timestamp last_recovery_at
        timestamp created_at
        timestamp updated_at
    }

    task_execution_metadata {
        uuid id PK
        uuid task_execution_id FK
        uuid pipeline_execution_id FK
        varchar task_name
        uuid context_id FK
        timestamp created_at
        timestamp updated_at
    }

    task_executions {
        uuid id PK
        uuid pipeline_execution_id FK
        varchar task_name
        varchar status
        timestamp started_at
        timestamp completed_at
        int4 attempt
        int4 max_attempts
        text error_details
        jsonb trigger_rules
        jsonb task_configuration
        timestamp retry_at
        text last_error
        int4 recovery_attempts
        timestamp last_recovery_at
        timestamp created_at
        timestamp updated_at
    }

    contexts ||--o{ pipeline_executions : "has many (context_id)"
    contexts ||--o{ task_execution_metadata : "has many (context_id)"
    task_execution_metadata ||--o{ task_executions : "references (task_execution_id)"
    pipeline_executions ||--o{ task_executions : "contains (pipeline_execution_id)"
{{< /mermaid >}}

## Data Flow Patterns

### Context Flow

{{< mermaid >}}
graph TD
    A[Pipeline Definition] --> B[Pipeline Execution]
    C[Initial Context] -->|injected| B
    B --> D[Task Execution]
    D --> E{Has Dependencies?}
    E -->|No| F[Use Injected Context]
    E -->|Yes| G{Multiple Upstreams?}
    G -->|No| H[Load Dependency Context]
    G -->|Yes| I[Merge Dependency Contexts]
    F --> J[Execute Task & Allow Context Mutations]
    H --> J
    I --> J
    J --> K[Save Task's Modified Context]
    K --> L[Update Task Metadata]
    L --> M{More Tasks?}
    M -->|Yes| D
    M -->|No| N[Pipeline Complete]
{{< /mermaid >}}

Data flows between tasks through two main mechanisms:

1. **Initial Context** {{< api-link path="cloacina::scheduler::Scheduler::load_context_for_task" type="fn" >}}
   - Initial context is injected when pipeline starts
   - Available to all tasks in the pipeline
   - Tasks can read and modify this data
   - Changes are persisted between task executions
   - Used as base when no dependencies exist

2. **Transitive Dependencies** {{< api-link path="cloacina::executor::types::DependencyLoader::load_from_dependencies" type="fn" >}}
   - Task contexts are automatically loaded for dependent tasks
   - Dependencies are transitive - can access data from dependencies' dependencies
   - Values are merged based on dependency order
   - Later dependencies override earlier ones
   - Implemented through the DependencyLoader's "latest wins" strategy

### Context Merge Algorithm

When multiple upstream tasks feed into a single task (a "reduce" in the topology), their contexts are merged using a "latest wins" strategy:

1. **Dependency Order**
   - The order of dependencies is determined by how they are declared in the task's `dependencies()` method
   - For example, if a task declares `dependencies() -> &[String] { &["task1", "task2", "task3"] }`, this defines the base order
   - The system processes dependencies in reverse order (from last to first) to implement "latest wins"

2. **Merge Process** {{< api-link path="cloacina::executor::types::DependencyLoader::load_from_dependencies" type="fn" >}}
   - Each upstream task's context is loaded from the database
   - Dependencies are processed in reverse order of declaration
   - For each key in the upstream contexts:
     - If only one upstream task sets the key: use that value
     - If multiple tasks set the key: use the value from the last dependency in the list


For example, if a task has dependencies `["task1", "task2", "task3"]`:
- The system will process them in reverse order: `task3`, then `task2`, then `task1`
- If all three tasks set a key "result", the value from `task3` will be used because it's processed last
- This is different from execution order (which is determined by the task graph topology) - it's purely based on the order in which dependencies are declared in the task's `dependencies()` method

### Concrete Example: Merging Two Task Dependencies

Let's walk through a simple example with two task dependencies to see how context merging works:

```rust
// Task1's context
{
    "user_id": 123,
    "status": "pending"
}

// Task2's context
{
    "status": "completed",
    "result": 42
}

// Final merged context (processing in reverse order: Task2, then Task1)
{
    "user_id": 123,    // From Task1 (only Task1 set this)
    "status": "completed",  // From Task2 (overrides Task1's value)
    "result": 42       // From Task2 (only Task2 set this)
}
```

{{< hint type=warning >}}
**Context Key Management**

When designing tasks and workflows, be mindful of context key naming to avoid unintended overrides during merging. Since the merge process uses a "latest wins" strategy, tasks that set the same key will override each other's values based on dependency order.

Best practices:
- Use namespaced keys (e.g., `task_name.key`) to avoid collisions
- Document which keys your task reads and writes
- Consider the full dependency chain when choosing key names
- Be explicit about key ownership in task documentation

Example of good key naming:
```rust
// Instead of generic keys like:
{
    "status": "pending",
    "result": 42
}

// Use namespaced keys:
{
    "user_validation.status": "pending",
    "data_processing.result": 42
}
```
{{< /hint >}}

{{< hint info >}}
**Why No Automatic Namespacing?**

While automatic key namespacing (e.g., prefixing all keys with task IDs or names) might seem like an obvious solution to collision prevention, we deliberately chose not to implement it. Here's why:

1. **Collision Resolution is Still Required**: Even with automatic namespacing, tasks would still need to coordinate on key names when they intentionally want to share data. The fundamental problem of coordination remains.

2. **Simple, Well-Defined Behavior**: By keeping the merge strategy simple ("latest wins" based on dependency order), we provide a clear, predictable behavior that developers can reason about and work with.

Rather than trying to solve this with technical means, we provide a simple, well-defined merge behavior and let developers handle coordination through good design and documentation.
{{< /hint >}}


## Performance Considerations

### Context Size and Data Transfer

Contexts are not designed for transferring large datasets between tasks. Instead:

- Use contexts to pass small, essential metadata and control information
- For large data transfers, store the data externally (files, databases, object storage) and pass references
- Keep context payloads small to maintain efficient serialization and database operations

Example of good practice:
```rust
// Instead of storing large data in context:
{
    "image_data": "base64_encoded_10mb_image...",
    "document_content": "very_long_text..."
}

// Store externally and reference:
{
    "image_uri": "s3://bucket/images/123.jpg",
    "document_id": "doc_456"
}
```

This approach:
- Keeps context operations fast and efficient
- Reduces memory pressure
- Maintains good database performance
- Allows for better error handling of large data transfers

## Conclusion

The Context system provides a simple but powerful way to share data between tasks. By understanding the merge behavior and keeping contexts focused on essential metadata, you can build reliable and efficient workflows. Remember: contexts are for coordination, not for data storage - keep them small and focused.
