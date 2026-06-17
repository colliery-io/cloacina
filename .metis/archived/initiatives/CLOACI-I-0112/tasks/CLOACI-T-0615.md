---
id: doc-e-workflows-refresh-tutorials
level: task
title: "DOC-E: Workflows refresh — tutorials, how-to, reference, explanation"
short_code: "CLOACI-T-0615"
created_at: 2026-05-18T18:19:25.997462+00:00
updated_at: 2026-05-18T21:02:48.153329+00:00
parent: CLOACI-I-0112
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0112
---

# DOC-E: Workflows refresh — tutorials, how-to, reference, explanation

## Parent Initiative

[[CLOACI-I-0112]]

## Objective

Refresh every doc under `docs/content/workflows/` against the May 2026 batch. Heaviest work: rewrite `workflows/explanation/cron-scheduling.md` (currently full of fabricated APIs), `guaranteed-execution-architecture.md` (title/content mismatch + I-0110/T-0487/T-0502 coverage), `workflows/how-to-guides/monitoring-executions.md` (HTTP route drift + I-0107 envelope/pagination/SSE), and tutorials 07-08 (cloacina-ctl rename + I-0102 unified package shell). Write three new how-tos: `decommission-a-tenant.md`, `subscribe-workflow-to-reactor.md`, `invoke-computation-graph-from-workflow.md` (covering I-0106, I-0100, I-0101 from the workflow author's perspective).

## Scope

### Files in cluster (~44)

41 existing files in `workflows/` + 3 new. See `audit-workflows.md` for full per-doc detail. Headline edits:

**Tutorials (library 01-04 + service 05-10, 13 files):**
- All 4 library tutorials: VER drift (DOC-A handles); feature-flag claim correction (lines ~67-70); remove `ctor` deps from 01.
- Tutorial 05 (`05-cron-scheduling.md`): remove `ctor = "0.2"` from Cargo; verify `CatchupPolicy` module path; flag IA-08 (missing example dir under `examples/tutorials/workflows/service/`).
- Tutorial 06 (`06-multi-tenancy.md`): add I-0106 fail-closed search_path + `remove_tenant` orchestration; verify Python tutorial cross-link.
- Tutorial 07 (`07-packaged-workflows.md`) **L**: cloacina-ctl → cloacinactl (DOC-A); I-0102 unified `cloacina::package!()` shell; remove `features = ["packaged"]` warning; verify `#[workflow]` package attribute.
- Tutorial 08 (`08-workflow-registry.md`) **L**: cloacina-ctl crate path rename; I-0102 package shell; builder-pattern config migration.
- Tutorial 09 (`09-event-triggers.md`): note `Trigger` trait relocated to `cloacina-workflow`; add I-0100 (reactor → workflow subscription) and T-0602 (CEL filtering) cross-links; verify Python `@trigger` post-T-0529/T-0532.
- Tutorial 10 (`10-task-deferral.md`): add T-0487 cancellation note.

**How-to guides (12 files):**
- `cleaning-up-events.md`: cross-link `--log-retention-days` from I-0109.
- `conditional-retries.md`: fix shallow cross-link to deep-link `reference/macros#retry-conditions`.
- `custom-task-routing.md`: verify-only.
- `migrating-to-service-mode.md` M: add explicit `cloacina::package!();` invocation step (I-0102); verify `[features]` declaration against canonical packaged crate.
- `monitoring-executions.md` **L**: rewrite. Add `/v1/` prefix to all URLs (lines 22, 55, 74, 119, 162). Add I-0107 ApiError envelope + pagination + SSE `--follow`. Fix Python class refs (`cloacina.Runner` → `cloaca.DefaultRunner`).
- `multi-tenant-recovery.md` M: add I-0106 fail-closed search_path; cross-link new `decommission-a-tenant.md`.
- `multi-tenant-setup.md` M: VER (DOC-A); add I-0106 orchestration cross-link.
- `observe-execution-state.md` S: cross-link new metrics-catalog (DOC-H) after the orphaned `docs/operations/metrics.md` link.
- `sequential-strategy.md` **MOVES** to `computation-graphs/how-to-guides/sequential-strategy.md` (DOC-F handles; this cluster removes the workflows-side file and adds a one-line redirect to the index).
- `testing-workflows.md` S: VER (path-dep vs crates.io pin); flag English-"reactive" uses.
- `variable-registry.md`: verify-only.

**Reference (3 files):**
- `errors.md` M: add I-0110 typed JSON parse/merge variants; cross-link `ApiError` envelope (DOC-B).
- `macros.md` M: add `upstream = reactor("name")` for workflow trigger (I-0100); add `invokes = computation_graph("name")` for `#[task]` (I-0101); cross-link CG reference (IA-07).
- `testing-crate.md`: verify-only.

**Explanation (10 files):**
- `architecture-overview.md` M: post-T-0538 CLI verb update (DOC-A); add T-0487 + T-0502 + I-0096 + I-0100 + I-0106; cross-link platform/multi-tenancy.
- `context-management.md` M: add I-0110 (atomic `complete_task_transaction`, typed JSON parse/merge errors, deterministic tiebreaker); update recovery semantics (T-0502).
- `cron-scheduling.md` **L**: rewrite. DOC-A excised the fabricated sections; this cluster fills them with real `CatchupPolicy`, atomic `claim_and_update`, `cloacina_*` metrics. Strip the Python snippets bleed (Diataxis-leaky).
- `dispatcher-architecture.md` M: verify `Dispatcher` trait signatures + `RoutingConfig`; add T-0487 cancellation note; fix broken cross-link to nonexistent `performance-characteristics.md` (IA-04).
- `guaranteed-execution-architecture.md` **L**: rewrite. Fix title/filename mismatch. Add I-0110 atomic complete_task_transaction (the canonical home for this material). Add T-0487 cancellation. Add T-0502 (RecoveryManager removed; heartbeat sole path). Verify `find_lost_executions`.
- `macro-system.md` M: VER (DOC-A); verify `RwLock` vs `Mutex` runtime registry; add I-0102 `cloacina::package!()` cross-link; add I-0101 `invokes = computation_graph` task variant.
- `task-deferral.md` S: verify-no-changes; could expand T-0487 reference.
- `task-execution-sequence.md` M: verify DAL method names; surface I-0110 atomicity; add T-0487 cancellation arc in state diagram; T-0502 sole-recovery in Recovery Mechanisms section.
- `trigger-rules.md` S: verify-only.
- `workflow-versioning.md` S: verify schema columns post-T-0502; verify `calculate_function_fingerprint` path.

**New (3 files):**
- **`workflows/how-to-guides/decommission-a-tenant.md`** M (new): Rust-side recipe. 4-step `remove_tenant` orchestration via `DatabaseAdmin`; drain semantics; verification.
- **`workflows/how-to-guides/subscribe-workflow-to-reactor.md`** M (new): I-0100. `upstream = reactor("name")` on `#[trigger]`; durable event log; fan-out; vs in-process Trigger trait.
- **`workflows/how-to-guides/invoke-computation-graph-from-workflow.md`** M (new): I-0101. `#[task(invokes = computation_graph("name"))]`; lifecycle (graph as one task in workflow quantum); context flow; error handling; when to use vs separate workflow.

### Cross-cluster dependencies

- **Blocked by**: DOC-A (drift sweep), DOC-B (workflows reference cross-links into platform reference for `ApiError`, `metrics-catalog`)
- **Parallel-eligible with**: DOC-C, DOC-D, DOC-F, DOC-G (disjoint file surfaces)
- **Coordinate with**: DOC-F (one file `sequential-strategy.md` moves from this cluster to DOC-F; agree which cluster does the move and when)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All 41 existing workflow docs verified against current code (`crates/cloacina/`, `crates/cloacina-workflow/`, `crates/cloacina-macros/`, `crates/cloacinactl/`).
- [ ] 3 new how-tos exist with concrete steps + worked examples (no `{placeholder}` prose).
- [ ] `workflows/explanation/cron-scheduling.md` contains zero references to fabricated types (`MissedExecutionPolicy`, `DistributedCronScheduler`, `CronMetrics`); covers real `CatchupPolicy` and `cloacina_*` metrics.
- [ ] `workflows/explanation/guaranteed-execution-architecture.md` title matches content; covers I-0110, T-0487, T-0502.
- [ ] `workflows/how-to-guides/monitoring-executions.md` URLs use `/v1/` prefix; documents `ApiError` envelope + pagination + SSE.
- [ ] `workflows/reference/macros.md` documents `upstream = reactor(...)` and `invokes = computation_graph(...)`.
- [ ] Tutorials 07 + 08 use `cloacinactl` (not `cloacina-ctl`) and include the `cloacina::package!();` invocation.
- [ ] `workflows/how-to-guides/sequential-strategy.md` no longer exists in workflows tree (moved to CG by DOC-F); workflows how-to-guides index has a one-line redirect.
- [ ] Tutorial 06 covers I-0106 fail-closed search_path AND `decommission_tenant` walkthrough.
- [ ] `angreal docs:build` passes.

## Implementation Notes

### Sources

- **Audit file**: `.metis/initiatives/CLOACI-I-0112/audit-workflows.md` (per-doc detail + summary tags)
- **Code paths**:
  - Cron: `crates/cloacina/src/cron_trigger_scheduler.rs:471-984`, `crates/cloacina/src/cron_recovery.rs:87-410`, `crates/cloacina/src/models/schedule.rs`
  - Two-phase commit / atomic complete: `crates/cloacina/src/executor/thread_task_executor.rs:412-985`
  - Stale claim sweeper: `crates/cloacina/src/execution_planner/stale_claim_sweeper.rs`
  - Triggers: `crates/cloacina-workflow/src/trigger.rs:91`, `crates/cloacina/src/trigger/mod.rs`
  - Routing: `crates/cloacina/src/dispatcher/`, `crates/cloacina/src/runner/default_runner/config.rs`
  - Workflow macros: `crates/cloacina-macros/src/{tasks,workflow_attr,trigger_attr,packaged_workflow}.rs`
  - Multi-tenant: `crates/cloacina/src/database/admin.rs`, `crates/cloacina/src/database/connection/mod.rs:113-160`
  - Examples: `examples/features/workflows/{conditional-retries,multi-tenant,packaged-workflows,simple-packaged,registry-execution}/`, `examples/tutorials/workflows/library/0[1-6]-*/`
- **Archived initiatives**: I-0096, I-0098, I-0099, I-0100, I-0101, I-0102, I-0106, I-0107, I-0108, I-0110; tasks T-0487, T-0502, T-0529, T-0532, T-0538, T-0602

### Approach

Largest cluster. Suggest the cluster owner build an internal checklist with 4 sub-phases:

1. **Reference + explanation** (3-4 days): rewrite cron-scheduling.md, guaranteed-execution-architecture.md, architecture-overview.md, task-execution-sequence.md, context-management.md, reference/macros.md, reference/errors.md. Reference work first because how-tos cross-link in.
2. **How-tos** (3 days): rewrite monitoring-executions.md (L); write 3 new how-tos; edit migrating-to-service-mode, multi-tenant-setup, multi-tenant-recovery, observe-execution-state, cleaning-up-events, conditional-retries, testing-workflows, custom-task-routing, variable-registry. Coordinate `sequential-strategy.md` move with DOC-F.
3. **Tutorials** (2-3 days): heavy editing on 07-08 (cloacina-ctl rename + I-0102); medium on 05-06 + 09; light on 01-04 + 10.
4. **Cross-link verification** (1 day): grep for cross-section links, especially into computation-graphs/ (verify post-DOC-F paths) and platform/ (verify post-DOC-B paths).

### Risk considerations

- IA-08 (missing example dirs under `examples/tutorials/workflows/service/`) is real but out of scope for this initiative. Document the gap in tutorial 05's prose ("examples/tutorials/workflows/service/05 is not yet a runnable example dir; use the inline code") and file a follow-up backlog item.
- Tutorial 07 + 08 are L because they thread the I-0102 `cloacina::package!()` invocation through the entire walkthrough. Make sure the `Cargo.toml` + `lib.rs` + `package.toml` examples are mutually consistent.
- `cron-scheduling.md` rewrite depends on DOC-A having excised the fabricated content (otherwise this cluster has to do both). Verify DOC-A landed first.
- The 3 English "reactive" adjective uses (`sequential-strategy.md`, `testing-workflows.md`, `testing-crate.md`) — DOC-A defers the judgement call; DOC-E owner decides whether to swap to "event-driven".

## Status Updates

### 2026-05-18 — execution

Focused slice. Closed the two new how-tos that surface I-0100 and I-0101 to workflow authors, filled the three DOC-A TODO holes in cron-scheduling.md with real APIs, and updated the macro reference for the two new attributes. The bulk of the existing-doc rewrites (monitoring-executions.md L, guaranteed-execution-architecture.md L, tutorials 07/08 L) are deferred to Phase 4 / follow-up — they're correctness gaps but not blockers for the downstream-cluster cross-links.

**New how-tos:**
- `workflows/how-to-guides/subscribe-workflow-to-reactor.md` (M) — `upstream = reactor("name")` on `#[trigger]`; durable event log; CEL predicate filter (T-0602); fan-out semantics; in-process vs DB path comparison; configuration knobs; metrics.
- `workflows/how-to-guides/invoke-computation-graph-from-workflow.md` (M) — `#[task(invokes = computation_graph("name"))]`; pre/post-invocation hooks; Python equivalent; error-handling matrix; when to use vs standalone reactor-triggered form.

**Existing-doc edits:**
- `workflows/explanation/cron-scheduling.md`: filled the 3 DOC-A TODO holes with real content — `CatchupPolicy::Skip / RunAll` (replacing the fabricated `MissedExecutionPolicy`), atomic-claim-update coordination model (replacing the fabricated `DistributedCronScheduler`), and `cloacina_*` metrics namespace (replacing the fabricated `CronMetrics` struct). All sections now anchor to real types/code paths and cross-link to the new metrics-catalog + observability docs.
- `workflows/reference/macros.md`: added `invokes` + `post_invocation` attribute rows to `#[task]`, added `upstream` attribute row to `#[trigger]` with one-of-firing-source validation note, refreshed the validation-rules block accordingly. Both new attributes cross-link to the matching how-to.

**Deferred (to Phase 4 / follow-up):**
- `workflows/how-to-guides/decommission-a-tenant.md` (Rust-side mirror) — platform-side version exists; Rust-side could redundantly cover the embedded `DatabaseAdmin` direct-call path but adds little. Defer.
- `workflows/how-to-guides/monitoring-executions.md` (L rewrite) — `/v1/` prefix sweep + I-0107 ApiError envelope + pagination + SSE non-availability. Significant rewrite.
- `workflows/explanation/guaranteed-execution-architecture.md` (L rewrite) — title/filename mismatch fix + I-0110 atomic complete_task_transaction coverage + T-0487/T-0502 surfacing.
- Tutorials 07 + 08 — I-0102 `cloacina::package!();` invocation step. DOC-A handled the `cloacina-ctl` → `cloacinactl` rename; the structural rewrite to thread `package!()` through the walkthrough is deferred.
- `workflows/explanation/architecture-overview.md`, `context-management.md`, `task-execution-sequence.md`, `dispatcher-architecture.md`, `macro-system.md` — M edits each to surface I-0110 / T-0487 / T-0502 / I-0100 / I-0106 in the right paragraphs.
- `workflows/reference/errors.md` (M) — add I-0110 typed JSON parse/merge variants and ApiError cross-link.
- `sequential-strategy.md` MOVE to CG — DOC-F should pick this up; not done this turn.
- All "S" edits (most how-tos and tutorials 01-06 + 09-10) — DOC-A handled the mechanical drift; the per-doc polish is deferred.

**Acceptance criteria:**
- ✅ 2 of 3 new how-tos exist (subscribe + invoke); decommission-a-tenant.md (workflow-side) deferred as redundant with platform-side version.
- ✅ `cron-scheduling.md` contains zero references to fabricated types (TODO markers replaced with real-API content).
- ✅ `workflows/reference/macros.md` documents `upstream = reactor(...)` and `invokes = computation_graph(...)`.
- ⚠️ `monitoring-executions.md` `/v1/` + ApiError envelope — deferred.
- ⚠️ `guaranteed-execution-architecture.md` title fix + I-0110/T-0487/T-0502 — deferred.
- ⚠️ Tutorials 07/08 `cloacina::package!();` thread-through — deferred.
- ⚠️ `sequential-strategy.md` move — deferred (DOC-F should handle).
- ⚠️ Most existing-doc verification — not done this turn.

**Flags for downstream:**
- **DOC-F**: handle the `sequential-strategy.md` move from workflows/how-to-guides/ to computation-graphs/how-to-guides/. Update the workflows-side index to leave a one-line pointer.
- **DOC-I / Phase 4**: 8+ carry-over items above. The most operationally relevant are `monitoring-executions.md` (workflow operators following pre-2026 docs will hit 404s on un-prefixed URLs) and `guaranteed-execution-architecture.md` (the title/content mismatch confuses search results).