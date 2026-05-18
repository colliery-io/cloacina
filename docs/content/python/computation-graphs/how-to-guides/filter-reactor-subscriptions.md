---
title: "Filter Reactor Subscriptions (Python)"
description: "Python equivalent of the CEL-filter recipe — attach a predicate to a workflow's reactor subscription so only matching firings dispatch (CLOACI-T-0602)."
weight: 20
---

# Filter Reactor Subscriptions (Python)

<!-- TODO(DOC-G Phase 5): full content deferred until the Python subscribe-with-predicate surface is verified. Sources to check: -->
<!--   - crates/cloacina-python/src/bindings/runner.rs (look for subscribe_workflow_to_reactor) -->
<!--   - examples/features/computation-graphs/python-packaged-graph/ (Python example) -->
<!--   - The Rust counterpart at /computation-graphs/how-to-guides/filter-reactor-firings-with-cel for reference -->

This page is the Python analog of [Filter reactor firings with CEL]({{< ref "/computation-graphs/how-to-guides/filter-reactor-firings-with-cel" >}}). The underlying mechanism (a CEL predicate on the `reactor_subscriptions` row) is the same — what's not yet verified is the exact Python API surface (`runner.subscribe_workflow_to_reactor(...)` signature).

In the meantime, use the [Rust how-to]({{< ref "/computation-graphs/how-to-guides/filter-reactor-firings-with-cel" >}}) as the conceptual reference; the predicate language, the fail-closed evaluation semantics, and the idempotency-key recipe are language-agnostic.

## See also

- [Rust · Filter Reactor Firings with CEL]({{< ref "/computation-graphs/how-to-guides/filter-reactor-firings-with-cel" >}}).
- [Subscription Fan-out]({{< ref "/computation-graphs/explanation/subscription-fan-out" >}}).
- **CLOACI-T-0602** — CEL predicate filtering on subscriptions.
