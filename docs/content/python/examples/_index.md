---
title: "Python Examples"
description: "Worked examples for Cloaca Python workflows."
weight: 50
---

# Python Examples

Worked end-to-end examples for the Cloaca Python bindings.

## Available examples

- [Basic Workflow]({{< ref "basic-workflow" >}}) — a complete
  end-to-end example: define tasks with `@cloaca.task`, build a
  workflow, run it via `DefaultRunner.execute()`, and inspect the
  result.

## Looking for more?

The example library here is intentionally small. For broader
coverage, use the dedicated tracks:

- **[Python Tutorials]({{< ref "/python/tutorials" >}})** — the
  step-by-step learning path: first workflow, context handling,
  complex workflows, error handling, multi-tenancy, cron
  scheduling, event triggers, packaged workflows, computation
  graphs, accumulators.
- **[Python How-To Guides]({{< ref "/python/how-to-guides" >}})** —
  task-oriented recipes: testing workflows, performance
  optimization, packaging Python workflows, backend selection.
- **[Python API Reference]({{< ref "/python/api-reference" >}})** —
  the full surface for `cloaca`'s decorators, types, and runner
  configuration.
- **The repo's [`examples/tutorials/python/`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/python)
  directory** — runnable sources for every Python tutorial, kept
  in lockstep with the prose.

If you need an example that isn't covered by any of the above,
opening an issue with the use case is the most direct path to a
new tutorial or guide.

## Prerequisites for running examples

```bash
# Install Python bindings
pip install cloaca[sqlite]    # or cloaca[postgres]
```

```python
import cloaca

# SQLite for development
runner = cloaca.DefaultRunner("sqlite:///examples.db")

# PostgreSQL for production
runner = cloaca.DefaultRunner("postgresql://user:pass@localhost:5432/cloacina")
```

## Related Resources

- **[Tutorials]({{< ref "/python/tutorials/workflows/" >}})** — step-by-step learning.
- **[How-to Guides]({{< ref "/python/how-to-guides/" >}})** — problem-solving recipes.
- **[API Reference]({{< ref "/python/api-reference/" >}})** — complete API documentation.
- **[Repository examples](https://github.com/colliery-io/cloacina/tree/main/examples)** — source code.
