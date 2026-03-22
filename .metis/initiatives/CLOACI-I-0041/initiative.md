---
id: test-coverage-and-code-quality
level: initiative
title: "Test Coverage and Code Quality — MockDAL, Stub Tests, Python Tests, DAL Dedup"
short_code: "CLOACI-I-0041"
created_at: 2026-03-21T18:39:28.781656+00:00
updated_at: 2026-03-21T18:39:28.781656+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: test-coverage-and-code-quality
---

# Test Coverage and Code Quality — MockDAL, Stub Tests, Python Tests, DAL Dedup Initiative

## Context

Test coverage audit revealed systemic gaps: 5+ stub tests with no assertions, zero unit tests across the entire Python subsystem (1,599 lines), zero tests on cloacinactl commands, and no MockDAL to enable testing DAL-dependent code. Tech debt audit found 162 duplicated postgres/sqlite DAL function pairs, 6 files over 1,000 lines, and 16 `#[allow(dead_code)]` annotations.

## Goals & Non-Goals

**Goals:**
- Build test infrastructure (MockDAL, test_db helper) to unblock meaningful tests
- Replace all stub tests with real assertions
- Add unit tests for the Python subsystem
- Reduce code duplication in the DAL layer
- Clean up dead code and abandoned features

**Non-Goals:**
- 100% line coverage (focus on meaningful coverage of risky code)
- Rewriting working code that's merely ugly
- Performance benchmarking

## Findings (from audit)

### Stub tests (false coverage)

| Test | File | Issue |
|------|------|-------|
| `test_is_schedule_active` | `cron_scheduler.rs` | No assertions, just comments |
| `test_calculate_execution_times_skip_policy` | `cron_scheduler.rs` | Stub — "would need proper mocking" |
| `test_calculate_execution_times_run_all_policy` | `cron_scheduler.rs` | Stub |
| `test_recovery_attempts_tracking` | `cron_recovery.rs` | Only asserts config value, not logic |
| Workflow registry tests | `workflow_registry/mod.rs` | Only assert `temp_dir.path().exists()` |

### Zero-test modules (high risk)

| Module | Lines | What's untested |
|--------|-------|-----------------|
| `python/loader.rs` | 244 | PyO3 import bridge, ensure_cloaca_module, task collection |
| `python/task.rs` | 524 | @task decorator, PythonTaskWrapper, context stack |
| `python/workflow.rs` | 324 | PyWorkflowBuilder, workflow registration |
| `python/context.rs` | 236 | PyContext get/set/serialize roundtrip |
| `cloacinactl/commands/daemon.rs` | 597 | Register, schedule CRUD, daemon lifecycle |
| `cloacinactl/commands/package.rs` | 369 | Build (Python + Rust), validate, inspect |
| `cloacinactl/commands/api_key.rs` | 198 | Admin key creation |
| `security/db_key_manager.rs` | 1,200 | Key lifecycle, trust chains, ACL traversal |
| `dispatcher/default.rs` | 273 | Task dispatch, executor routing |

### Tech debt

| Finding | Scope | Impact |
|---------|-------|--------|
| 162 duplicated `_postgres`/`_sqlite` DAL function pairs | 13 DAL files, ~290 functions | Doubles maintenance surface |
| 6 files over 1,000 lines | runner.rs (2,263), scheduler.rs (1,487), pipeline_execution.rs (1,412), workflow/mod.rs (1,377), packaged_workflow.rs (1,266), db_key_manager.rs (1,200) | Hard to navigate/review |
| 16 `#[allow(dead_code)]` annotations | admin.rs, dispatcher, executor, macros, testing | Abandoned features? |
| 5 no-op `.map_err(\|e\| e)` calls | database/connection/mod.rs | Useless error transforms |
| `WorkflowRegistryDAL` is a complete stub | dal/unified/workflow_registry.rs | Exported but does nothing |
| `eprintln!` debug output in production FFI code | dynamic_task.rs:148-151, 204-207 | Leaks context data to stderr |
| Incomplete OpenTelemetry stub | observability.rs:62 | Just logs endpoint |

### Missing test infrastructure

- No `MockDAL` — root cause of stub tests (can't test without real DB)
- No `test_db()` helper for in-memory SQLite with migrations
- No `MockKeyManager` for security testing

## Implementation Plan

### Phase 1: Test infrastructure (unblocks everything)
- Create `MockDAL` in `cloacina-testing` implementing DAL trait with in-memory storage
- Create `test_db()` helper: in-memory SQLite with migrations applied
- Create `MockKeyManager` for security tests

### Phase 2: Fix stub tests + add critical coverage
- Replace 5 stub tests with real assertions (cron scheduler, recovery, workflow registry)
- Add Python subsystem tests (loader, task decorator, workflow builder, context)
- Add cloacinactl command tests (daemon, package)
- Add security tests (db_key_manager lifecycle, trust chains)

### Phase 3: Soak test expansion
- Add chaos scenario: kill daemon mid-execution, verify recovery
- Add Python workflow soak (build + register + execute alongside Rust)
- Add key rotation scenario (revoke key during active session)

### Phase 4: Code quality
- Extract DAL backend dispatch macro to eliminate 162 duplicate function pairs
- Split 6 files over 1,000 lines into sub-modules
- Remove dead code behind `#[allow(dead_code)]`
- Remove no-op `.map_err(|e| e)` calls
- Clean up stubs (WorkflowRegistryDAL, OpenTelemetry)
- Replace `eprintln!` with `tracing::debug!` in FFI code
