---
id: add-greaterorequal-lessorequal-in
level: task
title: "Add GreaterOrEqual, LessOrEqual, In, NotIn trigger rule operators"
short_code: "CLOACI-T-0249"
created_at: 2026-03-25T12:37:49.955500+00:00
updated_at: 2026-03-25T12:37:49.955500+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Add GreaterOrEqual, LessOrEqual, In, NotIn trigger rule operators

## Objective

Add four missing comparison operators to the trigger rules system so users don't need workarounds like `any(greater_than, equals)` to express `>=`.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Cleaner, more intuitive trigger rule expressions. Currently `>=` requires composing `any(greater_than, equals)` which is verbose and error-prone for integer-only workarounds (`less_than, 81` instead of `less_or_equal, 80`).
- **Effort Estimate**: S — straightforward addition to existing patterns

## Acceptance Criteria

- [ ] `GreaterOrEqual` and `LessOrEqual` variants added to `ValueOperator` enum in `crates/cloacina/src/task_scheduler/trigger_rules.rs`
- [ ] `In` and `NotIn` variants added to `ValueOperator` (value-in-set checks)
- [ ] Macro parser in `crates/cloacina-macros/src/tasks.rs` (`parse_value_operator`) handles `greater_or_equal`, `less_or_equal`, `in`, `not_in`
- [ ] Evaluation logic in `trigger_rules.rs` implements the new operators
- [ ] Unit tests cover all four new operators
- [ ] Documentation updated: `docs/content/reference/trigger-rules.md`, `docs/content/explanation/trigger-rules.md`, `docs/content/reference/task-macro.md`

## Implementation Notes

### Technical Approach

Three files need changes:

1. **`crates/cloacina/src/task_scheduler/trigger_rules.rs`** — Add 4 variants to `ValueOperator` enum and implement evaluation logic in the match arm that handles `ContextValue` conditions.

2. **`crates/cloacina-macros/src/tasks.rs`** — Add 4 arms to `parse_value_operator()` (line ~499):
   ```rust
   "greater_or_equal" => Ok("GreaterOrEqual".to_string()),
   "less_or_equal" => Ok("LessOrEqual".to_string()),
   "in" => Ok("In".to_string()),
   "not_in" => Ok("NotIn".to_string()),
   ```

3. **Documentation** — Re-add the operators to the reference and explanation docs (they were removed during the docs overhaul because they didn't exist yet).

### Dependencies
None — self-contained change.

## Status Updates

*To be added during implementation*
