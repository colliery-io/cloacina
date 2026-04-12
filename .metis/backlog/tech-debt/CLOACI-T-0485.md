---
id: enforce-tenant-schema-isolation-at
level: task
title: "Enforce tenant schema isolation at the DAL layer for server mode"
short_code: "CLOACI-T-0485"
created_at: 2026-04-11T15:51:33.003385+00:00
updated_at: 2026-04-11T15:51:33.003385+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Enforce tenant schema isolation at the DAL layer for server mode

## Objective

Tenant-scoped HTTP handlers check `auth.can_access_tenant()` but then query the shared admin database connection. All DAL queries return global data regardless of tenant_id in the URL. A tenant-scoped key for tenant_a sees all tenants' data.

Split from CLOACI-T-0473 (which covered the WebSocket ticket fix). This task addresses SEC-006.

## Review Finding References

SEC-006 (from architecture review `review/10-recommendations.md` REC-001)

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P0 - Critical (blocks users/revenue)

### Technical Debt Impact
- **Current Problems**: Server creates a single `DefaultRunner` with admin DB connection (serve.rs:157). All tenant-scoped handlers use this shared runner/database. `list_executions`, `execute_workflow`, `list_workflows`, etc. return/operate on global data.
- **Benefits of Fixing**: Multi-tenant deployments become actually isolated at the data layer.
- **Risk Assessment**: Without this, the schema isolation promise is not realized. Cross-tenant data leakage is possible.

## Acceptance Criteria

- [ ] Tenant-scoped handlers query only their tenant's schema
- [ ] A tenant-scoped key for tenant_a cannot see tenant_b's executions/workflows
- [ ] `execute_workflow` schedules into the correct tenant schema
- [ ] Integration test: create two tenants, execute workflow in each, verify isolation

## Implementation Notes

### Design Decision: Option B — Per-tenant `Database` instances (decided 2026-04-11)

Chosen for clean isolation without search_path leakage risk. Recommended deployment is one server per tenant, but Option B provides defense-in-depth for multi-tenant processes.

**Implementation plan:**
1. Add `TenantDatabaseCache` to `AppState` — `RwLock<HashMap<String, Database>>` lazily populated via `Database::try_new_with_schema()`
2. Add helper `fn resolve_tenant_db(state: &AppState, tenant_id: &str) -> Result<Database>` that checks cache, creates on miss
3. Read handlers (`list_executions`, `list_workflows`, `get_execution`, `get_execution_events`) create `DAL::new(tenant_db)` instead of using `state.database`
4. `execute_workflow` creates a tenant-scoped DAL for the insert; the shared scheduler picks up the execution from the tenant schema's tables
5. `upload_workflow`, `get_workflow`, `delete_workflow` use tenant-scoped registry storage
6. Validate tenant schema exists before creating Database (reject unknown tenants at the DB layer, not just auth)

**Connection budget:** Each tenant gets a small pool (e.g., `pool_size=2`). With per-tenant deployment as best practice, this is typically 1 tenant = 1 cache entry.

### Affected Handlers
- `executions.rs`: `execute_workflow` (L68), `list_executions` (L107), `get_execution` (L154), `get_execution_events` (L185)
- `workflows.rs`: `upload_workflow` (L78), `list_workflows` (L128), `get_workflow` (L172), `delete_workflow` (L216)

### Key Files
- `crates/cloacinactl/src/commands/serve.rs` — `AppState`, single runner creation (L157, L177)
- `crates/cloacinactl/src/server/executions.rs` — all handlers
- `crates/cloacinactl/src/server/workflows.rs` — all handlers
- `crates/cloacina/src/database/connection/mod.rs` — `Database::try_new_with_schema()`

### Dependencies
None, but approach should be decided before implementation starts.

## Status Updates

*To be added during implementation*
