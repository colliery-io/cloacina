 ---
id: t5-reconciler-and-package-loading
level: task
title: "T5: Reconciler and package loading tests"
short_code: "CLOACI-T-0346"
created_at: 2026-04-03T13:09:25.400850+00:00
updated_at: 2026-04-03T13:09:25.400850+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# T5: Reconciler and package loading tests

## Parent Initiative
[[CLOACI-I-0068]] — Tier 2 (~485 missed lines)

## Objective
Add tests for the reconciler loading pipeline and extraction module. loading.rs is at 18% (337 missed) and extraction.rs is at 0% (148 missed). These handle detecting, extracting, compiling, and registering workflow packages.

## Acceptance Criteria
- [ ] extraction.rs: test extract_source_package, detect_language, validate_manifest
- [ ] loading.rs: test load_rust_package (with test fixture), load_python_package (with fixture)
- [ ] reconciler/mod.rs: test reconcile() with added/removed/modified packages (70% → >80%)
- [ ] Use checked-in test fixture packages (python-workflow.cloacina, rust-workflow.cloacina from cloacinactl/test-fixtures/)
- [ ] Coverage of reconciler/ moves from ~11% to >40%

## Source Files
- crates/cloacina/src/registry/reconciler/loading.rs (337 missed, 18%)
- crates/cloacina/src/registry/reconciler/extraction.rs (148 missed, 0%)
- crates/cloacina/src/registry/reconciler/mod.rs (65 missed, 70%)

## Implementation Notes
The loading pipeline requires either compiled Rust packages (hard to test) or Python packages (easier). Python packages can be created with the helper from python_package.rs tests. The extraction module is pure filesystem operations — testable with temp dirs.

## Status Updates
*To be added during implementation*
