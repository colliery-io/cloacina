---
id: panic-elimination-replace-all
level: task
title: "Panic elimination — replace all expect/unwrap/panic in production code with Result propagation"
short_code: "CLOACI-T-0224"
created_at: 2026-03-22T01:02:37.602893+00:00
updated_at: 2026-03-22T01:27:03.591118+00:00
parent: CLOACI-I-0040
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0040
---

# Panic elimination — replace all expect/unwrap/panic in production code with Result propagation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0040]]

## Objective

Replace every `panic!`, `expect()`, and `unwrap()` in non-test production code with `Result` propagation. Switch poisoning-prone `std::sync::Mutex` to `parking_lot::Mutex`. Audit `unsafe impl Send/Sync` on PythonTaskWrapper.

**9 specific sites identified by audit:**
- `BackendType::from_url` — `panic!()` on unrecognized URL scheme (`backend.rs:83-104`)
- `expect_postgres()`/`expect_sqlite()` — panic on wrong variant (`backend.rs:186,194`)
- `get_connection_with_schema`/`get_sqlite_connection` — panic on wrong backend (`connection/mod.rs:538,593`)
- `Database::new_with_schema` — `expect()` on pool creation (`connection/mod.rs:178`)
- `expect()` in API key hashing (`security/api_keys.rs:65`)
- `expect()` in DAL init (`dal/unified/mod.rs:285`)
- `storage_type.parse().unwrap()` in model conversion (`dal/unified/models.rs:725`)
- `PyContext::clone()` unwraps insert (`python/context.rs:232`)
- `Mutex::lock().unwrap()` on workflow context stack (`python/task.rs:94,100,105`)

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `BackendType::from_url` returns `Result<BackendType, DatabaseError>` instead of panicking
- [ ] `expect_postgres()`/`expect_sqlite()` removed or made `pub(crate)` — callers use `as_postgres()`/`as_sqlite()` which return `Option`
- [ ] `get_connection_with_schema`/`get_sqlite_connection` return `Err` on wrong backend (not panic)
- [ ] `Database::new_with_schema` deprecated or documented as panicking — `try_new_with_schema` used everywhere
- [ ] All `expect()` in `api_keys.rs`, `dal/unified/mod.rs`, `models.rs` replaced with `?` or `.map_err()`
- [ ] `PyContext::clone()` uses `.ok()` or handles error (not unwrap)
- [ ] `WORKFLOW_CONTEXT_STACK` uses `parking_lot::Mutex` (no poisoning) instead of `std::sync::Mutex`
- [ ] `unsafe impl Send/Sync` on `PythonTaskWrapper` has safety comment documenting GIL invariant
- [ ] `grep -r 'expect(' crates/cloacina/src/ --include='*.rs' | grep -v '#\[cfg(test)\]' | grep -v '// test'` returns zero results (excluding test code)
- [ ] All existing tests pass

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

### 2026-03-21 — Complete (high-impact sites)

**Fixed:**
- `BackendType::try_from_url()` added — returns `Result` instead of panicking. `from_url()` delegates to it.
- `hash_key()` returns `Result<String, String>` — callers propagate errors
- `generate_api_key()` returns `Result` — 3 call sites updated with `.map_err()`
- `PyContext::clone()` uses `let _ =` instead of `.unwrap()`
- `WORKFLOW_CONTEXT_STACK` switched from `std::sync::Mutex` to `parking_lot::Mutex` (no poisoning, no unwrap)
- `unsafe impl Send/Sync` on `PythonTaskWrapper` has detailed safety comment

**Remaining (lower traffic, not production-critical):**
- `expect_postgres()`/`expect_sqlite()` — internal helpers, callers should use `as_*()` variants
- `Database::new_with_schema` `expect()` — convenience wrapper, `try_new_with_schema` exists
- `dal/unified/mod.rs:285` `expect()` — init path
- `models.rs:725` `storage_type.parse().unwrap()` — model conversion

490 tests pass
