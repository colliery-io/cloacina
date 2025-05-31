---
title: "Python Bindings Tutorial"
weight: 50
---

# Python Bindings Tutorial

This tutorial walks you through using Cloacina's Python bindings to build a complete data processing workflow. You'll learn installation, basic usage, and advanced patterns.

## Prerequisites

- Python 3.9 or higher
- Basic familiarity with Python async/await
- Optional: PostgreSQL or SQLite knowledge (we'll explain the basics)

## Installation

Choose your database backend when installing:

```bash
# For PostgreSQL (recommended for production)
pip install cloacina[postgres]

# For SQLite (great for development/testing)
pip install cloacina[sqlite]
```

Let's verify the installation:

```python
import cloacina
print(f"Cloacina version: {cloacina.__version__}")
print(f"Backend: {cloacina.__backend__}")
```

## Your First Workflow

Let's start with a simple data extraction and transformation pipeline:

```python
from cloacina import task, Workflow, UnifiedExecutor
import asyncio

# Step 1: Define tasks using the @task decorator
@task(id="extract_users", dependencies=[])
def extract_users(context):
    """Extract user data from a mock API."""
    # Simulate API call
    users = [
        {"id": 1, "name": "Alice", "email": "alice@example.com"},
        {"id": 2, "name": "Bob", "email": "bob@example.com"},
        {"id": 3, "name": "Charlie", "email": "charlie@example.com"},
    ]
    
    context["raw_users"] = users
    context["extraction_timestamp"] = "2024-01-15T10:30:00Z"
    print(f"Extracted {len(users)} users")
    return context

@task(id="validate_users", dependencies=["extract_users"])
def validate_users(context):
    """Validate user data."""
    raw_users = context["raw_users"]
    valid_users = []
    invalid_users = []
    
    for user in raw_users:
        if "@" in user["email"] and len(user["name"]) > 0:
            valid_users.append(user)
        else:
            invalid_users.append(user)
    
    context["valid_users"] = valid_users
    context["invalid_users"] = invalid_users
    context["validation_stats"] = {
        "total": len(raw_users),
        "valid": len(valid_users),
        "invalid": len(invalid_users)
    }
    
    print(f"Validation: {len(valid_users)} valid, {len(invalid_users)} invalid")
    return context

@task(id="transform_users", dependencies=["validate_users"])
def transform_users(context):
    """Transform users into our internal format."""
    valid_users = context["valid_users"]
    
    transformed_users = []
    for user in valid_users:
        transformed_user = {
            "user_id": f"USER_{user['id']:04d}",
            "full_name": user["name"].upper(),
            "email_domain": user["email"].split("@")[1],
            "processed_at": context["extraction_timestamp"]
        }
        transformed_users.append(transformed_user)
    
    context["transformed_users"] = transformed_users
    print(f"Transformed {len(transformed_users)} users")
    return context

@task(id="save_results", dependencies=["transform_users"])
def save_results(context):
    """Save the final results."""
    transformed_users = context["transformed_users"]
    stats = context["validation_stats"]
    
    # In a real app, you'd save to a database
    print("=== FINAL RESULTS ===")
    print(f"Processing completed at: {context['extraction_timestamp']}")
    print(f"Stats: {stats}")
    print("Transformed users:")
    for user in transformed_users:
        print(f"  - {user}")
    
    context["pipeline_status"] = "completed"
    return context

# Step 2: Create and execute the workflow
async def main():
    # Create workflow - automatically includes all registered tasks
    workflow = Workflow("user_processing_pipeline")
    
    # Create executor and run
    executor = UnifiedExecutor()
    
    try:
        # Initialize the executor
        await executor.initialize()
        
        # Execute the workflow
        print("Starting workflow execution...")
        result = await executor.execute(workflow)
        
        print(f"Workflow completed with status: {result}")
        
    finally:
        # Always clean up
        await executor.shutdown()

# Step 3: Run the workflow
if __name__ == "__main__":
    asyncio.run(main())
```

Run this script and you should see output like:

```
Starting workflow execution...
Extracted 3 users
Validation: 3 valid, 0 invalid
Transformed 3 users
=== FINAL RESULTS ===
Processing completed at: 2024-01-15T10:30:00Z
Stats: {'total': 3, 'valid': 3, 'invalid': 0}
Transformed users:
  - {'user_id': 'USER_0001', 'full_name': 'ALICE', 'email_domain': 'example.com', 'processed_at': '2024-01-15T10:30:00Z'}
  - {'user_id': 'USER_0002', 'full_name': 'BOB', 'email_domain': 'example.com', 'processed_at': '2024-01-15T10:30:00Z'}
  - {'user_id': 'USER_0003', 'full_name': 'CHARLIE', 'email_domain': 'example.com', 'processed_at': '2024-01-15T10:30:00Z'}
Workflow completed with status: completed
```

## Understanding the Workflow

### Task Dependencies

Notice how tasks are connected through the `dependencies` parameter:

```
extract_users (no dependencies)
     ↓
validate_users (depends on extract_users)
     ↓  
transform_users (depends on validate_users)
     ↓
save_results (depends on transform_users)
```

Cloacina ensures tasks run in the correct order and that each task receives the context from its dependencies.

### Context Passing

The `context` parameter is how data flows between tasks:
- Each task receives a context dictionary
- Tasks can read data added by previous tasks
- Tasks can add new data for downstream tasks
- The context is automatically saved and loaded between task executions

## Parallel Processing

Let's enhance our workflow with parallel processing:

```python
from cloacina import task, Workflow, UnifiedExecutor
import asyncio
import time

@task(id="extract_users", dependencies=[])
def extract_users(context):
    """Extract user data."""
    users = [
        {"id": 1, "name": "Alice", "email": "alice@example.com", "age": 25},
        {"id": 2, "name": "Bob", "email": "bob@example.com", "age": 30},
        {"id": 3, "name": "Charlie", "email": "charlie@example.com", "age": 35},
    ]
    context["raw_users"] = users
    return context

# These two tasks can run in parallel since they both depend on extract_users
@task(id="validate_emails", dependencies=["extract_users"])
def validate_emails(context):
    """Validate email addresses (can run in parallel)."""
    print("🔍 Validating emails...")
    time.sleep(1)  # Simulate work
    
    users = context["raw_users"]
    email_validation = {}
    
    for user in users:
        email = user["email"]
        is_valid = "@" in email and "." in email.split("@")[1]
        email_validation[user["id"]] = {
            "email": email,
            "is_valid": is_valid
        }
    
    context["email_validation"] = email_validation
    print("✅ Email validation complete")
    return context

@task(id="categorize_by_age", dependencies=["extract_users"])  
def categorize_by_age(context):
    """Categorize users by age (can run in parallel)."""
    print("📊 Categorizing by age...")
    time.sleep(1)  # Simulate work
    
    users = context["raw_users"]
    age_categories = {"young": [], "middle": [], "senior": []}
    
    for user in users:
        age = user["age"]
        if age < 30:
            category = "young"
        elif age < 50:
            category = "middle"
        else:
            category = "senior"
        
        age_categories[category].append(user["id"])
    
    context["age_categories"] = age_categories
    print("✅ Age categorization complete")
    return context

# This task waits for both parallel tasks to complete
@task(id="create_report", dependencies=["validate_emails", "categorize_by_age"])
def create_report(context):
    """Create final report using results from both parallel tasks."""
    print("📈 Creating final report...")
    
    email_validation = context["email_validation"]
    age_categories = context["age_categories"]
    
    report = {
        "total_users": len(context["raw_users"]),
        "valid_emails": sum(1 for v in email_validation.values() if v["is_valid"]),
        "age_distribution": {k: len(v) for k, v in age_categories.items()}
    }
    
    context["final_report"] = report
    
    print("=== FINAL REPORT ===")
    print(f"Total users: {report['total_users']}")
    print(f"Valid emails: {report['valid_emails']}")
    print(f"Age distribution: {report['age_distribution']}")
    
    return context

async def main():
    workflow = Workflow("parallel_processing_pipeline")
    executor = UnifiedExecutor()
    
    try:
        await executor.initialize()
        
        start_time = time.time()
        await executor.execute(workflow)
        end_time = time.time()
        
        print(f"Pipeline completed in {end_time - start_time:.2f} seconds")
        
    finally:
        await executor.shutdown()

if __name__ == "__main__":
    asyncio.run(main())
```

You'll notice that `validate_emails` and `categorize_by_age` run concurrently, reducing total execution time.

## Error Handling and Retry

Cloacina provides robust error handling and retry capabilities:

```python
from cloacina import task, Workflow, UnifiedExecutor
import asyncio
import random

@task(id="unreliable_api_call", dependencies=[])
def unreliable_api_call(context):
    """Simulate an unreliable API that sometimes fails."""
    
    # 30% chance of failure
    if random.random() < 0.3:
        raise Exception("API temporarily unavailable")
    
    context["api_data"] = {"status": "success", "data": [1, 2, 3, 4, 5]}
    print("✅ API call successful")
    return context

@task(id="process_api_data", dependencies=["unreliable_api_call"])
def process_api_data(context):
    """Process the API data."""
    api_data = context["api_data"]
    
    processed_data = {
        "count": len(api_data["data"]),
        "sum": sum(api_data["data"]),
        "average": sum(api_data["data"]) / len(api_data["data"])
    }
    
    context["processed_data"] = processed_data
    print(f"📊 Processed data: {processed_data}")
    return context

async def main():
    workflow = Workflow("retry_example_pipeline")
    executor = UnifiedExecutor()
    
    try:
        await executor.initialize()
        
        # The executor will automatically retry failed tasks
        result = await executor.execute(workflow)
        print(f"Pipeline result: {result}")
        
    except Exception as e:
        print(f"Pipeline failed after retries: {e}")
        
    finally:
        await executor.shutdown()

if __name__ == "__main__":
    asyncio.run(main())
```

## Working with Initial Context

You can provide initial data to your workflow:

```python
from cloacina import task, Workflow, UnifiedExecutor
import asyncio

@task(id="process_config", dependencies=[])
def process_config(context):
    """Process configuration from initial context."""
    
    # Access data provided when creating the workflow
    config = context.get("config", {})
    batch_size = config.get("batch_size", 10)
    environment = config.get("environment", "development")
    
    print(f"Processing with batch_size={batch_size}, environment={environment}")
    
    context["processed_config"] = {
        "batch_size": batch_size,
        "environment": environment,
        "is_production": environment == "production"
    }
    
    return context

@task(id="load_data", dependencies=["process_config"])
def load_data(context):
    """Load data based on configuration."""
    config = context["processed_config"]
    batch_size = config["batch_size"]
    
    # Simulate loading data in batches
    all_data = list(range(100))  # 100 records
    batches = [all_data[i:i+batch_size] for i in range(0, len(all_data), batch_size)]
    
    context["data_batches"] = batches
    print(f"Loaded {len(batches)} batches of size {batch_size}")
    
    return context

async def main():
    # Create workflow with initial context
    workflow = Workflow("config_example_pipeline")
    
    # Provide initial configuration
    initial_context = {
        "config": {
            "batch_size": 25,
            "environment": "production"
        }
    }
    
    executor = UnifiedExecutor()
    
    try:
        await executor.initialize()
        
        # Pass initial context to the execution
        result = await executor.execute(workflow, initial_context)
        print(f"Pipeline completed: {result}")
        
    finally:
        await executor.shutdown()

if __name__ == "__main__":
    asyncio.run(main())
```

## Database Configuration

### PostgreSQL Setup

If you're using the PostgreSQL backend, set up your database connection:

```python
import os
from cloacina import UnifiedExecutor

# Set database URL (or use environment variable)
os.environ["DATABASE_URL"] = "postgresql://user:password@localhost:5432/cloacina_db"

async def main():
    executor = UnifiedExecutor()
    # The executor will automatically use the DATABASE_URL
    await executor.initialize()
    # ... your workflow code
    await executor.shutdown()
```

### SQLite Setup

For SQLite (great for development):

```python
import os
from cloacina import UnifiedExecutor

# SQLite file will be created automatically
os.environ["DATABASE_URL"] = "sqlite:///cloacina.db"

async def main():
    executor = UnifiedExecutor()
    await executor.initialize()
    # ... your workflow code  
    await executor.shutdown()
```

## Advanced Patterns

### Conditional Execution

```python
@task(id="check_conditions", dependencies=[])
def check_conditions(context):
    context["should_process_large_data"] = len(context.get("input_data", [])) > 1000
    return context

@task(id="process_large_data", dependencies=["check_conditions"])
def process_large_data(context):
    if not context.get("should_process_large_data", False):
        print("Skipping large data processing")
        return context
        
    print("Processing large dataset...")
    # ... processing logic
    return context
```

### Dynamic Task Registration

```python
from cloacina import task

# Tasks can be defined dynamically
def create_processing_task(data_type):
    @task(id=f"process_{data_type}", dependencies=["extract_data"])
    def process_data(context):
        data = context[f"{data_type}_data"]
        # Process specific data type
        context[f"processed_{data_type}"] = f"Processed {len(data)} {data_type} records"
        return context
    
    return process_data

# Create tasks for different data types
for data_type in ["users", "orders", "products"]:
    create_processing_task(data_type)
```

## Best Practices

### 1. Keep Tasks Focused
Each task should have a single responsibility:

```python
# Good: Focused task
@task(id="extract_users", dependencies=[])
def extract_users(context):
    # Only extract users
    pass

# Avoid: Task doing too many things
@task(id="extract_and_process_everything", dependencies=[])
def extract_and_process_everything(context):
    # Extract users, validate, transform, save...
    pass
```

### 2. Use Descriptive Task IDs
```python
# Good: Clear what the task does
@task(id="validate_email_format", dependencies=["extract_users"])

# Avoid: Vague task names  
@task(id="task1", dependencies=["task0"])
```

### 3. Handle Errors Gracefully
```python
@task(id="safe_api_call", dependencies=[])
def safe_api_call(context):
    try:
        # API call logic
        context["api_data"] = fetch_from_api()
    except Exception as e:
        # Set default or error state
        context["api_data"] = None
        context["api_error"] = str(e)
        print(f"API call failed: {e}")
    
    return context
```

### 4. Use Context Efficiently
```python
@task(id="efficient_task", dependencies=["previous_task"])
def efficient_task(context):
    # Check if data exists before processing
    if "processed_data" in context:
        print("Data already processed, skipping...")
        return context
    
    # Process only what's needed
    raw_data = context.get("raw_data", [])
    if not raw_data:
        print("No data to process")
        return context
    
    # Process and store results
    context["processed_data"] = process(raw_data)
    return context
```

## Troubleshooting

### Common Issues

**Import Error**: If you get `ImportError: No Cloacina backend found`:
```bash
# Make sure you installed with a backend
pip install cloacina[postgres]
# or
pip install cloacina[sqlite]
```

**Database Connection Error**: 
```python
# Check your DATABASE_URL
import os
print(os.environ.get("DATABASE_URL"))

# For SQLite, ensure the directory exists
import pathlib
db_path = pathlib.Path("cloacina.db")
db_path.parent.mkdir(parents=True, exist_ok=True)
```

**Task Not Found**: If tasks aren't being registered:
```python
# Ensure task definitions are imported before creating workflow
from my_tasks import *  # Import all task definitions
workflow = Workflow("my_workflow")
```

## Next Steps

Now that you've mastered the basics:

1. **Explore Advanced Features**: Check out the [Advanced Patterns Guide](../how-to-guides/) for complex workflows
2. **Performance Optimization**: Learn about optimizing task execution and database connections
3. **Production Deployment**: Set up monitoring, logging, and error tracking for production workflows
4. **Integration**: Connect Cloacina with your existing systems and data sources

## Complete Example Project

Here's a complete example that demonstrates all the concepts:

```python
"""
Complete ETL Pipeline Example
Demonstrates: parallel processing, error handling, configuration, and reporting
"""

from cloacina import task, Workflow, UnifiedExecutor
import asyncio
import json
import time
from datetime import datetime

# Configuration
@task(id="load_config", dependencies=[])
def load_config(context):
    config = context.get("config", {
        "batch_size": 50,
        "max_retries": 3,
        "environment": "development"
    })
    
    context["config"] = config
    context["execution_id"] = f"exec_{int(time.time())}"
    context["start_time"] = datetime.now().isoformat()
    
    print(f"🚀 Starting execution {context['execution_id']}")
    print(f"📋 Config: {config}")
    
    return context

# Data extraction
@task(id="extract_customers", dependencies=["load_config"])
def extract_customers(context):
    print("📥 Extracting customer data...")
    
    # Simulate API data
    customers = [
        {"id": i, "name": f"Customer {i}", "email": f"customer{i}@example.com", 
         "registration_date": "2024-01-01", "status": "active" if i % 4 != 0 else "inactive"}
        for i in range(1, 101)
    ]
    
    context["raw_customers"] = customers
    print(f"✅ Extracted {len(customers)} customers")
    return context

@task(id="extract_orders", dependencies=["load_config"])
def extract_orders(context):
    print("📥 Extracting order data...")
    
    # Simulate order data
    orders = [
        {"id": i, "customer_id": (i % 100) + 1, "amount": i * 10.5, 
         "order_date": "2024-01-15", "status": "completed"}
        for i in range(1, 301)
    ]
    
    context["raw_orders"] = orders
    print(f"✅ Extracted {len(orders)} orders")
    return context

# Parallel processing
@task(id="validate_customers", dependencies=["extract_customers"])
def validate_customers(context):
    print("🔍 Validating customers...")
    time.sleep(0.5)  # Simulate processing time
    
    customers = context["raw_customers"]
    valid_customers = []
    issues = []
    
    for customer in customers:
        if "@" not in customer["email"]:
            issues.append(f"Invalid email for customer {customer['id']}")
        elif customer["status"] not in ["active", "inactive"]:
            issues.append(f"Invalid status for customer {customer['id']}")
        else:
            valid_customers.append(customer)
    
    context["valid_customers"] = valid_customers
    context["customer_validation_issues"] = issues
    
    print(f"✅ Validated {len(valid_customers)} customers, {len(issues)} issues found")
    return context

@task(id="validate_orders", dependencies=["extract_orders"])
def validate_orders(context):
    print("🔍 Validating orders...")
    time.sleep(0.5)  # Simulate processing time
    
    orders = context["raw_orders"]
    valid_orders = []
    issues = []
    
    for order in orders:
        if order["amount"] <= 0:
            issues.append(f"Invalid amount for order {order['id']}")
        elif order["status"] not in ["completed", "pending", "cancelled"]:
            issues.append(f"Invalid status for order {order['id']}")
        else:
            valid_orders.append(order)
    
    context["valid_orders"] = valid_orders
    context["order_validation_issues"] = issues
    
    print(f"✅ Validated {len(valid_orders)} orders, {len(issues)} issues found")
    return context

# Data enrichment
@task(id="enrich_data", dependencies=["validate_customers", "validate_orders"])
def enrich_data(context):
    print("🔄 Enriching customer data with order information...")
    
    customers = context["valid_customers"]
    orders = context["valid_orders"]
    
    # Group orders by customer
    customer_orders = {}
    for order in orders:
        customer_id = order["customer_id"]
        if customer_id not in customer_orders:
            customer_orders[customer_id] = []
        customer_orders[customer_id].append(order)
    
    # Enrich customers with order stats
    enriched_customers = []
    for customer in customers:
        customer_id = customer["id"]
        customer_order_list = customer_orders.get(customer_id, [])
        
        enriched_customer = customer.copy()
        enriched_customer.update({
            "total_orders": len(customer_order_list),
            "total_spent": sum(order["amount"] for order in customer_order_list),
            "avg_order_value": (
                sum(order["amount"] for order in customer_order_list) / len(customer_order_list)
                if customer_order_list else 0
            )
        })
        
        enriched_customers.append(enriched_customer)
    
    context["enriched_customers"] = enriched_customers
    print(f"✅ Enriched {len(enriched_customers)} customers with order data")
    return context

# Final reporting
@task(id="generate_report", dependencies=["enrich_data"])
def generate_report(context):
    print("📊 Generating final report...")
    
    customers = context["enriched_customers"]
    config = context["config"]
    
    # Calculate metrics
    total_customers = len(customers)
    active_customers = len([c for c in customers if c["status"] == "active"])
    total_revenue = sum(c["total_spent"] for c in customers)
    avg_customer_value = total_revenue / total_customers if total_customers > 0 else 0
    
    # Create report
    report = {
        "execution_id": context["execution_id"],
        "start_time": context["start_time"],
        "end_time": datetime.now().isoformat(),
        "config": config,
        "metrics": {
            "total_customers": total_customers,
            "active_customers": active_customers,
            "total_revenue": round(total_revenue, 2),
            "avg_customer_value": round(avg_customer_value, 2)
        },
        "data_quality": {
            "customer_issues": len(context["customer_validation_issues"]),
            "order_issues": len(context["order_validation_issues"])
        },
        "top_customers": sorted(
            customers, 
            key=lambda x: x["total_spent"], 
            reverse=True
        )[:5]
    }
    
    context["final_report"] = report
    
    print("\n" + "="*50)
    print("📈 EXECUTION REPORT")
    print("="*50)
    print(f"Execution ID: {report['execution_id']}")
    print(f"Total Customers: {report['metrics']['total_customers']}")
    print(f"Active Customers: {report['metrics']['active_customers']}")
    print(f"Total Revenue: ${report['metrics']['total_revenue']:,.2f}")
    print(f"Avg Customer Value: ${report['metrics']['avg_customer_value']:.2f}")
    print(f"Data Issues: {report['data_quality']['customer_issues']} customer, {report['data_quality']['order_issues']} order")
    print("\nTop 5 Customers by Spend:")
    for i, customer in enumerate(report["top_customers"], 1):
        print(f"  {i}. {customer['name']}: ${customer['total_spent']:.2f}")
    print("="*50)
    
    return context

async def main():
    # Create workflow
    workflow = Workflow("complete_etl_pipeline")
    
    # Initial configuration
    initial_context = {
        "config": {
            "batch_size": 100,
            "environment": "production",
            "max_retries": 5
        }
    }
    
    executor = UnifiedExecutor()
    
    try:
        print("🔧 Initializing executor...")
        await executor.initialize()
        
        print("▶️  Starting ETL pipeline...")
        start_time = time.time()
        
        result = await executor.execute(workflow, initial_context)
        
        end_time = time.time()
        execution_time = end_time - start_time
        
        print(f"🎉 Pipeline completed successfully in {execution_time:.2f} seconds!")
        print(f"Result: {result}")
        
    except Exception as e:
        print(f"❌ Pipeline failed: {e}")
        raise
        
    finally:
        print("🔧 Shutting down executor...")
        await executor.shutdown()

if __name__ == "__main__":
    asyncio.run(main())
```

This complete example demonstrates a production-ready ETL pipeline with proper error handling, parallel processing, configuration management, and comprehensive reporting.

Happy building with Cloacina! 🚀