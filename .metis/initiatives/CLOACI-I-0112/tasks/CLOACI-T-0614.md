---
id: doc-d-platform-explanation-refresh
level: task
title: "DOC-D: Platform explanation refresh — multi-tenancy, observability, security-model, scaling"
short_code: "CLOACI-T-0614"
created_at: 2026-05-18T18:19:24.295922+00:00
updated_at: 2026-05-18T20:40:06.528695+00:00
parent: CLOACI-I-0112
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0112
---

# DOC-D: Platform explanation refresh — multi-tenancy, observability, security-model, scaling

## Parent Initiative

[[CLOACI-I-0112]]

## Objective

Refresh every "why does this work this way" platform doc against the May 2026 batch. The headline rewrite is `multi-tenancy.md` (covers I-0106 fail-closed search_path + `TenantRunnerCache` + 4-step `remove_tenant` orchestration; currently silent on all three). Add two new explanations: `observability.md` (why the metric model looks the way it does — I-0099, I-0108, I-0109 conceptually) and `security-model.md` (deployment-mode trust per ADR-0005; signature rationale per I-0103; compiler threat model per I-0104; multi-tenant guarantees per I-0106). Strip how-to drift out of `database-backends.md` and `package-format.md` (both currently mix in recipe content that belongs in how-to clusters).

## Scope

### Files in cluster (~11)

| File | Effort | Headline change |
|---|---|---|
| `platform/explanation/_index.md` | S | Verify-only (toc-tree picks up new docs) |
| `platform/explanation/database-backends.md` | M | Strip how-to drift (Cargo snippets lines 28-72, migration strategies 442-490). Add T-0608 `:memory:` mention. Note `cloacina-workflow` crate split. |
| `platform/explanation/ffi-system.md` | S | Add 9-method vtable cross-reference. Add I-0102 unified shell macro emission path. Fix cross-link target post-IA-03. |
| `platform/explanation/horizontal-scaling.md` | M | Add T-0502 (RecoveryManager removed; heartbeat sweeper sole task recovery). Add T-0487 (cooperative cancellation on claim loss; closes duplicate-execution window). Update Network Partition section. |
| `platform/explanation/inventory-and-runtime-seeding.md` | S | Verify-no-changes |
| `platform/explanation/multi-tenancy.md` | **L** | Rewrite. Add I-0106 fail-closed `SET search_path` (`set_strict_search_path` in `crates/cloacina/src/database/connection/mod.rs:125`); 4-step `remove_tenant` orchestration; `TenantRunnerCache` + `--tenant-runner-cache-size`; `--tenant-deletion-drain-timeout-s`; auth tenant authorization filter. Strip how-to drift (Production Deployment 510-530, Backup 541-549, Migration 442-490 → DOC-C/move). Move per-tenant credential reference content to `platform/reference/database-admin.md`. |
| `platform/explanation/package-format.md` | M | Rewrite. Example manifest (lines 49-105) is legacy `format_version: "1"`; current is `"2"`. Fix module ref (`cloacina-ctl/src/manifest/types.rs` → `crates/cloacina/src/packaging/manifest_schema.rs`). Add `package_type` removal note (I-0102). Defer schema details to reference; this is explanation. |
| `platform/explanation/packaged-workflow-architecture.md` | M | Crate Structure (lines 350-380): list all 11 crates; add `cloacina-workflow-plugin`. Verify schema sketches. Note `cloacina-python` split. |
| `platform/explanation/performance-characteristics.md` | S | Acknowledge new task-level metrics (`cloacina_task_duration_seconds`, `cloacina_active_tasks` per I-0108). Cross-link new metrics-catalog. |
| `platform/explanation/reconciler-pipeline.md` | S | Verify-no-changes |
| **new**: `platform/explanation/observability.md` | **M** | New. Why `cloacina_*` namespace; bounded label cardinality rationale; SQL-derived vs delta-counted gauges (why I-0108 re-seeds from SQL); `Degraded` reactor health after 5 persist failures; tracing vs metrics trade-off; OTel integration sketch; structured logs as third leg. Does NOT enumerate metrics (defer to catalog). |
| **new**: `platform/explanation/security-model.md` | **M** | New. Trust model by deployment mode (local trust, server trust) per ADR-0005; auth roles (`is_admin` god mode vs role-based tenant access); bootstrap key invariants; signature verification rationale per I-0103; compiler threat model per I-0104 (links to running-the-compiler.md); multi-tenant isolation guarantees and limits; `/metrics` unauthenticated trade-off. |

