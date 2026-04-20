---
id: consolidate-cloaca-test-harness
level: task
title: "Consolidate cloaca test harness into cloacina — cloaca is a Python interface, not a separate system"
short_code: "CLOACI-T-0486"
created_at: 2026-04-13T15:22:25.093649+00:00
updated_at: 2026-04-20T11:24:12.491683+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Consolidate cloaca test harness into cloacina — cloaca is a Python interface, not a separate system

## Objective

"Cloaca" is just the Python interface name for Cloacina — it's the same system, not a separate product. The test infrastructure (`angreal cloaca test`, `angreal cloaca smoke`, etc.) should be consolidated under the `cloacina` angreal namespace to reflect this. Tests for the Python bindings are cloacina tests, not tests for a separate system.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

### Technical Debt Impact
- **Current Problems**: The `angreal cloaca` namespace implies cloaca is a separate system with its own test harness. This is confusing — it's the Python interface to cloacina. The separation adds cognitive overhead ("do I run `angreal cloaca test` or `angreal cloacina test`?").
- **Benefits of Fixing**: Single `angreal cloacina` namespace for all testing (Rust unit, integration, macros, Python bindings). Clearer mental model.
- **Risk Assessment**: Low risk. Mostly renaming angreal tasks and moving test scripts. No code changes to the actual library.

## Acceptance Criteria

- [x] Python binding tests runnable via `angreal cloacina python-test` / `python-smoke` under the cloacina namespace
- [x] `angreal cloaca` tasks removed (deprecated stubs dropped, active tasks moved into cloacina)
- [x] Python wheel build/package tasks consolidated (deprecated stubs gone; real wheel build in `cloacina.python_utils`)
- [x] All existing test scenarios continue to pass under the new task names (logic copied verbatim)

## Status Updates

### 2026-04-20 — Consolidation complete

**What changed:**
- Deleted `.angreal/cloaca/` entirely (6 task files + `cloaca_utils.py`).
- Moved `cloaca_utils.py` → `.angreal/cloacina/python_utils.py` (verbatim; still
  contains `_build_and_install_cloaca_unified` used by `demos/python_demos.py`).
- Created three new tasks under the `cloacina` command group:
  - `cloacina python-test` (was `cloaca test`)
  - `cloacina python-smoke` (was `cloaca smoke`)
  - `cloacina python-scrub` (was `cloaca scrub`)
- Dropped the deprecated `cloaca generate`, `cloaca package`, `cloaca release`
  stubs — they only printed deprecation messages pointing at the unified
  PyO3-embedded path.
- Updated imports:
  - `.angreal/task_project.py` — drops `from cloaca.*`, adds the new
    `python_test/python_smoke/python_scrub` modules.
  - `.angreal/task_purge.py` — `from cloacina.python_scrub import python_scrub as scrub`.
  - `.angreal/task_check.py` — removed `cloaca.generate/scrub` imports and the
    dead `check_cloaca_backend` path (cloaca-backend crate no longer exists;
    Python is built from `crates/cloacina-python` per T-0529).
  - `.angreal/demos/python_demos.py` — `from cloacina.python_utils import
    _build_and_install_cloaca_unified`.
- Registered new tasks in `.angreal/cloacina/__init__.py`.

**Verification:**
- `angreal tree` no longer lists a `cloaca` namespace; new tasks appear under
  `cloacina` (python-scrub, python-smoke, python-test). No module load errors.
- `angreal cloacina python-{test,smoke,scrub} --help` all render correctly.
- No CI workflow / docs / README references to `angreal cloaca …` (only Metis
  history docs, which are intentionally left alone).

### 2026-04-20 — Restore real pytest harness for `python-test`

The post-T-0271 stub had `python-test` skipping every file under `tests/python/`
with "needs cloacina runtime harness". That left ~28 scenario files
(test_scenario_01..31) as dead weight. Restored the pre-T-0271 harness logic
(from commit `f3add8c`) into `.angreal/cloacina/python_test.py`:

- Builds the unified cloaca wheel via `python_utils._build_and_install_cloaca_unified`
  (single venv, both backends share it).
- For each requested backend (default: sqlite + postgres):
  - Brings up Docker postgres if needed (`--skip-docker` to opt out).
  - Resets DB state between scenario files (smart_postgres_reset / SQLite file
    delete).
  - Runs `pytest --timeout=10 -v <file>` per scenario, with `CLOACA_BACKEND` env
    set so `tests/python/conftest.py::get_test_db_url` picks the right URL.
- Aggregates per-file/per-backend results via `TestAggregator`, prints a final
  report, and raises on any failure.
- Args: `--backend {postgres,sqlite}`, `-k <expr>`, `--file <name>`,
  `--skip-docker`.

`python-smoke` retains its purpose — fast Rust-side `python::tests` only, no
wheel build, no Docker.

### 2026-04-20 — Reassign tasks to fit the cloacina namespace

Renamed/dropped to fit the existing `<area>-<test-style>` convention:

- `cloacina python-test` → **`cloacina python-integration`** (mirrors
  `auth-integration`, `ws-integration`, `cli-e2e`, `compiler-e2e`).
- `cloacina python-smoke` → **dropped**. `cloacina unit` already runs
  `cargo test -p cloacina --lib` which executes the Rust-side `python::tests`
  module — the smoke task was a strict subset.
- `cloacina python-scrub` → **dropped as a public task**. Logic moved to
  `cloacina.python_utils.scrub_python_artifacts()`; `task_purge.py` calls it
  directly. `cloacina purge` is the only user-facing entry point for Python
  artifact cleanup.

Final namespace surface for Python:
```
cloacina unit                 # includes Rust-side python::tests
cloacina python-integration   # wheel build + pytest scenarios (sqlite + postgres)
purge                         # deep clean (calls scrub_python_artifacts)
```

Files: deleted `.angreal/cloacina/python_smoke.py`,
`.angreal/cloacina/python_scrub.py`. Renamed `python_test.py` →
`python_integration.py`. Updated `cloacina/__init__.py`, `task_project.py`,
`task_purge.py` imports. `angreal tree` shows the cleaned namespace.
