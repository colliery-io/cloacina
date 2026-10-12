---
title: "Workflows"
description: "The durable-DAG cluster: Workflow, Task, Context, and the Runner that executes them."
sidebar:
  order: 10
---

The durable-DAG cluster — the primitives for reliable, database-backed task
pipelines. A **Workflow** is a DAG of **Tasks**; a **Context** carries typed data
between them; a **Runner** executes the workflow against a database with retries
and recovery.

Each page describes the object once, with both its **Rust** and **Python**
interfaces.

- [Workflow](workflow/)
- [Task](task/)
- [Context](context/)
- [Runner](runner/)
- [How-to Guides](../../../how-to/workflows/)
  - [Build Workflows with WorkflowBuilder](../../../how-to/workflows/build-workflows-with-the-builder/)
  - [Use Secrets in a Workflow](../../../how-to/workflows/use-secrets/)
