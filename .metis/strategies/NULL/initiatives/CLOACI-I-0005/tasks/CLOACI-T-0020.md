---
id: apply-schema-validation-to-setup
level: task
title: "Apply schema validation to setup_schema and get_connection_with_schema"
short_code: "CLOACI-T-0020"
created_at: 2025-12-06T01:40:35.031718+00:00
updated_at: 2025-12-06T01:40:35.031718+00:00
parent: CLOACI-I-0005
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0005
---

# Apply schema validation to setup_schema and get_connection_with_schema

## Parent Initiative

[[CLOACI-I-0005]]

## Objective

Integrate the `validate_schema_name()` function into all code paths that use schema names in raw SQL queries. This eliminates the SQL injection vulnerability by ensuring malicious schema names are rejected before reaching the database.

## Acceptance Criteria

- [ ] `setup_schema()` method calls `validate_schema_name()` before any SQL operations
- [ ] `get_connection_with_schema()` method validates schema before SET search_path
- [ ] Invalid schema names return clear error messages (not panic)
- [ ] Error type properly propagates through async boundaries
- [ ] Existing functionality preserved for valid schema names

## Implementation Notes

### Affected Methods in connection.rs

1. **`setup_schema()`** (lines 387-431)
   - Line 404: `format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name)`
   - Line 413: `format!("SET search_path TO {}, public", schema_name_clone)`

2. **`get_connection_with_schema()`** (lines 437-465)
   - Line 458: `format!("SET search_path TO {}, public", schema_name)`

### Changes Required

```rust
pub async fn setup_schema(&self, schema: &str) -> Result<(), String> {
    // Add validation at the start
    let validated = validate_schema_name(schema)
        .map_err(|e| e.to_string())?;

    // Use validated name in SQL...
}
```

### Error Handling
- Convert `SchemaError` to `String` for compatibility with existing return type
- Consider future refactor to use proper error enum

### Dependencies
- T-0019 must be completed first (provides `validate_schema_name()`)

## Status Updates

*To be added during implementation*
