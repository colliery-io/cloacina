---
id: python-runtime-edges-unkillable
level: task
title: "Python runtime edges — unkillable import hang bricks the subsystem, global workflow-context stack race"
short_code: "CLOACI-T-0919"
created_at: 2026-08-02T16:33:45.835985+00:00
updated_at: 2026-08-07T17:33:29.207147+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


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

## Acceptance Criteria

- [x] A module-scope infinite loop in a packaged upload results in a visible degraded state (health surface + log) and, if interruption is implemented, a recovered runtime — test with a hostile fixture
- [x] Concurrent loads of two packages from two threads namespace their tasks correctly (test), or cross-thread use hard-errors

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (python-integration report; DEEPDIVE.md register high tier for #1). Verified against main @ 5216e632.
- 2026-08-07: COMPLETED — PR #237 merged (squash).

  FINDING 1, import hang. Implemented the full ladder rather than settling for
  the "table stakes" alerting the ticket allowed as a floor. On deadline the
  import thread is interrupted with PyThreadState_SetAsyncExc, carrying a
  custom ImportTimeout that derives from BaseException specifically so that a
  user's `except Exception` at module scope cannot swallow the deadline. The
  injection follows a revert protocol: if the thread does not die, the
  exception is cleared rather than left armed, so a thread that later recovers
  is not killed by a stale async exception.

  Interruption is genuinely best-effort and this is the honest limit: async
  exceptions are only delivered at bytecode boundaries, so a hang inside a C
  extension call never sees it. That is why the health surface is not
  optional — when interruption fails, the runtime latches a wedged flag that
  /ready reports, so an operator SEES the process is disabled instead of
  watching imports silently hang forever.

  FINDING 2, context stack. WORKFLOW_CONTEXT_STACK moved to a thread-local,
  joining ScopedRuntime/ScopedRegistration on the same seam. The reconciler's
  serialization is no longer load-bearing for namespace correctness.

  RESIDUALS, carried knowingly:
  - Module code that catches BaseException is indistinguishable from a C-level
    hang from the outside; both land in the wedged state.
  - The wedged flag never self-clears by design — a half-initialized
    single interpreter is not a state to optimistically declare recovered.
    Clearing it requires a restart.
  - CLOACINA_PYTHON_IMPORT_TIMEOUT_SECS is documented only in rustdoc; it is
    not yet in the operator-facing docs.

  PROCESS NOTE worth keeping: this PR was nearly merged on a false green. After
  the rebase force-push, GitHub reported MERGEABLE with a successful CI run
  attached — but that run had executed against the PRE-rebase commit
  (217add5e), and the rebased head carried zero checks. Both signals were true
  statements about a commit that no longer existed. Fix was to amend for a new
  SHA and force CI onto the real head (f959257c). After any force-push, compare
  the run's headSha to the PR head before trusting green.
