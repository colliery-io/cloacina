---
id: workflowpatterncheck-glob-matching
level: task
title: "WorkflowPatternCheck glob matching utility for handler-level ABAC"
short_code: "CLOACI-T-0191"
created_at: 2026-03-16T20:01:05.855076+00:00
updated_at: 2026-03-16T20:30:58.761874+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# WorkflowPatternCheck glob matching utility for handler-level ABAC

## Objective

Implement the `check_workflow_access` glob matching utility used at the handler level for fine-grained ABAC. This function determines whether a given workflow name matches the API key's allowed workflow patterns.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `check_workflow_access(patterns: &[String], workflow_name: &str) -> bool` function
- [ ] Empty patterns slice returns true (unrestricted access)
- [ ] Non-empty patterns: returns true if at least one pattern matches
- [ ] `*` glob matches any sequence of characters within a namespace segment
- [ ] `::` is the namespace separator (e.g., `etl::daily_load`)
- [ ] Exact match works: `"etl::daily_load"` matches `"etl::daily_load"`
- [ ] Glob match works: `"etl::*"` matches `"etl::daily_load"`
- [ ] No match returns false: `"etl::*"` does not match `"reports::monthly"`
- [ ] Multiple patterns: any match is sufficient
- [ ] Unit tests covering: empty patterns, exact match, glob match, no match, multiple patterns with mixed results

## Implementation Notes

### Matching Logic
- Simple glob: convert pattern to a check where `*` matches any chars
- Can use a lightweight approach — no need for full regex or the `glob` crate
- Convert `*` to regex `.*` or implement a simple two-pointer match
- `::` is treated as literal characters in the pattern (not special beyond being a convention)

### Usage
- Called in request handlers that operate on specific workflows (e.g., trigger execution, get workflow status)
- Handler reads `AuthContext` from extensions, calls `check_workflow_access(ctx.workflow_patterns, &workflow_name)`
- Returns 403 if false

### Dependencies
- No external dependencies — pure string matching utility
- Used by handlers in CLOACI-I-0032 (Core API) and CLOACI-I-0033 (Tenant API)

## Status Updates

### 2026-03-16 — Completed
- Created auth/pattern.rs: check_workflow_access() + glob_match()
- Empty patterns = unrestricted, non-empty = at least one must glob-match
- Supports * (any sequence) and ? (single char) wildcards
- 7 unit tests covering all cases
