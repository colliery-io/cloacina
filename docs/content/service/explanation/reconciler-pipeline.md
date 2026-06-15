---
title: "Reconciler Pipeline"
description: "The six-step ordered load and unload pipeline the registry reconciler runs per package, and why the order is fixed."
weight: 21
---

# Reconciler Pipeline

The `RegistryReconciler` is the host-side component that turns a
`.cloacina` package on disk into running constructors in the host
`Runtime` and live entries in the computation-graph scheduler. It is
also the component that *unwinds* a package cleanly when it's
unloaded. Both directions run the same six steps, in opposite order.

## The six steps

Per package, on **load**:

1. **Cron triggers** — read via FFI method 5
   (`get_trigger_metadata`); cron-shaped entries
   (`cron_expression: Some(...)`) are routed to the cron scheduler
   through the configured `CronWorkflowRegistrar`. Each cron schedule
   gets a database row keyed by `cron_schedule_id`; the scheduler
   begins firing the workflow at the declared cadence.

2. **Custom-poll triggers** — non-cron entries from the same FFI
   call get a host-side `FfiTriggerImpl` adapter, which holds the
   trigger's metadata (poll interval, allow-concurrent flag) and
   proxies `Trigger::poll()` back into the plugin via FFI method 6.
   Custom triggers register against the host's polling scheduler.

3. **Reactors** — read via FFI method 4 (`get_reactor_metadata`).
   For each declaration, the reconciler builds a
   `ReactorRegistration` and calls
   `ComputationGraphScheduler::load_reactor()`, which spawns the
   reactor task, wires up its accumulators, and registers the reactor
   under its declared endpoint keys.

4. **Trigger-less computation graphs** — read via FFI method 7
   (`get_triggerless_graph_metadata`). For each, the reconciler builds
   a `TriggerlessGraphRegistration` whose `graph_fn` invokes FFI
   method 8 (`invoke_triggerless_graph`) on every call, and registers
   it in the host `Runtime`. Workflow tasks declared with
   `#[task(invokes = "graph_name")]` consume these.

5. **Reactor-bound computation graphs** — read via FFI method 2
   (`get_graph_metadata`). For each bundled-form CG (one per
   cdylib), the reconciler calls
   `ComputationGraphScheduler::load_graph()`, which binds the graph
   to its declared upstream reactor (which **must already be loaded**
   from step 3 — that's why this step comes after).

6. **Workflows** — read via FFI method 0 (`get_task_metadata`).
   The reconciler registers `DynamicLibraryTask` constructors in the
   host `Runtime` per task and registers the workflow itself. Trigger
   subscription validation runs here: any `#[workflow(triggers =
   ["foo"])]` subscription must resolve against a trigger registered
   in steps 1 or 2.

**Unload runs steps 6 → 1**, reversing each side effect:

- Workflows + tasks unregistered.
- Reactor-bound CGs unbound from their reactors.
- Trigger-less CGs unregistered.
- Reactors torn down (scheduler-side `unload_reactor()` plus the
  runtime-side constructor cleanup; see [Reactor
  Lifecycle]({{< ref "/computation-graphs/explanation/reactor-lifecycle" >}})).
- Custom triggers unregistered from the polling scheduler.
- Cron schedules deleted from the database.

## Why this ordering

The forward order encodes the dependency graph between subsystems:

- A workflow may declare `triggers = ["my_trigger"]`. That trigger
  must already be registered by the time the workflow loads, otherwise
  validation fails. **Triggers before workflows.**
- A reactor-bound CG declares
  `trigger = reactor(MyReactor)`. The reactor must already be
  loaded in the scheduler, otherwise `load_graph()` errors with
  "reactor 'MyReactor' not loaded." **Reactors before CGs.**
- Trigger-less CGs are independent of reactors, so they can load any
  time — but a workflow task with `#[task(invokes = "my_graph")]`
  expects the graph to be in the runtime registry. **Trigger-less
  CGs before workflows.**

The unload reversal is the same dependency graph followed backwards:
you cannot delete a reactor while a CG still subscribes to it, so
CG-from-reactor unbinding (step 5 reversed) runs before reactor
teardown (step 3 reversed).

## Failure isolation

Each step is fail-fast within the package. If step 3 fails (e.g., a
reactor with an invalid accumulator declaration), the reconciler
unwinds steps 1–2 (cron + custom triggers) before returning the
error, leaving the host in a clean state. Steps 1–6 are not atomic —
there's no transaction wrapping the load — but the unload symmetry
gives you the equivalent: a partial load is followed by a partial
unload that leaves no residue.

Cross-package failures are also bounded. Loading package B never
mutates state owned by package A. Unloading package A while B has a
CG bound to A's reactor is rejected with a clear error message
(`"reactor 'foo' has 1 bound subscriber(s): ['bar']; unbind them
first"`); operators must unload B first.

## Inputs the reconciler tracks

For each loaded package, the reconciler keeps a `PackageState`
struct that records every side effect, so unload knows exactly what
to undo:

- `task_namespaces` — for runtime task unregistration.
- `workflow_name` — for workflow unregistration.
- `trigger_names` — for custom-poll trigger unregistration.
- `cron_schedule_ids` — for cron schedule deletion.
- `reactor_names` — for scheduler-side and runtime-side reactor
  teardown.
- `graph_name` — for `unload_graph()`.
- `triggerless_graph_names` — for trigger-less CG unregistration.

Without this state tracking, unload would have to introspect the
plugin's metadata again — which is fine in the happy path but
catastrophic if the plugin file has been deleted or replaced before
unload runs.

## Calling the reconciler

In the local daemon (`cloacinactl daemon start`), the reconciler is
started with a filesystem-backed registry that watches the daemon's
package directory. In the HTTP server (`cloacinactl server start`),
the reconciler is started with a database-backed registry that polls
for new package rows.

Both paths run the same six-step pipeline. The difference is only the
source of "load this package now" signals — filesystem-watcher events
vs. database-polled rows.

## Related

- [FFI Vtable Reference]({{< ref "/platform/reference/ffi-vtable" >}}) — the methods the reconciler calls in each step.
- [Reactor Lifecycle]({{< ref "/computation-graphs/explanation/reactor-lifecycle" >}}) — step 3 detail.
- [Inventory and Runtime Seeding]({{< ref "/engine/explanation/inventory-and-runtime-seeding" >}}) — how step 6 differs between embedded and packaged paths.
