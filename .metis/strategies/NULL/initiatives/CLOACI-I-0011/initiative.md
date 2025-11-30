---
id: implement-execution-order-field-in
level: initiative
title: "Implement execution_order Field in Package Manifest"
short_code: "CLOACI-I-0011"
created_at: 2025-11-29T02:40:15.272951+00:00
updated_at: 2025-11-29T02:40:15.272951+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: S
strategy_id: NULL
initiative_id: implement-execution-order-field-in
---

# Implement execution_order Field in Package Manifest Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

In `cloacina/src/packaging/manifest.rs` (line 72), there's an incomplete TODO:

```rust
execution_order: vec![], // TODO: Generate from task dependencies based on extracted tasks
```

The `PackageManifest` struct has an `execution_order` field that is always set to an empty vector, despite task dependency information being available from the extracted metadata.

**Impact:** API consumers expecting `execution_order` to contain valid topological ordering get an empty list, potentially causing incorrect execution planning.

## Goals & Non-Goals

**Goals:**
- Implement topological sorting of tasks based on dependencies
- Populate `execution_order` with valid execution sequence
- Handle cycles gracefully with clear error messages

**Non-Goals:**
- Changing the dependency specification format
- Supporting dynamic/runtime dependency resolution

## Detailed Design

### Topological Sort Implementation

```rust
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;

fn compute_execution_order(tasks: &[TaskMetadata]) -> Result<Vec<String>, ManifestError> {
    let mut graph = DiGraph::<&str, ()>::new();
    let mut node_indices = HashMap::new();
    
    // Add all tasks as nodes
    for task in tasks {
        let idx = graph.add_node(&task.id);
        node_indices.insert(&task.id, idx);
    }
    
    // Add dependency edges
    for task in tasks {
        let task_idx = node_indices[&task.id];
        for dep in &task.dependencies {
            if let Some(&dep_idx) = node_indices.get(dep) {
                graph.add_edge(dep_idx, task_idx, ());
            } else {
                return Err(ManifestError::UnknownDependency {
                    task: task.id.clone(),
                    dependency: dep.clone(),
                });
            }
        }
    }
    
    // Compute topological order
    toposort(&graph, None)
        .map(|order| order.into_iter().map(|idx| graph[idx].to_string()).collect())
        .map_err(|cycle| ManifestError::CyclicDependency {
            task: graph[cycle.node_id()].to_string(),
        })
}
```

### Integration

```rust
pub fn extract_manifest(path: &Path) -> Result<PackageManifest, ManifestError> {
    let tasks = extract_tasks(path)?;
    let execution_order = compute_execution_order(&tasks)?;
    
    Ok(PackageManifest {
        tasks,
        execution_order,
        // ...
    })
}
```

## Testing Strategy

- Test linear dependency chains
- Test parallel independent tasks
- Test diamond dependency patterns
- Test cycle detection and error messages
- Test missing dependency references

## Alternatives Considered

1. **Lazy computation** - Compute on first access instead of at load time
2. **Remove the field** - Breaking API change, doesn't solve the underlying need

## Implementation Plan

1. Add `compute_execution_order()` function
2. Integrate into manifest extraction
3. Add error types for cycles and missing deps
4. Add comprehensive test cases
5. Update documentation