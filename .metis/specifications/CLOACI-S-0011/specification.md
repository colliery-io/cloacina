---
id: cloacina-primitive-nomenclature
level: specification
title: "Cloacina primitive nomenclature — trigger, reactor, accumulator, workflow, computation graph"
short_code: "CLOACI-S-0011"
created_at: 2026-04-22T12:48:03.804042+00:00
updated_at: 2026-04-22T13:28:38.906745+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/published"


exit_criteria_met: false
initiative_id: NULL
---

# Cloacina primitive nomenclature

## Overview

Cloacina has accumulated vocabulary overlap between concepts. In particular, the word *reactor* has been used to mean three different things — a node type inside a computation graph, the per-graph trigger implementation, and, colloquially, the entire reactive computation-graph subsystem. This document locks down the primitives, fixes the drift, and defines the rules that enforce the distinction at every surface (code, HTTP API, CLI, docs).

The five primitives below are the **only** concepts that should appear in user-facing language. Internal implementation details (schedulers, runtimes, supervisors, endpoint registries) are *not* primitives — they are the machinery that implements a primitive, and they should be named after the primitive they serve.

## Primitives

### Trigger

A **trigger** is an event source that starts execution. Examples: cron schedules, external event signals, HTTP push. A trigger fires a workflow or a computation graph traversal.

- User-facing noun: **trigger**.
- Associated surfaces: `@trigger` decorator, trigger schedule, `cloacinactl trigger`, server trigger endpoints.

### Reactor

A **reactor** is a specialized trigger that consumes accumulator boundary events and fires downstream execution when its configured firing criteria are met. Unlike generic triggers, reactors are bound to accumulators and have a distinct runtime shape (long-lived supervised task, health state machine, pause/resume).

Reactors are kept as a first-class noun despite being "a kind of trigger" because their implementation, operational model, and mental model are specialized enough that a unique word carries its weight.

- User-facing noun: **reactor**.
- Associated surfaces: `#[reactor]` / `ReactorDeclaration`, `Reactor` trait, per-reactor health, `/v1/ws/reactor/{name}`.
- **Topological fact (today)**: a computation graph contains exactly one reactor. This 1:1 relationship is the source of the historical "reactor = graph" shorthand; this spec rejects that shorthand.

### Accumulator

An **accumulator** is the reactor's stream-input adapter. It consumes an external stream and emits boundary events into the reactor. Accumulators are architecturally distinct from reactors (own traits, own runtimes, own supervisor hooks) for implementation reasons but are **semantically part of the reactor subsystem** — they do not exist independently of a reactor that consumes them.

- User-facing noun: **accumulator**.
- Associated surfaces: `#[stream_accumulator]`, `#[passthrough_accumulator]`, `#[polling_accumulator]`, `#[batch_accumulator]`, `Accumulator` trait, per-accumulator health, `/v1/health/accumulators`, `/v1/ws/accumulator/{name}`.

### Workflow

A **workflow** is a DAG of tasks where **the task is the quantum of scheduling and execution**. The orchestrator claims, runs, and completes one task at a time; state flows between tasks via context passed along DAG edges. Tasks are idempotent at their own granularity.

- User-facing noun: **workflow**.
- Associated surfaces: `Workflow`, `WorkflowBuilder`, `@task`, `cloacinactl workflow`, `/v1/workflows`.

### Computation graph

A **computation graph** (CG) is a DAG where **the graph traversal is the quantum of scheduling and execution**. Once triggered, all nodes in the traversal run as a single unit with in-memory channels between them; the graph is idempotent at the traversal granularity, not the node. A computation graph can appear as a node inside a workflow (see T-0500); when it does, the graph is **semantically subsumed by the workflow** — it becomes one task in the workflow's quantum, without its own reactor or standalone trigger surface. The fast-fire reactor-plus-graph model applies only to standalone graphs.

- User-facing noun: **computation graph** (short form: **graph**).
- Associated surfaces: `#[computation_graph]`, `ComputationGraphDeclaration`, `ComputationGraphScheduler`, `cloacinactl graph`, `/v1/health/graphs`.

## Relationships

```
┌─────────┐   fires    ┌────────────┐
│ Trigger ├───────────▶│  Workflow  │
└─────────┘            │    or      │
     │                 │  Graph     │
     │                 └────────────┘
     │ (specialized)
     ▼
┌─────────┐   consumes ┌──────────────┐
│ Reactor │◀───────────│ Accumulator  │
└─────────┘            └──────────────┘
    │ fires (today: exactly one per graph)
    ▼
┌──────────────────────┐
│ Computation graph    │
│ traversal            │
└──────────────────────┘
```

