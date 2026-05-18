---
title: "Tutorials"
description: "Learning-oriented walkthroughs for the Python workflow surface."
weight: 10
---

# Python Workflow Tutorials

Step-by-step walkthroughs. Each tutorial is a working program you can run end-to-end via `angreal demos tutorials python NN`.

## Sequence

1. [00 — Basic Workflow]({{< ref "00-basic-workflow" >}}) — The smallest possible workflow. A single task, default runner, in-memory SQLite.
2. [01 — Your First Python Workflow]({{< ref "01-first-python-workflow" >}}) — Multi-step workflow with dependencies.
3. [02 — Context Handling]({{< ref "02-context-handling" >}}) — Passing typed state between tasks via the `Context`.
4. [03 — Complex Workflows]({{< ref "03-complex-workflows" >}}) — Parallel tasks, fan-out/fan-in.
5. [04 — Error Handling]({{< ref "04-error-handling" >}}) — Retries, dead-lettering, recoverable vs unrecoverable.
6. [05 — Cron Scheduling]({{< ref "05-cron-scheduling" >}}) — `@cloaca.cron_trigger(...)` and `CatchupPolicy`.
7. [06 — Multi-tenancy]({{< ref "06-multi-tenancy" >}}) — Schema-based tenant isolation, fail-closed `search_path`.
8. [07 — Event Triggers]({{< ref "07-event-triggers" >}}) — `@cloaca.trigger` for external/event-driven workflows.
9. [08 — Packaged Triggers]({{< ref "08-packaged-triggers" >}}) — Building a `.cloacina` archive for server-mode deployment.

> Computation graph tutorials (09–11) have moved to [Python · Computation Graphs · Tutorials]({{< ref "/python/computation-graphs/tutorials" >}}).
