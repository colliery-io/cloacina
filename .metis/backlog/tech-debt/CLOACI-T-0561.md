---
id: audit-t7-clean-up-build-test
level: task
title: "Audit T7: clean up build/test/migration config drift"
short_code: "CLOACI-T-0561"
created_at: 2026-05-04T16:10:27.128917+00:00
updated_at: 2026-05-05T03:45:17.492431+00:00
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

# Audit T7: clean up build/test/migration config drift

Build/test/migration config drift surfaced by the post-I-0102 audit. None of these is itself a bug; collectively they show config decay.

## Objective

Tighten the build/test config: remove no-op feature gates, audit `#[ignore]`'d tests for revival, and clean up migrations that have been wholly superseded.

## Backlog Item Details

### Type
- [x] Tech Debt — config hygiene.

### Priority
- [x] P3 — Low. The migration cleanup is most consequential (clearer history when consolidating); the no-op feature gates are pure noise.

### Technical Debt Impact
- **Current Problems**: feature flags that don't do anything (operators waste time toggling them). `#[ignore]`'d tests with stale rationales (coverage gaps no one notices). Migrations that reference renamed tables.
- **Benefits of Fixing**: clearer ops surface; revived test coverage; cleaner migration history.
- **Risk Assessment**: Low for feature gates. Medium for migration consolidation (touches DB tooling).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### No-op Cargo features

- [ ] `crates/cloacinactl/Cargo.toml`: `sqlite` feature has zero `#[cfg(feature = "sqlite")]` gates in `crates/cloacinactl/src/`. Same for `kafka`. Either delete the feature lines OR document why they exist (forward-looking re-export switches?).
- [ ] `crates/cloacina-compiler/Cargo.toml:31-34`: `sqlite` feature has zero source-level cfg gates. Delete or document.
- [ ] `crates/cloacina-testing/src/lib.rs:60, 70`: `// TODO: remove continuous feature gate — continuous scheduling is always on` (×2). Continuous gating was removed at the engine level (I-0069 closure); the testing crate still gates helpers behind the dead `continuous` feature. Drop the gate.

### `#[ignore]`'d tests

- [ ] `crates/cloacina/tests/integration/signing/{trust_chain,key_rotation,security_failures}.rs` (six cases marked `#[ignore = "Requires database connection"]`). The angreal harness has DB containers; either revive or convert to `#[cfg(test_with_db)]`-style explicit guard.
- [ ] `crates/cloacina/tests/integration/workflow/subgraph.rs:17` — `// TODO(I-0058/T-0306): Migrate subgraph tests to use #[workflow] macro.` Both tickets are long closed. Either migrate or delete.
- [ ] `crates/cloacina/src/dal/unified/task_execution_metadata.rs:1065` — `#[ignore = "requires matching task_name format with internal query — needs investigation"]`. Open ticket or remove.

### Stale migrations

- [ ] **Postgres**: `004_create_cron_schedules_table` + `005_create_cron_executions_table` + `009_create_trigger_tables` — wholly DROPped by `015_drop_old_schedule_tables`. Files reference the renamed `pipeline_executions(id)` foreign key (renamed to `workflow_executions` in `020_rename_pipeline_to_workflow`).
- [ ] **SQLite**: same pattern — `003`, `004`, `008_create_trigger_tables` wholly dropped by `014`.
- [ ] Decision (needs ADR): consolidate migrations on a fresh baseline (squash 001-019 → `001_initial_schema_after_pipeline_workflow_rename`) OR keep history append-only and add migration-changelog comments mapping old→new. Diesel migrations don't easily support consolidation, so the second option is more likely. Document the decision and add the comments.

### Suspicious `#[allow(dead_code)]` annotations

- [ ] `crates/cloacina-macros/src/registry.rs` (5 annotations at lines 60, 154, 258, 265, 273) — suspicious post-I-0096 ctor→inventory flip. Each was put there because a `#[ctor]` constructor consumed the field at runtime; now that inventory replaced ctor, the runtime consumer might be gone. Audit each and either drop the annotation (proving live), drop the field (proving dead), or document why both stay.

### Compiler / packaging service drift

