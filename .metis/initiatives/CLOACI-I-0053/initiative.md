---
id: continuous-scheduling-complete
level: initiative
title: "Continuous Scheduling — Complete Implementation with Packaged Deployment"
short_code: "CLOACI-I-0053"
created_at: 2026-03-26T05:35:56.310027+00:00
updated_at: 2026-03-26T05:35:56.310027+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: continuous-scheduling-complete
---

# Continuous Scheduling — Complete Implementation with Packaged Deployment

## Context

Continuous reactive scheduling is the most complex feature in Cloacina. It was partially implemented across I-0023/24/25/30 but never got a deployment path through packages. This initiative completes the entire feature end-to-end.

The continuous scheduling system:
- **DataSources** describe external data (tables, streams, files)
- **Detector workflows** watch data sources for changes, emit ComputationBoundaries
- **SignalAccumulators** buffer boundaries per task
- **TriggerPolicies** decide when to fire (Immediate, Count, Time-based)
- **Watermarks** track data completeness and handle late arrivals
- **ExecutionLedger** records all activity
- **ContinuousScheduler** orchestrates the reactive graph

Previous implementation (archive/main-pre-reset):
- Core reactive scheduling works (I-0023, commit `bbc6e0a`)
- Watermarks + late arrival + LedgerTrigger (I-0024, commit `8a6bf67`)
- Persistence + crash recovery (I-0025, commit `ea1e50d`)
- Wired into DefaultRunner (I-0030, commit `ef0bdcd`)

What was MISSING (and caused the performance bench to stall):
- **Packaged continuous tasks (I-0037)** — No way to declare data sources, detectors, or continuous tasks in a `.cloacina` manifest. The only way to use continuous scheduling is through the in-process library API.
- **Daemon/server support** — No CLI commands or API endpoints for continuous scheduling.
- **Real-world testing** — Soak tests used in-memory ledger injection, not real package deployment.

## Goals

- Re-apply core continuous scheduling (I-0023/24/25/30) with proper testing
- Extend ManifestV2 to declare data sources, detector workflows, and continuous tasks
- Reconciler auto-registers continuous scheduling components on package load
- Daemon and server support continuous scheduling from packages
- Bench scenario: INSERT rows into a watched table, detector picks them up, tasks fire
- Soak test with real packaged continuous workflows

## Non-Goals

- Changing the core scheduling algorithm
- Multi-cluster support

## Acceptance Criteria

- Core continuous scheduling works (boundaries, accumulators, watermarks, ledger)
- ContinuousScheduler wired into DefaultRunner
- ManifestV2 supports `data_sources`, `detectors`, `continuous_tasks` declarations
- Package reconciler registers continuous components on load
- A `.cloacina` package can declare a detector that watches a SQLite table
- INSERT rows into watched table -> detector emits boundaries -> tasks fire
- `angreal soak` includes continuous scheduling scenario
- Performance bench includes continuous scheduling scenario
- All existing tests pass

## Prior Art

Reference implementations on archive branches:
- `archive/main-pre-reset`: Core continuous scheduling (`bbc6e0a`, `8a6bf67`, `93b25b8`, `ea1e50d`), wired into DefaultRunner (`ef0bdcd`)
- `archive/cloacina-server-week1`: Continuous soak tests (`e2c8f0b`, `74e3038`, `39f3ef8`)

What was missing (caused I-0046 bench to stall):
- No ManifestV2 fields for data sources, detectors, or continuous tasks
- No reconciler support for auto-registering continuous components from packages
- No daemon/server CLI for continuous scheduling
- Soak tests used in-memory ledger injection, not real package deployment
- The bench couldn't test continuous scheduling because there was no deployment path

## Implementation Notes

This is intentionally last in the roadmap because:
1. It is the most complex feature
2. It depends on packages, triggers, server/daemon all working
3. The packaged deployment (I-0037) builds on the same reconciler that packaged triggers use
4. We need the bench infrastructure in place before we can properly validate it
