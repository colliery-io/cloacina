---
id: datasourcegraph-graphedge-and
level: task
title: "DataSourceGraph, GraphEdge, and graph assembly from registrations"
short_code: "CLOACI-T-0124"
created_at: 2026-03-15T11:46:39.118466+00:00
updated_at: 2026-03-15T12:12:40.268943+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# DataSourceGraph, GraphEdge, and graph assembly from registrations

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Implement `DataSourceGraph`, `GraphEdge`, `ContinuousTaskConfig`, `JoinMode`, and graph assembly logic as specified in CLOACI-S-0008. The graph is assembled from registered data sources and `#[continuous_task]` declarations.

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

## Acceptance Criteria

## Acceptance Criteria

- [ ] `DataSourceGraph` struct with `data_sources: HashMap`, `tasks: HashMap`, `edges: Vec<GraphEdge>`
- [ ] `GraphEdge` struct with `source`, `task`, `accumulator: Box<dyn SignalAccumulator>`, `late_arrival_policy` (AccumulateForward only in this initiative)
- [ ] `ContinuousTaskConfig` with `triggered_edges`, `referenced_sources`, `join_mode`
- [ ] `JoinMode` enum: `Any`, `All` (only `Any` implemented in this initiative)
- [ ] Graph assembly function: collects registered data sources + continuous tasks, creates edges, validates
- [ ] Validation: no cycles, all referenced sources exist, all detector workflows exist
- [ ] Unit tests: graph assembly from registrations, validation errors (missing source, cycle)

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

## Implementation Notes

### Technical Approach
- In `continuous/graph.rs`
- Graph is assembled at startup from: data source registrations + `#[continuous_task]` metadata
- Each `source` in a continuous task creates a `GraphEdge` with default `SimpleAccumulator` + `Immediate` policy
- Referenced sources have no edges — just listed in `ContinuousTaskConfig::referenced_sources`
- Validation uses existing petgraph infrastructure for cycle detection

### Dependencies
- T-0118 (DataSource), T-0120 (SignalAccumulator), T-0121 (TriggerPolicy), T-0123 (macro registrations)

## Status Updates

- Created `continuous/graph.rs`
- `DataSourceGraph` with data_sources, tasks, edges HashMaps/Vec
- `GraphEdge` with source, task, `Arc<Mutex<Box<dyn SignalAccumulator>>>`, late_arrival_policy
- `ContinuousTaskConfig` with triggered_edges, referenced_sources, join_mode
- `JoinMode::Any`/`All`, `LateArrivalPolicy::AccumulateForward` (default)
- `ContinuousTaskRegistration` struct for assembly input
- `assemble_graph()`: creates edges with default SimpleAccumulator+Immediate, validates sources exist, checks duplicates
- Query helpers: `edges_for_task()`, `edges_for_source()`, `task_ids()`
- 8 passing tests: simple graph, multi-source, unknown source, unknown referenced, duplicate task, edges queries, empty graph
