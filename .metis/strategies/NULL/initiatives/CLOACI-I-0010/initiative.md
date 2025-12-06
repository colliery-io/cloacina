---
id: replace-scattered-expect-unwrap
level: initiative
title: "Replace Scattered expect/unwrap with Proper Error Handling"
short_code: "CLOACI-I-0010"
created_at: 2025-11-29T02:40:15.160144+00:00
updated_at: 2025-12-06T14:56:35.276406+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
strategy_id: NULL
initiative_id: replace-scattered-expect-unwrap
---

# Replace Scattered expect/unwrap with Proper Error Handling Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

Multiple files across the codebase use `.expect()` and `.unwrap()` which will panic on failure:

**Identified locations:**
- `packaging/tests.rs` (lines 45, 49, 61, 77): Test setup panics
- `database/mod.rs` (line 154): Migration panics on error
- `database/connection.rs` (lines 155, 189): Connection pool creation panics
- `packaging/manifest.rs` (line 306): Regex compilation panic

**Risk:** Production code crashes instead of returning errors, making the system unreliable in edge cases (network issues, misconfiguration, resource exhaustion).

## Goals & Non-Goals

**Goals:**
- Replace `.expect()` with proper `Result` returns in production code
- Keep `.expect()` in test code where panics are appropriate
- Add context to unavoidable `.expect()` calls (truly impossible conditions)
- Improve error messages for initialization failures

**Non-Goals:**
- Eliminating all `.unwrap()` (some are safe after validation)
- Adding recovery mechanisms for truly unrecoverable errors

## Detailed Design

### Pattern 1: Initialization Functions

Change functions that panic to return `Result`:

```rust
// Before
pub fn create_pool(url: &str) -> Pool {
    Pool::builder()
        .build(url)
        .expect("Failed to create connection pool")
}

// After
pub fn create_pool(url: &str) -> Result<Pool, DatabaseError> {
    Pool::builder()
        .build(url)
        .map_err(|e| DatabaseError::PoolCreation {
            url: url.to_string(),
            source: e
        })
}
```

### Pattern 2: Unavoidable Expects

For truly impossible conditions, document why:

```rust
// This regex is hardcoded and tested - failure indicates a bug
static WORKFLOW_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(PATTERN).expect("BUG: Invalid hardcoded regex pattern")
});
```

### Pattern 3: Test Code

Keep `.expect()` in tests but with descriptive messages:

```rust
#[test]
fn test_workflow_creation() {
    let pool = create_pool(&test_url())
        .expect("Test setup: failed to create connection pool");
    // ...
}
```

## Files to Update

| File | Line | Current | Action |
|------|------|---------|--------|
| `database/connection.rs` | 155 | `.expect()` | Return `Result` |
| `database/connection.rs` | 189 | `.expect()` | Return `Result` |
| `database/mod.rs` | 154 | `.expect()` | Return `Result` |
| `packaging/manifest.rs` | 306 | `.expect()` | Use `Lazy` (see I-0009) |

## Testing Strategy

- Verify error propagation through call stack
- Test initialization with invalid configs
- Ensure error messages are actionable

## Alternatives Considered

1. **Use `anyhow` everywhere** - Loses type information
2. **Panic and let supervisor restart** - Poor user experience

## Implementation Plan

1. Audit all `.expect()` and `.unwrap()` calls
2. Categorize as: test-only, truly-impossible, or should-be-result
3. Update function signatures to return `Result`
4. Update callers to handle errors
5. Add integration tests for error paths
