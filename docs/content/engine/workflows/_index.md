---
title: "Workflows"
description: "The durable-DAG cluster: Workflow, Task, Context, and the Runner that executes them."
weight: 10
aliases:
  - "/workflows/"

---

# Workflows

The durable-DAG cluster — the primitives for reliable, database-backed task
pipelines. A **Workflow** is a DAG of **Tasks**; a **Context** carries typed data
between them; a **Runner** executes the workflow against a database with retries
and recovery.

Each page describes the object once, with both its **Rust** and **Python**
interfaces.

{{< toc-tree >}}
