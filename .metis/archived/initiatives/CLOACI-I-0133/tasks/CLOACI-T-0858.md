---
id: secrets-accessor-no-leak
level: task
title: "Secrets accessor + no-leak resolution (embedded path) + NFR-001 leak test"
short_code: "CLOACI-T-0858"
created_at: 2026-07-07T11:52:21.227332+00:00
updated_at: 2026-07-07T16:46:02.344869+00:00
parent: CLOACI-I-0133
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0133
---

# Secrets accessor + no-leak resolution (embedded path) + NFR-001 leak test

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0133]]

## Objective **[REQUIRED]**

The `Secrets` accessor (D-1) and the no-leak resolution path for the **embedded / in-process** execution (server IS the executor — no envelope needed). A task/constructor reads resolved fields via `ctx.secret("name")`, structurally distinct from `Context` so a secret is NEVER serialized into the durable context / `schedules.params` / fires log. Ships with the **NFR-001 leak test** as the gate for the whole initiative.

**Dependencies:** T-0857 (store). Blocks T-0861 (fleet path reuses this accessor + leak guarantee).

**Design refs:** [[CLOACI-I-0133]] D-1, REQ-003, NFR-001. Contrast the params seam `workflow_instance.rs::merge_instance_params` (which folds into `Context` — the thing to avoid).

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

- [ ] A `Secrets` accessor on the task execution scope (`ctx.secret("name")` → resolved fields), distinct from `Context`.
- [ ] Resolution happens at fire/run time for the in-process path; resolved plaintext lives only in the accessor for the run.
- [ ] The accessor value is NOT part of `Context` serialization — a compile/type-level or test guarantee it can't be `.insert()`ed into the durable context.
- [ ] **NFR-001 leak test (the gate):** run a workflow that resolves a secret, then assert the plaintext appears in NONE of: `schedules.params`, the serialized `Context`, the fires log, audit rows, execution history, or stdout/stderr logs.
- [ ] Accessor returns a clear error when a named secret is absent or not granted (ties to T-0860).

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
Accessor lives on `Context` (cloacina-workflow): new `secret.rs` (`SecretResolver` trait + errors); `Context` gains `secrets: Option<Arc<dyn SecretResolver>>` (Context serializes ONLY `data`, so the resolver is structurally unserializable — the `#[serde(skip)]` equivalent), manual `Debug` redacts it, async `context.secret(name)`/`secret_field(name,field)`. Backend: `security/secret_resolver.rs::SecretStoreResolver` over `SecretStore`+org_id+KEK; KEK from **`CLOACINA_SECRET_KEK`** (base64/hex, 32 bytes) or `new(store,org_id,kek)`. Threaded embedded/runner→executor→context via `DefaultRunnerBuilder::secret_resolver(...)`; unconfigured → `NotConfigured`, missing name → `NotFound`.
**Verified (re-run myself): angreal check both crates clean; workflow secret tests 4/0; cloacina resolver+workflow 15/0; NFR-001 leak test 1/0** (plaintext absent from serialized Context, durable `contexts` rows, `schedules.params`; task confirmed it DID resolve). Gaps for later: cloacina-server doesn't yet construct a per-tenant `SecretStoreResolver` (needs tenant→org_id map + KEK read + `.secret_resolver()` per runner); stdout/stderr covered structurally; no zeroize on KEK Vec.