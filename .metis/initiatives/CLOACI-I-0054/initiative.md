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

## Blocked By

- I-0049 (Server & Daemon — Deployment Infrastructure)

## Detailed Design

### Soak Tests
- `angreal soak --mode daemon` — Sustained load against daemon process with concurrent injectors
- `angreal soak --mode server` — Sustained load against server process (containerized with postgres)
- Configurable duration, injector count, and failure thresholds

### Performance Benchmarks
- Python-based (`tests/performance/scheduler_bench.py`)
- Build real packages, spawn daemon/server, measure e2e latency
- `angreal performance daemon` for daemon-mode bench
- Server bench requires Docker compose (server + postgres containers)

### Continuous Scheduling Bench
- Deferred until I-0053 (Continuous Scheduling) ships packaged continuous tasks

## Prior Art

Reference implementation on `archive/cloacina-server-week1`:
- Performance bench v1 (Rust, replaced): commits `5e11e57`, `7fd3184`
- Performance bench v2 (Python): commit `3e7e2da`
- Soak test infrastructure: within commit `5c4387a`

## Alternatives Considered

- **In-process Rust benchmarks (criterion)**: Rejected. Prior attempts (I-0045) showed library-level benchmarks do not capture real deployment overhead (process spawn, IPC, network, database).
- **Separate CI repo for soak/perf jobs**: Rejected. Keeping workflows in the same repo ensures tests stay in sync with code.

## Implementation Plan

1. **Soak tests** — Daemon soak first, server soak after Docker orchestration is ready
2. **Performance bench** — Python-based daemon bench, then server bench with Docker compose
3. **Continuous scheduling bench** — After I-0053 ships
