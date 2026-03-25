---
id: trigger-persistence-crud-api
level: task
title: "Trigger persistence + CRUD API — migration, DAL, endpoints, startup loading"
short_code: "CLOACI-T-0241"
created_at: 2026-03-24T21:19:56.464317+00:00
updated_at: 2026-03-25T00:50:28.982433+00:00
parent: CLOACI-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0044
---

# Trigger persistence + CRUD API — migration, DAL, endpoints, startup loading

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0044]]

## Objective

Extend ManifestV2 with trigger declarations and implement built-in trigger types (webhook, http_poll, file_watch) that can be instantiated from manifest config. This is the foundation — triggers are defined in packages, not via API.

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

- [ ] `TriggerDefinitionV2` struct added to ManifestV2 (name, type, workflow, config)
- [ ] ManifestV2 gains `triggers: Vec<TriggerDefinitionV2>` field (default empty for backward compat)
- [ ] `WebhookTrigger` struct implementing `Trigger` trait — channel-based (webhook handler pushes, poll drains)
- [ ] `HttpPollTrigger` struct implementing `Trigger` trait — configurable URL, interval, condition matching
- [ ] `FileWatchTrigger` struct implementing `Trigger` trait — directory glob scan on poll
- [ ] `create_trigger_from_config(def: &TriggerDefinitionV2) -> Result<Box<dyn Trigger>>` factory function
- [ ] Unit tests: each trigger type, manifest serialization roundtrip with triggers
- [ ] Existing manifest tests + package builder tests still pass
- [ ] All existing tests pass

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

### 2026-03-24 — Design pivoted to package-first triggers

Original exploration found existing trigger infrastructure (DAL, scheduler, tables). However, I-0044 was rewritten: triggers are now declared in `.cloacina` package manifests, not created via API. This task now focuses on:
1. Extending ManifestV2 with `TriggerDefinitionV2` and `triggers: Vec<TriggerDefinitionV2>`
2. Implementing built-in trigger types (WebhookTrigger, HttpPollTrigger, FileWatchTrigger)
3. Factory function to instantiate triggers from manifest config

The existing DAL/scheduler infrastructure remains useful — T-0243 will wire package loading into it.

### 2026-03-24 — Implementation complete

**ManifestV2 extension:**
- Added `TriggerType` enum (webhook, http_poll, file_watch, python) with snake_case serde
- Added `TriggerDefinitionV2` struct (name, type, workflow, poll_interval, allow_concurrent, config)
- Added `triggers: Vec<TriggerDefinitionV2>` to ManifestV2 with `#[serde(default)]` for backward compat
- Added `parse_duration_str()` utility (supports s, m, h, ms)
- Added validation: duplicate trigger names, invalid poll intervals
- Added `DuplicateTriggerName` and `InvalidPollInterval` validation errors
- Updated all ManifestV2 construction sites (python_builder.rs, python_loader.rs)

**Built-in trigger types** (`trigger/builtin.rs`):
- `WebhookTrigger` — channel-based (mpsc sender/receiver), fires on received payload
- `HttpPollTrigger` — reqwest-based HTTP polling with configurable method/expect_status
- `FileWatchTrigger` — glob-based directory scanning with seen-file tracking
- `create_trigger_from_config()` factory function — instantiates from TriggerDefinitionV2

**Tests:** 22 manifest tests (8 new) + 10 builtin trigger tests. All 473 lib tests pass.
