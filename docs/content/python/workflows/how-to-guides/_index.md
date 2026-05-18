---
title: "How-to Guides"
description: "Task-oriented recipes for Python workflow development and operations."
weight: 20
---

# Python Workflow How-to Guides

Practical recipes — each solves one specific problem.

## Development

- [Backend Selection]({{< ref "backend-selection" >}}) — Choosing between `cloaca[sqlite]` and `cloaca[postgres]`.
- [Testing Workflows]({{< ref "testing-workflows" >}}) — Unit and integration testing strategies.
- [Performance Optimization]({{< ref "performance-optimization" >}}) — Throughput / latency tuning knobs.

## Packaging + Deployment

- [Packaging Python Workflows]({{< ref "packaging-python-workflows" >}}) — Build a `.cloacina` archive from a Python module.

## Operations

<!-- TODO(DOC-G Phase 5/6): land Python-side analogs of decommission-a-tenant.md once the underlying `runner.decommission_tenant` Python surface is verified against `crates/cloacina-python/src/bindings/admin.rs`. The Rust-side how-to is at /platform/how-to-guides/decommission-a-tenant. -->

> Operational recipes (multi-tenant decommission, signed-package enforcement, CLI profiles) currently live on the [Rust how-to-guides side]({{< ref "/platform/how-to-guides" >}}) — the Python runner inherits the same operational surface.
