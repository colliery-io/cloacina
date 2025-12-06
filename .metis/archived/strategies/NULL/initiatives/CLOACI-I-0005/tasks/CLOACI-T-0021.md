---
id: add-comprehensive-tests-for-schema
level: task
title: "Add comprehensive tests for schema validation"
short_code: "CLOACI-T-0021"
created_at: 2025-12-06T01:40:35.251685+00:00
updated_at: 2025-12-06T02:38:48.478017+00:00
parent: CLOACI-I-0005
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0005
---

# Add comprehensive tests for schema validation

## Parent Initiative

[[CLOACI-I-0005]]

## Objective

Create comprehensive unit tests that verify the schema validation correctly rejects all SQL injection attempts and accepts valid schema names. Tests should cover edge cases and ensure the security fix is robust.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Tests for valid schema names (simple, with underscores, with numbers)
- [ ] Tests for SQL injection attempts (semicolons, comments, quotes)
- [ ] Tests for length boundary conditions (0, 1, 63, 64 characters)
- [ ] Tests for invalid start characters (numbers, special chars)
- [ ] Tests for reserved PostgreSQL schema names
- [ ] Tests for Unicode/non-ASCII characters
- [ ] Tests verify correct error variants are returned
- [ ] All tests pass

## Test Cases

### Valid Schema Names
- `my_schema` - simple valid name
- `_private` - starts with underscore
- `schema123` - contains numbers
- `a` - single character
- 63-character name - maximum valid length

### SQL Injection Attempts
- `test; DROP TABLE users; --` - command injection
- `test' OR '1'='1` - quote injection
- `test/*comment*/` - comment injection
- `test\x00null` - null byte injection

### Invalid Names
- `` (empty) - InvalidLength
- `123abc` - InvalidStart (starts with number)
- `my-schema` - InvalidCharacters (hyphen)
- `my.schema` - InvalidCharacters (dot)
- `my schema` - InvalidCharacters (space)
- 64+ characters - InvalidLength

### Reserved Names
- `public` - ReservedName
- `pg_catalog` - ReservedName
- `information_schema` - ReservedName
- `pg_temp` - ReservedName
- `PUBLIC` - ReservedName (case insensitive)

## Implementation Notes

### Location
Add tests to the existing `#[cfg(test)] mod tests` block in `connection.rs`

### Dependencies
- T-0019 (SchemaError and validate_schema_name must exist)
- T-0020 (integration with methods)

## Status Updates

*To be added during implementation*
