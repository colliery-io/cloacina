---
title: "Engine & Primitives"
description: "The core objects Cloacina is built from — described once, shared by both the embedded and service paths."
sidebar:
  order: 30
---

The concepts here are the heart of Cloacina, independent of how you run it. They
apply whether you **[embed the library](../embed/)** or
**[run the service](../service/)** — each object is described once,
with both its Rust and Python interfaces.

**Who this is for:** anyone learning the engine primitives — workflows,
computation graphs, scheduling, and packaging — regardless of language. This
material is language-agnostic.

**Prerequisites:** basic familiarity with the concepts from
**[Start Here](../start/)**.

The core primitives:

- **Workflow** — a durable, database-backed DAG; the task is the unit of scheduling.
- **Task** — a unit of work in a workflow.
- **Context** — typed data passed between tasks, persisted and recovered.
- **Computation Graph** — an in-process, event-driven dataflow; the whole traversal is the unit.
- **Node** — a vertex in a computation graph.
- **Reactor** — binds accumulators to a graph and fires it when criteria are met.
- **Accumulator** — turns a source or stream into boundary events (passthrough, stream, polling, batch, state).
- **Boundary event** — the typed event an accumulator emits and a reactor reacts to.
- **Trigger** — fires a workflow (poll or cron).
- **Cron schedule** — time-based workflow scheduling.
- **Package (`.cloacina`)** — the distributable unit.
- **Runner** — the host that executes workflows against a database.

- [Workflows](../explanation/primitives/workflows/)
  - [Workflow](../explanation/primitives/workflows/workflow/)
  - [Task](../explanation/primitives/workflows/task/)
  - [Context](../explanation/primitives/workflows/context/)
  - [Runner](../explanation/primitives/workflows/runner/)
  - [How-to Guides](../how-to/workflows/)
    - [Build Workflows with WorkflowBuilder](../how-to/workflows/build-workflows-with-the-builder/)
    - [Use Secrets in a Workflow](../how-to/workflows/use-secrets/)
- [Computation Graphs](../explanation/primitives/computation-graphs/)
  - [Computation Graph](../explanation/primitives/computation-graphs/computation-graph/)
  - [Node](../explanation/primitives/computation-graphs/node/)
  - [Reactor](../explanation/primitives/computation-graphs/reactor/)
  - [Accumulator](../explanation/primitives/computation-graphs/accumulator/)
  - [Boundary event](../explanation/primitives/computation-graphs/boundary/)
  - [How-to Guides](../how-to/computation-graphs/)
    - [Package a Python Computation Graph](../how-to/computation-graphs/package-a-python-computation-graph/)
    - [Choosing and Using Accumulator Types](../how-to/computation-graphs/accumulator-types/)
    - [Filter Reactor Subscriptions (Python)](../how-to/computation-graphs/filter-reactor-subscriptions/)
    - [Monitoring Computation Graph Health](../how-to/computation-graphs/computation-graph-health/)
    - [Manually Driving Graph Surfaces](../how-to/computation-graphs/drive-graph-surfaces-manually/)
    - [Computation Graph in a Workflow Task](../how-to/computation-graphs/computation-graph-in-workflow/)
    - [Triggering Workflows from Reactor Firings](../how-to/computation-graphs/reactor-triggered-workflows/)
    - [Using when_all Reaction Criteria](../how-to/computation-graphs/when-all-criteria/)
    - [Filter reactor firings with CEL](../how-to/computation-graphs/filter-reactor-firings-with-cel/)
    - [Using Sequential Input Strategy](../how-to/computation-graphs/sequential-strategy/)
- [Scheduling & Triggers](../explanation/primitives/scheduling/)
  - [Workflow Instances](../explanation/primitives/scheduling/workflow-instances/)
  - [Trigger](../explanation/primitives/scheduling/trigger/)
  - [Cron schedule](../explanation/primitives/scheduling/cron-schedule/)
- [Packaging](../explanation/primitives/packaging/)
  - [Package](../explanation/primitives/packaging/package/)
- [Constructors & Providers](../explanation/primitives/constructors/)
  - [Author a Constructor Provider](../how-to/constructors/author-a-provider/)
  - [Consume a Constructor Provider](../how-to/constructors/consume-a-provider/)
  - [Capability Grants](../explanation/primitives/constructors/grants/)
  - [Seed Providers](../explanation/primitives/constructors/seed-providers/)
- [Explanation](../explanation/engine/)
  - [Architecture Overview](../explanation/engine/architecture-overview/)
  - [Workflow Versioning](../explanation/engine/workflow-versioning/)
  - [Context System](../explanation/engine/context-management/)
  - [Trigger Rules](../explanation/engine/trigger-rules/)
  - [Macro System](../explanation/engine/macro-system/)
  - [Computation Graph Architecture](../explanation/engine/architecture/)
  - [Python CG Decorator Surface](../explanation/engine/python-cg-decorator-surface/)
  - [Accumulator Design](../explanation/engine/accumulator-design/)
  - [Inventory and Runtime Seeding](../explanation/engine/inventory-and-runtime-seeding/)
  - [Package Format](../explanation/engine/package-format/)
  - [FFI System](../explanation/engine/ffi-system/)
  - [Packaged Workflow Architecture](../explanation/engine/packaged-workflow-architecture/)
  - [Guaranteed Cron Scheduling](../explanation/engine/guaranteed-execution-architecture/)
  - [Reactor Lifecycle](../explanation/engine/reactor-lifecycle/)
  - [Trigger-less Computation Graphs](../explanation/engine/trigger-less-graphs/)
  - [Packaging & FFI](../explanation/engine/packaging/)
  - [Task Execution](../explanation/engine/task-execution-sequence/)
  - [Task Deferral](../explanation/engine/task-deferral/)
  - [Dispatcher Architecture](../explanation/engine/dispatcher-architecture/)
  - [Graph Scheduling](../explanation/engine/computation-graph-scheduling/)
  - [Performance Characteristics](../explanation/engine/performance/)
  - [Subscription fan-out](../explanation/engine/subscription-fan-out/)
  - [Cron Scheduling Architecture](../explanation/engine/cron-scheduling/)
