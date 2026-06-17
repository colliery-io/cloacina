---
title: "Build Workflows with WorkflowBuilder"
description: "How to assemble, validate, and debug in-process workflows using the WorkflowBuilder pattern, including dynamic, conditional, and composite construction."
weight: 10
aliases:
  - "/workflows/how-to-guides/build-workflows-with-the-builder/"
  - "/python/workflows/how-to-guides/build-workflows-with-the-builder/"

---

# Build Workflows with WorkflowBuilder

This guide shows how to assemble in-process workflows with `WorkflowBuilder`:
configuring and adding tasks, building dynamic or conditional variants, validating
and inspecting the resulting structure, and handling errors robustly.

For the full method-by-method API surface (signatures, parameters, return types,
and raised errors), see the [WorkflowBuilder reference]({{< ref "/reference/python-api/workflow-builder" >}}).

## Prerequisites

- Tasks declared with `@cloaca.task` (see the [Task Decorator reference]({{< ref "/reference/python-api/task" >}}))
- A `DefaultRunner` to execute the built workflow (see the [DefaultRunner reference]({{< ref "/reference/python-api/runner" >}}))

{{< hint info >}}
This guide covers **in-process** construction. For **packaged `.cloacina`** workflows,
declare tasks with bare `@cloaca.task` decorators and do not construct a
`WorkflowBuilder` — see [Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}}).
{{< /hint >}}

## Build a complete workflow

The most common case: define tasks, assemble them with a builder inside a factory
function, register a constructor, and execute.

```python
import cloaca
from datetime import datetime

# Define tasks
@cloaca.task()
def fetch_users(context):
    """Fetch user data from API."""
    # Simulate API call
    users = [
        {"id": 1, "name": "Alice", "email": "alice@example.com"},
        {"id": 2, "name": "Bob", "email": "bob@example.com"}
    ]
    context.set("users", users)
    context.set("fetch_time", datetime.now().isoformat())
    return context

@cloaca.task(dependencies=["fetch_users"])
def validate_users(context):
    """Validate user data."""
    users = context.get("users", [])
    valid_users = []

    for user in users:
        if user.get("email") and "@" in user["email"]:
            valid_users.append(user)

    context.set("valid_users", valid_users)
    context.set("validation_count", len(valid_users))
    return context

@cloaca.task(dependencies=["validate_users"])
def process_users(context):
    """Process validated users."""
    valid_users = context.get("valid_users", [])

    processed_users = []
    for user in valid_users:
        processed_user = {
            **user,
            "processed_at": datetime.now().isoformat(),
            "status": "active"
        }
        processed_users.append(processed_user)

    context.set("processed_users", processed_users)
    return context

@cloaca.task(dependencies=["process_users"])
def save_results(context):
    """Save processed results."""
    processed_users = context.get("processed_users", [])

    # Simulate saving to database
    context.set("saved_count", len(processed_users))
    context.set("save_time", datetime.now().isoformat())
    context.set("workflow_complete", True)
    return context

# Build workflow using builder pattern
def create_user_processing_workflow():
    """Create and return the user processing workflow."""
    builder = cloaca.WorkflowBuilder("user_processing_workflow")

    # Configure workflow
    builder.description("Fetch, validate, process, and save user data")
    builder.tag("category", "data_processing")
    builder.tag("frequency", "hourly")
    builder.tag("department", "engineering")

    # Add tasks in any order (dependencies determine execution order)
    builder.add_task("save_results")      # Can add in any order
    builder.add_task("fetch_users")
    builder.add_task("process_users")
    builder.add_task("validate_users")

    return builder.build()

# Register workflow
cloaca.register_workflow_constructor(
    "user_processing_workflow",
    create_user_processing_workflow
)

# Execute workflow
if __name__ == "__main__":
    runner = cloaca.DefaultRunner("sqlite:///users.db")

    context = cloaca.Context({
        "batch_id": "batch_001",
        "requested_by": "scheduler"
    })

    result = runner.execute("user_processing_workflow", context)

    if result.status == "Completed":
        final_context = result.final_context
        print(f"Processed {final_context.get('saved_count')} users")
        print(f"Completed at: {final_context.get('save_time')}")

    runner.shutdown()
```

## Advanced patterns

### Add tasks dynamically

Build a workflow whose task set is determined at construction time, then register
each variant under its own name.

