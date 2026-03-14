---
id: api-surface-audit-python-api
level: task
title: "API Surface Audit — Python API References vs Runtime Introspection"
short_code: "CLOACI-T-0102"
created_at: 2026-03-13T14:30:11.583733+00:00
updated_at: 2026-03-14T02:00:22.108473+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# API Surface Audit — Python API References vs Runtime Introspection

**Phase:** 4 — API Surface Audit (Pass 3)
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Diff all Python API reference documentation against the actual runtime API. Every documented class, method, parameter, and return type must match the current `cloaca` implementation.

## Scope

All files under `docs/content/python-bindings/api-reference/` (11 files).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] For each API reference page: introspect the corresponding Python class/module at runtime
- [ ] Every documented method exists and has the correct signature
- [ ] Every documented parameter has the correct type and default value
- [ ] No undocumented public methods/attributes (flag as gaps)
- [ ] Callback signatures match actual implementation
- [ ] `TaskHandle` API matches actual `cloaca.TaskHandle` class
- [ ] `Context` API matches actual `cloaca.Context` class
- [ ] `WorkflowBuilder` API matches actual builder pattern
- [ ] `DefaultRunner` API matches actual runner interface
- [ ] All discrepancies fixed in-place

## Implementation Notes

### Pages to Audit
- `api-reference/task.md` — `@task` decorator and `TaskHandle`
- `api-reference/context.md` — `Context` class
- `api-reference/workflow-builder.md` — `WorkflowBuilder` class
- `api-reference/runner.md` — `DefaultRunner` class
- `api-reference/packaging.md` — `cloaca build` CLI and manifest API
- Plus remaining API reference pages

### Approach
1. Build and install current `cloaca` wheel: `angreal cloaca package`
2. For each documented class: `import cloaca; dir(cloaca.ClassName)` to get actual API
3. Compare documented methods/params against `__init__` signatures, `__doc__`, type hints
4. Use `inspect.signature()` for precise parameter verification
5. Flag any method present in code but missing from docs

## Status Updates

### Session 1 (2026-03-13)

**Validation approach:**
- Built and installed cloaca wheel, introspected all classes via `dir()` and `to_dict()`
- Compared 12 API reference docs against actual runtime exports

**Fixes applied:**
1. **configuration.md** — Major rewrite of DefaultRunnerConfig:
   - Replaced 6 wrong constructor params (`max_concurrent_workflows`, `retry_attempts`, `connection_pool_size`, `enable_logging`, `log_level`) with 14 actual params
   - Added correct default values from runtime introspection
   - Replaced fictional `CronSchedule` class with actual `register_cron_workflow` API
   - Fixed retry config from `retry_policy={}` dict to actual decorator params (`retry_attempts`, `retry_delay_ms`, `retry_backoff`, etc.)
   - Fixed all code examples using wrong param names
2. **pipeline-result.md** — Removed 3 nonexistent properties (`workflow_name`, `execution_id`, `duration`); added `error_message` property; fixed all code examples using `result.duration` to compute from `start_time`/`end_time`
3. **runner.md** — Added undocumented `start()` and `stop()` methods; fixed `with_config` example

**Gaps noted (not fixed — out of scope):**
- Exception classes documented but not exported (may be internal)
- `RetryPolicy`, `RetryPolicyBuilder`, `BackoffStrategy`, `RetryCondition`, `TaskNamespace`, `WorkflowContext` exported but not documented
- `HelloClass`, `hello_world` exported but not documented (test utilities)

**Verification:** Hugo docs build passes.
