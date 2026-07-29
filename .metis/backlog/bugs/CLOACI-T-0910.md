---
id: residual-segfault-class-mid
level: task
title: "Residual segfault class — mid-scenario native crash on tokio-rt-worker (postgres/ubuntu)"
short_code: "CLOACI-T-0910"
created_at: 2026-07-29T01:57:16.426726+00:00
updated_at: 2026-07-29T01:57:16.426726+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Residual segfault class — mid-scenario native crash on tokio-rt-worker (postgres/ubuntu)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Root-cause and fix the RESIDUAL native segfault that survived the I-0140 campaign. The 2026-07-29 v0.10.0 release gate reproduced it: `test_scenario_03_function_based_dag_topology.py` (postgres/ubuntu) died `Fatal Python error: Segmentation fault` **mid-scenario** — outside the interpreter-teardown window that round 4's atexit backstop closed. Core `core.tokio-rt-worker.80177`: the crash is on a **tokio runtime worker thread**, with multiple workers parked inside `cloaca.abi3.so` frames (thread shape from the first venv-intact capture; frames still unsymbolized due to the exe-detection bug fixed in PR #212).

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [ ] P1 - High (important for user experience)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: CI lanes (rotating scenario failures: 03, 16, 20, 27 sighted); risk profile for production embedders unknown until the faulting frame is named — mid-execution worker-thread crash is potentially production-relevant, unlike the teardown class.
- **Reproduction**: never reproduced locally — 800+ clean iterations across macOS/postgres and arm64-linux/postgres (docker). All 4 sightings: GitHub runners, ubuntu, x86_64, 2-core. Suspected differentiators: x86_64 and/or tight-core scheduling. Candidate next repro attempts: `--cpus 2` docker throttle; x86_64 emulation; or simply wait — CI now self-symbolizes (unstripped wheel #208 + venv survives failure #208 + real-ELF resolution #212), so the NEXT natural kill names the frame.
- **Expected vs Actual**: scenarios run to completion; instead an intermittent (~1-in-tens-of-lane-runs) SIGSEGV on a tokio-rt-worker mid-scenario.

**Prior art / evidence trail:** I-0140 Status Updates (GIL audit findings B2/D = PyObject lifecycle on runtime threads — still prime suspects for a refcount/UAF race); `.angreal/gil_stress.py` is the harness; the 2026-07-29 release-gate log is the best capture to date.

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
