---
id: compile-time-pass-prune-imports
level: task
title: "Compile-time pass — prune imports/deps, dedupe versions, trim features across the workspace"
short_code: "CLOACI-T-0942"
created_at: 2026-09-05T16:44:43.481673+00:00
updated_at: 2026-09-05T16:44:43.481673+00:00
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

# Compile-time pass — prune imports/deps, dedupe versions, trim features across the workspace

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Reduce workspace compile times by pruning dead dependencies, deduplicating
crate versions, and trimming features. Baseline: cold `cargo build -p
cloacina-server` (dev) = 2m55s in a fresh target dir.

## Status Updates (working)

- 2026-09-05: cargo-machete sweep verified by grep (no `::name` usage, no
  feature refs, no build.rs use). REMOVED 21 dead deps across 7 core crates:
  cloacina (bzip2, chrono-tz, croner, dirs, dotenvy, urlencoding, and a
  [dependencies] tracing-test duplicated from dev-deps), cloacina-compiler
  (chrono, serde, tower), cloacina-server (cloacina-workflow-plugin, hyper,
  toml), cloacinactl (base64, bincode, tokio-tungstenite),
  cloacina-computation-graph (once_cell, parking_lot, tracing),
  cloacina-python (serde), cloacina-client (base64), cloacina-agent
  (async-trait). Kept: libsqlite3-sys (optional/feature-linked pin);
  providers + example fixtures SKIPPED (constructor macros route deps —
  machete false-positive class, see feedback_macro_generated_deps_invisible).
  bzip2 0.4/0.5 duplicate eliminated. Workspace check + ui wasm check green.
- TIMING TRUTH: cold `cargo build -p cloacina-server` unchanged (2m55s →
  2m56s) — removed units were off the critical path (cloacina 57s →
  cloacina-server 29s chain + the deliberate constructors-wasm/wasmtime stack
  ~120 unit-seconds dominate). Value is tree hygiene, ~30 fewer units,
  smaller lockfiles, supply-chain surface.
- VENDORING pass (usage-density table over all direct deps, ≤3 call sites):
  - dirs (4 crates, 5 sites, all home_dir()) → 8-line vendored helper per
    crate; dirs/dirs-sys/option-ext GONE from the tree (incl. ui lockfile).
  - mime_guess (1 site, embedded-UI content types) → vendored extension
    match; NOTE: still transitive via reqwest, so hygiene not a unit saved.
  - regex (1 direct site) — SKIPPED: transitive via cel-interpreter anyway.
  - chrono-tz/croner (cloacina-workflow, 1 site each) — KEPT: tz-correct
    cron is correctness-critical; vendoring a tz DB is a non-starter.
  - tokio-postgres (2 sites) — KEPT (async LISTEN/NOTIFY; diesel can't).
  embedded-ui feature check green, wasm client green, workspace green.

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