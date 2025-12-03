---
id: optimize-memory-allocation-reduce
level: initiative
title: "Optimize Memory Allocation - Reduce Unnecessary Cloning in Hot Paths"
short_code: "CLOACI-I-0013"
created_at: 2025-11-29T02:40:20.554866+00:00
updated_at: 2025-11-29T02:40:20.554866+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
strategy_id: NULL
initiative_id: optimize-memory-allocation-reduce
---

# Optimize Memory Allocation - Reduce Unnecessary Cloning in Hot Paths Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

In `cloacina/src/graph.rs` (lines 126-127), task IDs are cloned on every access:

```rust
pub fn task_ids(&self) -> Vec<String> {
    self.task_index.keys().cloned().collect()  // Clones every key
}
```

**Problems:**
- Every call clones all task IDs (potentially expensive with many tasks)
- Called frequently in validation loops and scheduler operations
- With 1000+ tasks, this adds significant memory pressure
- No caching mechanism

Similar patterns exist elsewhere in hot paths.

## Goals & Non-Goals

**Goals:**
- Return references instead of owned data where possible
- Add caching for frequently-accessed computed values
- Profile and identify other cloning hot spots
- Reduce memory allocations in scheduler loop

**Non-Goals:**
- Premature optimization of cold paths
- Changing public API signatures (unless necessary)

## Detailed Design

### Pattern 1: Return References

```rust
// Before
pub fn task_ids(&self) -> Vec<String> {
    self.task_index.keys().cloned().collect()
}

// After - return iterator
pub fn task_ids(&self) -> impl Iterator<Item = &str> {
    self.task_index.keys().map(|s| s.as_str())
}

// Or return slice if cached
pub fn task_ids(&self) -> &[String] {
    &self.cached_task_ids
}
```

### Pattern 2: Cache Computed Values

```rust
pub struct WorkflowGraph {
    task_index: HashMap<String, TaskNode>,
    // Cache frequently accessed values
    cached_task_ids: OnceCell<Vec<String>>,
    cached_root_tasks: OnceCell<Vec<String>>,
}

impl WorkflowGraph {
    pub fn task_ids(&self) -> &[String] {
        self.cached_task_ids.get_or_init(|| {
            self.task_index.keys().cloned().collect()
        })
    }
}
```

### Pattern 3: Use Cow for Conditional Ownership

```rust
use std::borrow::Cow;

pub fn get_task_name(&self, id: &str) -> Cow<'_, str> {
    self.task_index
        .get(id)
        .map(|t| Cow::Borrowed(t.name.as_str()))
        .unwrap_or_else(|| Cow::Owned(format!("unknown:{}", id)))
}
```

### Hot Spots to Address

| Location | Current | Proposed |
|----------|---------|----------|
| `graph.rs:126` | `Vec<String>` clone | Return `&[String]` cached |
| `graph.rs:task_dependencies()` | Clone deps | Return `&[String]` |
| `scheduler tick loop` | Clone task list | Use references |
| `context serialization` | Clone context | Use Cow |

## Testing Strategy

- Benchmark before/after with large workflows
- Memory profiling with `heaptrack` or similar
- Verify no regressions in functionality
- Test with 1000+ task workflows

## Alternatives Considered

1. **Intern strings** - Good for deduplication but adds complexity
2. **Use `Arc<str>`** - Adds ref-counting overhead
3. **Accept the cloning** - Performance impact is measurable

## Implementation Plan

1. Profile current memory usage with large workflows
2. Identify top cloning hot spots
3. Implement caching for `task_ids()` and similar
4. Change return types to references where safe
5. Benchmark improvements
6. Document performance characteristics
