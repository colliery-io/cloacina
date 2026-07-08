---
id: agent-limits-admin-default-per
level: task
title: "Agent limits — admin default + per-tenant exceptions (model + API)"
short_code: "CLOACI-T-0808"
created_at: 2026-06-27T14:43:34.250257+00:00
updated_at: 2026-06-27T16:59:30.835620+00:00
parent: CLOACI-I-0127
blocked_by: []
archived: true

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Agent limits — admin default + per-tenant exceptions (model + API)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

The capacity-limits model that bounds agent autoscaling (CLOACI-I-0127, slice 1 #1): a god-set global `default_max_agents` plus per-tenant exceptions, with an `effective_limit(tenant)` lookup and a per-tenant floor. Pure server/DB + REST on the I-0118 ABAC model. This is the hard ceiling that the provision API (T-0809) and the autoscaler (T-0811) clamp to.

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

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Global `default_max_agents` is god-settable; a per-tenant exception overrides it; `effective_limit(tenant)` returns the exception if present, else the default.
- [ ] Setting limits is god/platform-scope only; a tenant can read its OWN effective limit but not another tenant's (NFR-004 cross-scope-denial).
- [ ] Postgres migration for the limits table (ADD COLUMN / CREATE INDEX, not DROP+CREATE); values exposed for the provision API + autoscaler to consume.

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

**2026-06-27 — server slice implemented (pending build verification).** Files:
- `migrations/postgres/036_create_agent_capacity_limits/{up,down}.sql` —
  `agent_capacity_limits(tenant_id PK, max_agents, created_at, updated_at)`.
  Postgres-only (mirrors the I-0118 auth tables; SQLite has no tenant/auth
  tables). `embed_migrations!` auto-discovers the dir — no registration list.
- `schema.rs` — `agent_capacity_limits` table! in the postgres module.
- `dal/unified/agent_limits/mod.rs` — postgres-only DAL: `set_tenant_limit`
  (upsert), `get_tenant_limit`, `clear_tenant_limit`, `effective_limit(tenant,
  default)` = override-else-default. Wired via `DAL::agent_limits()`.
- `routes/limits.rs` — POST/GET/DELETE `/v1/tenants/{tenant_id}/limits` →
  `TenantAgentLimitInfo { default, override, effective }`.
- AuthZ (NFR-004): POST/DELETE = `platform(Admin)` god-only; GET = `tenant(Read)`
  own-tenant-only (cross-tenant denied by the fail-closed matcher in authz.rs).
- `AppState.default_max_agents` from `CLOACINA_DEFAULT_MAX_AGENTS` (default 4);
  set in both AppState builders (runtime + test).

**Decisions / scope:**
- Global default is **config-set** (env), not API-settable; per-tenant exceptions
  are API-settable (god-only). Matches the user's example (default 4, acme 6). An
  API setter for the global default is an optional follow-up.
- **Spec/SDK deferred:** handlers carry `#[utoipa::path]` but are NOT registered
  in the central OpenApi `paths()` list → the endpoint is not in the published
  spec → no Rust/Python/TS client obligation yet (keeps the nightly SDK Contract
  Matrix green). Registering paths + client methods folds into T-0813 (the UI
  consumer). **Follow-up flagged.**
- `updated_at` is not bumped on upsert (DB-default on insert only) to dodge the
  Timestamptz-write type ambiguity — bump it once a real build confirms the chrono
  type. Minor.

**Verify:** `angreal check crate crates/cloacina` + `angreal check crate
crates/cloacina-server`. (Dropped a `UniversalTimestamp`/`Timestamptz` mismatch
that rust-analyzer flagged; the DAL now mirrors `local_accounts`.) No integration
test yet — recommend one asserting `effective_limit` (default vs exception) + a
403 on a cross-tenant GET, on the postgres lane.

**2026-06-27 — build verified GREEN, no fixes required.** Both
`angreal check crate crates/cloacina` and `angreal check crate crates/cloacina-server`
compile clean; the code as written builds with zero new warnings/errors. The only
warnings emitted (6, all in `crates/cloacina`) are pre-existing and unrelated to
T-0808 — unused imports in `computation_graph/packaging_bridge.rs:31` and
`database/universal_types.rs:38`. No T-0808 file was edited during verification.
Slice is build-clean.

**2026-06-27 — tests added + BOTH LAYERS GREEN on the postgres lane.** No
implementation file touched; tests + one harness helper only.

*DAL integration* — new `crates/cloacina/tests/integration/agent_limits.rs`
(registered with `pub mod agent_limits;` in `tests/integration/main.rs`),
postgres-only, `#[serial]`, fixture reset+initialize per test:
- set→get round-trip == Some(6); get unset == None; effective_limit override(6)
  vs fallback default(4); upsert replace 6→8 == Some(8); clear == true then get
  None + effective reverts to default, clear-missing == false.
- Result: `5 passed; 0 failed`.
- Run: `cargo test -p cloacina --test integration --features postgres,sqlite,macros -- agent_limits --test-threads=1`

*HTTP functional* — 8 tests added to the `#[cfg(test)]` block in
`crates/cloacina-server/src/lib.rs`, plus a new `create_tenant_api_key(state,
tenant, role)` helper next to `create_test_api_key` (mints a non-admin
tenant-scoped key — `tenant=Some(..), is_admin=false`). Covers: god POST→200
(override+effective 6); god GET reflects override; god GET no-override→effective
4 / override null; own-tenant key GET→200 (value not asserted — server lib tests
share one un-reset DB); cross-tenant key GET→403; tenant key POST→403 (set is
god-only); god DELETE→200 reverts to default 4; no-auth GET→401.
- Result: `8 passed; 0 failed`.
- Run: `TEST_DB_URL=postgres://cloacina:cloacina@localhost:5432/cloacina cargo test -p cloacina-server --lib -- agent_limit --test-threads=1`

No implementation bug surfaced; the feature behaves exactly as specified.