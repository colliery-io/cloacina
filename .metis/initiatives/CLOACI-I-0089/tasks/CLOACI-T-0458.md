---
id: fix-workflowbuilder-context
level: task
title: "Fix WorkflowBuilder context manager to preserve description and tags (API-08)"
short_code: "CLOACI-T-0458"
created_at: 2026-04-09T13:51:27.950467+00:00
updated_at: 2026-04-09T14:03:58.244633+00:00
parent: CLOACI-I-0089
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0089
---

# Fix WorkflowBuilder context manager to preserve description and tags (API-08)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0089]]

## Objective

Users who set `builder.description("ETL pipeline")` and `builder.tag("production")` inside a `with` block find those values missing from the registered workflow. The `__exit__` method creates a fresh `Workflow` via auto-discovery, dropping metadata set during the block.

**Effort**: 1-2 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `description` set via `builder.description()` inside `with` block survives `__exit__` and appears on the registered workflow
- [ ] `tags` set via `builder.tag()` inside `with` block survive `__exit__` and appear on the registered workflow
- [ ] Manual `builder.build()` path continues to preserve description and tags (no regression)
- [ ] Python test verifies both paths produce identical metadata

## Implementation Notes

### Technical Approach

In `crates/cloacina/src/python/workflow.rs`, the `__exit__` method:
1. Creates a new `Workflow::new(workflow_id)` via task auto-discovery
2. This drops `self.inner.description` and `self.inner.tags`

Fix: after creating the new workflow in `__exit__`, copy `self.inner.description` and `self.inner.tags` onto it before registering.

### Dependencies
None.

## Status Updates

- **2026-04-09**: Added `get_description()` and `get_tags()` accessors to `WorkflowBuilder`. In `__exit__`, after creating the new `Workflow`, copy description and tags from `self.inner` before registering. The manual `build()` path was already correct (it uses the builder's internal workflow directly). Compiles clean.
