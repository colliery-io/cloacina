---
id: final-sweep-eliminate-remaining
level: task
title: "Final sweep — eliminate remaining pipeline references"
short_code: "CLOACI-T-0490"
created_at: 2026-04-14T00:57:36.243621+00:00
updated_at: 2026-04-16T02:49:11.526353+00:00
parent: CLOACI-I-0094
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0094
---

# Final sweep — eliminate remaining pipeline references

## Parent Initiative

[[CLOACI-I-0094]]

## Objective

Case-insensitive sweep across all crates, tests, examples, tutorials, docs, and configs to eliminate any remaining "pipeline" references. Rename test functions, update comments, verify no consumer-facing breakage.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `rg -i pipeline crates/` returns zero hits (excluding changelogs and intentional "pipeline" in computation graph context)
- [ ] Test function names updated (`test_pipeline_*` → `test_workflow_*`)
- [ ] Examples and tutorials updated
- [ ] Documentation references updated
- [ ] All tests pass

## Implementation Notes

### Approach
- Case-insensitive grep across entire repo
- Categorize hits: code (fix), comments (fix), docs (fix), changelogs (leave), computation graph "pipeline" (leave — different concept)
- Rename test functions
- Verify with full test suite

### Dependencies
- T-0488 and T-0489 must be completed first

## Status Updates

*To be added during implementation*
