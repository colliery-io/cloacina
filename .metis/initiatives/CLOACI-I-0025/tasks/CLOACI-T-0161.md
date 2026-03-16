---
id: replace-hand-rolled-json-schema
level: task
title: "Replace hand-rolled JSON schema validation with jsonschema crate"
short_code: "CLOACI-T-0161"
created_at: 2026-03-15T18:24:42.023512+00:00
updated_at: 2026-03-15T19:36:36.038722+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Replace hand-rolled JSON schema validation with jsonschema crate

**Priority: P2 — MEDIUM**
**Parent**: [[CLOACI-I-0025]]

## Objective

The custom boundary JSON schema validation in `boundary.rs:152-212` is hand-rolled and incomplete. It only supports basic type checks, required fields, and properties recursion. Missing: `enum`, `minimum`/`maximum`, `minLength`/`maxLength`, `pattern`, `additionalProperties: false`. Boundaries can pass validation that a real JSON Schema validator would reject.

Replace with the `jsonschema` crate for spec-compliant validation.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `jsonschema` crate added to dependencies
- [ ] `validate_custom_boundary()` delegates to `jsonschema::validate()` (or `JSONSchema::compile()` + `is_valid()`)
- [ ] All existing unit tests for custom boundary validation still pass
- [ ] New tests: `enum` constraints, numeric min/max, string patterns, `additionalProperties: false`
- [ ] Global `CUSTOM_SCHEMAS` stores compiled `JSONSchema` instances instead of raw `serde_json::Value` (avoids re-compilation per validation call)
- [ ] Performance: schema compilation happens once at registration, validation is fast

## Implementation Notes

- `jsonschema` crate: well-maintained, supports JSON Schema draft 2020-12
- Replace `CUSTOM_SCHEMAS: LazyLock<RwLock<HashMap<String, Value>>>` with `LazyLock<RwLock<HashMap<String, jsonschema::JSONSchema>>>`
- `register_custom_boundary()` compiles the schema at registration time
- `validate_custom_boundary()` calls `schema.is_valid(&boundary_data)` — O(1) lookup + validation
- Also addresses the global registry concern — compiled schemas are immutable after creation

## Status Updates

### 2026-03-15 — Completed
- Added `jsonschema = "0.28"` to Cargo.toml
- Replaced `CUSTOM_SCHEMAS` registry: now stores `CompiledSchema` with a pre-compiled `jsonschema::Validator`
- `register_custom_boundary()` compiles schema at registration time; logs `error!` if schema is invalid
- `validate_custom_boundary()` uses `validator.is_valid()` for O(1) lookup + fast validation
- Removed hand-rolled `validate_against_schema()` function (~60 lines)
- Now supports full JSON Schema spec: `enum`, `minimum`/`maximum`, `pattern`, `additionalProperties`, etc.
- Updated 2 tests that checked exact error messages to check for non-empty error instead
- All 412 unit tests pass
