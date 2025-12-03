---
id: fix-sql-injection-risk-in-schema
level: initiative
title: "Fix SQL Injection Risk in Schema Setup"
short_code: "CLOACI-I-0005"
created_at: 2025-11-29T02:40:07.115570+00:00
updated_at: 2025-11-29T02:40:07.115570+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: S
strategy_id: NULL
initiative_id: fix-sql-injection-risk-in-schema
---

# Fix SQL Injection Risk in Schema Setup Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

SQL injection vulnerability exists in `cloacina/src/database/connection.rs` (lines 221-236) where schema names are directly interpolated into SQL queries using `format!()`:

```rust
let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name);  // Line 222
diesel::sql_query(&create_schema_sql).execute(conn)

let set_search_path_sql = format!("SET search_path TO {}, public", schema_name_clone);  // Line 231
diesel::sql_query(&set_search_path_sql).execute(conn)
```

**Risk:** Schema names come from user input (multi-tenant configurations). A malicious schema name like `test; DROP TABLE users; --` would execute arbitrary SQL.

While Diesel usually prevents SQL injection through parameterized queries, `sql_query()` bypasses that safety by executing raw SQL strings.

## Goals & Non-Goals

**Goals:**
- Sanitize all schema names before use in SQL
- Reject invalid schema names with clear error messages
- Add validation at the API boundary (not just at SQL execution)

**Non-Goals:**
- Changing multi-tenant architecture
- Adding schema name configuration UI

## Detailed Design

### Schema Name Validation Function

Create a validation function that enforces safe schema names:

```rust
const MAX_SCHEMA_NAME_LENGTH: usize = 63;  // PostgreSQL limit

fn validate_schema_name(name: &str) -> Result<&str, SchemaError> {
    // Check length
    if name.is_empty() || name.len() > MAX_SCHEMA_NAME_LENGTH {
        return Err(SchemaError::InvalidLength {
            name: name.to_string(),
            max: MAX_SCHEMA_NAME_LENGTH
        });
    }

    // Must start with letter or underscore
    if !name.chars().next().map(|c| c.is_ascii_alphabetic() || c == '_').unwrap_or(false) {
        return Err(SchemaError::InvalidStart(name.to_string()));
    }

    // Only allow alphanumeric and underscore
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(SchemaError::InvalidCharacters(name.to_string()));
    }

    // Reject SQL keywords
    const RESERVED: &[&str] = &["public", "pg_catalog", "information_schema", "pg_temp"];
    if RESERVED.contains(&name.to_lowercase().as_str()) {
        return Err(SchemaError::ReservedName(name.to_string()));
    }

    Ok(name)
}
```

### Apply Validation at Entry Points

Update `setup_schema()` to validate before use:

```rust
pub fn setup_schema(conn: &mut PgConnection, schema_name: &str) -> Result<(), DatabaseError> {
    let validated_name = validate_schema_name(schema_name)?;

    // Now safe to use in format! since we've validated the input
    let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS {}", validated_name);
    diesel::sql_query(&create_schema_sql).execute(conn)?;

    let set_search_path_sql = format!("SET search_path TO {}, public", validated_name);
    diesel::sql_query(&set_search_path_sql).execute(conn)?;

    Ok(())
}
```

## Testing Strategy

- Test with SQL injection attempts: `; DROP TABLE`, `--`, `'OR 1=1`
- Test with Unicode characters
- Test with maximum length names
- Test with reserved PostgreSQL names
- Test with empty and whitespace-only names

## Alternatives Considered

1. **Use quoted identifiers** (`"schema_name"`) - Still requires escaping internal quotes, doesn't solve the problem
2. **Parameterized DDL** - PostgreSQL doesn't support parameterized schema names in DDL
3. **Allowlist approach** - Too restrictive for dynamic multi-tenant use cases

## Implementation Plan

1. Create `SchemaError` error type
2. Implement `validate_schema_name()` function
3. Add validation call in `setup_schema()`
4. Add validation call in `set_search_path()`
5. Add comprehensive test cases
6. Update documentation