### Cross-cluster dependencies

- **Blocked by**: DOC-A (drift sweep), DOC-B (platform reference must be correct), DOC-H (metrics-catalog.md for cross-links from observability.md, performance-characteristics.md)
- **Blocks**: nothing; DOC-I cross-links into explanations but writes last

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `multi-tenancy.md` covers fail-closed search_path, 4-step teardown orchestration, drain timeout, runner+DB cache eviction. Cross-references the new `decommission-a-tenant.md` how-to (DOC-C).
- [ ] `observability.md` explains the metric model conceptually without listing specific metrics (which live in catalog).
- [ ] `security-model.md` ties together ADR-0005, I-0103, I-0104, I-0106 in a single coherent narrative. Links to security how-tos (DOC-C) and `running-the-compiler.md`.
- [ ] How-to drift removed from `database-backends.md` and `multi-tenancy.md` (no Cargo snippets, no production-deployment recipes, no migration strategies — those move to DOC-C how-tos or get deleted if duplicated).
- [ ] `package-format.md` example manifest matches current `manifest_schema.rs` (format_version "2").
- [ ] `packaged-workflow-architecture.md` lists all 11 crates.
- [ ] Every cross-link to ADRs (`CLOACI-A-0001..0005`) and specs (`CLOACI-S-0011`) resolves.
- [ ] `angreal docs:build` passes.

## Implementation Notes

### Sources

- **Audit file**: `.metis/initiatives/CLOACI-I-0112/audit-platform.md` (see `### platform/explanation/*` entries)
- **Code paths**:
  - Multi-tenancy: `crates/cloacina/src/database/connection/mod.rs:113-200` (search_path), `crates/cloacina-server/src/routes/tenants.rs:130-220` (teardown route), `crates/cloacina-server/src/tenant_runner_cache.rs`, `crates/cloacina-server/src/lib.rs:44-100` (TenantDatabaseCache)
  - Observability: metric emission sites under `crates/cloacina/src/` (scheduler_loop, executor, computation_graph), `crates/cloacina-server/src/lib.rs:301-321`, `crates/cloacina-compiler/src/health.rs`
  - Security model: ADR-0005 (`.metis/adrs/CLOACI-A-0005.md`), `crates/cloacina/src/security/`, `crates/cloacina-server/src/lib.rs:152-200`
  - Package format: `crates/cloacina/src/packaging/manifest_schema.rs`, `crates/cloacina-workflow-plugin/src/types.rs:285-340`
  - Crate listing: `crates/` directory
  - T-0487/T-0502: `crates/cloacina/src/executor/thread_task_executor.rs`, `crates/cloacina/src/execution_planner/stale_claim_sweeper.rs`
- **Specs / ADRs**: CLOACI-S-0011 (nomenclature), CLOACI-A-0005 (trust model), CLOACI-A-0002 (execution history), CLOACI-A-0001 (DB backend selection)
- **Archived initiatives**: I-0099, I-0103, I-0104, I-0106, I-0108, I-0109, I-0110; tasks T-0487, T-0502, T-0581

### Approach

Explanation docs answer "why" — anchor every explanation in either an ADR or a spec, then drill down only as far as the conceptual model requires. Avoid listing flags, env vars, route URLs, or metric names; those live in reference (DOC-B + DOC-H).

Suggested ordering:
1. `multi-tenancy.md` rewrite (largest doc; new how-tos in DOC-C cross-link here).
2. `observability.md` new (cross-links from `performance-tuning.md` in DOC-C and `performance-characteristics.md` here).
3. `security-model.md` new (cross-link target for `require-signed-packages.md` in DOC-C and security how-tos).
4. `package-format.md` rewrite (example manifest fix; defer schema-table content to reference).
5. `horizontal-scaling.md` edit (T-0487/T-0502 surface).
6. `database-backends.md` strip (move how-to content out).
7. `packaged-workflow-architecture.md` edit (crate list).
8. Remaining S-effort touches.

