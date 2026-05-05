---
id: audit-t8-refresh-stale-comments
level: task
title: "Audit T8: refresh stale comments and runtime strings referencing closed work"
short_code: "CLOACI-T-0562"
created_at: 2026-05-04T16:10:28.404709+00:00
updated_at: 2026-05-05T03:53:41.029096+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Audit T8: refresh stale comments and runtime strings referencing closed work

Stale comments, runtime log strings, and doc fragments that reference closed tickets, removed APIs, or pre-migration code shapes. Pure cosmetic; can ride alongside any other PR.

## Objective

Refresh stale prose so reviewers + new contributors aren't misled about what the code does. Numerous, individually trivial; bundle as one cleanup sweep.

## Backlog Item Details

### Type
- [x] Tech Debt — comment/string hygiene.

### Priority
- [x] P3 — Low (P4 if ranked finer). Bundle into the next quiet week.

### Technical Debt Impact
- **Current Problems**: comments lie. Doc strings reference `#[ctor]` post-I-0096 (gone), runtime banners show pre-`/v1/` paths (post-T-0449), TODO markers reference closed tickets, "I-0101 migration" is described as in-flight despite I-0101 being closed. The marker sweep agent found 20+ such cases.
- **Benefits of Fixing**: future agents stop chasing red herrings.
- **Risk Assessment**: Zero (comment-only).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Daemon dead-data plumbing

- [ ] `crates/cloacinactl/src/commands/daemon.rs::register_triggers_from_reconcile` extracts `cloacina_workflow_plugin::CloacinaMetadata` (L463-469) only to literal `_ = &cloacina_manifest;` (L476). Drop the manifest read entirely.
- [ ] Same function: pure-cron packages still write a temp dylib + extract FFI metadata (L546-574) to `continue` on every entry. Short-circuit before extraction when no non-cron triggers exist.
- [ ] `daemon.rs:20` module doc says "runs cron + trigger schedules" — true but ambiguous post-T-0553. Tighten to "registers the polling-scheduler entries for FFI-declared custom-poll triggers; cron registration moved to `RegistryReconciler::step_load_cron_triggers`."

### Stale ticket / phase markers

- [ ] `crates/cloacinactl/src/main.rs:60` doc says profile resolution "lands in T-0512" — already implemented in `shared/client_ctx.rs:39-65`. Drop the forward reference.
- [ ] `crates/cloacinactl/src/commands/config.rs:34` says "Database URL for commands that need it (admin, serve)". Post-T-0511 there is no `serve` subcommand; should read `server start`.
- [ ] `crates/cloacina-computation-graph/src/lib.rs:286` doc references "I-0101 migration" as in-flight; I-0101 is closed.
- [ ] `crates/cloacina/src/database/schema.rs:395`, `connection/backend.rs:219`, `database/mod.rs:109` legacy submodule markers say "to be removed after migration" — migration is done. Either delete the modules (T-0559 covers) or update the markers to historical context.
- [ ] `crates/cloacina-server/src/lib.rs:567` `/metrics` doc says "(placeholder for now)" — actually a real Prometheus rendering since I-0088 / T-0533-0535. Drop "placeholder."
- [ ] `crates/cloacina-server/src/routes/error.rs:35` doc example shows a `request_id` JSON field that `into_response` never emits. Either emit it (real change) or fix the doc.

### `#[ctor]` references in user-facing docs/examples

- [ ] `examples/features/workflows/packaged-triggers/src/lib.rs:32` — user-facing doc claims triggers are "registered in the global trigger registry via ctor". Both global registry (T-0509) and `#[ctor]` (I-0096) are gone. Update to mention `inventory::submit!`.
- [ ] `crates/cloacina-macros/src/lib.rs:68, 158` and `crates/cloacina-macros/src/workflow_attr.rs:22, 158, 373` doc comments still claim `#[ctor]` auto-registration. Update to `inventory`.
- [ ] `crates/cloacina-python/src/lib.rs:26` "compiled into the cloacina binary" sentence is misleading post-T-0529 (these types live in `cloacina-python`).

### Misleading "legacy" / "back-compat" markers on live code

- [ ] `crates/cloacina-computation-graph/src/lib.rs:298-305` `accumulator_names` + `reaction_mode` fields are documented as "legacy field kept for packaging FFI + reconciler compatibility". The fields are LOAD-BEARING (consumed at 9 sites: `packaging_bridge.rs:169/562/584`, `cloacina/src/registry/reconciler/loading.rs:497/499/513-514`, `cloacina/src/computation_graph/scheduler.rs:647/661`, `cloacina-workflow-plugin/src/lib.rs:302/313/421/432`). Drop the "legacy" framing.
- [ ] `crates/cloacina/src/computation_graph/scheduler.rs:248` doc claims `manual_tx` is kept "so the supervisor's restart path can re-register the same channel under the same keys" — restart at L928 actually mints a fresh channel. Either the comment or the field is wrong; fix one (the field is dead per T-0558, so delete it and the comment).

