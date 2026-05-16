---
id: audit-t5-drop-stale-wire-format
level: task
title: "Audit T5: drop stale wire-format and schema fields surviving I-0094 / I-0102 cutovers"
short_code: "CLOACI-T-0559"
created_at: 2026-05-04T16:10:24.858391+00:00
updated_at: 2026-05-04T20:12:02.025608+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Audit T5: drop stale wire-format and schema fields surviving I-0094 / I-0102 cutovers

Wire-format and schema fields that survived the I-0094 (Pipeline → Workflow) and I-0102 (`[[triggers]]` / `package_type` removal) cutovers but no longer have producers or consumers.

## Objective

Drop the dead schema fields and the dead I-0094 fallback parsers. Where a field is on a serde-deserialized type with `deny_unknown_fields`, removal is a wire-format break and needs a CHANGELOG entry.

## Backlog Item Details

### Type
- [x] Tech Debt — schema simplification.

### Priority
- [x] P2 — Medium. The dead `TriggerDefinition` schema is the loudest; it keeps a Python test fixture alive that the loader would reject if anyone actually ran it through the reconciler.

### Technical Debt Impact
- **Current Problems**: schema types that look load-bearing but aren't; user reading `manifest_schema.rs` gets misled into thinking `[[triggers]]` is valid.
- **Benefits of Fixing**: clearer schema; smaller public type surface; one less broken Python test fixture.
- **Risk Assessment**: Low for the fully-dead fields; Medium for `pipeline_*` enum string parsing if any external system still emits the old strings.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### CloacinaMetadata dead fields

- [ ] `crates/cloacina-workflow-plugin/src/types.rs:314 reaction_mode: Option<String>` — zero production readers (only `types.rs:541` test references it). Macros now set reaction mode via `#[computation_graph(reaction = ...)]` attributes that flow through FFI metadata, not the manifest. Remove the field; update the test.
- [ ] `crates/cloacina-workflow-plugin/src/types.rs:317 input_strategy: Option<String>` — zero readers anywhere. Same reasoning. Remove.

### Manifest schema dead `TriggerDefinition` chain

- [ ] `crates/cloacina/src/packaging/manifest_schema.rs:139 TriggerDefinition` struct — zero readers post-`[[triggers]]` rejection. The struct, the `triggers: Vec<TriggerDefinition>` field on `PackageManifest` (`:178`), the validation block (`:251-281`), and the test data (`:400, :408`) all go.
- [ ] `crates/cloacina/src/packaging/mod.rs:37` re-export of `TriggerDefinition` removed.
- [ ] `crates/cloacina-python/tests/trigger_packaging.rs` — entire test file deleted. It exists exclusively to exercise the dead schema path; the loader would reject the constructed `Manifest` it builds (lines 57-66, 134).
- [ ] Confirm no producer in the workspace still emits `triggers: Vec<TriggerDefinition>`.

### I-0094 stale `pipeline_*` enum aliases

- [ ] `crates/cloacina/src/models/execution_event.rs:190-196 from_str` — accepts both `pipeline_started`/`workflow_started`, `pipeline_completed`/`workflow_completed`, etc. Workspace grep finds zero producers of the `pipeline_*` strings post-I-0094. Drop the legacy arms.
- [ ] Same audit for any other `from_str` in `models/` that still parses pre-I-0094 names.

### Database schema legacy submodules

- [ ] `crates/cloacina/src/database/schema.rs:395, 1071-1072` legacy `schema::postgres` and `schema::sqlite` submodules — comment says "to be removed after migration" (the migration finished). Last consumer: `crates/cloacina/src/dal/unified/api_keys/crud.rs:25` uses `schema::postgres::api_keys`. Migrate that single consumer to `schema::unified` and delete the legacy submodules.
- [ ] `crates/cloacina/src/database/connection/backend.rs:219` "Legacy Type Aliases (for backward compatibility during migration)" block — confirm none of the aliases have surviving callers and delete.
- [ ] `crates/cloacina/src/database/mod.rs:109` "Legacy type aliases - only available when exactly one backend is enabled" — same audit + delete.

### Misleading `legacy_*` identifiers in macro codegen

- [ ] `crates/cloacina-macros/src/computation_graph/codegen.rs:170-187` — `legacy_acc_names_expr` / `legacy_reaction_mode_expr` are NOT legacy; they populate `ComputationGraphRegistration::accumulator_names` / `reaction_mode` which are read by 9 production sites. Rename the variables to drop the `legacy_` prefix and update the comments at L170-171, `cloacina-computation-graph/src/lib.rs:298-305` to reflect that these are load-bearing (not legacy).

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

