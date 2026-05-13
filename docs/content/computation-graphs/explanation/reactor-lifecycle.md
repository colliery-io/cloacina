---
title: "Reactor Lifecycle"
description: "How reactors are loaded and unloaded — the dual-layer teardown, the bound-subscriber guard, and why both arms exist."
weight: 25
---

# Reactor Lifecycle

A reactor is the runtime orchestrator for a computation graph: it
owns the [InputCache]({{< ref "/glossary#inputcache" >}}), evaluates
reaction criteria after each accumulator update, and fires the
compiled graph function when criteria are met. This document covers
the *lifecycle* of a reactor — how it gets created, how it gets
torn down, and the invariants that keep both halves safe.

## Declaration model

As of CLOACI-I-0101 a reactor is its own top-level primitive. You
declare one with `#[reactor(name = "...", accumulators = [...],
criteria = ...)]` on a unit struct; this names the reactor, lists
its accumulators, and fixes its reaction mode. One or more
`#[computation_graph(trigger = reactor("name"), graph = ...)]`
declarations then bind their graphs to that reactor by string name.
The reactor is loaded once, and N subscribers can bind to it across
the same or different packages.

The previously bundled form (`#[computation_graph(react = ...,
graph = ...)]` synthesizing a reactor inside the same macro) has
been removed; there is no longer a "synthesized" reactor. Every
reactor is explicit.

Cross-package binding is a real use case: a "publishing" package
owns a reactor exposed to other tenants; "subscriber" packages bind
their own CGs to that reactor without having to redeclare the
upstream side.

## The two registries

A loaded reactor is recorded in two places:

1. **The scheduler-side runtime state** — `ComputationGraphScheduler`
   holds a `RunningGraph` per loaded reactor. It owns the spawned
   reactor task, the spawned accumulator tasks, the manual-command
   channel, the subscriber map, and the endpoint-registry keys
   under which the reactor is registered for `cloacinactl
   reactor force-fire` style operator commands.

2. **The host runtime registry** — `Runtime` holds a *constructor*
   per reactor name, in `runtime.reactor_names()`. Constructors are
   how the runtime mints fresh `ReactorRegistration` values when
   needed; they're populated either by `seed_from_inventory()` (for
   embedded reactors) or by the reconciler projecting FFI method 4
   (`get_reactor_metadata`) into per-name constructors (for packaged
   reactors).

Both registries must agree at all times. The lifecycle's job is to
make sure they do.

## Load

The reconciler runs step 3 of the [pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}}) for each package:

1. Read FFI method 4 (`get_reactor_metadata`) — get the package's
   declared reactors.
2. For each declaration, register a constructor in `Runtime` keyed
   by the reactor name.
3. Call `ComputationGraphScheduler::load_reactor()`:
   - Spawn the reactor task with a manual-command channel and a
     boundary-receiver channel.
   - Spawn accumulator tasks and connect their output sockets to
     the reactor's boundary-receiver.
   - Register the reactor under its declared name in the endpoint
     registry, so `cloacinactl reactor force-fire <name>` resolves.
4. Record the reactor name in `PackageState::reactor_names` so
   unload knows what to tear down.

The reconciler then runs step 4 (trigger-less CGs) and step 5
(reactor-bound CGs). Step 5's `load_graph()` looks up the reactor by
name in the scheduler — the reactor must already be present, which is
why it loaded in step 3.

## Unload

Unload runs step 3 in reverse:

1. For each reactor name recorded in `PackageState::reactor_names`,
   call `ComputationGraphScheduler::unload_reactor()`:
   - **Bound-subscriber guard.** If the reactor's subscriber map
     contains any cross-package CG, return an error:
     `"reactor 'foo' has 1 bound subscriber(s): ['bar']; unbind
     them first"`. The unload aborts cleanly; the reactor stays
     loaded.
   - Otherwise, send the shutdown signal, await the reactor task,
     await the accumulator tasks (with a 5s timeout each), and
     deregister all endpoint-registry keys.
2. Drop the reactor constructor from the `Runtime` registry via
   `Runtime::unregister_reactor(name)`. **This second arm is what
   makes hot-reload safe** — without it, a package reload leaves the
   old reactor's constructor permanently registered in `Runtime`,
   and re-loading the same package accumulates dead entries every
   cycle.

The scheduler-side teardown (step 1 above) and the runtime-side
teardown (step 2) are *both* required. Earlier versions of Cloacina
only had step 1; the second arm was added after operators observed
reactor-name leaks across hot-reload cycles in long-running daemons
(every reload accumulated a stale constructor entry in `Runtime`).

## Cross-package unload ordering

If package A owns reactor R, and package B has a CG subscribed to R,
unloading A is rejected with the bound-subscriber error. Operators
must:

1. Unload package B first (which unbinds its CG from R).
2. Unload package A (which now succeeds — R has no subscribers).

The reconciler does not implement automatic dependent-package
detection; it surfaces the rejection error with the subscriber list
so operators can decide. This is intentional: silently cascading
unloads across packages would mask configuration mistakes.

## Restart vs unload

The supervisor inside `ComputationGraphScheduler` restarts crashed
reactors on a 5-second cadence. Restart is *not* unload + load; it
preserves the existing `RunningGraph` (subscribers, endpoint-registry
keys, accumulator handles) and replaces the failing internals.
Restarts are recorded in failure metrics and respect the
`MAX_RECOVERY_ATTEMPTS` cap (5 consecutive failures before a
component is permanently abandoned).

## Related

- [Reconciler Pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}}) — step 3 detail.
- [Computation Graph Scheduling]({{< ref "/computation-graphs/explanation/computation-graph-scheduling" >}}) — runtime model.
- [FFI Vtable Reference]({{< ref "/platform/reference/ffi-vtable" >}}) — methods 4 (`get_reactor_metadata`) and the rest.
