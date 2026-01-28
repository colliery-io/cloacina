---
id: encapsulate-defaultrunnerconfig
level: task
title: "Encapsulate DefaultRunnerConfig struct fields"
short_code: "CLOACI-T-0057"
created_at: 2026-01-27T13:56:56.716783+00:00
updated_at: 2026-01-28T04:27:56.122993+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# Encapsulate DefaultRunnerConfig struct fields

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Make DefaultRunnerConfig fields private with accessor methods to enable future validation and prevent breaking changes.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [x] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [x] P2 - Medium (nice to have)
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
- **Current Problems**: All 20+ config fields are `pub`, allowing direct construction and mutation
- **Benefits of Fixing**: Can add validation, change internal representation, deprecate fields gracefully
- **Risk Assessment**: Any internal restructuring is currently a breaking change; users can construct invalid configs
- **Location**: `crates/cloacina/src/runner/default_runner/config.rs:41-126`

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Make config fields private with accessor methods
- [x] Enforce construction via `DefaultRunnerBuilder` only
- [x] Add `#[non_exhaustive]` to prevent struct literal construction
- [ ] Add validation in builder's `build()` for invalid combinations (deferred - no invalid combinations currently defined)
- [x] Update documentation to reflect builder-only construction

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

### 2026-01-27: Completed
- Made all 25 DefaultRunnerConfig fields private
- Added getter methods for each field with appropriate return types:
  - `Option<&str>` for optional string fields
  - `Option<&Path>` for optional path fields
  - `Option<&RoutingConfig>` for optional config references
  - Direct copies for Copy types (Duration, usize, u32, bool)
- Created `DefaultRunnerConfigBuilder` with setter methods for all fields
- Added `#[non_exhaustive]` attribute to prevent struct literal construction
- Updated `Default` impl to use builder
- Updated all internal usages to use getter methods:
  - `config.rs` (builder implementation)
  - `mod.rs` (with_config method)
  - `services.rs` (background service configuration)
  - `cron_api.rs` (cron scheduling checks)
  - `pipeline_executor_impl.rs` (pipeline timeout)
- Updated integration tests to use builder pattern:
  - `cron_basic.rs`
  - `task_execution.rs`
  - `context_merging.rs`
  - `runner_configurable_registry_tests.rs`
- All 223 unit tests pass
- Commit: aa67af9