- [ ] `crates/cloacinactl/src/nouns/package/{pack,publish}.rs:36/38` print `"detached sig side-car not yet wired in T-0514"` at runtime. T-0514 is closed; either complete the side-car wiring (this becomes a bug, see T-0557) or rephrase the warning.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green (with revived tests counted).

## Implementation Notes

### Technical Approach

Three sub-PRs:
1. Cargo feature cleanup (mechanical).
2. `#[ignore]` audit — for each test, decide revive vs. delete vs. ticket.
3. Migration history decision (needs ADR-level discussion).

### Dependencies

None.

### Risk Considerations

- **Migration consolidation** is high-blast-radius if mishandled. Recommend ADR before action.
- **`#[allow(dead_code)]` audit in macros** could surface that a "live" field is actually dead (pure win) OR that a "dead" field has a hidden generated-code consumer (regression risk if removed).

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

### 2026-05-04 — Partial completion (items 1–4 of 6)

User-approved scope: items 1–4 only. Items 5 (`#[ignore]`'d test revival) and 6 (migration consolidation) are deferred — they need their own tickets (item 6 needs an ADR per audit's own note).

**Item 1 — No-op Cargo features: SKIPPED.** The audit framing was wrong. `cloacinactl`'s `sqlite`/`kafka` features and `cloacina-compiler`'s `sqlite` feature are functional pass-throughs to cloacina's backend selection (`sqlite = ["cloacina/sqlite"]` etc.). They enable downstream operators to invoke `cargo install cloacinactl --no-default-features --features sqlite` cleanly without writing `--features cloacina/sqlite`. Removing them would break a real public surface.

**Item 2 — `cloacina-testing` `continuous` feature gate: REMOVED.**
- Dropped 4 `#[cfg(feature = "continuous")]` annotations in `crates/cloacina-testing/src/lib.rs`.
- Removed the `continuous` feature line from `Cargo.toml`.
- Promoted `chrono` from `optional = true` to a non-optional dependency (the Cargo.toml's own TODO had foreshadowed this).

**Item 3 — T-0514 warning rephrase: DONE.**
- `crates/cloacinactl/src/nouns/package/pack.rs` and `publish.rs` printed warnings citing closed ticket T-0514.
- Rephrased to: "`--sign <key>` accepted but ignored — detached signature side-car generation is not implemented in the CLI yet." Drops the closed-ticket fingerprint; honest about the state of the feature.
- Note: side-car generation is real outstanding work, not a documentation issue. Wiring it is out of scope for this ticket.

**Item 4 — `cloacina-macros/src/registry.rs` `#[allow(dead_code)]` audit: DONE.**
Removed all 5 annotations. Findings:
- L60 (`impl CompileTimeTaskRegistry`): impl-level annotation was over-broad. `register_task` and `get_all_task_ids` are live; the annotation hid genuinely-dead siblings.
- L154 `validate_single_dependency`, L258 `clear`, L265 `size`: zero callers anywhere. Deleted.
- L273 (`enum CompileTimeError`): annotation hid 3-of-4 dead variants. After removing the annotation:
  - `validate_dependencies` deleted along with `detect_cycles` + `dfs_cycle_detection` private helper.
  - `MissingDependency`, `CircularDependency`, `TaskNotFound` variants deleted.
  - `to_compile_error` simplified from a 4-arm match to a single irrefutable destructure.
  - `find_similar_task_names` + `levenshtein_distance` (only used by the deleted `MissingDependency` arm) deleted.

Net: ~150 LOC of post-I-0096-flip residue removed from registry.rs.

**Out-of-scope follow-up:**
- Pre-existing cloacina-macros warnings unrelated to this ticket (`pascal_case_ident`, `GraphIR::entry_accumulators`/`terminal_nodes`/`incoming_sources`, error enum variants). These are in `computation_graph/{codegen,graph_ir,parser}.rs` and would extend the audit's narrow registry.rs scope. Flag for a follow-up lint sweep ticket.
- Items 5 (`#[ignore]`'d test revival) and 6 (migration history) split out as separate tickets when prioritized.

**Test gates:**
- `cargo check --workspace --all-features` green.
- `angreal test unit` green (45 + 658 tests).
