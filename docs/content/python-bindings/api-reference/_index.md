---
title: "API Reference"
description: "Complete API reference for Cloaca"
weight: 30
reviewer: "automation"
review_date: "2025-01-07"
---

# Cloaca API Reference

Complete reference documentation for all Cloaca classes, methods, and functions.

## Core Classes

{{< toc-tree >}}

## Quick Reference

### Import and Basic Usage
```python
import cloaca

# Define task
@cloaca.task(id="my_task")
def my_task(context):
    return context

# Build and register workflow
def create_workflow():
    builder = cloaca.WorkflowBuilder("my_workflow")
    builder.add_task("my_task")
    return builder.build()

cloaca.register_workflow_constructor("my_workflow", create_workflow)

# Execute workflow
runner = cloaca.DefaultRunner("sqlite:///app.db")
context = cloaca.Context({"key": "value"})
result = runner.execute("my_workflow", context)
runner.shutdown()
```

## Module Functions

- **`cloaca.task()`** - Decorator for defining workflow tasks
- **`cloaca.register_workflow_constructor()`** - Register workflow constructor
- **`cloaca.get_backend()`** - Get compiled backend ("sqlite" or "postgres")