---
id: python-secret-read-accessor
level: task
title: "Python secret read accessor — context.secret()/secret_field() PyO3 binding"
short_code: "CLOACI-T-0864"
created_at: 2026-07-08T01:11:22.790678+00:00
updated_at: 2026-07-08T01:32:11.880209+00:00
parent: CLOACI-I-0133
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0133
---

# Python secret read accessor — context.secret()/secret_field() PyO3 binding

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0133]]

## Objective **[REQUIRED]**

Close the Python read gap (surfaced by T-0863): Python packaged workflows can DECLARE (`@cloaca.workflow_secrets`) + BIND (`{"$secret":...}`) secrets but a Python task body CANNOT read the resolved value — `context.secret()`/`secret_field()` are Rust-`Context`-only. Add the PyO3 accessor so Python reaches read parity (Python is a core capability).

**Dependencies:** T-0858 (Rust accessor + `SecretResolver`), T-0859 (`$secret` alias map). Last piece of I-0133.

**Design refs:** [[CLOACI-I-0133]] D-1/D-4. `cloacina-python/src/context.rs` / `bindings/context.rs` (the Python Context wrapping the Rust `Context`); call through to the SAME async `SecretResolver` the Rust `Context::secret` uses, bridging async→sync the way other cloacina-python bindings do. Respect [[project_scenario32_cg_invocation_deadlock]] — never `with_gil` inside an async executor body.

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

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Python `context.secret(name)` returns the resolved field map; `context.secret_field(name, field)` returns one field — via the SAME resolver as Rust (alias-aware through the `$secret` map).
- [ ] Async→sync bridged with the codebase's existing pattern; no GIL/deadlock footgun (never with_gil in an async executor body).
- [ ] Clear Python exceptions for not-configured / not-found / not-granted.
- [ ] Resolved value never enters the Python Context's serialized data (no-leak preserved on the Python path).
- [ ] A test proves read works + a no-leak assertion.

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

### 2026-07-08 — DONE (branch feat/i0133-secrets)
PyO3 `Context.secret(name)`→dict / `Context.secret_field(name,field)`→str in `cloacina-python/src/context.rs`, calling through to Rust `Context::secret`/`secret_field` (same resolver, alias-aware). Async→sync via `block_on_secret_access` = `py.allow_threads` then `Handle::current().block_on` (same pattern as `PyTaskHandle::defer_until`; GIL released before await → safe per scenario-30/32/33). Errors: NotConfigured/Backend→RuntimeError, NotFound/FieldNotFound→KeyError, NotGranted→PermissionError.
**LOAD-BEARING BUG FOUND + FIXED:** `PyContext::clone` rebuilt via `Context::new()`+re-insert, DROPPING the non-serialized `secrets` resolver — so `context.secret()` would fail `NotConfigured` on the REAL execution path (task.rs hands the body `py_context.clone()`). Fixed to delegate to `Context::clone_data()`. `test_clone_preserves_secret_resolver` guards it.
Doc callout in `use-secrets.md` flipped from "no Python read" → documented parity + example.
**Verified (re-run myself): cloacina-python context::tests 29/0 (clone-preserves-resolver, alias-aware, no-leak, error variants); full lib 135/0; angreal check clean; docs build clean.** Not run: full cloaca wheel/pytest lane (PyO3 rpath issue locally) — Rust-side binding tests exercise the real PyContext methods.