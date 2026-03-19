---
id: packaged-continuous-tasks-dynamic
level: initiative
title: "Packaged Continuous Tasks — Dynamic Graph Loading and Hot-Reload"
short_code: "CLOACI-I-0037"
created_at: 2026-03-18T13:42:15.458679+00:00
updated_at: 2026-03-18T13:42:15.458679+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: packaged-continuous-tasks-dynamic
---

# Packaged Continuous Tasks — Dynamic Graph Loading and Hot-Reload Initiative

## Context

Continuous scheduling (I-0023/I-0024/I-0025) is fully implemented as a library-level feature: data sources emit boundaries, accumulators buffer them, trigger policies decide when to fire, and tasks execute in a reactive DAG. However, **continuous tasks cannot be packaged and uploaded** to the server or daemon. They must be wired in Rust code at compile time.

Regular workflow tasks have a complete distribution story: `#[packaged_workflow]` → FFI symbols → `.cloacina` package → upload → reconciler loads → execute. Continuous tasks have none of this. The `#[continuous_task]` macro generates a struct but no FFI symbols, no package manifest, and no dynamic loading path.

This creates a fundamental asymmetry: regular workflows can be authored locally, packaged, and deployed to a running server without restart. Continuous tasks require recompilation of the server binary.

### Specification Update Required

**CLOACI-S-0001** (Continuous Reactive Scheduling) currently states that graph topology changes require a restart. This decision must be revisited as part of this initiative — if users upload packages containing continuous tasks, hot-reload of the graph becomes a requirement, not an option.

## Goals & Non-Goals

**Goals:**
- Extend `.cloacina` package format to include continuous task declarations, data source bindings, and graph topology
- Extend `#[continuous_task]` macro to emit FFI symbols (matching `#[packaged_workflow]` pattern)
- Extend `RegistryReconciler` to detect and load continuous components from packages
- Implement dynamic graph mutation on `ContinuousScheduler` — add nodes/edges at runtime when a package is loaded, without disrupting existing state
- Handle package removal: tear down associated nodes/edges, clean up accumulator state and drain cursors
- Support this in both server (HTTP upload) and daemon (directory watch) modes

**Non-Goals:**
- Live editing of existing continuous task code (requires re-upload of the package)
- Cross-package data source sharing (each package declares its own sources)
- Python continuous tasks (covered by I-0026)
- Continuous scheduling UI (future)

## Detailed Design

### What exists today

| Component | Status | Location |
|-----------|--------|----------|
| `#[continuous_task]` macro | Generates struct + Task impl, **no FFI** | `cloacina-macros/src/continuous_task.rs` |
| `ContinuousTaskRegistration` | Metadata struct (task_id, dependencies, trigger policy) | `continuous/graph.rs` |
| `DataSource` | Enum of source types (S3, Kafka, filesystem, custom) | `continuous/datasource.rs` |
| `ContinuousScheduler` | Assembles graph once at startup, runs fixed topology | `continuous/scheduler.rs` |
| `DefaultRunner` registration | `register_data_source()`, `register_continuous_task()`, `register_continuous_task_impl()` — consumed at startup | `runner/default_runner/mod.rs` |
| Accumulator state persistence | Drain cursors, boundary WAL, detector state | `continuous/state_management.rs` |

### What needs to change

**1. Package format extension**

The `.cloacina` archive needs to declare continuous components alongside regular tasks. Options:
- Extend the existing `cloacina_get_task_metadata` FFI symbol to include continuous metadata
- Add a new FFI symbol `cloacina_get_continuous_metadata` that returns data source declarations and graph edges
- Embed a `manifest.toml` in the archive declaring the continuous topology

**2. `#[continuous_task]` FFI symbols**

The macro needs to emit:
- Task constructor (already exists as `Arc<dyn Task>`)
- Metadata: task_id, upstream data sources, trigger policy configuration, dependencies
- Data source declarations: type, connection config, boundary format

**3. Reconciler integration**

When the reconciler loads a package:
- Check for continuous components in the metadata
- If present, extract data source declarations and task registrations
- Call into the `ContinuousScheduler` to dynamically add the new subgraph

**4. Dynamic graph mutation (the hard part)**

The `ContinuousScheduler` currently:
- Assembles a `DataSourceGraph` at startup
- Creates accumulators, drain cursors, and watermarks for all edges
- Runs a fixed polling loop

For hot-reload, it needs:
- `add_subgraph(data_sources, tasks, edges)` — insert new nodes/edges into the running graph
- New edges get fresh accumulators with zero drain cursors
- Existing edges are untouched — their state (watermarks, pending boundaries) is preserved
- `remove_subgraph(package_id)` — tear down edges/nodes from a removed package, clean up DB state

**5. State management for dynamic edges**

- New accumulators: initialized with empty state, cursor at 0
- New data sources: start polling from current position (no backfill by default)
- Removal: drain pending boundaries, delete accumulator state, remove drain cursors
- Crash recovery: existing restore sequence must handle a graph that differs from what was persisted (edges may have been added/removed since last run)

## Alternatives Considered

**Restart required (current S-0001 decision):**
- Simpler implementation — graph is immutable after startup
- Rejected because it contradicts the packaging model — uploading a package should make it active, not require operator intervention

**Full graph rebuild on package change:**
- Tear down the entire continuous scheduler and rebuild from scratch
- Simpler than incremental mutation but causes downtime for all continuous tasks
- Rejected because it penalizes existing workflows when a new package is added

**Incremental graph mutation (chosen):**
- Add/remove subgraphs without affecting existing edges
- Most complex but provides the best UX — upload a package, it starts working, nothing else is disrupted

## Implementation Plan

### Existing Tasks to Absorb

- **CLOACI-T-0214** (backlog) — Packaged Triggers. Same FFI/packaging/loading work applied to `Trigger` trait implementations. Should be reassigned to this initiative when decomposition begins.

### Phase 1: FFI and Packaging
- Extend `#[continuous_task]` to emit FFI metadata symbols
- Define the manifest format for continuous components in `.cloacina` packages
- Extend `PackageLoader` to extract continuous metadata

### Phase 2: Reconciler Integration
- Extend `RegistryReconciler` to detect continuous components in loaded packages
- Bridge between reconciler and `ContinuousScheduler` for dynamic registration

### Phase 3: Dynamic Graph Mutation
- Implement `ContinuousScheduler::add_subgraph()` and `remove_subgraph()`
- State initialization for new accumulators and drain cursors
- State cleanup for removed edges
- Update S-0001 to reflect the hot-reload decision

### Phase 4: Testing and Crash Recovery
- Integration tests: upload a continuous package to a running server, verify data flows
- Crash recovery with dynamic graph: restart with edges that were added after initial startup
- Soak test: continuous + regular workflows running together under load

### Phase 5: Documentation
- Update S-0001 specification
- Tutorial: packaging and deploying continuous workflows
- Reference: continuous task FFI symbol format
