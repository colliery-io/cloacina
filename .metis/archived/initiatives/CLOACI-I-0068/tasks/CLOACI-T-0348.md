---
id: t7-workflow-and-registry-database
level: task
title: "T7: Workflow and registry database coverage"
short_code: "CLOACI-T-0348"
created_at: 2026-04-03T13:09:27.283964+00:00
updated_at: 2026-04-03T18:23:16.603819+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# T7: Workflow and registry database coverage

## Parent Initiative
[[CLOACI-I-0068]] — Tier 3 (~687 missed lines)

## Objective
Improve coverage for workflow operations and the DB-backed workflow registry. workflow/mod.rs is at 74%, registry/workflow_registry/database.rs at 53%, dal/workflow_packages.rs at 36%.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] workflow/mod.rs: test get_task, get_dependencies, remove_task, remove_dependency, subgraph operations
- [ ] workflow_registry/database.rs: test store_binary, retrieve_binary, list_packages, delete_package
- [ ] dal/workflow_packages.rs: test CRUD operations for workflow package records
- [ ] workflow/graph.rs: test topological_sort, find_cycle, unreachable detection (89% → >95%)
- [ ] Coverage of target files moves to >65%

## Source Files
- crates/cloacina/src/workflow/mod.rs (146 missed, 74%)
- crates/cloacina/src/registry/workflow_registry/database.rs (251 missed, 53%)
- crates/cloacina/src/dal/unified/workflow_packages.rs (290 missed, 36%)
- crates/cloacina/src/workflow/graph.rs (44 missed, 89%)

## Implementation Notes
Workflow tests are mostly unit tests (no DB needed). Registry database tests need the TestFixture. workflow_packages DAL follows the same pattern as other DAL tests.

## Status Updates

### 2026-04-03 — Complete (49 new tests, 74 total)

graph.rs (17), workflow/mod.rs (16), workflow_packages.rs (8), database.rs (8). All passing.