```python
def create_dynamic_workflow(task_count):
    """Create workflow with dynamic number of tasks."""
    builder = cloaca.WorkflowBuilder(f"dynamic_workflow_{task_count}")
    builder.description(f"Dynamic workflow with {task_count} parallel tasks")

    # Add initial task
    builder.add_task("initialize")

    # Add dynamic parallel tasks
    for i in range(task_count):
        builder.add_task(f"parallel_task_{i}")

    # Add final aggregation task
    builder.add_task("aggregate_results")

    return builder.build()

# Register multiple variants
for count in [2, 4, 8]:
    workflow_name = f"dynamic_workflow_{count}"
    cloaca.register_workflow_constructor(
        workflow_name,
        lambda c=count: create_dynamic_workflow(c)
    )
```

### Build conditionally

Adapt the task set to a runtime parameter such as the target environment.

```python
def create_environment_specific_workflow(environment):
    """Create workflow adapted for specific environment."""
    builder = cloaca.WorkflowBuilder(f"deploy_workflow_{environment}")
    builder.tag("environment", environment)

    # Common tasks
    builder.add_task("prepare_deployment")
    builder.add_task("run_tests")

    # Environment-specific tasks
    if environment == "production":
        builder.add_task("backup_database")
        builder.add_task("notify_stakeholders")
        builder.add_task("create_rollback_point")
    elif environment == "staging":
        builder.add_task("load_test_data")
        builder.add_task("run_integration_tests")

    # Common final tasks
    builder.add_task("deploy_application")
    builder.add_task("verify_deployment")

    return builder.build()
```

### Compose multiple phases

Group tasks into phases (ingestion, processing, analysis, reporting) within a single
workflow, relying on declared dependencies to order them.

```python
def create_composite_workflow():
    """Create workflow that combines multiple sub-workflows."""
    builder = cloaca.WorkflowBuilder("composite_workflow")
    builder.description("Composite workflow combining multiple processes")

    # Data ingestion phase
    builder.add_task("ingest_customer_data")
    builder.add_task("ingest_product_data")
    builder.add_task("ingest_order_data")

    # Processing phase (depends on ingestion)
    builder.add_task("process_customers")     # depends on ingest_customer_data
    builder.add_task("process_products")      # depends on ingest_product_data
    builder.add_task("process_orders")        # depends on ingest_order_data

    # Analysis phase (depends on processing)
    builder.add_task("analyze_sales_trends")  # depends on all processing tasks
    builder.add_task("analyze_customer_behavior")

    # Reporting phase (depends on analysis)
    builder.add_task("generate_executive_report")
    builder.add_task("generate_detailed_reports")

    return builder.build()
```

## Validate and debug a workflow

### Validate before relying on a build

Wrap `build()` to surface structural problems and report the resulting topology.

```python
def validate_workflow_structure(builder):
    """Validate workflow before building."""
    try:
        workflow = builder.build()
        print("✓ Workflow validation passed")

        # Check workflow properties
        print(f"Workflow name: {workflow.name}")
        print(f"Description: {workflow.description}")

        # Analyze structure
        roots = workflow.get_roots()
        leaves = workflow.get_leaves()
        levels = workflow.get_execution_levels()

        print(f"Root tasks: {roots}")
        print(f"Leaf tasks: {leaves}")
        print(f"Execution levels: {len(levels)}")

        return workflow

    except Exception as e:
        print(f"✗ Workflow validation failed: {e}")
        return None

# Use validation
builder = cloaca.WorkflowBuilder("test_workflow")
builder.add_task("task_1")
builder.add_task("task_2")

workflow = validate_workflow_structure(builder)
```

### Inspect execution order and parallelism

Use the built workflow's topology accessors to see execution order and which tasks
run in parallel.

```python
def inspect_workflow(workflow):
    """Inspect workflow structure and dependencies."""
    print(f"Workflow: {workflow.name}")
    print(f"Version: {workflow.version}")
    print(f"Description: {workflow.description}")

    # Show topological order
    topo_order = workflow.topological_sort()
    print(f"Execution order: {' → '.join(topo_order)}")

    # Show execution levels (parallel groups)
    levels = workflow.get_execution_levels()
    print("\nExecution levels:")
    for i, level in enumerate(levels):
        print(f"  Level {i}: {level}")

    # Tasks within the same execution level have no dependency between them
    # and therefore run in parallel.
    print("\nParallelism analysis:")
    for i, level in enumerate(levels):
        if len(level) > 1:
            print(f"  Level {i} runs in parallel: {level}")
```

## Handle errors

### Recognize common build errors

All builder failures surface as `ValueError`. The two most common are referencing a
task that was never defined, and introducing a circular dependency.

