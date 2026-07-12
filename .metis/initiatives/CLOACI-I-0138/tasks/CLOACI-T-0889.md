---
id: gold-path-example-parameterized
level: task
title: "Gold-path example: parameterized workflow instances (I-0116) — params, named instances, schedules"
short_code: "CLOACI-T-0889"
created_at: 2026-07-11T22:03:08.051490+00:00
updated_at: 2026-07-11T22:31:55.275713+00:00
parent: CLOACI-I-0138
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0138
---

# Gold-path example: parameterized workflow instances (I-0116) — params, named instances, schedules

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0138]]

## Objective **[REQUIRED]**

I-0116 (parameterized workflow instances — named, scheduled, param-bound) shipped with ZERO user-facing example coverage; only test fixtures (`demo-py-cron`, `demo-py-workflow`) use `workflow_params`. Build the gold-path example that demonstrates it end-to-end through the primary interface.

**Surface to exercise (grounded):**
- Rust authoring: `#[workflow] params( name: Type [= default], … )` (workflow_attr.rs:281)
- Python authoring: `@cloaca.workflow_params(**kwargs)` (lib.rs:135, workflow.rs:451)
- Instance registration: named instances with bound params + optional schedule (I-0116; runner `register_workflow_instance` runner.rs:1015 is the embedded API — find and use the SERVER-side equivalent: how do packaged workflows declare/bind instances via upload + API? The compiler parses `declared_params` from package source at build (build.rs run_build) — trace how the server exposes param binding + scheduled instances, and demo THAT)
- Run with per-execution param values via `cloacinactl workflow run` (context/params input)

**Shape (per the T-0886 standard):** `examples/features/workflows/parameterized-instances/` — package.toml + version-dep Cargo.toml + `#[workflow]` with `params(...)` + gold-path README (pack → upload → build → bind instance(s) → run with values → observe) + a bespoke or auto `demos features` runner (auto-joins CI via `demos matrix`).

**Acceptance:** example builds on the demo stack; two named instances with different param bindings both reach Completed; README verified command-by-command; CI runs it.

**Loud-failure clause (I-0137 lesson):** if the server path can't express something I-0116 promised (e.g. schedule binding for packaged workflows), that's a FINDING to surface, not to paper over.

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

### 2026-07-11 — LOUD FINDING + example built; live verification running
**Finding (filed as [[CLOACI-T-0894]]):** I-0116 instance registration is embedded-runner-only — `register_cron_workflow_instance` exists on DefaultRunner + python bindings, but there is NO server route and NO cloacinactl noun. Named/scheduled instances cannot be created through the primary interface. What the gold path DOES support: `params(...)` declaration → compiler extracts typed InputSlots → server validates `workflow run --context` values (T-0757) → bound values arrive as top-level context keys.

**Built accordingly** (`examples/features/workflows/parameterized-workflow/`, branch feat/i0138-examples):
- `sync_file` template: `params(source: String, dst: String, mode: String = "copy", max_files: i64 = 100)` (syntax grounded on the acme-billing fixture); 3 tasks reading bound params off context (plan_sync validates mode, execute_sync, report).
- T-0886 standard shape: package.toml + version-dep Cargo.toml + build.rs + gold-path README (two runs with different --context bindings; a missing-required-param run shown REJECTED; honest "About named instances" note pointing at the doc + the T-0894 gap).
- Harness: refactored `.angreal/demos/features/features.py` — the simple-packaged bespoke runner generalized into `_run_gold_path(label, dir, run_steps)` + `_run_to_completed(ctl, home, workflow, context_path)`; new `demos features parameterized-workflow` command runs the template twice to Completed with different bindings AND asserts the missing-param run is rejected pre-execution. Auto-joined the CI matrix via `demos matrix` (now 13 examples); added to the auto-registration exclude set.

Live run of `angreal demos features parameterized-workflow` in progress (cold dep build).
