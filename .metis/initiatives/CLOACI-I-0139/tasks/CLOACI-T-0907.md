---
id: kafka-provider-consumption-proof
level: task
title: "Kafka provider consumption proof + provider-authoring docs — demo-stack CG, cg-feature-tour kafka lane"
short_code: "CLOACI-T-0907"
created_at: 2026-07-15T12:09:23.381471+00:00
updated_at: 2026-07-15T12:09:23.381471+00:00
parent: CLOACI-I-0139
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0139
---

# Kafka provider consumption proof + provider-authoring docs — demo-stack CG, cg-feature-tour kafka lane

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0139]]

## Objective **[REQUIRED]**

Prove + document the end-to-end story. Build a packaged CG workflow that CONSUMES the kafka provider — `constructor!(from = "cloacina-provider-kafka@…", constructor = "kafka_source", grants = { net = […] })` — streaming a topic into a graph on the demo stack, as a harness lane (re-enable/replace the [[CLOACI-T-0891]] `cg-feature-tour` kafka surface, currently deferred). Write the "author your first provider" doc using kafka as the worked example, and surface the native-vs-wasm trust tier in `constructor!`/CLI output.

**Scope:** a features example under the demos harness (discovery-driven CI lane) that packs → uploads → compiles → reconciles → consumes the kafka provider → sees messages fire the graph; docs page (authoring + consuming + trust tiers); CLI surfacing that a native provider is "trusted, unsandboxed" vs wasm "sandboxed".

**Acceptance:**
- [ ] A green CI lane streams Kafka → CG through the consumed provider on the demo stack.
- [ ] The cg-feature-tour kafka surface is re-enabled (or its replacement lane is green).
- [ ] Docs cover authoring + consuming a provider + the trust-tier distinction; `constructor!`/CLI shows the runtime/trust tier.

Parent: [[CLOACI-I-0139]]. Depends on [[CLOACI-T-0906]] (the kafka provider) + core cleanup ([[CLOACI-T-0898]]).

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