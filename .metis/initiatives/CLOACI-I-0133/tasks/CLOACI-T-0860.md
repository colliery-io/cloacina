---
id: secrets-grant-integration-named
level: task
title: "Secrets grant integration — named allow-list in GrantSpec, fail-closed enforcement"
short_code: "CLOACI-T-0860"
created_at: 2026-07-07T11:52:24.269469+00:00
updated_at: 2026-07-07T11:52:24.269469+00:00
parent: CLOACI-I-0133
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0133
---

# Secrets grant integration — named allow-list in GrantSpec, fail-closed enforcement

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0133]]

## Objective **[REQUIRED]**

Grant-gated access (D-3). Add a `secrets = [...]` named allow-list to `GrantSpec` alongside `capabilities`/`egress`, fail-closed (empty = none), with tenant-scope as the outer boundary. Enforce at resolution: a package/constructor resolves ONLY the secrets its tenant granted it.

**Dependencies:** T-0857 (store). Parallel with T-0859. Consumed by T-0858/T-0861 (resolution checks the grant) and T-0861 (fleet).

**Design refs:** [[CLOACI-I-0133]] D-3, REQ-006. Insertion point: `crates/cloacina/src/registry/loader/grants.rs` (`GrantSpec`, `translate`, `ResolvedGrants`) — mirror the existing fail-closed egress/capability list shape. [[CLOACI-I-0132]] constructors.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] `GrantSpec` gains a `secrets` allow-list (named), lowered through `translate` into `ResolvedGrants`.
- [ ] Fail-closed: an empty/absent secrets grant means the consumer can resolve NO secrets.
- [ ] Resolution enforces the grant: resolving a secret not in the allow-list errors (test: granted name succeeds, ungranted name denied).
- [ ] Tenant-scope is the outer boundary — a grant can only name secrets in the caller's tenant.
- [ ] Grant surface is authorable (macro/manifest) and visible in the resolved-grants audit/log line (no secret VALUES logged, only names).

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

*To be added during implementation*