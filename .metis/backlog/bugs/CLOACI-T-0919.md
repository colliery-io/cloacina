---
id: python-runtime-edges-unkillable
level: task
title: "Python runtime edges — unkillable import hang bricks the subsystem, global workflow-context stack race"
short_code: "CLOACI-T-0919"
created_at: 2026-08-02T16:33:45.835985+00:00
updated_at: 2026-08-02T16:33:45.835985+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


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

- [x] A module-scope infinite loop in a packaged upload results in a visible degraded state (health surface + log) and, if interruption is implemented, a recovered runtime — test with a hostile fixture
- [x] Concurrent loads of two packages from two threads namespace their tasks correctly (test), or cross-thread use hard-errors

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (python-integration report; DEEPDIVE.md register high tier for #1). Verified against main @ 5216e632.
- 2026-08-05 (agent, worktree .claude/worktrees/t-0919 @ 35a1769c): CODE SHAPE VERIFIED before touching anything.
  - Import hang: `crates/cloacina-python/src/loader.rs:38` `const IMPORT_TIMEOUT_SECS: u64 = 60` (hardcoded, no knob); import runs on `std::thread::spawn` at `loader.rs:206` inside `Python::with_gil`; poll-join watchdog at `loader.rs:411-427` (`handle.is_finished()` / 100ms sleep / return Err on elapsed>timeout) — it abandons the thread, no kill, no health surface. Same pattern repeats for CGs in `import_python_computation_graph` (`loader.rs:443+`, timeout at `loader.rs:452`).
  - `PythonRuntime` trait (`crates/cloacina/src/python_runtime.rs:67`) is a 2-method seam + `OnceLock` registry (`:96-110`). No health/state surface today → wedged flag will be added as free functions in this module (cloacina has the `metrics` dep at `crates/cloacina/Cargo.toml:98`; cloacina-python does NOT, so metric emission helpers live here and cloacina-python calls them).
  - `/ready` is `crates/cloacina-server/src/lib.rs:2075` (`async fn ready`), post-#231 predicate is DB-only; utoipa annotation `:2067-2074`; route wired `:1986`. Regression guard test `ready_stays_200_with_crashed_graph_visible_in_health_routes` at `:2747`.
  - Item 2: `static WORKFLOW_CONTEXT_STACK: Mutex<Vec<WorkflowBuilderRef>>` at `crates/cloacina-python/src/task.rs:87`; push/pop/current at `:90/:97/:102`. Consumers audited — ALL are import-time/same-thread: `task.rs:577` (TaskDecorator::__call__), `constructor.rs:71`, `workflow.rs:161/173` (`__enter__`/`__exit__`), `loader.rs:303/308/316`, plus tests. Nothing pushes on one thread and pops on another → thread-local move is safe.
  - Thread-local runtime seam already exists: `crates/cloacina-python/src/runtime_scope.rs` (`CURRENT_RUNTIME` thread_local + `ScopedRuntime` RAII, errors on nesting). Item 2 will mirror it.
  - Test patterns: `crates/cloacina-python/tests/python_package.rs` (`#[serial_test::serial(python_import)]`, `pyo3::prepare_freethreaded_python()`, temp-dir fixture packages) — new hostile-fixture test follows this.
- 2026-08-05 (agent): ITEM 1a/1b LANDED (compiles, `cargo check -p cloacina-python --all-targets` clean).
  - NEW `crates/cloacina-python/src/import_guard.rs` (registered `pub mod import_guard` in lib.rs): `import_timeout()` (env knob `CLOACINA_PYTHON_IMPORT_TIMEOUT_SECS`, default 60s), `ImportThreadIdent` (Arc<AtomicI64>, `.record(py)` via `threading.get_ident()` — pyo3-ffi 0.25 has NO `PyThread_get_thread_ident`), and `supervise_import()` = the ladder. Custom exception `ImportTimeout` via `create_exception!(cloaca, ImportTimeout, PyBaseException, ...)` — BaseException on purpose so a module-scope `except Exception` cannot swallow the deadline.
  - `PyThreadState_SetAsyncExc` signature in pyo3-ffi 0.25.1 is `(c_long, *mut PyObject) -> c_int` (`pystate.rs:64`). Revert protocol IS handled: `if modified > 1 { call again with null_mut() }`.
  - Injection runs on a scratch `std::thread` doing `Python::with_gil` and is poll-joined for 5s (`GIL_ACQUIRE_TIMEOUT`) — not tokio spawn_blocking, because the loader entrypoints are sync fns with no runtime handle; same "can't get the GIL fast ⇒ C-level hold" evidence, no tokio coupling. Grace join 5s, 2 injection attempts.
  - Wedged surface: free functions in `crates/cloacina/src/python_runtime.rs` (NOT a trait method — the trait is per-impl, the wedge is per-process, and cloacina-server must read it without linking pyo3): `mark_python_runtime_wedged(reason)` (latching, keeps FIRST reason, gauge `cloacina_python_runtime_wedged`=1 + `error!`), `is_python_runtime_wedged()`, `python_runtime_wedged_reason()`, `record_python_import_interrupted()` (counter `cloacina_python_import_interrupted_total`), `init_python_runtime_health_metrics()` (publishes gauge 0 baseline, called from `register_python_runtime`). Metrics live here because cloacina has the `metrics` dep and cloacina-python does not.
  - Both loader entrypoints now use the ladder: `import_and_register_python_workflow_named` and `import_python_computation_graph` (their hand-rolled poll-join loops deleted).
  - NEXT: /ready wiring, item 2, tests.
- 2026-08-05 (agent): /READY + ITEM 2 LANDED; new tests GREEN.
  - `/ready` (`crates/cloacina-server/src/lib.rs`) now returns 503 with `reason` = the wedged reason when `cloacina::python_runtime::python_runtime_wedged_reason()` is Some. #231 semantics preserved: the wedge is set by THIS process's own import thread (replica-local), tenant workloads still cannot fail readiness; `ready_stays_200_with_crashed_graph_visible_in_health_routes` untouched. New test `ready_reports_503_when_python_runtime_is_wedged` (asserts 200 first, then 503 + reason names the package, resets the flag before asserting so a failure can't poison the binary). Added `#[doc(hidden)] reset_python_runtime_wedged_for_tests()` because the production flag is latching and process-global.
  - Utoipa annotation on `/ready` WAS edited (200/503 descriptions) + the doc comment → openapi.json + TS client regen REQUIRED. Not yet done at this checkpoint.
  - Item 2: `WORKFLOW_CONTEXT_STACK` is now `thread_local! { ContextStack(RefCell<Vec<WorkflowBuilderRef>>) }` in `crates/cloacina-python/src/task.rs`, plus a process-global `ACTIVE_CONTEXT_PUSHES: AtomicUsize` used ONLY to diagnose cross-thread misuse. `ContextStack` has a `Drop` that decrements the counter by whatever is left, so an interrupted import thread cannot leave it inflated. Cross-thread decorator evaluation = a hard `PyValueError` in EVERY build (a `debug_assert!` panic across the pyo3 boundary would be worse, and it is user-reachable); the `debug_assert!` sits on the `pop_workflow_context()` path, which only internal code can reach.
  - NEW `crates/cloacina-python/tests/python_import_edges.rs` — 3 tests, ALL PASS (`cargo test -p cloacina-python --test python_import_edges`, 2.20s): hostile `while True: pass` at module scope fails via the interrupt path (asserts the error says "interrupted", asserts NOT wedged, asserts the env knob shortened the deadline) and a SUBSEQUENT good package still loads; two threads loading two packages concurrently keep their own tenant/package/workflow; a foreign thread calling the decorator seam gets the deterministic cross-thread error.
  - Documented in the test-file header: a true C-level hang is NOT portably testable (needs a per-platform hostile native module, and a failure would wedge the test binary itself) — the wedged path is covered by the flag unit test + the /ready test instead.
- 2026-08-05 (agent): COMPLETE in the worktree (uncommitted, per instruction). Final state:
  - Added `ensure_python_runtime_usable()` fail-fast at BOTH loader entrypoints — once wedged, a further load would otherwise park a fresh thread on a GIL nobody releases (a thread leak per attempt, and a hang instead of an error).
  - OPENAPI + TS REGEN DONE (the utoipa annotation and its doc comment were both edited, so both gates applied): `cargo run -p cloacina-server --bin cloacina-server -- emit-openapi` → `docs/static/openapi.json` (204941 bytes, parses as JSON; diff vs HEAD is 13 lines, ALL under `paths./ready`). `cd clients/typescript && npm ci && npm run generate` then `npm run check:generated` → exit 0 (clean diff).
  - VALIDATION: `cargo check -p cloacina-python --all-targets`, `-p cloacina --no-default-features --features postgres,sqlite`, `-p cloacina-server --all-targets` — all clean. `cargo fmt --all` run (only the two new files needed formatting).
  - TESTS GREEN: `python_import_edges` 3/3; `cloacina-python --lib` 137/137; `python_package` 13/13, `python_reactor_library` 1/1, `trigger_packaging` 4/4, `cross_language_fan_out` 10/10; `cloacina --lib python_runtime` 2/2; `cloacina-server --lib ready` 3/3 (incl. the #231 guard `ready_stays_200_with_crashed_graph_visible_in_health_routes`). The postgres-backed tests need the dev stack — started `docker compose up -d postgres` from `.angreal/` (port 15432); before that, 3 python_package tests failed on connection-refused only.
  - RESIDUALS: (1) a true C-level hang stays untestable and unrecoverable by design — the wedged flag is the deliverable there; (2) module-scope code that catches `BaseException` in its own loop can swallow the injected `ImportTimeout` and is then indistinguishable from a C hang (it wedges); (3) the wedged flag is latching and never self-clears — `reset_python_runtime_wedged_for_tests()` is `#[doc(hidden)]` test support only; (4) a thread that stays wedged keeps its `ACTIVE_CONTEXT_PUSHES` increment (the `ContextStack::drop` refund only runs if the thread exits) — harmless, the process is already 503; (5) the new env knob `CLOACINA_PYTHON_IMPORT_TIMEOUT_SECS` is documented only in rustdoc — no env-var reference page exists in `docs/` to add it to.