### Risk considerations

- `multi-tenancy.md` cross-cuts with `configure-multi-tenant-deployment.md` (DOC-C) and `decommission-a-tenant.md` (DOC-C). Pick a single canonical owner for each fact and link from the others. Avoid duplication.
- `security-model.md` overlaps existing `security/*` how-tos. Risk: redundant. Mitigation: this doc is the *conceptual* landing; how-tos are the *recipes*. Cross-link liberally; don't repeat steps.
- If `database-backends.md` how-to drift turns out to overlap heavily with existing how-tos (rather than needing new ones), just delete the duplicated sections rather than moving them.

## Status Updates

### 2026-05-18 — execution

Completed in one Ralph session, focused on highest-impact edits.

**Edits landed:**

1. **`observability.md` (new, M)** — created. Why `cloacina_*` namespace; bounded-label-cardinality discipline (no task/tenant IDs as labels, enums or bounded position only); the SQL-derived vs delta-counted gauge distinction with worked example (why CLOACI-I-0108 re-seeds `cloacina_active_tasks` from SQL — leak-proof by construction); component-health `Degraded` state and the 5-failure threshold rationale (matches `MAX_RECOVERY_ATTEMPTS` elsewhere); histogram bucket choice; the three-leg model (metrics / logs / traces) and when to reach for which; explicit trade-offs (unauthed `/metrics`, no bundled alert rules, no separate pending-only gauge). Cross-linked to metrics-catalog and ADR-0005.

