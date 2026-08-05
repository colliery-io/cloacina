---
id: python-runtime-edges-unkillable
level: task
title: "Python runtime edges — unkillable import hang bricks the subsystem, global workflow-context stack race"
short_code: "CLOACI-T-0919"
created_at: 2026-08-02T16:33:45.835985+00:00
updated_at: 2026-08-04T04:59:55.885467+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Python runtime edges — unkillable import hang bricks the subsystem, global workflow-context stack race

## Objective

Fix the two residual structural edges in the Python integration found by the deep dive (which otherwise rated the subsystem production-mature post-I-0140: the flake class is closed, the xfail allowlist empty by design).

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

## Findings

1. IMPORT HANG BRICKS THE PYTHON SUBSYSTEM. Packaged-Python import runs on a dedicated thread under with_gil with a 60s poll-join timeout — but a timeout cannot KILL the thread. User code looping at module scope (import time) keeps the GIL held forever: the leak is not one package failing but the process-wide Python runtime silently disabled until restart (single-interpreter design has no answer). Mitigations to evaluate, in order of invasiveness: Py_SetInterruptFromThread/PyThreadState_SetAsyncExc injection at timeout; sys.settrace-based cooperative deadline installed pre-import; documenting + alerting (loud health state: python_runtime=wedged) so operators at least SEE it. The last is table stakes even if interruption proves unsafe.
2. WORKFLOW_CONTEXT_STACK IS PROCESS-GLOBAL. Registration targeting was made thread-local (ScopedRuntime, errors on nesting) but the @task namespace source is still a process-global stack (crates/cloacina-python/src/task.rs:87) — safe today ONLY because the reconciler serializes package loads. Any future concurrent load (or a user importing cloaca-decorated modules from two threads) mis-namespaces tasks silently. Fix: move the stack to the same thread-local ScopedRuntime seam; add a debug assertion that catches cross-thread use.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] A module-scope infinite loop in a packaged upload results in a visible degraded state (health surface + log) and, if interruption is implemented, a recovered runtime — test with a hostile fixture
- [ ] Concurrent loads of two packages from two threads namespace their tasks correctly (test), or cross-thread use hard-errors

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (python-integration report; DEEPDIVE.md register high tier for #1). Verified against main @ 5216e632.