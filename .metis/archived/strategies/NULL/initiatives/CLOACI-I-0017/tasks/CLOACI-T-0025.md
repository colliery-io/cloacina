---
id: split-workflow-rs-into-module
level: task
title: "Split workflow.rs into module hierarchy"
short_code: "CLOACI-T-0025"
created_at: 2025-12-07T01:16:44.672954+00:00
updated_at: 2025-12-07T02:26:02.744085+00:00
parent: CLOACI-I-0017
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0017
---

# Split workflow.rs into module hierarchy

## Objective

Split `src/workflow.rs` (1,798 lines) into a module hierarchy with focused, single-responsibility files.

## Current State

The file mixes multiple responsibilities:
- Workflow struct definition and metadata management
- DependencyGraph implementation (cycle detection, topological sort)
- WorkflowBuilder with fluent API
- Global workflow registry and constructor registration
- Dependency validation and versioning logic
- Content-based hash calculation

## Target Structure

```
src/workflow/
  mod.rs          (~150 lines - public API and re-exports)
  metadata.rs     (~150 lines - WorkflowMetadata, versioning)
  graph.rs        (~400 lines - DependencyGraph, cycle detection, topo sort)
  builder.rs      (~300 lines - WorkflowBuilder fluent API)
  registry.rs     (~300 lines - global registry, constructors)
  types.rs        (~150 lines - Workflow struct, serialization)
```

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Create `src/workflow/` directory with module files
- [ ] Move `WorkflowMetadata` and versioning logic to `metadata.rs`
- [ ] Move `DependencyGraph` and cycle detection to `graph.rs`
- [ ] Move `WorkflowBuilder` to `builder.rs`
- [ ] Move global registry functions to `registry.rs`
- [ ] Move `Workflow` struct to `types.rs` or keep in `mod.rs`
- [ ] Update `mod.rs` with re-exports maintaining public API
- [ ] All existing tests pass
- [ ] `cargo check` passes
- [ ] No public API changes (same exports from `crate::workflow::*`)

## Implementation Notes

### Technical Approach
1. Create the directory structure first
2. Move types one at a time, updating imports
3. Run tests after each major move
4. Keep re-exports in mod.rs to maintain API compatibility

### Dependencies
None - this is a standalone refactor

### Risk Considerations
- Low risk: internal refactor only
- Tests will catch any broken imports
