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

- **[Context](context/)** - Data flow container for workflows
- **[DefaultRunner](runner/)** - Execute workflows and manage scheduling
- **[WorkflowBuilder](workflow-builder/)** - Build complex workflows with dependencies
- **[Workflow](workflow/)** - Executable workflow objects
- **[Task Decorator](task/)** - Define workflow tasks
- **[Configuration](configuration/)** - Runner and system configuration
- **[PipelineResult](pipeline-result/)** - Workflow execution results
- **[Exceptions](exceptions/)** - Error handling and exception types
- **[DatabaseAdmin](database-admin/)** - Multi-tenant database administration

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

### Multi-Tenant Admin (PostgreSQL only)
```python
import cloaca

# Create database admin
admin = cloaca.DatabaseAdmin("postgresql://admin:password@localhost/db")

# Provision new tenant
config = cloaca.TenantConfig(
    schema_name="tenant_acme",
    username="acme_user",
    password=""  # Auto-generate secure password
)
credentials = admin.create_tenant(config)

# Use tenant-specific runner
runner = cloaca.DefaultRunner(credentials.connection_string)
```

## Module Functions

- **`cloaca.task()`** - Decorator for defining workflow tasks
- **`cloaca.register_workflow_constructor()`** - Register workflow constructor
- **`cloaca.get_backend()`** - Get compiled backend ("sqlite" or "postgres")

## Admin Classes (PostgreSQL only)

- **`cloaca.DatabaseAdmin`** - Database administration for multi-tenant deployments
- **`cloaca.TenantConfig`** - Configuration for new tenant provisioning
- **`cloaca.TenantCredentials`** - Returned credentials for tenant access
