---
id: create-schemaerror-type-and
level: task
title: "Create SchemaError type and validate_schema_name function"
short_code: "CLOACI-T-0019"
created_at: 2025-12-06T01:40:34.940143+00:00
updated_at: 2025-12-06T01:40:34.940143+00:00
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

# Create SchemaError type and validate_schema_name function

## Parent Initiative

[[CLOACI-I-0005]]

## Objective

Create a dedicated `SchemaError` enum and a `validate_schema_name()` function to prevent SQL injection attacks in PostgreSQL schema operations. The validation function will enforce safe schema naming rules before any schema name is used in raw SQL queries.

## Acceptance Criteria

- [ ] `SchemaError` enum created with variants for: invalid length, invalid start character, invalid characters, and reserved names
- [ ] `validate_schema_name()` function enforces PostgreSQL identifier rules
- [ ] Maximum length check (63 characters - PostgreSQL limit)
- [ ] Must start with letter or underscore
- [ ] Only alphanumeric and underscore characters allowed
- [ ] Reserved PostgreSQL schema names rejected (public, pg_catalog, information_schema, pg_temp)
- [ ] Function returns `Result<&str, SchemaError>` for zero-copy validation

## Implementation Notes

### Location
Add to `crates/cloacina/src/database/connection.rs` or create a new `schema.rs` module

### SchemaError Variants
```rust
#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("Schema name length invalid: '{name}' (max {max} characters)")]
    InvalidLength { name: String, max: usize },

    #[error("Schema name must start with letter or underscore: '{0}'")]
    InvalidStart(String),

    #[error("Schema name contains invalid characters: '{0}'")]
    InvalidCharacters(String),

    #[error("Schema name is reserved: '{0}'")]
    ReservedName(String),
}
```

### Validation Rules
- PostgreSQL identifiers limited to 63 bytes (NAMEDATALEN - 1)
- Must start with letter (a-z, A-Z) or underscore
- Subsequent characters: letters, digits (0-9), underscores
- Case-insensitive comparison for reserved names

### Dependencies
None - this is foundational for T-0020

## Status Updates

*To be added during implementation*
