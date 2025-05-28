---
title: "Workflow Versioning"
description: "A deep dive into how Cloacina handles workflow versioning and task fingerprinting"
date: 2024-03-19
weight: 1
---


## Introduction

Cloacina's workflow versioning system is a core feature that tracks changes to workflows and their constituent tasks. The system is agnostic to what these changes mean - it simply detects and records them. Consumers of Cloacina can then decide how to handle these changes based on their specific needs.

## Why Versioning Matters

In workflow systems, versioning is crucial for:

- **Change Detection**: Identifying when workflows or their components have changed
- **State Tracking**: Maintaining a history of workflow modifications
- **Consumer Control**: Allowing consumers to implement their own versioning policies
- **Audit Trail**: Providing a record of workflow evolution

## The Versioning System

### How Versions Work

Cloacina generates unique versions for each workflow based on:

1. **Workflow Structure** {{< api-link path="cloacina::workflow::Workflow" type="struct" >}}
   - Task IDs and their relationships
   - Dependency graph topology
   - Task ordering
   - Implementation detail: The structure is hashed using a deterministic algorithm that considers task IDs and their dependencies
   ```rust
   fn hash_topology(&self, hasher: &mut DefaultHasher) {
       // Get tasks in deterministic order
       let mut task_ids: Vec<_> = self.tasks.keys().collect();
       task_ids.sort();

       for task_id in task_ids {
           task_id.hash(hasher);

           // Include dependencies in deterministic order
           let mut deps: Vec<_> = self.tasks[task_id].dependencies().to_vec();
           deps.sort();
           deps.hash(hasher);
       }
   }
   ```

2. **Task Definitions** {{< api-link path="cloacina::task::Task" type="trait" >}}
   - Task implementation code (fingerprints)
   - Task dependencies
   - Task configuration
   - Implementation detail: Task definitions are hashed using a deterministic algorithm that considers task metadata and code fingerprints
   ```rust
   fn hash_task_definitions(&self, hasher: &mut DefaultHasher) {
       // Get tasks in deterministic order
       let mut task_ids: Vec<_> = self.tasks.keys().collect();
       task_ids.sort();

       for task_id in task_ids {
           let task = &self.tasks[task_id];

           // Hash task metadata
           task.id().hash(hasher);
           task.dependencies().hash(hasher);

           // Hash task code fingerprint (if available)
           if let Some(code_hash) = self.get_task_code_hash(task_id) {
               code_hash.hash(hasher);
           }
       }
   }
   ```
   - Task fingerprints are calculated by:
   ```rust
   fn calculate_function_fingerprint(func: &ItemFn) -> String {
       let mut hasher = DefaultHasher::new();

       // Hash function signature (excluding name)
       func.sig.inputs.iter().for_each(|input| {
           if let syn::FnArg::Typed(pat_type) = input {
               quote::quote!(#pat_type).to_string().hash(&mut hasher);
           }
       });

       // Hash return type
       quote::quote!(#(&func.sig.output)).to_string().hash(&mut hasher);

       // Hash function body (this is the key part for detecting changes)
       let body_tokens = quote::quote!(#(&func.block)).to_string();
       body_tokens.hash(&mut hasher);

       // Include async info
       func.sig.asyncness.is_some().hash(&mut hasher);

       format!("{:016x}", hasher.finish())
   }
   ```

3. **Workflow Configuration** {{< api-link path="cloacina::workflow::WorkflowMetadata" type="struct" >}}
   - Workflow name and description
   - Workflow tags
   - Global settings
   - Implementation detail: Configuration is hashed using a deterministic algorithm that considers all metadata fields
   ```rust
   fn hash_configuration(&self, hasher: &mut DefaultHasher) {
       // Hash Workflow-level configuration (excluding version and timestamps)
       self.name.hash(hasher);
       self.metadata.description.hash(hasher);

       // Hash tags in deterministic order
       let mut tags: Vec<_> = self.metadata.tags.iter().collect();
       tags.sort_by_key(|(k, _)| *k);
       tags.hash(hasher);
   }
   ```

The final version is calculated by combining all three components:
```rust
pub fn calculate_version(&self) -> String {
    let mut hasher = DefaultHasher::new();

    // 1. Hash Workflow structure (topology)
    self.hash_topology(&mut hasher);

    // 2. Hash task definitions
    self.hash_task_definitions(&mut hasher);

    // 3. Hash Workflow configuration
    self.hash_configuration(&mut hasher);

    // Return hex representation of hash
    format!("{:016x}", hasher.finish())
}
```

### What Gets Tracked

The workflow version hash changes when any of these components change:

1. **Task Implementation**
   - Function body changes
   - Function signature changes (parameters, return type)
   - Async status changes
   - Task dependencies changes

2. **Workflow Structure**
   - Task IDs added or removed
   - Task dependency relationships changed
   - Task ordering changed
   - Task configuration changes

3. **Workflow Configuration**
   - Workflow name changes
   - Description changes
   - Tags added, removed, or modified
   - Global settings changes

The version hash is deterministic - the same workflow configuration will always produce the same hash, making it reliable for change detection and version tracking.

## Version Management in Practice

### Storage and Tracking

Workflow versions are:
- Stored in the database with each pipeline execution
- Tracked using content-based hashing
- Made available to consumers through the API

Example of version tracking in the database:
```sql
CREATE TABLE pipeline_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pipeline_name VARCHAR NOT NULL,
    pipeline_version VARCHAR NOT NULL,  -- Content-based version hash
    status VARCHAR NOT NULL CHECK (status IN ('Pending', 'Running', 'Completed', 'Failed', 'Cancelled')),
    context_id UUID REFERENCES contexts(id),
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    error_details TEXT,
    recovery_attempts INTEGER DEFAULT 0 NOT NULL,
    last_recovery_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Indexes for efficient querying
CREATE INDEX pipeline_executions_name_version_idx ON pipeline_executions(pipeline_name, pipeline_version);
CREATE INDEX pipeline_executions_status_idx ON pipeline_executions(status);
```

### Consumer Implementation

Consumers can detect version changes using the following pattern:

```rust
let current_version = workflow.metadata().version.clone();
let last_version = dal.pipeline_execution().get_last_version(workflow_name)?;

if last_version.as_ref() != Some(&current_version) {
    // Version has changed - handle as needed
    info!(
        "Workflow '{}' version changed: {} -> {}",
        workflow_name,
        last_version.unwrap_or_else(|| "none".to_string()),
        current_version
    );
}
```

This pattern allows consumers to detect when a workflow's version has changed, indicating modifications to the workflow's structure, task implementations, or configuration.

## Conclusion

Cloacina's workflow versioning system uses content-based hashing to track changes to workflows. The version hash is calculated from the workflow's structure, task definitions, and configuration, providing a reliable way to detect when any of these components change. This deterministic versioning system enables consumers to detect changes while remaining agnostic to how those changes are handled.
