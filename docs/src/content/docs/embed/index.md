---
title: "Embed the Library"
description: "Integrate the Cloacina engine into your own Rust or Python application as a library."
sidebar:
  order: 10
---

Run orchestration **inside your application**. You add a dependency — `cloacina`
(Rust) or `cloaca` (Python) — and the engine runs in your process, backed by a
database you already operate. No separate service to stand up.

This is a first-class, production-legitimate way to run Cloacina. If instead you
want to operate Cloacina as a shared, multi-tenant service, see
**[Run the Service](../service/)**. For what the two share — the core
objects you author against — see **[Engine & Primitives](../engine/)**.

**Who this is for:** application developers embedding the engine as a library
inside their own Rust or Python code.

**Prerequisites:** an existing Rust or Python project to add the dependency to,
and a database you already operate (SQLite is fine to start).

- [Quick Start](../tutorials/embed/quick-start/)
- [Tutorials](../tutorials/embed/)
  - [01 — Your First Workflow](../tutorials/embed/01-first-workflow/)
  - [02 — Passing data with Context](../tutorials/embed/02-context/)
  - [03 — Dependencies & parallelism](../tutorials/embed/03-dependencies/)
  - [04 — Error handling & retries](../tutorials/embed/04-error-handling/)
  - [05 — Cron Scheduling](../tutorials/embed/05-cron-scheduling/)
  - [06 — Multi-Tenancy](../tutorials/embed/06-multi-tenancy/)
  - [07 — Event Triggers](../tutorials/embed/07-event-triggers/)
  - [08 — Task Deferral](../tutorials/embed/08-task-deferral/)
  - [09 — Working with the Workflow Registry](../tutorials/embed/09-workflow-registry/)
  - [10 — Your First Computation Graph](../tutorials/embed/10-computation-graph/)
  - [11 — Accumulators](../tutorials/embed/11-accumulators/)
  - [12 — Full Multi-Source Pipeline](../tutorials/embed/12-full-pipeline/)
  - [13 — Routing and Enum Dispatch](../tutorials/embed/13-routing/)
  - [14 — Packaged Triggers](../tutorials/embed/14-packaged-triggers/)
- [How-to Guides](../how-to/embed/)
  - [Testing Workflows](../how-to/embed/testing-workflows/)
  - [Running embedded in production](../how-to/embed/running-embedded-in-production/)
  - [Subscribe a workflow to a reactor](../how-to/embed/subscribe-workflow-to-reactor/)
  - [Declare and validate workflow inputs](../how-to/embed/declare-workflow-inputs/)
  - [Invoke a computation graph from a workflow task](../how-to/embed/invoke-computation-graph-from-workflow/)
  - [Configure a Database Connection URL](../how-to/embed/backend-selection/)
  - [Conditional Retries](../how-to/embed/conditional-retries/)
  - [Observe Execution State](../how-to/embed/observe-execution-state/)
  - [Performance Optimization](../how-to/embed/performance-optimization/)
  - [Packaging Python Workflows](../how-to/embed/packaging-python-workflows/)
  - [Running the Daemon](../how-to/embed/running-the-daemon/)
  - [Monitoring Executions](../how-to/embed/monitoring-executions/)
  - [Variable Registry (var/var_or)](../how-to/embed/variable-registry/)
  - [Cleaning Up Events](../how-to/embed/cleaning-up-events/)
- [Explanation](../explanation/embed/)
  - [Python Runtime Architecture](../explanation/embed/python-runtime-architecture/)
  - [Workflow Performance and Design Trade-offs](../explanation/embed/performance/)
