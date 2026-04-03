---
id: soak-tests-performance-benchmarks
level: initiative
title: "Soak Tests & Performance Benchmarks"
short_code: "CLOACI-I-0054"
created_at: 2026-03-26T14:08:37.320020+00:00
updated_at: 2026-03-26T14:08:37.320020+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: soak-tests-performance-benchmarks
---

# Soak Tests & Performance Benchmarks Initiative

## Context

Extracted from I-0052 (Quality & Observability). Soak/chaos test scenarios and performance benchmarks require the opinionated server and daemon infrastructure from I-0049 to be in place — they test through real deployment paths, not library API calls.

Key learnings from prior iterations (I-0045, I-0046):
- Performance bench must test through real deployment paths (daemon/server), not library API
- Python is the right language for bench scripts (easy to iterate, same pattern as soak tests)
- Server bench needs Docker orchestration (server + postgres in containers)
- Continuous scheduling bench needs packaged continuous tasks first (I-0053)

## Goals & Non-Goals

**Goals:**
- Soak tests for daemon and server modes passing reliably
- Performance benchmarks through real deployment paths
- Chaos scenarios (process crashes, network partitions, resource exhaustion)
- Continuous scheduling bench once I-0053 ships

**Non-Goals:**
- Unit/integration test coverage (covered by I-0052)
- CI pipeline restructure (covered by I-0052)
- Automated regression detection or trend analysis (future work)
- Multi-tenant performance testing

## Status — Mostly Complete

Soak tests were built as part of I-0049 (server) and I-0061 (daemon):

### Done
- **Daemon soak** (`angreal cloacina soak`) — drops real fidius source package, waits for compilation, verifies sustained cron execution over 120s, checks daemon health throughout. Logs to `target/soak-test/`.
- **Server soak** (`angreal cloacina server-soak`) — starts Postgres + server, bootstraps auth, uploads workflow package, waits for reconciler compilation, then 60s sustained load: triggers workflow executions every 3 iterations while querying all API endpoints. Verified: 94 executions, 94 pipelines completed, 0 errors.
- **Performance demos** — `angreal performance simple/parallel/pipeline` exist and work.

### Remaining
- **Chaos scenarios** — process crashes, network partitions, resource exhaustion. Not started.
- **Continuous scheduling bench** — deferred until I-0053 ships.
- **Multi-tenant performance** — not in scope yet.
- **CI integration** — soak tests run locally, not in CI pipeline (too slow for PR checks).

## Alternatives Considered

- **In-process Rust benchmarks (criterion)**: Rejected. Library-level benchmarks don't capture real deployment overhead.
- **Separate CI repo for soak/perf jobs**: Rejected. Keeping in same repo ensures tests stay in sync.

## Implementation Plan

1. ~~Soak tests~~ ✓ Done (I-0049, I-0061)
2. ~~Performance demos~~ ✓ Exist
3. **Chaos scenarios** — Future work
4. **Continuous scheduling bench** — After I-0053