Sequence:
1. Delete `cloacina-python/tests/trigger_packaging.rs` first (it would block any change to the schema types it imports).
2. Drop `TriggerDefinition` and the `triggers` field from `manifest_schema.rs`.
3. Drop `reaction_mode` / `input_strategy` from `CloacinaMetadata`.
4. Migrate `api_keys/crud.rs` to `schema::unified`, then delete legacy schema submodules.
5. Drop `pipeline_*` enum aliases.
6. Rename `legacy_acc_names_expr` etc.

### Dependencies

- `TriggerDefinition` removal couples to **T-0560 (T6)** which audits re-export shims.
- The schema submodule cleanup touches `dal/unified/api_keys/crud.rs`; coordinate if T-0558 (T4) is in flight.

### Risk Considerations

- **`pipeline_*` enum aliases**: if any external system still emits `pipeline_started`-style strings (older clients?), this is a wire-format break. Confirm with whoever owns I-0094 closure that the rename is complete on every emitter.
- **`CloacinaMetadata` field removal**: changes the `package.toml` schema. Unlikely to break in-tree fixtures (none set the fields) but flag in CHANGELOG.

## Status Updates

### 2026-05-04 — Won't fix; schemas serve as wire-format / data-format back-compat

Re-evaluated during the audit-cleanup sweep. Decision: **do not execute this ticket**. Schemas don't get cleaned up like dead code — they're part of the contract with archives, databases, and external producers, and "no in-tree caller reads this field" is not the same thing as "no value sitting on disk depends on this field."

Per-AC analysis:

**AC #1 — `CloacinaMetadata::reaction_mode` + `input_strategy`** (`package.toml` schema). KEEP. The struct uses `#[serde(deny_unknown_fields)]`. Dropping these fields means any existing `.cloacina` archive whose `package.toml` sets either key fails on re-upload — even though no production code reads them. The "internally dead" framing misses that `package.toml` is a user-facing wire format. Fields stay.

**AC #2 — `manifest_schema.rs::TriggerDefinition` + `Manifest.triggers`** (the `manifest.json` shape inside `.cloacina` archives). KEEP. Same reasoning — it's an archive-format schema. The `cloacina-python/tests/trigger_packaging.rs` test that exercises it is real coverage of the deserialize path, not "dead code keeping dead code alive."

**AC #3 — `models/execution_event.rs::from_str pipeline_*` aliases**. KEEP. These deserialize column values from the database. Migration `020_rename_pipeline_to_workflow` rewrote the schema (table rename) but cannot retroactively rewrite every audit-log row's `event_type` column, especially if the column is `varchar`-shaped. Aliases serve as data-format back-compat for any pre-rename row that still gets rehydrated. Removing them risks silent runtime errors on old data.

**AC #4 — `database/schema.rs` legacy submodules** (Diesel `table!` macros for `postgres_schema` / `sqlite_schema`). DEFER. Technically dead Rust code (only one caller in `dal/unified/api_keys/crud.rs:25` keeps them alive), and the Diesel macros are pure compile-time artifacts that don't touch the DB or migration history. But the cleanup yields ~600 LOC of mechanical schema-mod removal with zero functional benefit. Not worth the churn vs. the small risk that the unification helper for `api_keys` was incomplete. Leave as-is until someone touches that area for an unrelated reason.

**AC #5 — `legacy_acc_names_expr` rename**. DEFER. Pure cosmetic rename; the doc comment in T-0556's commit message already records that the `legacy_` prefix is misleading, and T-0562 (stale comments) covers comment cleanup more broadly. Roll into T-0562 if the prefix is grating; otherwise leave.

### Decision

Closing T-0559 without executing the ACs. The audit findings are valid as observations ("no in-tree code reads X"), but the conclusion ("therefore X is dead") doesn't apply to wire-format / data-format / archive-format schemas. Future audits should distinguish "dead Rust code" from "fields that look dead because the producer/reader sits outside this repository."

Open follow-ups inherited from this ticket:
- Consider rolling `legacy_*` identifier rename into T-0562 if doing the broader comment-and-naming sweep.
- If `database/schema.rs::{postgres,sqlite}` legacy submodules ever do get cleaned up, the migration is a one-line caller swap in `dal/unified/api_keys/crud.rs:25` plus deletion of the corresponding `mod` blocks. ~600 LOC delete with zero risk.

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

## Status Updates **[REQUIRED]**

*To be added during implementation*
