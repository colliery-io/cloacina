---
id: quality-observability-tests-ci
level: initiative
title: "Quality & Observability — Tests, CI, Soak, Benchmarks"
short_code: "CLOACI-I-0052"
created_at: 2026-03-26T05:35:32.086116+00:00
updated_at: 2026-03-26T16:58:07.621289+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: quality-observability-tests-ci
---

# Quality & Observability — Tests, CI, Soak, Benchmarks Initiative

## Context

Quality and observability work consolidated from the previous development cycle:

1. **Test Coverage** (I-0041) — Improved coverage across security modules (db_key_manager, package_signer, verification), dispatcher, database, retry logic, and the cloacina-testing crate. Added 60+ new tests including stub tests, Python tests, and soak chaos scenarios.

2. **CI Restructure** (T-0247) — Moved slow jobs (macOS integration, performance benchmarks, examples) to a nightly workflow. PR CI retains only fast jobs (unit, integration, macros). Soak tests run in both PR and nightly pipelines.

3. **Nightly Workflow** — Scheduled at 3am UTC with manual dispatch support. Jobs: continuous-soak, daemon-soak, server-soak, macOS integration, performance benchmarks, examples-docs validation. Auto-creates a GitHub issue on any failure.

4. **Performance Benchmarking** — Python-based bench (`tests/performance/scheduler_bench.py`) that exercises real deployment paths: build packages, spawn daemon/server processes, measure end-to-end latency. Prior attempts (I-0045, I-0046) taught us that in-process library API benchmarks are not representative. The bench must go through actual daemon or server deployment, and server bench requires Docker orchestration (server + postgres in containers). Continuous scheduling bench is blocked until packaged continuous tasks ship (I-0037).

**Key learnings from prior iterations:**
- Performance bench must test through real deployment paths (daemon/server), not library API calls
- Python is the right language for bench scripts (easy to iterate, same pattern as soak tests)
- Do not build the bench until the features it tests are complete
- Server bench needs Docker orchestration (server + postgres in containers)
- Continuous scheduling bench needs packaged continuous tasks first (I-0037)

## Goals & Non-Goals

**Goals:**
- Improve test coverage across security, dispatcher, and database modules
- CI pipeline: fast PR checks + nightly extended test suite
- Nightly workflow with failure alerting and manual dispatch
- All tests green before shipping any release

**Non-Goals:**
- Soak/chaos test scenarios (deferred to post-I-0049 initiative)
- Performance benchmarks (deferred to post-I-0049 initiative)
- Automated regression detection or trend analysis (future work)
- Multi-tenant performance testing

## Detailed Design

### Test Coverage
Continue expanding unit and integration tests for under-covered modules. Priority areas: security (db_key_manager, package_signer, verification), dispatcher edge cases, database DAL methods, and retry/recovery logic. Use the `cloacina-testing` crate for no-DB workflow unit tests.

### CI Pipeline
- **PR CI**: `angreal cloacina all` (unit + integration + macros). Fast, under 10 minutes.
- **Nightly CI**: Full suite including soak tests, performance benchmarks, macOS integration, and examples validation. Runs at 3am UTC. Creates GitHub issue on failure.

### Soak Tests & Performance Benchmarks
Deferred to a separate initiative after I-0049 (Server & Daemon) ships. See that initiative for soak/chaos scenarios, daemon/server performance benchmarks, and continuous scheduling bench.

## Prior Art

Reference implementation on `archive/cloacina-server-week1`:
- Test coverage: commit `5c4387a` (feat: test coverage improvements + CI restructure)
- Test infrastructure: commit `78c49af` (feat: test infrastructure — test_db/test_dal helpers)
- Test quality: commit `88695f3` (feat: test coverage and code quality)
- CI nightly: within `5c4387a`
- Performance bench v1 (Rust, replaced): commits `5e11e57`, `7fd3184`
- Performance bench v2 (Python): commit `3e7e2da`

Key learnings:
- Performance bench MUST test through real deployment paths (daemon/server), not library API
- Bench should be Python (stdlib only, same pattern as soak tests)
- Don't build the bench until the features it tests are complete
- Continuous scheduling bench needs packaged continuous tasks (I-0053) first
- Old Rust bench recorded fake latency values — all metrics must be real measured durations

## Alternatives Considered

- **In-process Rust benchmarks (criterion)**: Rejected. Prior attempts (I-0045) showed library-level benchmarks do not capture real deployment overhead (process spawn, IPC, network, database). Python e2e bench through actual daemon/server is more representative.
- **Separate CI repo for nightly jobs**: Rejected. Keeping nightly workflow in the same repo simplifies maintenance and ensures tests stay in sync with code.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- `angreal cloacina all` passes (unit + integration + macros)
- Nightly CI workflow running at 3am UTC with macOS integration, examples validation
- Nightly auto-creates GitHub issue on failure
- PR CI stays fast (under 10 minutes)
- Test coverage improved for security, dispatcher, and database modules
- No regressions from previous test suite

## Implementation Plan

1. **Test coverage expansion** — Fill gaps in security, dispatcher, database modules using `cloacina-testing` crate
2. **CI restructure** — Move slow jobs to nightly, keep PR CI fast
3. **Nightly workflow** — Schedule at 3am UTC, manual dispatch, auto-create GitHub issue on failure
