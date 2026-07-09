---
id: fleet-actuator-does-not-serve-the
level: task
title: "Fleet actuator does not serve the null/default tenant realm (cron triggers + public packages)"
short_code: "CLOACI-T-0817"
created_at: 2026-06-28T00:23:51.818617+00:00
updated_at: 2026-06-28T03:58:56.385969+00:00
parent: CLOACI-I-0127
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Fleet actuator does not serve the null/default tenant realm (cron triggers + public packages)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

During the T-0816 fleet-actuator demo + soak, the Docker actuator served named tenants end-to-end (acme proven) but **cannot serve the null/default realm**. Both actuators (Docker + K8s) mint only **tenant-scoped** keys, so every provisioned agent registers under a named `tenant_id`. But the demo's `public` packages and the server's **cron triggers** dispatch work with `tenant = None`, and the fleet executor filters agents by `a.tenant_id == task_tenant` → `no available fleet agent in tenant None`. The old static bootstrap-key `agent` covered the null realm; the pure-actuator self-management model does not.

**Options:** (a) seed `public` as a real tenant and route the null realm to it; (b) add null/default-realm provisioning to the actuator (mint a null-tenant agent key); (c) retain one bootstrap/null-realm agent alongside the actuator. A coherent self-managing public/cron dashboard needs one of these. Surfaced by [[CLOACI-I-0127]] T-0816.

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

### 2026-06-27 — "public" made a first-class WORK/AGENT tenant (option (a))

Implemented option (a): retire the `None == "public"` duality in the WORK/AGENT
realm so public work runs as `Some("public")` and matches `Some("public")`
agents. The admin/bootstrap `None` + `is_admin` key is untouched. Branch
`i0127-agent-control-plane`. NOT committed (tenant-boundary change under review).

**Code changes**
- `crates/cloacina-server/src/fleet_executor.rs`: the `task_tenant` mapping for
  the task namespace now resolves `"public"` -> `Some("public")` (was `None`);
  both branches collapse to `Some(namespace.tenant_id.clone())`. Extracted the
  inline agent selection into a testable `select_fleet_agent()` helper (tenant
  gate `&a.tenant_id == task_tenant`). The package lookup deliberately STAYS
  `get_active_dispatch_for_package(pkg, None)` — public packages live in the
  ADMIN schema with `tenant_id IS NULL`; `Some("public")` would find nothing.
  Added 3 isolation unit tests.
- `crates/cloacina-server/src/routes/auth.rs`: `can_access_tenant` non-admin
  `None` arm `tenant_id == "public"` -> `false`. `is_admin` still permits any
  tenant incl. "public"; `Some("public")` reaches "public" via same-tenant.
  Added 4 unit tests.
- `crates/cloacina-server/src/agent_registry.rs`: public-agent fixtures
  `None` -> `Some("public")`.
- `docker/docker-compose.demo.yml`: `agent` and `agent-x86` services repointed
  from `*bootstrap_key` to `*public_key` so they register as `Some("public")`.

**auth.rs verification (riskiest spot).** Non-admin `None` keys CAN be produced
by one endpoint — `POST /auth/keys` (`routes/keys.rs:81-85`, hard-codes
`tenant=None, is_admin=false`). Every other producer mints `Some(tenant)`
(bootstrap = `None`+`is_admin`; `/tenants/{t}/keys`, demo seed, actuator
`DalKeyMinter` all `Some`). After the change those `/auth/keys` keys access
nothing (previously "public-only"). Blast radius LOW: UI/actuator/demo use
tenant-scoped endpoints; `/auth/keys` is god-only-callable and the UI doesn't
use it. FLAGGED for reviewer: consider repointing `/auth/keys` to mint
`Some("public")` or deprecating it.

**Package-lookup choice:** kept `None` (IS NULL, admin schema) — confirmed by
`routes/agent.rs:246` (`resolve("public")` -> admin db) and the cdylib fetch
path; `Some("public")` would break public-package dispatch.

**Latent divergence flagged:** `routes/authz.rs` (dormant ABAC matcher, T-0783,
not yet mounted) still encodes `None => requested == "public"` via its local
`ref_can_access_tenant`; parity tests pass against its own copy. Reconcile when
T-0783 wires it in. Agent crate (`cloacina-agent/src/main.rs:750`) comment still
says "public rides as tenant_id = None" — benign (`register_package_tasks` does
`unwrap_or("public")`; tenant is derived from the task namespace), left per scope.

**Validation:** `cargo test -p cloacina-server --lib` for
auth/authz/fleet/agent_registry = 38 passed (incl. new isolation + can_access
tests and the authz parity suite). Both crates compile clean. NOT validated
here: full demo cron/public dispatch + UI e2e (needs the container stack +
postgres lane).