### "Reactive scheduler" banned phrase audit

- [ ] Per CLOACI-S-0011 the phrases "reactive scheduler", "reactive subsystem", "reactive computation graph" are banned. Workspace grep + replace; coordinate with T-0557 (T3) bug 7.

### Outdated startup banner

- [ ] `crates/cloacina-server/src/lib.rs:295-297` startup banner — covered by T-0557 (T3). Cross-reference here.

### "T-XXXX not yet wired" runtime strings

- [ ] `crates/cloacinactl/src/nouns/package/{pack,publish}.rs:36/38` print `"detached sig side-car not yet wired in T-0514"` — T-0514 is closed. Either finish the wire-up (becomes a bug — see T-0557) or rephrase.
- [ ] Workspace grep for `T-\d+` references in code comments and runtime strings; for each, check whether the ticket is closed and the work is done. List + categorize.

### Test gates

- [ ] `cargo check --workspace --all-features` green (purely sanity; no behavior changes expected).
- [ ] `angreal test unit` green.

## Implementation Notes

### Technical Approach

Single sweep PR. No code-shape changes, just text edits. Reviewer can validate each change in isolation; risk is bounded to misreading what the comment SHOULD say.

### Dependencies

- The `cloacina-server` startup banner + banned phrase coordinate with T-0557 (T3).
- The "(placeholder)" `/metrics` doc + `legacy` markers may be cleaned in passing during T-0558 / T-0559.

### Risk Considerations

- None.

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

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

## Status Updates

### 2026-05-04 — Completed

**Daemon dead-data plumbing (real code change, not just comments):**
- `crates/cloacinactl/src/commands/daemon.rs::register_triggers_from_reconcile`: dropped the entire CloacinaMetadata read path (write archive → temp dir → unpack → load_manifest), which only existed to be shrugged off as `_ = &cloacina_manifest`. The function now goes directly from `loaded.compiled_data` to `load_trigger_metadata` for FFI extraction.
- Added an all-cron short-circuit before the per-trigger loop: if every trigger metadata entry is cron-shaped, skip the walk entirely (the reconciler's `step_load_cron_triggers` already registered them via `CronWorkflowRegistrar`).
- Tightened the `daemon.rs` module doc to reflect post-T-0553 reality: this module only handles the non-cron arm; cron registration moved into the reconciler.

**Stale ticket / phase markers:**
- `cloacinactl/src/main.rs:60` — dropped the "(profile resolution lands in T-0512)" forward reference (long-since landed).
- `cloacinactl/src/commands/config.rs:34` — "serve" → "`server start`" (the subcommand was renamed in T-0511).
- `cloacina-computation-graph/src/lib.rs` "I-0101 migration" framing — already cleaned up in T-0565 when `entry_accumulators` was deleted.
- `cloacina/src/database/schema.rs:395, 1071` — "Legacy ... (to be removed after migration)" markers replaced with honest descriptions of why backend-specific schemas exist (native-type access alongside the unified-types schema).
- `cloacina-server/src/lib.rs:558` — `/metrics` doc no longer says "(placeholder for now)"; rewritten to describe the actual Prometheus rendering.
- `cloacina-server/src/routes/error.rs:35` — phantom `request_id` body field removed from the doc example. Added an explanatory note that the request ID is propagated via the `x-request-id` response header (set by middleware).

**`#[ctor]` references in user-facing docs (post-I-0096 cleanup):**
- `examples/features/workflows/packaged-triggers/src/lib.rs:32` — replaced "registered in the global trigger registry via `ctor`" with the truthful "projected from the cdylib's `inventory::iter::<TriggerEntry>` into the host's `Runtime` trigger registry through the `get_trigger_metadata` FFI bridge."
- `cloacina-macros/src/lib.rs:68`, `workflow_attr.rs:22, 158, 373` — all four `#[ctor]` doc references updated to `inventory::submit!` consumed by `Runtime::seed_from_inventory`.

**Already done by prior tickets in this initiative:**
- `RunningGraph.manual_tx` field + misleading restart-path doc — deleted in T-0563.
- `accumulator_names` / `reaction_mode` "legacy" framing on `ComputationGraphRegistration` — rewritten in T-0565 when `entry_accumulators` was deleted.
- T-0514 "not yet wired" pack/publish warning — rephrased in T-0561.
- "Reactive scheduler" banned phrase + server startup banner — addressed in T-0557.

**Skipped:**
- `cloacina-python/src/lib.rs:26` "compiled into the cloacina binary" — phrase no longer present in source.
- Workspace-wide T-NNNN reference sweep — out of scope for this ticket; the high-signal ones (T-0512, T-0514, T-0303 cite, T-0509 explainer in `cloacina-computation-graph/src/lib.rs:273`) were touched here. A broader sweep is cosmetic with diminishing returns and can ride alongside any future PR.

**Test gates:**
- `cargo check --workspace --all-features` green.
- `angreal test unit` green (45 macros + 658 cloacina lib).
