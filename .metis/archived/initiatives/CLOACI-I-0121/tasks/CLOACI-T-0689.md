---
id: ia-tree-old-new-page-map
level: task
title: "IA tree + old→new page map (authoritative)"
short_code: "CLOACI-T-0689"
created_at: 2026-06-15T14:02:06.661392+00:00
updated_at: 2026-06-15T14:04:31.118984+00:00
parent: CLOACI-I-0121
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0121
---

# IA tree + old→new page map (authoritative)

## Parent Initiative

[[CLOACI-I-0121]]

## Objective

The authoritative IA tree and the old→new page map that guides I-0121 (skeleton)
and I-3 (body moves). Ratified with the user 2026-06-15.

## Locked decisions

- Shared core-objects section is named **`/engine`**.
- **Tutorial model A**: one track per topic with **language tabs** (Rust + Python),
  **Rust is the default tab**. "How do I do X" expressed in two dialects.
- **Concept lives in `/engine`; tutorials live in the doors.** "Workflows" is
  described once in `/engine`; *building* a workflow has a separate `/embed`
  tutorial and a separate `/service` tutorial (different onboarding experiences).
- Two **co-equal** doors, no recommended badge. Embedded-first is a stated
  architectural principle, not the onboarding default.

## Target tree + source mapping

```
/                      Home (dual doors)              ← reframe /_index.md (I-3)
/start                 orientation hub                ← /quick-start/_index.md
  what-is-cloacina     two-ways + embedded principle  ← NEW (from /_index + /quick-start)
  is-cloacina-for-you                                 ← /quick-start/when-to-use.md
  concepts                                            ← /quick-start/concepts.md
  features                                            ← /quick-start/features.md
  install                                             ← /quick-start/install.md

/engine                SHARED core objects (I-2 fills; Rust+Python)
  workflows            concept+reference              ← /workflows/_index.md, /workflows/reference/*
  computation-graphs   concept+reference              ← /computation-graphs/_index.md, /computation-graphs/reference/*
  task|context|runner|trigger|cron-schedule|reactor|
  accumulator|node|boundary|package                  ← NEW per-primitive (I-2)
  explanation          engine design                 ← /workflows/explanation/* (architecture-overview, macro-system,
                                                        guaranteed-execution, dispatcher, versioning, context-mgmt,
                                                        task-execution-sequence, trigger-rules, cron-scheduling),
                                                        /computation-graphs/explanation/*,
                                                        /platform/explanation/{package-format,packaged-workflow-architecture,
                                                        ffi-system,inventory-and-runtime-seeding}
  (primitive how-tos: accumulator-types, when-all, sequential-strategy,
   filter-reactor-firings-with-cel, cg-health, reactor-triggered-workflows,
   invoke-cg-from-workflow, subscribe-workflow-to-reactor, python CG how-tos → I-2 places)

/embed                 DOOR A — integrate into your app (Rust default + Python)
  quick-start                                         ← NEW (from /python/quick-start.md + Rust first-workflow)
  tutorials            tabbed Rust/Python             ← /workflows/tutorials/library/01-04,
                                                        /computation-graphs/tutorials/library/07-10,
                                                        Python in-process tutorials 00-04 + CG 09-11
  how-to                                              ← /workflows/how-to/{conditional-retries,variable-registry,
                                                        observe-execution-state,monitoring-executions,cleaning-up-events,
                                                        testing-workflows}, /python/workflows/how-to/{backend-selection,
                                                        testing-workflows,performance-optimization}
  explanation                                         ← /workflows/explanation/{context-management,task-deferral},
                                                        /python/workflows/explanation/python-runtime-architecture

/service               DOOR B — operate it as a service
  quick-start                                         ← NEW (condensed /platform/tutorials/01-deploy-a-server)
  tutorials                                           ← /platform/tutorials/*,
                                                        /workflows/tutorials/service/05-10,
                                                        /computation-graphs/tutorials/service/07-10,
                                                        Python service tutorials 05-08
  how-to                                              ← /platform/how-to-guides/* (all),
                                                        /workflows/how-to/{migrating-to-service-mode,multi-tenant-setup,
                                                        multi-tenant-recovery}
  explanation                                         ← /platform/explanation/{database-backends,execution-agent-fleet,
                                                        horizontal-scaling,multi-tenancy,observability,reconciler-pipeline,
                                                        security-model,performance-characteristics}

/reference             consolidated lookup
  python-api                                          ← /python/api-reference/*
  rust-api (generated)                                ← /api-reference/rust/* (route/nav, do not hand-edit)
  python-bindings (generated)                         ← /api-reference/cloaca/* (route/nav)
  cli|http-api|websocket|config|metrics|errors|macros|
  package-manifest|api-error-envelope|database-admin|
  repository-structure|testing-crate                 ← /platform/reference/*, /workflows/reference/*,
                                                        /python/workflows/reference/environment-variables.md
  sdks                                                ← /sdks/*
  glossary                                            ← /glossary.md
  troubleshooting                                     ← /troubleshooting.md

/contributing          stays                          ← /contributing/*
```

Notes: leaf-level placement within `/engine` and `/reference` is finalized by
I-2/I-3; I-0121 builds the section landings and applies redirects to those
landings. Every moved page gets the old path in the new page's `aliases:` when
its body is relocated (bulk in I-3).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] IA tree finalized and ratified with the user
- [x] Locked decisions recorded (`/engine`, tabbed-Rust-default, concept-in-engine)
- [x] Category-level source mapping captured for every current section
- [ ] (I-3) per-page `aliases` applied as bodies move

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