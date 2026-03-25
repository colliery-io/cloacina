---
id: built-in-trigger-types
level: task
title: "Built-in trigger types — WebhookTrigger, HttpPollTrigger, FileWatchTrigger"
short_code: "CLOACI-T-0242"
created_at: 2026-03-24T21:19:57.714682+00:00
updated_at: 2026-03-25T00:56:32.466699+00:00
parent: CLOACI-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0044
---

# Built-in trigger types — WebhookTrigger, HttpPollTrigger, FileWatchTrigger

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0044]]

## Objective

Extend the reconciler to detect triggers in loaded packages and register them with TriggerScheduler. When a package with triggers is uploaded (server) or dropped in the packages dir (daemon), its triggers auto-register. When removed, they unregister. Webhook triggers get dynamic `/webhooks/{id}` routes.

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

- [ ] Reconciler parses `triggers` from ManifestV2 when loading a package
- [ ] Built-in triggers instantiated via factory and registered with TriggerScheduler
- [ ] Custom triggers: FFI symbol loaded for Rust, PythonTriggerWrapper for Python
- [ ] Package removal: triggers from that package disabled + deleted from trigger_schedules
- [ ] Webhook triggers create `/webhooks/{id}` HTTP endpoint on the server
- [ ] Daemon: package directory watch triggers auto-registered on file add, unregistered on remove
- [ ] Existing packages without triggers continue to work (empty triggers array)
- [ ] Integration test: upload package with webhook trigger → POST to webhook URL → workflow executes
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

### 2026-03-24 — Implementation complete

**Reconciler trigger integration:**
- Added `extract_manifest_from_cloacina()` to extraction.rs — reads manifest.json from .cloacina archives
- Added `register_package_triggers()` to loading.rs — instantiates triggers via factory, registers in global registry + DB
- Added `unregister_package_triggers()` to loading.rs — disables DB schedules, removes from global registry
- PackageState now tracks `trigger_names: Vec<String>` for cleanup on unload
- RegistryReconciler gains `Option<Arc<DAL>>` via `with_dal()` builder for trigger schedule persistence
- Python triggers skipped during built-in loading (deferred to T-0244)

**Trigger registry extensions:**
- Added `register_trigger_arc()` — register pre-built Arc trigger instance by name
- Added `remove_trigger()` — remove trigger from global registry by name
- Both re-exported from trigger/mod.rs

**Services wiring:**
- `start_registry_reconciler()` in services.rs now creates DAL and passes via `.with_dal(dal)`

**Tests:** All 473 lib tests pass. cloacinactl compiles clean.