```python
import cloaca

# Error: Missing task
try:
    builder = cloaca.WorkflowBuilder("broken_workflow")
    builder.add_task("nonexistent_task")  # Task not defined
    workflow = builder.build()
except ValueError as e:
    print(f"Missing task error: {e}")

# Error: Circular dependency
@cloaca.task(dependencies=["task_b"])
def task_a(context):
    return context

@cloaca.task(dependencies=["task_a"])  # Circular!
def task_b(context):
    return context

try:
    builder = cloaca.WorkflowBuilder("circular_workflow")
    builder.add_task("task_a")
    builder.add_task("task_b")
    workflow = builder.build()
except ValueError as e:
    print(f"Circular dependency error: {e}")
```

### Build defensively

Wrap construction so a single bad task or build failure degrades gracefully instead
of raising.

```python
def build_workflow_safely(name, task_list, description=None):
    """Build workflow with comprehensive error handling."""
    try:
        builder = cloaca.WorkflowBuilder(name)

        if description:
            builder.description(description)

        # Add tasks with validation
        for task in task_list:
            try:
                builder.add_task(task)
            except Exception as e:
                print(f"Warning: Failed to add task {task}: {e}")
                continue

        # Build with validation
        workflow = builder.build()
        print(f"✓ Successfully built workflow: {name}")
        return workflow

    except Exception as e:
        print(f"✗ Failed to build workflow {name}: {e}")
        return None

# Usage
tasks = ["fetch_data", "process_data", "save_results"]
workflow = build_workflow_safely(
    "safe_workflow",
    tasks,
    "Safely built workflow"
)
```

## Best practices

### Name workflows clearly

Use descriptive, consistently-cased names. Stick to a single separator convention
(snake_case or kebab-case) across an application.

```python
# Good: Descriptive, consistent naming
builder = cloaca.WorkflowBuilder("user_registration_workflow")
builder = cloaca.WorkflowBuilder("daily_sales_report")
builder = cloaca.WorkflowBuilder("database_backup_process")

# Avoid: Unclear or inconsistent names
builder = cloaca.WorkflowBuilder("workflow1")        # Not descriptive
builder = cloaca.WorkflowBuilder("UserWorkflow")     # Inconsistent case
builder = cloaca.WorkflowBuilder("my-workflow")      # Mixed separators
```

### Organize construction code

Encapsulate construction in factory functions, static factory methods, or
configuration-driven builders so workflows are easy to register and reuse.

```python
# Pattern 1: Factory functions
def create_etl_workflow():
    builder = cloaca.WorkflowBuilder("etl_workflow")
    builder.description("Extract, transform, and load data")
    builder.add_task("extract")
    builder.add_task("transform")
    builder.add_task("load")
    return builder.build()

# Pattern 2: Class-based builders
class WorkflowFactory:
    @staticmethod
    def create_reporting_workflow():
        builder = cloaca.WorkflowBuilder("reporting_workflow")
        builder.description("Generate business reports")
        builder.tag("category", "reporting")
        builder.add_task("collect_data")
        builder.add_task("generate_report")
        return builder.build()

# Pattern 3: Configuration-driven
def create_workflow_from_config(config):
    builder = cloaca.WorkflowBuilder(config["name"])
    builder.description(config["description"])

    for tag_key, tag_value in config.get("tags", {}).items():
        builder.tag(tag_key, tag_value)

    for task in config["tasks"]:
        builder.add_task(task)

    return builder.build()
```

### Register workflows centrally

Register all workflow constructors from a single function called at startup, so the
runtime knows every workflow name your application can execute.

```python
# Centralized registration
def register_all_workflows():
    """Register all application workflows."""
    workflows = {
        "user_onboarding": create_user_onboarding_workflow,
        "daily_reports": create_daily_reports_workflow,
        "data_cleanup": create_data_cleanup_workflow,
        "backup_process": create_backup_workflow
    }

    for name, constructor in workflows.items():
        cloaca.register_workflow_constructor(name, constructor)
        print(f"Registered workflow: {name}")

# Call during application startup
register_all_workflows()
```

## Related reference

- **[WorkflowBuilder]({{< ref "/reference/python-api/workflow-builder" >}})** — full builder API surface
- **[Context]({{< ref "/reference/python-api/context" >}})** — data passed through workflows
- **[DefaultRunner]({{< ref "/reference/python-api/runner" >}})** — executes built workflows
- **[Task Decorator]({{< ref "/reference/python-api/task" >}})** — defines tasks added to workflows
- **[Workflow]({{< ref "/reference/python-api/workflow" >}})** — built workflow objects
