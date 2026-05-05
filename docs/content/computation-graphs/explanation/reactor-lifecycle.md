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

## Bundled-form vs split-form

A reactor can be declared two ways:

**Bundled form** — the original model, and still the simplest path.
The `#[computation_graph(react = ..., graph = ...)]` macro on a single
module produces both a synthesized reactor and the graph that
subscribes to it. There's a 1:1 reactor-to-graph mapping; loading
the package creates one reactor with one subscriber.

**Split form** — the decoupled model. A `#[reactor(...)]` declaration
defines a reactor as a unit struct with named accumulators and a
reaction mode. One or more `#[computation_graph(trigger =
reactor(MyReactor), ...)]` declarations bind separate graphs to that
reactor. The reactor is loaded once, and N subscribers can bind to
it across the same or different packages.

Cross-package binding (split form across packages) is a real use
case: a "publishing" package owns a reactor exposed to other tenants;
"subscriber" packages bind their own CGs to that reactor without
having to redeclare the upstream side.

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
   - Register the reactor under its name + any back-compat aliases
     (e.g., the bundled-form CG name) in the endpoint registry, so
     `cloacinactl reactor force-fire <name>` resolves.
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

## The bundled-form back-compat path

For bundled-form CG packages, the reactor is implicitly tied to the
graph: there's one subscriber and it's the package's own graph. To
keep the unload story simple for these packages, the bundled `unload_graph()` call also tears down the reactor when the graph was the
last subscriber. Unload then runs through step 3 again for the
explicitly-declared reactors and finds nothing to do — the
scheduler-side teardown returns "reactor 'foo' not loaded", which
the reconciler treats as a clean no-op (and still fires the
runtime-side constructor cleanup).

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
