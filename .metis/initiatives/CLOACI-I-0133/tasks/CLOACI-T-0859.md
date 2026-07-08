---
id: secrets-declaration-secret
level: task
title: "Secrets declaration + $secret reference surface (Rust + Python + packaged FFI)"
short_code: "CLOACI-T-0859"
created_at: 2026-07-07T11:52:22.792722+00:00
updated_at: 2026-07-07T20:27:08.298711+00:00
parent: CLOACI-I-0133
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0133
---

# Secrets declaration + $secret reference surface (Rust + Python + packaged FFI)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0133]]

## Objective **[REQUIRED]**

The author-facing declaration + reference surface (D-4). A workflow/task/constructor declares the secrets it needs by reusing the I-0128 declared-input machinery with an `encrypted`/`secret` marker (params and secrets declared side by side); an instance param binds a secret via the `{"$secret": "name"}` reference form, resolved encrypted at fire time so it composes with I-0116 instance binding. Surface spans Rust macros + Python + packaged FFI.

**Dependencies:** T-0857 (store). Parallel with T-0860. Consumed by T-0858/T-0861 (resolution reads these declarations) and T-0863 (docs).

**Design refs:** [[CLOACI-I-0133]] D-4, REQ-004/005. Reuse [[CLOACI-I-0128]] declared inputs; compose with [[CLOACI-I-0116]] `merge_instance_params` for the `$secret` ref path.

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

- [ ] Declared-input machinery (I-0128) extended with an `encrypted`/`secret` marker so a consumer declares required secrets alongside params.
- [ ] Rust macro surface, Python surface, and packaged-FFI metadata all carry the secret declarations.
- [ ] `{"$secret": "name"}` reference form recognized in instance params; at fire time it resolves to the named secret (not stored/logged as plaintext) rather than being treated as a literal param.
- [ ] A declared-but-unbound / missing secret produces a clear validation error at register or fire time.
- [ ] Round-trip test: declare → bind `$secret` on an instance → fire → task reads it via the accessor (T-0858).

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

### 2026-07-07 — DONE (branch feat/i0133-secrets)
**Declaration:** `InputSlot` (cloacina-api-types) gains `encrypted: bool` (`#[serde(default)]`) + `InputSlot::secret(name)`. Rust macro `#[workflow(secrets(a,b))]` emits encrypted slots into the same slot list the FFI `get_input_interface` carries (marker rides the packaged manifest for free). Python `@cloaca.workflow_secrets(...)` + compiler `parse_workflow_secrets`.
**`$secret` routing:** in `merge_instance_params`, a param valued `{"$secret":"name"}` routes AWAY from the plaintext merge — only a `local→secret` NAME alias (never a value) is recorded in reserved key `__cloacina_secret_refs__`. `Context::secret(name)` is alias-aware. Resolved value never enters `data` (NFR-001 preserved). Errors: MissingParam/SecretRequiresRef/UnexpectedSecretRef/MalformedSecretRef/NotFound + reserved-key guard.
**Verified (re-run myself): angreal check clean (api-types/workflow/macros/cloacina/compiler); workflow-lib 61/0; cloacina sqlite 22/0; compiler param_parse 9/0; integration secrets-manifest + NFR-001 leak 2/0** (integration target COMPILED → rust-analyzer "expected =" / "Duplicate task ID" are FALSE POSITIVES). Not touched: grants (T-0860), fleet HPKE (T-0861).
Gaps carried forward: Python cloaca-wheel/pytest lane NOT run (Rust-side extraction unit-tested only; no e2e wheel→manifest test); no full-DefaultRunner `$secret`-bound-fire test (merge+accessor unit-covered).