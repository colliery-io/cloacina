---
title: "WorkflowBuilder"
description: "WorkflowBuilder class for creating workflows"
weight: 30
reviewer: "automation"
review_date: "2025-01-07"
---

# WorkflowBuilder

The `WorkflowBuilder` class provides a builder pattern for constructing workflows. It allows you to add tasks, set descriptions, configure dependencies, and build executable workflow objects.

## Constructor

### `WorkflowBuilder(name)`

Create a new workflow builder.

**Parameters:**
- `name` (str): Unique workflow name

**Example:**
```python
import cloaca

builder = cloaca.WorkflowBuilder("data_processing_workflow")
```

**Naming Rules:**
- Must be unique within your application
- Recommended: Use snake_case or kebab-case
- Avoid spaces and special characters
- Should be descriptive of the workflow's purpose

## Configuration Methods

### `description(description)`

Set the workflow description.

**Parameters:**
- `description` (str): Human-readable description of the workflow

**Example:**
```python
builder = cloaca.WorkflowBuilder("etl_pipeline")
builder.description("Extract data from API, transform format, and load to database")
```

### `tag(key, value)`

Add a tag to the workflow for metadata and organization.

**Parameters:**
- `key` (str): Tag key
- `value` (str): Tag value

**Example:**
```python
builder = cloaca.WorkflowBuilder("daily_report")
builder.description("Generate daily sales report")
builder.tag("department", "sales")
builder.tag("frequency", "daily")
builder.tag("priority", "high")
```

**Common Tag Patterns:**
- `department`: Team or department responsible
- `environment`: dev, staging, production
- `priority`: low, medium, high, critical
- `schedule`: daily, weekly, monthly, on-demand
- `category`: etl, reporting, monitoring, cleanup

## Task Management

### `add_task(task)`

Add a task to the workflow.

**Parameters:**
- `task` (str or callable): Task ID string or task function reference

**Example:**
```python
# Method 1: Add by task ID (string)
@cloaca.task(id="extract_data")
def extract_data(context):
    return context

@cloaca.task(id="transform_data", dependencies=["extract_data"])
def transform_data(context):
    return context

builder = cloaca.WorkflowBuilder("etl_workflow")
builder.add_task("extract_data")
builder.add_task("transform_data")

# Method 2: Add by function reference
builder = cloaca.WorkflowBuilder("etl_workflow")
builder.add_task(extract_data)
builder.add_task(transform_data)
```

**Task Resolution:**
- **String**: Must match the `id` parameter of a `@cloaca.task` decorated function
- **Function**: Must be a `@cloaca.task` decorated function

## Building Workflows

### `build()`

Build the workflow and validate its structure.

**Returns:** Workflow object ready for execution

**Raises:** 
- `ValueError`: If workflow has structural problems
- `KeyError`: If referenced tasks don't exist

**Example:**
```python
builder = cloaca.WorkflowBuilder("my_workflow")
builder.description("Sample workflow")
builder.add_task("task_1")
builder.add_task("task_2")

# Build and validate
workflow = builder.build()
```

**Validation Checks:**
- All referenced tasks exist
- No circular dependencies
- All dependencies are resolvable
- Workflow has at least one task

## Context Manager Support

WorkflowBuilder supports context manager protocol for automatic registration.

### `with WorkflowBuilder(...) as builder:`

**Example:**
```python
import cloaca

@cloaca.task(id="hello_task")
def hello_task(context):
    context.set("message", "Hello, World!")
    return context

# Automatic registration
with cloaca.WorkflowBuilder("hello_workflow") as builder:
    builder.description("Simple hello world workflow")
    builder.add_task("hello_task")
    # Workflow automatically registered when exiting context

# Can execute immediately
runner = cloaca.DefaultRunner("sqlite:///app.db")
context = cloaca.Context()
result = runner.execute("hello_workflow", context)
```

## Complete Workflow Example

```python
import cloaca
from datetime import datetime

# Define tasks
@cloaca.task(id="fetch_users")
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

@cloaca.task(id="validate_users", dependencies=["fetch_users"])
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

@cloaca.task(id="process_users", dependencies=["validate_users"])
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

@cloaca.task(id="save_results", dependencies=["process_users"])
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

## Advanced Patterns

### Dynamic Task Addition

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

### Conditional Workflow Building

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

### Workflow Composition

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

## Validation and Debugging

### Manual Validation

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

### Workflow Inspection

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
    
    # Check parallelism opportunities
    print("\nParallelism analysis:")
    for i, task1 in enumerate(topo_order):
        for task2 in topo_order[i+1:]:
            if workflow.can_run_parallel(task1, task2):
                print(f"  {task1} can run parallel with {task2}")
```

## Error Handling

### Common Errors

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
@cloaca.task(id="task_a", dependencies=["task_b"])
def task_a(context):
    return context

@cloaca.task(id="task_b", dependencies=["task_a"])  # Circular!
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

### Robust Workflow Building

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

## Best Practices

### Naming Conventions

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

### Organization Patterns

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

### Workflow Registration

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

## Related Classes

- **[Context](/python-bindings/api-reference/context/)** - Data passed through workflows
- **[DefaultRunner](/python-bindings/api-reference/runner/)** - Executes built workflows
- **[Task Decorator](/python-bindings/api-reference/task/)** - Defines tasks added to workflows
- **[Workflow](/python-bindings/api-reference/workflow/)** - Built workflow objects