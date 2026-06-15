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

Operational recipes (multi-tenant decommission, signed-package enforcement, CLI
profiles) live on the [platform how-to-guides]({{< ref "/platform/how-to-guides" >}})
— the Python runner inherits the same operational surface, so they apply
regardless of authoring language.
