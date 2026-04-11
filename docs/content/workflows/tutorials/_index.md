---
title: "Tutorials"
description: "Step-by-step guides for learning Cloacina's workflow system"
weight: 10
---

# Workflow Tutorials

Learn Cloacina's workflow orchestration system through progressive, hands-on tutorials.

## Library (Embedded)

Start here. These tutorials teach you to define and run workflows directly in your Rust application.

1. [Your First Workflow]({{< ref "/workflows/tutorials/library/01-first-workflow" >}}) — Define a task, build a workflow, execute it
2. [Context Handling]({{< ref "/workflows/tutorials/library/02-context-handling" >}}) — Pass data between tasks using Context
3. [Complex Workflows]({{< ref "/workflows/tutorials/library/03-complex-workflows" >}}) — Parallel tasks, fan-out/fan-in, dependencies
4. [Error Handling]({{< ref "/workflows/tutorials/library/04-error-handling" >}}) — Retries, fallbacks, callbacks, and conditional execution

## Service (Server Mode)

Once comfortable with the library, move to service mode for production features.

5. [Cron Scheduling]({{< ref "/workflows/tutorials/service/05-cron-scheduling" >}}) — Time-based workflow execution
6. [Multi-tenancy]({{< ref "/workflows/tutorials/service/06-multi-tenancy" >}}) — Isolated tenant environments
7. [Packaged Workflows]({{< ref "/workflows/tutorials/service/07-packaged-workflows" >}}) — Build and distribute .cloacina packages
8. [Workflow Registry]({{< ref "/workflows/tutorials/service/08-workflow-registry" >}}) — Register and manage packages
9. [Event Triggers]({{< ref "/workflows/tutorials/service/09-event-triggers" >}}) — Condition-based workflow firing
10. [Task Deferral]({{< ref "/workflows/tutorials/service/10-task-deferral" >}}) — Release concurrency slots while waiting
