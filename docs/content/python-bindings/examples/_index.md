---
title: "Python Examples"
description: "Complete examples and real-world patterns for Cloaca Python workflows"
weight: 50
---

# Python Examples

Explore comprehensive examples and real-world patterns for building production-ready workflows with Cloaca Python bindings.

## Available Examples

{{< toc-tree >}}

## Example Categories

### Basic Patterns
- Simple workflow construction
- Task dependency management
- Context handling patterns
- Error handling strategies

### Advanced Workflows
- Multi-stage data processing
- Conditional execution flows
- Dynamic workflow generation
- Complex dependency chains

### Integration Examples
- Database integration patterns
- API service workflows
- File processing pipelines
- Message queue integration

### Production Patterns
- Multi-tenant workflow management
- Performance optimization examples
- Monitoring and logging patterns
- Deployment configuration examples

## Quick Reference

Each example includes:

1. **Complete working code** - Ready to run examples
2. **Detailed explanations** - Step-by-step breakdown
3. **Configuration examples** - Database and environment setup
4. **Testing patterns** - How to test the workflows
5. **Production considerations** - Deployment and scaling notes

## Running Examples

### Prerequisites

```bash
# Install Python bindings
pip install cloaca

# For PostgreSQL examples
pip install cloaca[postgres]
```

### Basic Setup

```python
import cloaca

# SQLite for development
runner = cloaca.DefaultRunner("sqlite:///examples.db")

# PostgreSQL for production examples
runner = cloaca.DefaultRunner("postgresql://user:pass@localhost:5432/cloacina")
```

## Example Structure

Each example follows a consistent structure:

```
example_name/
├── README.md                    # Overview and setup instructions
├── main.py                      # Main example code
├── tasks.py                     # Task definitions
├── workflows.py                 # Workflow builders
├── config.py                    # Configuration handling
├── tests/                       # Example tests
│   ├── test_tasks.py
│   └── test_workflows.py
└── requirements.txt             # Dependencies
```

## Featured Examples

### Data Processing Pipeline

A complete example showing how to build a data processing pipeline with error handling, retries, and monitoring.

```python
# Preview of the data processing example
@cloaca.task(id="extract_data")
def extract_data(context):
    # Extract data from multiple sources
    pass

@cloaca.task(id="transform_data", dependencies=["extract_data"])
def transform_data(context):
    # Apply transformations with error handling
    pass

@cloaca.task(id="load_data", dependencies=["transform_data"])
def load_data(context):
    # Load into target system with validation
    pass
```

### Multi-Tenant SaaS Example

Demonstrates how to build a multi-tenant SaaS application with isolated workflow execution per customer.

```python
# Preview of multi-tenant example
class TenantWorkflowManager:
    def __init__(self, admin_db_url):
        self.admin = cloaca.DatabaseAdmin(admin_db_url)
        self.tenant_runners = {}

    def execute_for_customer(self, customer_id, workflow_name, context):
        runner = self._get_customer_runner(customer_id)
        return runner.execute(workflow_name, context)
```

### Real-World Scenarios

Production-ready examples covering common use cases:

- **E-commerce Order Processing** - Handle orders, payments, inventory
- **Content Management** - Process uploads, generate thumbnails, notifications
- **Analytics Pipeline** - Collect, process, and report on user data
- **Backup and Sync** - Automated backup and synchronization workflows

## Learning Path

### Available Examples
1. [Basic Workflow]({{< ref "/python-bindings/examples/basic-workflow/" >}}) - Your first workflow

### Tutorial-Based Learning
For comprehensive examples, follow the step-by-step tutorials:
1. [Tutorial 01: First Python Workflow]({{< ref "/python-bindings/tutorials/01-first-python-workflow/" >}}) - Basic concepts
2. [Tutorial 02: Context Handling]({{< ref "/python-bindings/tutorials/02-context-handling/" >}}) - Data flow
3. [Tutorial 03: Complex Workflows]({{< ref "/python-bindings/tutorials/03-complex-workflows/" >}}) - Dependencies
4. [Tutorial 04: Error Handling]({{< ref "/python-bindings/tutorials/04-error-handling/" >}}) - Resilience
5. [Tutorial 05: Cron Scheduling]({{< ref "/python-bindings/tutorials/05-cron-scheduling/" >}}) - Automation
6. [Tutorial 06: Multi-Tenancy]({{< ref "/python-bindings/tutorials/06-multi-tenancy/" >}}) - Production patterns

## Contributing Examples

We welcome contributions of new examples! Each example should:

- Solve a real-world problem
- Include comprehensive documentation
- Provide working code that can be run as-is
- Include appropriate tests
- Follow Python best practices

## Related Resources

- **[Tutorials]({{< ref "/python-bindings/tutorials/" >}})** - Step-by-step learning guides
- **[How-to Guides]({{< ref "/python-bindings/how-to-guides/" >}})** - Problem-solving guides
- **[API Reference]({{< ref "/python-bindings/api-reference/" >}})** - Complete API documentation
- **[GitHub Examples](https://github.com/dstorey/cloacina/tree/main/examples)** - Source code repository
