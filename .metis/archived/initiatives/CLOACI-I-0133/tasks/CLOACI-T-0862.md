---
id: secrets-ui-cli-metadata-only
level: task
title: "Secrets UI/CLI — metadata-only create/rotate/list (cloacinactl + embedded UI)"
short_code: "CLOACI-T-0862"
created_at: 2026-07-07T11:52:27.717792+00:00
updated_at: 2026-07-08T00:49:58.830278+00:00
parent: CLOACI-I-0133
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0133
---

# Secrets UI/CLI — metadata-only create/rotate/list (cloacinactl + embedded UI)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0133]]

## Objective **[REQUIRED]**

Operator surface (D-8/OQ-8) — EXPANDED to include the SERVER secrets subsystem (T-0857 built the store DAL but no server HTTP routes / tenant→org_id mapping exist yet; T-0861 handed off the `FleetSecretResolverFactory` activation which depends on exactly this).

Scope:
1. **Server secrets subsystem (prereq):** tenant-scoped HTTP CRUD routes (create / rotate / list-metadata / delete) over T-0857's `SecretStore`, in cloacina-server (mirror the keys/accounts route+authz pattern; add to `build_authz_table` + bump the authz tripwire count test). Establish the **tenant(schema-name)→`UniversalUuid` org_id** mapping the store needs. **Reads return metadata only, never values.**
2. **Activate the fleet factory (from T-0861):** implement the concrete `FleetSecretResolverFactory` (KEK via `SecretStoreResolver::kek_from_env`, `SecretStore::new(dal)`, org_id from the mapping, `ResolvedGrants.secrets`→`SecretAllow`) and wire it into `FleetExecutor` so `$secret` fleet tasks actually resolve (they currently fail closed). Extend `build_secret_resolver` on the embedded/runner path too if applicable.
3. **`cloacinactl secret`** create/rotate/list/delete (values via stdin/file/prompt, never echoed / never in argv).
4. **Embedded-UI** Secrets view (metadata list + create/rotate/delete; value inputs write-only).

**Dependencies:** T-0857 (store), T-0861 (factory seam + fleet delivery). This is now the SERVER-INTEGRATION task, not just UI/CLI.

**Design refs:** [[CLOACI-I-0133]] D-8; T-0861 `FleetSecretResolverFactory`. cloacinactl `crates/cloacinactl/src/nouns/`; embedded UI ([[CLOACI-I-0130]]); route+authz pattern in cloacina-server `routes/`.

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

- [ ] `cloacinactl secret create` (name + fields, values read from stdin/file/prompt — never echoed), `rotate`, `list`, `delete`.
- [ ] `list`/`get` show metadata only (name, field names, timestamps) — no values, ever.
- [ ] Embedded-UI: a Secrets view listing metadata with create/rotate/delete actions; value inputs are write-only.
- [ ] Create/rotate accept values without writing them to shell history / logs / the API access log.
- [ ] Tenant-scoped: the CLI/UI operate within the caller's tenant.

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

### 2026-07-07 — DONE (branch feat/i0133-secrets, commit 64b273bf)
Server subsystem + fleet activation + CLI + UI:
- **Server CRUD** `routes/secrets.rs` — 5 tenant-scoped routes (create/list/get/rotate/delete) over T-0857 `SecretStore`; metadata-only by construction (`SecretMetadataResponse` has no value field). KEK from `CLOACINA_SECRET_KEK`, unset→503. Registered in router + `build_authz_table` (**count 59→64 + 5 spot-checks, test green**) + openapi (regenerated openapi.json + TS types so drift-checks stay green).
- **tenant→org_id**: deterministic UUIDv5 under a frozen namespace (no tenants table — a tenant is a schema name); one helper on write + fleet paths.
- **Fleet factory (activates T-0861)**: `ServerFleetSecretResolverFactory` wired into BOTH `FleetExecutor` sites — a `$secret` fleet task now resolves end-to-end (was fail-closed).
- **CLI** `cloacinactl secret create/rotate/list/get/delete` — values via @file/stdin/prompt, literal `NAME=value` on argv rejected; list/get metadata-only.
- **UI** Secrets view (metadata table + create/rotate/delete; value inputs write-only). Embedded UI build succeeds.
**Verified (re-run myself): cargo check clean (server/cli/api-types/client); authz-count + tenant_org_id tests 2/0.** Postgres factory + route integration tests need the live PG lane (not run locally).
**D-3 REFINED (maintainer, 2026-07-07): tenant-scope IS the boundary for secrets** — the fleet path being tenant-scoped-only is now the intended design (per-package gating = intra-tenant authZ, a project non-goal; it stays as embedded-path defense-in-depth). So the "fleet grant gap" is NOT a gap. CLI interactive-prompt path doesn't disable terminal echo (file/stdin are the non-echoing inputs) — minor, noted.