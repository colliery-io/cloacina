---
id: cross-workflow-dependencies-enable
level: task
title: "Cross-Workflow Dependencies - Enable workflows to depend on other workflow completion"
short_code: "CLOACI-T-0045"
created_at: 2025-12-13T14:58:58.277616+00:00
updated_at: 2025-12-13T14:58:58.277616+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# Cross-Workflow Dependencies - Enable workflows to depend on other workflow completion

## Objective

Allow workflows to wait for other workflows to complete before starting or continuing execution, enabling complex multi-workflow orchestration.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Enables complex orchestration patterns (ETL -> Analytics -> Reporting), environment pipelines (staging -> tests -> prod), workflow composition and reuse
- **Business Value**: Supports enterprise-scale workflow orchestration, reduces workflow duplication
- **Effort Estimate**: XL

## Acceptance Criteria

- [ ] `depends_on: [workflow_names]` parameter in `workflow!` macro
- [ ] `wait_for_workflow = "workflow_name"` parameter in `#[task()]` macro
- [ ] `context.wait_for_workflow(workflow_id)` for dynamic workflow waiting
- [ ] Cross-workflow state tracking and notifications
- [ ] Proper handling when dependency workflow fails
- [ ] Circular dependency detection at workflow definition time
- [ ] Timeout handling for long-running dependencies
- [ ] Performance considerations for cross-workflow polling/notifications
- [ ] Documentation with examples for ETL pipelines and environment deployments

## Implementation Notes

### Proposed Interface

```rust
// Workflow-level dependencies
let analytics_workflow = workflow! {
    name: "analytics_pipeline",
    description: "Run analytics after ETL",
    depends_on: ["etl_pipeline"],  // Wait for this workflow
    tasks: [analyze_data, generate_reports]
};

// Task-level cross-workflow dependencies
#[task(
    id = "deploy_production",
    dependencies = [],
    wait_for_workflow = "staging_tests"
)]
async fn deploy_production(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Only runs after staging_tests workflow completes successfully
}

// Dynamic workflow waiting
#[task(id = "conditional_wait", dependencies = [])]
async fn conditional_wait(context: &mut Context<Value>) -> Result<(), TaskError> {
    let upstream_workflow = context.get("upstream_id")?;
    context.wait_for_workflow(upstream_workflow).await?;
    Ok(())
}
```

### Technical Approach
- Extend workflow definition to track cross-workflow dependencies
- Implement cross-workflow state tracking and notification system
- Add circular dependency detection at workflow registration time
- Support both static (macro) and dynamic (runtime) workflow waiting

### Dependencies
- Workflow registry/catalog
- Cross-workflow notification system
- Database schema for dependency tracking

### Risk Considerations
- Circular dependency detection complexity
- Performance of cross-workflow polling/notifications at scale
- Workflow versioning compatibility
- Timeout and failure propagation across workflows

## Status Updates

- **2025-12-13**: Created from GitHub issue #8
