---
id: re-author-stale-packaged-examples
level: task
title: "Re-author stale packaged examples off the cloacina umbrella + path deps — lean version-dep form"
short_code: "CLOACI-T-0900"
created_at: 2026-07-12T18:51:33.225632+00:00
updated_at: 2026-07-12T18:51:33.225632+00:00
parent: CLOACI-I-0138
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0138
---

# Re-author stale packaged examples off the cloacina umbrella + path deps — lean version-dep form

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0138]]

## Objective **[REQUIRED]**

**Finding (2026-07-12, "why does complex-dag pull in so much"):** the OLD packaged examples were never re-authored to the lean version-dep form (the T-0887 shape that `simple-packaged` + my new examples use). They're stale two ways:

1. **Umbrella `cloacina` dep.** `complex-dag/Cargo.toml` has `cloacina = { path = ... }` — the FULL engine crate (server DAL, postgres, diesel, executor) — pulled in ONLY to `use cloacina::{Context, TaskError}`, which the lean `cloacina-workflow` re-exports. So every packaged build compiled the whole engine (multi-minute cold build) for two types.
2. **`../../../../crates/` path deps** (pre-T-0887). These don't resolve when the compiler stages the package to a temp dir, and `--dev-workspace` only patches crates.io VERSION deps — so these examples FAIL the gold-path lane outright.

**Stale packaged examples (have `package.toml` + the bad dep shape):** `complex-dag` (umbrella + 6 path deps), `packaged-workflows` (umbrella + 6), `packaged-triggers` (umbrella + 6), `packaged-graph` (5 path deps, no umbrella). NOTE: the embedded `cargo run` examples (cron-scheduling, multi-tenant, conditional-retries, …) legitimately use the `cloacina` engine + path deps — they are NOT packaged, so they're out of scope.

**Fix:** re-author each to the lean version-dep form (mirror `simple-packaged`): drop the umbrella `cloacina` dep, import `Context`/`TaskError` from `cloacina-workflow`, switch every `path = "../../../../crates/…"` → `"0.10"` version deps, drop cruft (tokio `full`→minimal, tracing-subscriber). Then they resolve via `--dev-workspace` + build in seconds. Each re-authored example joins the gold-path CI matrix (they're already discovered by the T-0138 registrar).

**Progress:** `complex-dag` DONE this session — re-authored + import fixed; offline `cargo build --lib` went from a multi-minute umbrella compile to **11s**; gold-path lane verification next. Remaining: `packaged-workflows`, `packaged-graph`, `packaged-triggers` (also needs a fire-the-trigger lane step — currently in `_PACKAGED_SKIP`).

**Acceptance:** all four packaged examples use lean version deps (no umbrella, no path deps), build fast, and pass the gold-path lane (or, for packaged-triggers, a trigger-fire lane).

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

### 2026-07-12 — 3 of 4 re-authored + verified; packaged-triggers remains
- **complex-dag** — re-authored (dropped umbrella `cloacina`; `Context`/`TaskError` now from `cloacina-workflow`; path→version deps; cruft trimmed). Offline `cargo build --lib`: multi-min → **11s**. Un-skipped.
- **packaged-workflows** — path→version deps (umbrella was already dev-only). Offline build **11.9s**. Un-skipped.
- **packaged-graph** — path→version deps. Offline build **13s**. **Gold-path lane VERIFIED live**: build_status=success (lean deps resolved via `--dev-workspace`) → inject → reactor `packaged_market_maker_reactor` fired.
- Committed `59873086`. complex-dag/packaged-workflows verified offline (same lean form + default workflow-run assertion); CI runs them.

**Remaining:** `packaged-triggers` — still stale-dep AND needs a fire-the-trigger lane step (trigger-fired, not `workflow run`); stays in `_PACKAGED_SKIP`. That's the one open item.
