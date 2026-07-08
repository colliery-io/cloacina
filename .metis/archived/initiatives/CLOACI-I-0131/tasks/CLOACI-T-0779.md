---
id: demo-multi-tenancy-seed-a-second
level: task
title: "Demo multi-tenancy — seed a second tenant with a scoped key to demonstrate isolation"
short_code: "CLOACI-T-0779"
created_at: 2026-06-23T00:46:41.389976+00:00
updated_at: 2026-06-23T12:53:05.366703+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Demo multi-tenancy — seed a second tenant with a scoped key to demonstrate isolation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

Demonstrate tenant isolation in the demo. Isolation = Postgres schema-per-tenant;
keys carry a tenant_id (or is_admin for cross-tenant). Today the demo seeds only
`public` with the admin bootstrap key. Add a 2nd tenant with a DISTINCT curated
data subset + per-tenant SCOPED keys, and a UI tenant switcher — so you can see
(a) data isolation (each tenant only its own runs/workflows) and (b) access
isolation (a scoped key 403s the other tenant). User chose: data+access (scoped
keys), distinct curated subset, in-app switcher.

## Plan (phased)

**P1 — Server + seeder (backend, demonstrable via API):**
- api_keys live in the admin schema with a tenant_id column; bootstrap already
  accepts a provided key value → add a demo-gated bootstrap step that seeds
  DETERMINISTIC tenant-scoped keys from an env (e.g. CLOACINA_DEMO_TENANT_KEYS=
  "acme:clk_demo_acme_key_0002:operator,public:clk_demo_public_key_0003:operator").
  Reuses create_key(hash, name, Some(tenant), is_admin=false, role).
- Compose: define acme + public scoped keys; add a 2nd harness `seed` service with
  HARNESS_TENANT=acme + a package filter so acme gets a curated subset.
- Harness: HARNESS_PACKAGES filter env (subset upload); ensureTenant already
  creates non-public tenants.
- VERIFY: acme scoped key → acme data OK, public data 403; public scoped key
  symmetric; admin sees both.

**P2 — UI tenant switcher:**
- Multi-connection store (named saved connections: serverUrl+key+tenant) + a
  switcher in the shell header; persist. Pre-populate the demo connections from
  the known keys. Flip active connection → all data rescopes.

## Status Updates **[REQUIRED]**

- 2026-06-23: Scoped (data+access isolation, distinct curated subset, in-app
  switcher). Building P1 (server+seeder) then P2 (UI switcher). Note: rides the
  next reseed, which also restores the T-0778 demo trigger to manual-only.
- 2026-06-23: P1 (server bootstrap_demo_tenant_keys + harness include/exclude
  filters) + P2 (multi-connection AuthContext + TenantSwitcher) built+committed.
  Access isolation verified (scoped keys 403 cross-tenant; no acme leak in public).
  Per feedback, acme gets its OWN distinctly-named packages (acme_billing/payroll/
  fulfillment), public excludes acme-*.
- 2026-06-23: BLOCKER found + RESOLVED. Tenant packages never compiled — the
  shared compiler's claim_next_build only scans its own (public) schema. User
  flagged that a single compiler scanning all schemas would expose tenants' source/
  logs to each other (defeats isolation). Investigated the fleet: it isolates via
  tenant-scoped agent registration (agent.tenant_id filter in fleet_executor) but
  agents are DB-less task-runners with NO toolchain, so compilation can't ride the
  fleet directly. FIX = per-tenant compiler (the fleet's tenant-scoped-worker
  principle applied to compilation): added --tenant-schema to cloacina-compiler
  (scopes claim/build/logs to one schema via try_new_with_schema; skips migrations;
  tolerates missing schema). Demo adds compiler-acme scoped to acme, sharing the
  framework build cache (cloacina crates aren't tenant data) while source/logs/
  schema/DB-artifacts stay per-tenant; sequenced after the public seed (no lock
  contention). TRIPLET note: build_target_triple is stamped from the server host
  (uniform-arch assumption — all same image); agents fail-closed refuse on mismatch.
  Per-package triple stamping (from PackageMetadata.architecture) is the follow-up
  for true per-arch tenant compilers. Reseed bmjajwkp5 verifying.
- 2026-06-23: Per-tenant compiler VERIFIED (acme_billing/payroll/fulfillment compile
  via compiler-acme; public compiler touched 0 acme packages). Demonstrating the
  demo surfaced the rest of runtime multi-tenancy, all now fixed: tenant AGENTS
  (agent-acme, acme key → tenant=acme), EXECUTION isolation (CLOACI-T-0781, 5-layer
  fix — verified all 3 acme workflows Complete on acme agents), and PER-TENANT OPS
  (gather() scoped per view; publish loop pushes a snapshot per connected tenant).
  VERIFIED via UI: acme Operations page shows acme's 2 agents + available=3 packages
  (not red); admin shows 5 agents + available=15 — each view sees its own state.
  Multi-tenancy isolation now complete end-to-end: DATA + ACCESS + BUILD + EXECUTION
  + OPS. P2 switcher built (multi-connection AuthContext + TenantSwitcher); the ops
  screenshots were taken via the switcher's connection model. DONE.

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

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

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