- Triggers fire workflows or CG traversals. Reactors are the specialized trigger type that fires CG traversals.
- Accumulators are reactor inputs only; they do not feed generic triggers.
- A workflow task may invoke a computation graph as a sub-unit (T-0500). An embedded graph is subsumed by the workflow's quantum: it runs as one task, with no reactor, no standalone trigger surface, and no separate fast-fire path.
- Accumulators as workflow triggers (T-0499) is a future generalization; when it lands, the reactor may be extended to fire workflows as well as CG traversals. The noun stays; the implementation can move. Out of scope for this spec.

## Naming rules (enforced)

### R1 — "Reactor" means the noun, never the subsystem
- ✅ *"The reactor fires when both accumulators have produced a boundary event."*
- ❌ *"The reactor runs when the package is loaded."* (should say "the computation graph" or "the graph's reactor")
- ❌ *"reactive scheduler"*, *"reactive computation graph"*, *"reactive subsystem"* — these conflate reactor with the CG execution engine and are banned.

### R2 — "Computation graph" (or "graph") is the quantum of execution
The graph is the unit of scheduling and execution. Its scheduler, runtime, registry, and health endpoints all bear the graph name, not the reactor name.

### R3 — `reactor` as a synonym for `computation graph` is drift
Anywhere code or docs use `reactor` to mean *"a loaded computation graph"* is drift to be corrected by this spec's rollout task (T-0528).

### R4 — Internal types are named after the primitive they serve
- `ComputationGraphScheduler` — not `ReactiveScheduler`.
- `graph_scheduler` — not `reactive_scheduler`.
- `health_graphs.rs` — not `health_reactive.rs`.

### R5 — Public surface names match the primitive exactly
| Surface | Before | After |
|---|---|---|
| Rust type | `ReactiveScheduler` | `ComputationGraphScheduler` |
| Rust fields / vars | `reactive_scheduler` | `graph_scheduler` |
| Module | `src/routes/health_reactive.rs` | `src/routes/health_graphs.rs` |
| HTTP | `GET /v1/health/reactors` | `GET /v1/health/graphs` |
| HTTP | `GET /v1/health/reactors/{name}` | `GET /v1/health/graphs/{name}` |
| HTTP response body | `{"reactors": [...]}` | `{"graphs": [...]}` |
| Per-graph field | `reactor_paused` | `paused` (scoped to the graph; comment ties it to the graph's reactor) |
| CLI | `cloacinactl reactor <verb>` | `cloacinactl graph <verb>` |
| CLI module | `cloacinactl/src/nouns/reactor/` | `cloacinactl/src/nouns/graph/` |
| Spec | `CLOACI-S-0008 Horizontal Scaling for Reactive Computation Graphs` | `… for Computation Graphs` |
| Docs | `docs/content/computation-graphs/explanation/reactive-scheduling.md` | `… /computation-graph-scheduling.md` |

### R6 — Untouched (already correct)
- `#[computation_graph]`, `#[reactor]`, `#[accumulator]` macros.
- `Reactor` trait, `ReactorDeclaration`, `Accumulator` trait, `AccumulatorRuntime`.
- `/v1/health/accumulators`, `/v1/ws/accumulator/{name}`, `/v1/ws/reactor/{name}` (node-scoped endpoints — correctly named).
- `CLOACI-S-0005 Reactor` spec title — the spec is about the reactor noun.
- Crate names (`cloacina-computation-graph`, `cloacina-workflow`).

## Endpoint semantics

The `/v1/health/graphs` endpoint (post-rename) reports **currently running graph instances** with their operational state. It is not a catalog of registered-but-not-running graphs; that is a registry concern, accessible via the package listing today and potentially a dedicated registry endpoint later. If/when that separation becomes meaningful, a distinct non-`/health/` URL will host it — this spec does not pre-fork.

## Out of scope

- The future generalization of reactor triggers to fire workflows (T-0499). When that lands, a follow-up spec amendment will define where reactor-as-trigger lives in the public surface.
- Computation-graph-as-workflow-node (T-0500) is a capability addition; it does not alter nomenclature.
- Multi-reactor-per-graph topologies. Today a graph has exactly one reactor; if that ever changes, this spec's wording about "the graph's reactor" becomes wrong and must be revised.

## Rollout

Tracked in **T-0528** (single PR). Scope:

1. Rust rename per R5.
2. HTTP rename per R5 (straight rename — `/v1` is young enough to absorb the breaking change).
3. CLI rename per R5 (straight rename).
4. Docs pass: rewrite `docs/content/computation-graphs/**` to respect R1–R3. Rename `reactive-scheduling.md`.
5. Metis: rename `CLOACI-S-0008` title.
6. This spec transitions to `published`.

Archived task docs and completed initiatives are **not** rewritten — they are historical records.

## Changelog

*To be filled after publication.*
