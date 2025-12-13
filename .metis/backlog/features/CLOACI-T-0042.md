---
id: conditional-retries-add-retry
level: task
title: "Conditional Retries - Add retry_condition parameter to task configuration"
short_code: "CLOACI-T-0042"
created_at: 2025-12-13T14:58:58.066041+00:00
updated_at: 2025-12-13T14:58:58.066041+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# Conditional Retries - Add retry_condition parameter to task configuration

## Objective

Allow tasks to specify conditional retry logic based on failure types, rather than retrying on all failures. Some failures are worth retrying (network timeouts, temporary database locks) while others are not (validation errors, permission denied). Currently all failures trigger retries up to the max retry limit.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Prevents unnecessary retries on non-recoverable errors (validation, permissions), reducing wasted compute and faster failure feedback
- **Business Value**: More efficient resource utilization, clearer error handling semantics
- **Effort Estimate**: M

## Acceptance Criteria

- [ ] `retry_condition` parameter added to `#[task()]` macro accepting predefined conditions or custom functions
- [ ] Predefined conditions available: "network_only", "transient_only", "always" (default)
- [ ] Custom condition functions can inspect `TaskError` to determine retry eligibility
- [ ] Integrates with existing retry configuration (max_retries, backoff)
- [ ] Condition function is serializable for workflow persistence
- [ ] Documentation updated with examples of both predefined and custom conditions

## Implementation Notes

### Proposed Interface

```rust
#[task(
    id = "api_call",
    dependencies = [],
    retry_condition = "network_errors_only"  // or custom function
)]
async fn api_call(context: &mut Context<Value>) -> Result<(), TaskError> {
    // implementation
}

// Or with a closure/function:
#[task(
    id = "api_call",
    dependencies = [],
    retry_condition = |error: &TaskError| matches!(error, TaskError::Network(_))
)]
```

### Technical Approach
- Extend the `#[task()]` proc macro to accept `retry_condition` parameter
- Implement predefined condition functions ("network_only", "transient_only")
- Support custom functions that receive `&TaskError` and return `bool`
- Integrate with existing retry logic in task executor

### Dependencies
- Existing retry configuration (max_retries, backoff)
- TaskError enum structure

### Risk Considerations
- Serialization of custom condition functions for workflow persistence
- Backward compatibility with existing task definitions

## Status Updates

- **2025-12-13**: Created from GitHub issue #1