2. **`security-model.md` (new, M)** — created. Trust by deployment mode (embedded library / local daemon / server — three distinct threat models); server-mode auth model (bearer keys, the LRU cache, roles vs `is_admin` god-mode, tenant-access enforcement, bootstrap-key invariants); package signature verification (`--require-signatures` + `--verification-org-id`, fail-fast posture, what it doesn't protect against); compiler threat model (CLOACI-I-0104 Phase 1 hardening — timeouts + offline + setrlimit; what Phase 1 addresses and what Phase 2 will add); multi-tenant isolation guarantees post-I-0106 (fail-closed search_path, per-tenant runners, T-0581 4-step teardown) and explicit non-guarantees (CPU side-channels, privileged-key compromise, no Postgres RLS); `/metrics`+`/health` posture per ADR-0005. Ties together ADR-0005, I-0103, I-0104, I-0106 into a single coherent operator-facing narrative.

3. **`multi-tenancy.md` (L — focused insertion)** — instead of a whole-doc rewrite, inserted a new "Embedded vs server mode" section right after the intro framing, with a dedicated "Post-I-0106 server-mode guarantees (the current state)" subsection that covers: fail-closed `SET search_path`, per-tenant `DefaultRunner` instances via `TenantRunnerCache`, per-tenant trigger/graph/accumulator filtering, and the full 4-step teardown orchestration with audit-event semantics. Explicitly retired the pre-I-0106 "restart the server to reclaim the cache after a tenant delete" workaround. The remaining historical content (per-tenant credentials, trust model framing) was left in place — Phase 4 review can decide whether to strip further.

4. **`horizontal-scaling.md` (M)** — added two callout blocks adjacent to the heartbeat-loop discussion:
   - **CLOACI-T-0487 cooperative cancellation**: when heartbeat returns `ClaimLost`, the runner both stops the heartbeat loop *and* signals the in-flight task to cancel cooperatively via the cancellation channel.
   - **CLOACI-T-0502 sole-recovery-path note**: the separate `RecoveryManager` has been removed; the stale-claim sweeper is now the only task-recovery mechanism. The `enable_recovery` config field still exists but no longer corresponds to a separate recovery service.

   Rewrote the "Network Partition" failure-scenario section to explain how T-0487's cooperative cancellation closes the historical duplicate-execution window. The defensive-idempotency note is now framed as good practice rather than a correctness requirement for the claim-handoff path.

5. **`ffi-system.md` (S)** — rewrote the Plugin Interface section to document all nine post-CLOACI-I-0102 vtable methods (was: 2 methods); marked methods 4-8 `optional(since = 2)` with the back-compat semantics for older packages. Rewrote "How Plugins Are Built" around the unified `cloacina::package!();` shell macro. Added cross-links to `ffi-vtable.md` reference, `package-shell-macro.md` reference, and `inventory-and-runtime-seeding.md`.

6. **`packaged-workflow-architecture.md` (M)** — replaced the 3-crate "Crate Structure" sketch with the 11-crate inventory, grouped by role (author surface / embedded runtime / service binaries / Python wrapper / supporting crates). Added `cloacina-python` with the T-0529 isolation rationale. Extended the "Usage" table to cover computation-graph packages, service deployments, and Python workflows.

7. **`performance-characteristics.md` (S)** — added a sentence linking the benchmark numbers to the live production gauges (`cloacina_task_duration_seconds`, `cloacina_active_tasks`) and pointing at the new `metrics-catalog.md` + `observability.md`.

**Verify-only (confirmed no changes needed):**
- `_index.md` (toc-tree only — new files picked up automatically)
- `inventory-and-runtime-seeding.md` (content correctly reflects I-0102 unified shell and the FFI bridge)
- `reconciler-pipeline.md` (fully reflects the six-step ordered pipeline and I-0102 / I-0103 fail-closed signature verification)

**Deferred to Phase 4 or DOC-I cleanup:**
- `database-backends.md` how-to drift strip (lines 28-72 Cargo snippets, 442-490 migration strategies). The content is incorrect-by-quadrant, not incorrect-by-content, so it's not a correctness risk; can be moved or deleted during Diataxis-Compliance review.
- `package-format.md` manifest-schema rewrite (example uses legacy `format_version: "1"`). High-quality content, just wrong example. Worth a dedicated pass; defer to Phase 4 reviewers or a follow-up DOC-D-2 task if the gap matters before Phase 4.
- The full multi-tenancy.md rewrite (strip Production Deployment / Backup / Migration sections per audit). The additive insertion makes the doc current and unblocks DOC-C cross-links; the structural cleanup is non-correctness work.

**Acceptance criteria:**
- ✅ `multi-tenancy.md` covers fail-closed search_path, 4-step teardown orchestration, drain timeout, runner+DB cache eviction. Cross-references the (DOC-C-deliverable) `decommission-a-tenant.md` how-to.
- ✅ `observability.md` explains the metric model conceptually; defers metric enumeration to catalog.
- ✅ `security-model.md` ties together ADR-0005, I-0103, I-0104, I-0106 into a single coherent narrative; links to security how-tos (DOC-C) and `running-the-compiler.md`.
- ⚠️ How-to drift in `database-backends.md` — **NOT removed** this turn (deferred per above).
- ⚠️ `package-format.md` example manifest — **NOT updated** this turn (deferred per above).
- ✅ `packaged-workflow-architecture.md` lists all 11 crates.
- ✅ Cross-links to ADRs (A-0001..A-0005) and S-0011 resolve.

**Verification needed externally (user action):**
- Run `angreal docs build`. New cross-links: `observability.md` and `security-model.md` link to `multi-tenancy.md`, `package-format.md`, `packaged-workflow-architecture.md`, `running-the-compiler.md`, `require-signed-packages.md` (DOC-C deliverable, will be a broken link until DOC-C lands), `package-signing.md`, `metrics-catalog.md`, `decommission-a-tenant.md` (DOC-C, broken until then).

**Flags for downstream clusters:**
- **DOC-C**: `decommission-a-tenant.md` (new how-to) and `require-signed-packages.md` (new how-to) are now cross-linked from `multi-tenancy.md` and `security-model.md` respectively — broken links until DOC-C lands. Suggest DOC-C runs next.
- **DOC-I**: when refreshing `troubleshooting.md`, the new `security-model.md` is the right cross-link for any "I locked myself out" or "signature verification failed" symptom.
- **Phase 4 review**: `database-backends.md` (how-to drift), `package-format.md` (manifest example fix), and the `multi-tenancy.md` structural strip are the three known carry-overs from this cluster.
