---
id: secrets-store-encrypted-tenant
level: task
title: "Secrets store — encrypted tenant table, per-tenant DEK, metadata-only CRUD"
short_code: "CLOACI-T-0857"
created_at: 2026-07-07T11:52:19.726122+00:00
updated_at: 2026-07-07T14:24:14.305812+00:00
parent: CLOACI-I-0133
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0133
---

# Secrets store — encrypted tenant table, per-tenant DEK, metadata-only CRUD

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0133]]

## Objective **[REQUIRED]**

Foundation task (D-1/D-7). The encrypted `secrets` store: a tenant-scoped table holding `(name, {field: ciphertext})`, encrypted at rest under a **per-tenant data key (DEK)** wrapped by a server KEK (envelope encryption), a DAL, and a CRUD API whose reads return **metadata only** (never plaintext). Everything else in I-0133 depends on this.

**Dependencies:** none (first task). Blocks T-0858, T-0859, T-0860, T-0862.

**Design refs:** [[CLOACI-I-0133]] D-7 (per-tenant DEK), REQ-001/002. Reuse `crates/cloacina/src/crypto/key_encryption.rs` (AEAD primitive) + `db_key_manager`. Schema-per-tenant → one `secrets` table per tenant schema. Prefer ADD COLUMN/CREATE migrations (no DROP+CREATE on sqlite).

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

- [ ] `secrets` table exists per tenant schema (migration, sqlite + postgres), storing name + per-field ciphertext + metadata (field names, timestamps).
- [ ] Per-tenant DEK generated on first use, wrapped by the server KEK; DEK unwrap happens server-side only.
- [ ] DAL: create / get-metadata / list-metadata / rotate (replace values) / delete.
- [ ] CRUD read paths return **metadata only** — a test asserts no plaintext field value is ever returned by list/get.
- [ ] Values round-trip: encrypt on write, decrypt on internal resolve; a unit test proves ciphertext at rest ≠ plaintext.
- [ ] Tenant isolation: a secret in tenant A is not readable from tenant B's schema (test).

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
Implemented mirroring `db_key_manager.rs`. New: `security/secret_store.rs` (`SecretStore` + `SecretError` + `SecretMetadata`, 10 sqlite tests), migration `041_create_secrets` (postgres + sqlite, `tenant_data_keys` + `secrets`, CREATE-only), schema.rs + `dal/unified/models.rs` (`TenantDataKey`/`Secret`), `crypto` `encrypt_bytes`/`decrypt_bytes` aliases. Per-tenant DEK wrapped by a caller-supplied 32-byte server KEK; secret field-map JSON encrypted under the DEK; list/get return metadata only. **Verified: angreal check clean; `angreal test unit secret_store` = 10 passed/0 failed** (no-plaintext-in-metadata AND raw-ciphertext-column ≠ plaintext; resolve round-trip; rotate; wrong-KEK fails; tenant isolation; delete; dup-name). rust-analyzer "second test attribute" warnings are FALSE POSITIVES (angreal green). Gap for T-0858+: no server config sources the KEK into a `SecretStore` yet. `resolve_secret` is the internal decrypt primitive later tasks call.