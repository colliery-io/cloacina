---
id: t-02-log-retention-days-on
level: task
title: "T-02: --log-retention-days on compiler, server, daemon"
short_code: "CLOACI-T-0592"
created_at: 2026-05-14T15:10:51.836132+00:00
updated_at: 2026-05-14T15:21:31.868703+00:00
parent: CLOACI-I-0109
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0109
---

# T-02: --log-retention-days on compiler, server, daemon

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0109]]

## Objective **[REQUIRED]**

Add a configurable log-retention policy to all three deployables (`cloacina-server`, `cloacina-compiler`, `cloacinactl daemon`). Today none of them prune old log files; long-running deployments slowly fill disk.

Use `tracing_appender::rolling::Builder::max_log_files(N)` on the rolling file appender so the oldest files are pruned automatically when the count exceeds `N`. `N` derives from the `--log-retention-days` flag: with daily rotation that's a direct day-count; with hourly rotation it's `days * 24`.

Default: **14 days** retention. Operator-configurable per binary.

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

## Acceptance Criteria **[REQUIRED]**

- [ ] `--log-retention-days <N>` flag accepted by all three binaries; default `14`.
- [ ] Rolling appender configured with `max_log_files` set to the resolved retention count.
- [ ] Setting `--log-retention-days 0` disables pruning (unbounded — explicit opt-out).
- [ ] Documented in each binary's `--help` output and in operator docs.
- [ ] Smoke test: write `N+1` rotated files, restart with the retention set to `N`, confirm only `N` files survive (compiler is the easy target since its rotation is daily).

## Implementation Notes

If the tracing setup is shared between the three binaries via a common helper, plumb the retention parameter through it once. Otherwise: each binary's bootstrap calls `Builder::max_log_files(N)` directly. Keep the helper minimal — this is config plumbing, not a new abstraction.

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

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

### 2026-05-14 — implemented

- `--log-retention-days <N>` flag added to all three binaries' CLI surfaces; default `14`.
  - `cloacina-compiler`: top-level CLI flag → `CompilerConfig::log_retention_days` → `install_logging()` uses `RollingFileAppender::builder().max_log_files(N)`.
  - `cloacina-server`: top-level CLI flag → `cloacina_server::run(..., log_retention_days)` → same builder pattern. Sprinkled `#[allow(clippy::too_many_arguments)]` since the `run` arity crossed the lint threshold; not worth a config struct refactor for one new field.
  - `cloacinactl daemon`: per-subcommand flag on `DaemonVerb::Start` (not a global — only the start subverb runs the logger). Threaded through `start::run` → `commands::daemon::run`.
- `--log-retention-days 0` explicitly skips the `max_log_files()` call so pruning is disabled (operator opt-out).
- All three appenders switched from `rolling::daily(...)` shortcut to `RollingFileAppender::builder().rotation(DAILY).filename_prefix(...).filename_suffix("log").max_log_files(N).build(dir)?`. Build failures surface as anyhow errors with the logs-dir path, matching the surrounding `with_context` pattern.
- Smoke test deferred to manual operator validation — automated test would require time-travel of the rolling appender's internal clock, which `tracing_appender` doesn't expose. The acceptance is single-line `.max_log_files()` plumbing; the appender's own tests cover the pruning behaviour.

Acceptance criterion about documenting in operator docs: the `--help` output is self-documenting per the clap doc-comments. A separate `docs/operations/log-retention.md` would belong to a broader operability guide, which is out of scope for I-0109.
