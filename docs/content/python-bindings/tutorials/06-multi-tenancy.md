---
title: "06 - Multi-Tenancy"
description: "Deploy isolated workflows for multiple tenants using PostgreSQL schemas"
weight: 16
reviewer: "automation"
review_date: "2025-01-07"
---

# Multi-Tenancy

Welcome to the final tutorial in our Python Cloacina series! In this tutorial, you'll learn how to deploy workflows for multiple tenants with complete data isolation. We'll explore schema-based multi-tenancy using PostgreSQL, tenant management patterns, security considerations, and scaling strategies for SaaS applications.

## Learning Objectives

- Understand schema-based multi-tenancy architecture
- Implement tenant-specific workflow runners
- Manage tenant isolation and security
- Handle tenant lifecycle and recovery
- Design scalable multi-tenant systems
- Apply best practices for SaaS deployments

## Prerequisites

- Completion of [Tutorial 5](/python-bindings/tutorials/05-cron-scheduling/)
- Access to PostgreSQL database
- Understanding of database schemas
- Basic knowledge of SaaS architecture concepts

## Time Estimate
25-30 minutes

## Multi-Tenancy Overview

Cloacina implements multi-tenancy using PostgreSQL schemas, providing complete data isolation between tenants without requiring separate databases or query filtering.

### Key Concepts

{{< tabs "mt-concepts" >}}
{{< tab "Schema Isolation" >}}
**Each tenant gets their own PostgreSQL schema:**
- Complete data isolation
- No cross-tenant data access possible
- Zero collision risk between tenants
- Native PostgreSQL performance
{{< /tab >}}

{{< tab "Automatic Management" >}}
**Cloacina handles schema lifecycle:**
- Creates schemas automatically on first use
- Runs migrations within tenant schemas
- Manages search paths transparently
- Recovers orphaned workflows per tenant
{{< /tab >}}

{{< tab "Security Benefits" >}}
**Enterprise-grade isolation:**
- Database-level security boundaries
- Per-tenant credentials (optional)
- Audit trail compliance
- No application-level filtering required
{{< /tab >}}

{{< tab "Developer Experience" >}}
**Seamless development:**
- No changes to existing workflow code
- Identical API across all tenants
- Easy testing with local schemas
- Clean migration from single-tenant
{{< /tab >}}
{{< /tabs >}}

## Multi-Tenancy Architecture

```mermaid
graph TD
    A[Application Layer] --> B[Tenant A Runner]
    A --> C[Tenant B Runner]
    A --> D[Tenant C Runner]

    B --> E[PostgreSQL Database]
    C --> E
    D --> E

    E --> F[Schema: tenant_a]
    E --> G[Schema: tenant_b]
    E --> H[Schema: tenant_c]

    F --> I[Tables: contexts, executions, etc.]
    G --> J[Tables: contexts, executions, etc.]
    H --> K[Tables: contexts, executions, etc.]

    style B fill:#e6e6ff,stroke:#333,stroke-width:2px,color:#000
    style C fill:#ffe6e6,stroke:#333,stroke-width:2px,color:#000
    style D fill:#e6ffe6,stroke:#333,stroke-width:2px,color:#000
    style F fill:#e6e6ff,stroke:#333,stroke-width:1px,color:#000
    style G fill:#ffe6e6,stroke:#333,stroke-width:1px,color:#000
    style H fill:#e6ffe6,stroke:#333,stroke-width:1px,color:#000
```

## Setting Up Multi-Tenant Infrastructure

### Prerequisites: PostgreSQL Setup

First, ensure you have PostgreSQL running and configured:

```bash
# Install PostgreSQL (macOS)
brew install postgresql
brew services start postgresql

# Create database and user
createuser -s cloacina
createdb -O cloacina cloacina

# Test connection
psql -U cloacina -d cloacina -c "SELECT version();"
```

### Basic Multi-Tenant Setup

```python
import cloaca
import time
from datetime import datetime

# Multi-tenant workflow runners
def create_tenant_runners():
    """Create isolated runners for different tenants."""

    database_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina"

    # Each tenant gets complete schema isolation
    tenant_runners = {
        "acme_corp": cloaca.DefaultRunner.with_schema(database_url, "acme_corp"),
        "globex_inc": cloaca.DefaultRunner.with_schema(database_url, "globex_inc"),
        "initech_llc": cloaca.DefaultRunner.with_schema(database_url, "initech_llc")
    }

    return tenant_runners

# Test tenant isolation
def test_tenant_isolation():
    """Demonstrate that tenants are completely isolated."""

    print("Setting up multi-tenant infrastructure...")
    runners = create_tenant_runners()

    # Define a simple workflow
    @cloaca.task(id="tenant_task")
    def tenant_task(context):
        tenant_id = context.get("tenant_id")
        timestamp = datetime.now().isoformat()

        context.set("task_executed_at", timestamp)
        context.set("tenant_specific_data", f"Data for {tenant_id}")

        print(f"Task executed for tenant: {tenant_id}")
        return context

    def create_tenant_workflow():
        builder = cloaca.WorkflowBuilder("tenant_isolation_test")
        builder.description("Test workflow for tenant isolation")
        builder.add_task("tenant_task")
        return builder.build()

    # Register workflow (shared across all tenants)
    cloaca.register_workflow_constructor("tenant_isolation_test", create_tenant_workflow)

    # Execute workflow for each tenant
    results = {}
    for tenant_id, runner in runners.items():
        print(f"\nExecuting workflow for {tenant_id}...")

        context = cloaca.Context({
            "tenant_id": tenant_id,
            "execution_time": time.time()
        })

        result = runner.execute("tenant_isolation_test", context)
        results[tenant_id] = result

        if result.status == "Completed":
            final_context = result.final_context
            print(f"✓ {tenant_id}: {final_context.get('tenant_specific_data')}")
        else:
            print(f"✗ {tenant_id}: Workflow failed")

    # Cleanup
    for runner in runners.values():
        runner.shutdown()

    return results

if __name__ == "__main__":
    test_tenant_isolation()
```

## Advanced Multi-Tenant Patterns

### 1. Tenant Manager with Dynamic Loading

```python
import cloaca
import asyncio
from typing import Dict, Optional
from threading import Lock

class TenantManager:
    """Manage multiple tenant runners with lazy loading."""

    def __init__(self, database_url: str):
        self.database_url = database_url
        self.runners: Dict[str, cloaca.DefaultRunner] = {}
        self.lock = Lock()

    def get_runner(self, tenant_id: str) -> cloaca.DefaultRunner:
        """Get or create a runner for the specified tenant."""

        # Validate tenant ID format
        if not self._is_valid_tenant_id(tenant_id):
            raise ValueError(f"Invalid tenant ID: {tenant_id}. Must contain only alphanumeric characters and underscores.")

        with self.lock:
            # Return existing runner if available
            if tenant_id in self.runners:
                return self.runners[tenant_id]

            # Create new runner for this tenant
            print(f"Creating new runner for tenant: {tenant_id}")
            runner = cloaca.DefaultRunner.with_schema(self.database_url, tenant_id)
            self.runners[tenant_id] = runner

            return runner

    def _is_valid_tenant_id(self, tenant_id: str) -> bool:
        """Validate tenant ID follows PostgreSQL schema naming rules."""
        if not tenant_id:
            return False

        # Must contain only alphanumeric characters and underscores
        return all(c.isalnum() or c == '_' for c in tenant_id)

    def execute_for_tenant(self, tenant_id: str, workflow_name: str, context: cloaca.Context):
        """Execute a workflow for a specific tenant."""
        runner = self.get_runner(tenant_id)
        return runner.execute(workflow_name, context)

    def list_tenants(self) -> list:
        """List all active tenants."""
        with self.lock:
            return list(self.runners.keys())

    def shutdown_tenant(self, tenant_id: str):
        """Shutdown and remove a specific tenant runner."""
        with self.lock:
            if tenant_id in self.runners:
                runner = self.runners.pop(tenant_id)
                runner.shutdown()
                print(f"Shutdown tenant runner: {tenant_id}")

    def shutdown_all(self):
        """Shutdown all tenant runners."""
        with self.lock:
            for tenant_id, runner in self.runners.items():
                runner.shutdown()
                print(f"Shutdown tenant runner: {tenant_id}")
            self.runners.clear()

# Example usage
def demonstrate_tenant_manager():
    """Demonstrate advanced tenant management."""

    print("=== Advanced Tenant Management ===")

    database_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina"
    manager = TenantManager(database_url)

    # Define multi-tenant workflow
    @cloaca.task(id="process_tenant_data")
    def process_tenant_data(context):
        tenant_id = context.get("tenant_id")
        data_type = context.get("data_type", "default")

        # Simulate tenant-specific processing
        processed_data = {
            "tenant": tenant_id,
            "type": data_type,
            "processed_at": datetime.now().isoformat(),
            "status": "completed"
        }

        context.set("processed_data", processed_data)
        print(f"Processed {data_type} data for tenant {tenant_id}")

        return context

    def create_tenant_workflow():
        builder = cloaca.WorkflowBuilder("tenant_data_processing")
        builder.description("Multi-tenant data processing workflow")
        builder.add_task("process_tenant_data")
        return builder.build()

    cloaca.register_workflow_constructor("tenant_data_processing", create_tenant_workflow)

    # Simulate multiple tenants with different workloads
    tenant_workloads = [
        {"tenant": "customer_alpha", "data_type": "sales_data"},
        {"tenant": "customer_beta", "data_type": "inventory_data"},
        {"tenant": "customer_gamma", "data_type": "user_analytics"},
        {"tenant": "customer_alpha", "data_type": "reporting_data"},  # Same tenant, different workflow
    ]

    results = []
    for workload in tenant_workloads:
        print(f"\nProcessing {workload['data_type']} for {workload['tenant']}")

        context = cloaca.Context({
            "tenant_id": workload["tenant"],
            "data_type": workload["data_type"],
            "request_id": f"req_{int(time.time())}"
        })

        try:
            result = manager.execute_for_tenant(
                workload["tenant"],
                "tenant_data_processing",
                context
            )
            results.append(result)

            if result.status == "Completed":
                final_context = result.final_context
                processed = final_context.get("processed_data")
                print(f"✓ Success: {processed}")
            else:
                print(f"✗ Failed: {result.status}")

        except Exception as e:
            print(f"✗ Error: {e}")

    print(f"\nActive tenants: {manager.list_tenants()}")
    print(f"Total workflows executed: {len(results)}")

    # Cleanup
    manager.shutdown_all()

    return results

if __name__ == "__main__":
    demonstrate_tenant_manager()
```

### 2. Tenant-Specific Configuration

```python
import cloaca
import json
from dataclasses import dataclass
from typing import Dict, Any, Optional

@dataclass
class TenantConfig:
    """Configuration for a specific tenant."""
    tenant_id: str
    display_name: str
    settings: Dict[str, Any]
    features: Dict[str, bool]
    limits: Dict[str, int]

    @classmethod
    def from_dict(cls, data: dict) -> 'TenantConfig':
        return cls(
            tenant_id=data["tenant_id"],
            display_name=data["display_name"],
            settings=data.get("settings", {}),
            features=data.get("features", {}),
            limits=data.get("limits", {})
        )

class ConfigurableTenantManager:
    """Tenant manager with per-tenant configuration."""

    def __init__(self, database_url: str):
        self.database_url = database_url
        self.runners: Dict[str, cloaca.DefaultRunner] = {}
        self.configs: Dict[str, TenantConfig] = {}

    def register_tenant(self, config: TenantConfig):
        """Register a tenant with specific configuration."""
        if not self._is_valid_tenant_id(config.tenant_id):
            raise ValueError(f"Invalid tenant ID: {config.tenant_id}")

        self.configs[config.tenant_id] = config
        print(f"Registered tenant: {config.display_name} ({config.tenant_id})")

    def get_tenant_runner(self, tenant_id: str) -> cloaca.DefaultRunner:
        """Get runner for tenant, creating if necessary."""
        if tenant_id not in self.configs:
            raise ValueError(f"Tenant {tenant_id} not registered")

        if tenant_id not in self.runners:
            runner = cloaca.DefaultRunner.with_schema(self.database_url, tenant_id)
            self.runners[tenant_id] = runner

        return self.runners[tenant_id]

    def execute_with_config(self, tenant_id: str, workflow_name: str, context: cloaca.Context):
        """Execute workflow with tenant-specific configuration."""
        if tenant_id not in self.configs:
            raise ValueError(f"Tenant {tenant_id} not registered")

        config = self.configs[tenant_id]
        runner = self.get_tenant_runner(tenant_id)

        # Inject tenant configuration into context
        context.set("tenant_config", {
            "tenant_id": config.tenant_id,
            "display_name": config.display_name,
            "settings": config.settings,
            "features": config.features,
            "limits": config.limits
        })

        return runner.execute(workflow_name, context)

    def _is_valid_tenant_id(self, tenant_id: str) -> bool:
        """Validate tenant ID for PostgreSQL schema naming."""
        return tenant_id and all(c.isalnum() or c == '_' for c in tenant_id)

    def shutdown_all(self):
        """Shutdown all runners."""
        for runner in self.runners.values():
            runner.shutdown()
        self.runners.clear()

# Tenant-aware workflow
@cloaca.task(id="configurable_task")
def configurable_task(context):
    """Task that adapts behavior based on tenant configuration."""

    tenant_config = context.get("tenant_config", {})
    tenant_id = tenant_config.get("tenant_id", "unknown")
    features = tenant_config.get("features", {})
    limits = tenant_config.get("limits", {})
    settings = tenant_config.get("settings", {})

    print(f"Executing configurable task for {tenant_id}")

    # Adapt behavior based on tenant features
    processing_options = {
        "basic_processing": True,
        "advanced_analytics": features.get("advanced_analytics", False),
        "real_time_processing": features.get("real_time", False),
        "data_retention_days": settings.get("data_retention_days", 30)
    }

    # Respect tenant limits
    max_items = limits.get("max_items_per_workflow", 1000)
    input_data = context.get("input_data", [])

    if len(input_data) > max_items:
        limited_data = input_data[:max_items]
        context.set("data_limited", True)
        context.set("original_count", len(input_data))
        context.set("processed_count", len(limited_data))
        print(f"Limited processing to {max_items} items for {tenant_id}")
    else:
        limited_data = input_data
        context.set("data_limited", False)
        context.set("processed_count", len(limited_data))

    # Process with tenant-specific options
    result = {
        "tenant_id": tenant_id,
        "processing_options": processing_options,
        "items_processed": len(limited_data),
        "features_used": [k for k, v in features.items() if v],
        "completed_at": datetime.now().isoformat()
    }

    context.set("processing_result", result)
    return context

def create_configurable_workflow():
    builder = cloaca.WorkflowBuilder("configurable_tenant_workflow")
    builder.description("Workflow that adapts to tenant configuration")
    builder.add_task("configurable_task")
    return builder.build()

cloaca.register_workflow_constructor("configurable_tenant_workflow", create_configurable_workflow)

# Demonstration
def demonstrate_configurable_tenants():
    """Demonstrate tenant-specific configuration."""

    print("=== Configurable Multi-Tenancy ===")

    database_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina"
    manager = ConfigurableTenantManager(database_url)

    # Register tenants with different configurations
    tenants = [
        TenantConfig(
            tenant_id="premium_client",
            display_name="Premium Enterprise Client",
            settings={"data_retention_days": 365, "priority": "high"},
            features={"advanced_analytics": True, "real_time": True, "custom_reports": True},
            limits={"max_items_per_workflow": 10000, "max_concurrent_workflows": 50}
        ),
        TenantConfig(
            tenant_id="standard_client",
            display_name="Standard Business Client",
            settings={"data_retention_days": 90, "priority": "normal"},
            features={"advanced_analytics": False, "real_time": False, "custom_reports": True},
            limits={"max_items_per_workflow": 1000, "max_concurrent_workflows": 10}
        ),
        TenantConfig(
            tenant_id="basic_client",
            display_name="Basic Startup Client",
            settings={"data_retention_days": 30, "priority": "low"},
            features={"advanced_analytics": False, "real_time": False, "custom_reports": False},
            limits={"max_items_per_workflow": 100, "max_concurrent_workflows": 5}
        )
    ]

    # Register all tenants
    for tenant in tenants:
        manager.register_tenant(tenant)

    # Execute workflows with different data loads to test limits
    test_scenarios = [
        {"tenant": "premium_client", "data_size": 5000},
        {"tenant": "standard_client", "data_size": 1500},  # Will be limited
        {"tenant": "basic_client", "data_size": 200},      # Will be limited
    ]

    for scenario in test_scenarios:
        tenant_id = scenario["tenant"]
        data_size = scenario["data_size"]

        print(f"\n--- Testing {tenant_id} with {data_size} items ---")

        # Generate test data
        input_data = [{"id": i, "value": f"item_{i}"} for i in range(data_size)]

        context = cloaca.Context({
            "input_data": input_data,
            "scenario": scenario
        })

        try:
            result = manager.execute_with_config(
                tenant_id,
                "configurable_tenant_workflow",
                context
            )

            if result.status == "Completed":
                final_context = result.final_context
                processing_result = final_context.get("processing_result")

                print(f"✓ Processed {processing_result['items_processed']} items")
                print(f"  Features: {processing_result['features_used']}")

                if final_context.get("data_limited"):
                    original = final_context.get("original_count")
                    processed = final_context.get("processed_count")
                    print(f"  Limited: {original} → {processed} items")
            else:
                print(f"✗ Failed: {result.status}")

        except Exception as e:
            print(f"✗ Error: {e}")

    # Cleanup
    manager.shutdown_all()

if __name__ == "__main__":
    demonstrate_configurable_tenants()
```

## Database Admin API

For production SaaS applications, Cloacina provides a Database Admin API that handles tenant provisioning with dedicated database credentials and proper security isolation.

### Tenant Provisioning

The admin API creates tenants with their own database users and schemas:

```python
import cloaca

def provision_tenant_with_admin():
    """Demonstrate tenant provisioning using the admin API."""

    # Create database admin with administrative privileges
    admin_url = "postgresql://admin:admin_password@localhost:5432/myapp"
    admin = cloaca.DatabaseAdmin(admin_url)

    # Configure new tenant
    tenant_config = cloaca.TenantConfig(
        schema_name="tenant_acme_corp",
        username="acme_corp_user",
        password=""  # Auto-generate secure password
    )

    try:
        # Create tenant with dedicated credentials
        credentials = admin.create_tenant(tenant_config)

        print(f"✓ Tenant provisioned successfully!")
        print(f"  Schema: {credentials.schema_name}")
        print(f"  Username: {credentials.username}")
        print(f"  Password: {credentials.password}")
        print(f"  Connection: {credentials.connection_string}")

        # Create runner with tenant-specific credentials
        tenant_runner = cloaca.DefaultRunner(credentials.connection_string)

        return {
            "credentials": credentials,
            "runner": tenant_runner
        }

    except Exception as e:
        print(f"✗ Failed to provision tenant: {e}")
        return None

# Example usage
if __name__ == "__main__":
    result = provision_tenant_with_admin()
    if result:
        print("Tenant is ready for workflow execution!")
```

### SaaS Integration Pattern

```python
import cloaca
from typing import Dict, Optional

class SaaSApplicationManager:
    """Production-ready tenant management for SaaS applications."""

    def __init__(self, admin_database_url: str):
        self.admin = cloaca.DatabaseAdmin(admin_database_url)
        self.tenant_runners: Dict[str, cloaca.DefaultRunner] = {}
        self.tenant_credentials: Dict[str, cloaca.TenantCredentials] = {}

    def onboard_customer(self, customer_id: str, customer_name: str) -> Dict[str, str]:
        """Complete customer onboarding with dedicated tenant."""
        print(f"Onboarding customer: {customer_name} ({customer_id})")

        # Create tenant configuration
        config = cloaca.TenantConfig(
            schema_name=f"tenant_{customer_id}",
            username=f"{customer_id}_user",
            password=""  # Auto-generate secure password
        )

        try:
            # Provision tenant with admin API
            credentials = self.admin.create_tenant(config)
            self.tenant_credentials[customer_id] = credentials

            # Create dedicated runner for this customer
            runner = cloaca.DefaultRunner(credentials.connection_string)
            self.tenant_runners[customer_id] = runner

            print(f"✓ Customer {customer_name} onboarded successfully")
            print(f"  Dedicated schema: {credentials.schema_name}")
            print(f"  Database isolation: ✓")

            return {
                "customer_id": customer_id,
                "status": "active",
                "schema": credentials.schema_name,
                "database_user": credentials.username
            }

        except Exception as e:
            print(f"✗ Failed to onboard customer {customer_name}: {e}")
            return {
                "customer_id": customer_id,
                "status": "failed",
                "error": str(e)
            }

    def execute_for_customer(self, customer_id: str, workflow_name: str, context: cloaca.Context):
        """Execute workflow for specific customer with their isolated environment."""
        runner = self.tenant_runners.get(customer_id)
        if not runner:
            raise ValueError(f"Customer {customer_id} not found or not onboarded")

        # Add customer context
        context.set("customer_id", customer_id)
        context.set("tenant_schema", self.tenant_credentials[customer_id].schema_name)

        return runner.execute(workflow_name, context)

    def get_customer_runner(self, customer_id: str) -> Optional[cloaca.DefaultRunner]:
        """Get the dedicated runner for a customer."""
        return self.tenant_runners.get(customer_id)

# Example usage
def demonstrate_saas_pattern():
    admin_url = "postgresql://admin:admin@localhost:5432/saas_app"
    app_manager = SaaSApplicationManager(admin_url)

    # Onboard multiple customers
    customers = [
        ("acme_corp", "Acme Corporation"),
        ("globex_inc", "Globex Industries"),
        ("initech", "Initech Solutions")
    ]

    for customer_id, customer_name in customers:
        result = app_manager.onboard_customer(customer_id, customer_name)
        print(f"Onboarding result: {result}")

    print("\n🎉 All customers onboarded with dedicated database isolation!")

if __name__ == "__main__":
    demonstrate_saas_pattern()
```

### Security Benefits

The Database Admin API provides enhanced security:

{{< tabs "admin-security" >}}
{{< tab "User Isolation" >}}
**Each tenant gets a dedicated database user:**
- Cannot access other tenants' data
- Cannot modify other tenants' schemas
- Cannot perform administrative operations
- PostgreSQL enforces access controls
{{< /tab >}}

{{< tab "Password Security" >}}
**Secure password generation:**
- 32-character random passwords
- Alphanumeric characters only (URL-safe)
- Unique per tenant
- Not stored in application code
{{< /tab >}}

{{< tab "Audit Trail" >}}
**Database-level auditing:**
- PostgreSQL logs show which tenant user performed operations
- Connection tracking per tenant
- Schema-level operation logging
- No shared credentials
{{< /tab >}}
{{< /tabs >}}

## Security and Validation

### Schema Name Validation

```python
import re
from typing import List

def validate_tenant_id(tenant_id: str) -> tuple[bool, List[str]]:
    """Validate tenant ID for PostgreSQL schema usage."""

    errors = []

    # Check basic requirements
    if not tenant_id:
        errors.append("Tenant ID cannot be empty")
        return False, errors

    if len(tenant_id) > 63:  # PostgreSQL identifier limit
        errors.append("Tenant ID cannot exceed 63 characters")

    # Check character requirements
    if not re.match(r'^[a-zA-Z][a-zA-Z0-9_]*$', tenant_id):
        errors.append("Tenant ID must start with a letter and contain only alphanumeric characters and underscores")

    # Check for reserved words (basic list)
    reserved_words = ['public', 'information_schema', 'pg_catalog', 'pg_temp', 'pg_toast']
    if tenant_id.lower() in reserved_words:
        errors.append(f"Tenant ID '{tenant_id}' is a reserved PostgreSQL schema name")

    # Additional business rules
    if tenant_id.startswith('pg_'):
        errors.append("Tenant ID cannot start with 'pg_' (reserved for PostgreSQL)")

    if tenant_id.startswith('_'):
        errors.append("Tenant ID cannot start with underscore")

    return len(errors) == 0, errors

# Test validation
def test_tenant_validation():
    """Test tenant ID validation rules."""

    test_cases = [
        # Valid cases
        ("acme_corp", True),
        ("client123", True),
        ("tenant_a", True),

        # Invalid cases
        ("", False),                    # Empty
        ("123tenant", False),           # Starts with number
        ("tenant-name", False),         # Contains hyphen
        ("tenant name", False),         # Contains space
        ("public", False),              # Reserved word
        ("pg_temp", False),             # PostgreSQL reserved
        ("_tenant", False),             # Starts with underscore
    ]

    print("=== Tenant ID Validation Tests ===")

    for tenant_id, expected_valid in test_cases:
        is_valid, errors = validate_tenant_id(tenant_id)
        status = "✓" if is_valid == expected_valid else "✗"

        print(f"{status} '{tenant_id}': {'Valid' if is_valid else 'Invalid'}")
        if errors:
            for error in errors:
                print(f"    - {error}")

if __name__ == "__main__":
    test_tenant_validation()
```

## Recovery and Persistence

```python
import cloaca
from datetime import datetime, timedelta

def demonstrate_tenant_recovery():
    """Demonstrate automatic recovery for multi-tenant deployments."""

    print("=== Multi-Tenant Recovery Demonstration ===")

    database_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina"

    # Step 1: Create tenant and start a long-running workflow
    print("Step 1: Starting workflow for tenant...")

    @cloaca.task(id="long_running_task")
    def long_running_task(context):
        tenant_id = context.get("tenant_id")
        print(f"Long-running task started for {tenant_id}")

        # Simulate work that might be interrupted
        context.set("task_started", datetime.now().isoformat())
        context.set("progress", "50%")

        # This task would normally take a long time
        # In real scenarios, this might be interrupted by server restart
        return context

    def create_recovery_workflow():
        builder = cloaca.WorkflowBuilder("recovery_test_workflow")
        builder.description("Workflow for testing recovery")
        builder.add_task("long_running_task")
        return builder.build()

    cloaca.register_workflow_constructor("recovery_test_workflow", create_recovery_workflow)

    # First runner - simulate this gets interrupted
    tenant_id = "recovery_test_tenant"
    first_runner = cloaca.DefaultRunner.with_schema(database_url, tenant_id)

    context = cloaca.Context({
        "tenant_id": tenant_id,
        "session_id": "session_1",
        "started_at": datetime.now().isoformat()
    })

    result1 = first_runner.execute("recovery_test_workflow", context)
    print(f"First execution: {result1.status}")

    # Simulate interruption - shutdown runner
    first_runner.shutdown()
    print("First runner shutdown (simulating interruption)")

    # Step 2: Create new runner for same tenant - should recover state
    print("\nStep 2: Creating new runner for same tenant...")

    second_runner = cloaca.DefaultRunner.with_schema(database_url, tenant_id)

    # Any orphaned workflows would be automatically recovered
    # The schema persists, so all data remains available

    # Execute new workflow to verify tenant is working
    new_context = cloaca.Context({
        "tenant_id": tenant_id,
        "session_id": "session_2",
        "recovery_test": True,
        "started_at": datetime.now().isoformat()
    })

    result2 = second_runner.execute("recovery_test_workflow", new_context)
    print(f"Recovery execution: {result2.status}")

    if result2.status == "Completed":
        final_context = result2.final_context
        print(f"✓ Tenant {tenant_id} recovered successfully")
        print(f"  Session: {final_context.get('session_id')}")
        print(f"  Task started: {final_context.get('task_started')}")

    second_runner.shutdown()
    print("Recovery demonstration complete")

if __name__ == "__main__":
    demonstrate_tenant_recovery()
```

## Running Multi-Tenant Examples

### Complete Example

Save this as `multi_tenant_demo.py` and run it:

```python
import cloaca
import time
from datetime import datetime

def run_complete_demo():
    """Run the complete multi-tenancy demonstration."""

    print("=== Complete Multi-Tenancy Demo ===")

    # Check if we should run this demo
    try:
        database_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina"

        # Test basic multi-tenant setup
        print("\n1. Testing basic tenant isolation...")
        test_tenant_isolation()

        print("\n2. Demonstrating tenant manager...")
        demonstrate_tenant_manager()

        print("\n3. Testing configurable tenants...")
        demonstrate_configurable_tenants()

        print("\n4. Testing validation...")
        test_tenant_validation()

        print("\n5. Demonstrating recovery...")
        demonstrate_tenant_recovery()

        print("\n✓ All multi-tenancy demonstrations completed successfully!")

    except Exception as e:
        print(f"Demo failed: {e}")
        print("Make sure PostgreSQL is running and accessible:")
        print("  - PostgreSQL server running")
        print("  - Database 'cloacina' exists")
        print("  - User 'cloacina' has access")

if __name__ == "__main__":
    run_complete_demo()
```

To run:

```bash
# Install PostgreSQL extras for Python
pip install cloaca[postgres]

# Run the demo
python multi_tenant_demo.py
```

## Best Practices

{{< tabs "mt-best-practices" >}}
{{< tab "Tenant Naming" >}}
**Follow PostgreSQL schema naming conventions:**

```python
# Good tenant IDs
tenant_ids = [
    "acme_corp",
    "customer_123",
    "tenant_production",
    "client_alpha"
]

# Avoid
bad_tenant_ids = [
    "tenant-123",      # Hyphens not allowed
    "tenant 123",      # Spaces not allowed
    "123_tenant",      # Cannot start with number
    "tenant.name",     # Dots not allowed
    "public",          # Reserved schema name
]
```
{{< /tab >}}

{{< tab "Security" >}}
**Implement proper tenant isolation:**

```python
class SecureTenantManager:
    def __init__(self, database_url: str):
        self.database_url = database_url
        self.authorized_tenants = set()  # Authorized tenant list

    def authorize_tenant(self, tenant_id: str, authorization_token: str):
        """Authorize a tenant for access."""
        # Validate authorization token
        if self._validate_token(tenant_id, authorization_token):
            self.authorized_tenants.add(tenant_id)
        else:
            raise SecurityError(f"Invalid authorization for tenant {tenant_id}")

    def get_runner(self, tenant_id: str) -> cloaca.DefaultRunner:
        """Get runner only for authorized tenants."""
        if tenant_id not in self.authorized_tenants:
            raise SecurityError(f"Tenant {tenant_id} not authorized")

        return cloaca.DefaultRunner.with_schema(self.database_url, tenant_id)
```
{{< /tab >}}

{{< tab "Performance" >}}
**Optimize for multi-tenant performance:**

```python
from functools import lru_cache

class PerformantTenantManager:
    def __init__(self, database_url: str):
        self.database_url = database_url
        self.runner_cache = {}
        self.cache_timestamps = {}
        self.cache_ttl = 3600  # 1 hour

    @lru_cache(maxsize=100)
    def _create_runner(self, tenant_id: str):
        """Create runner with caching."""
        return cloaca.DefaultRunner.with_schema(self.database_url, tenant_id)

    def get_runner(self, tenant_id: str):
        """Get cached runner or create new one."""
        current_time = time.time()

        # Check if cached runner is still valid
        if (tenant_id in self.runner_cache and
            current_time - self.cache_timestamps.get(tenant_id, 0) < self.cache_ttl):
            return self.runner_cache[tenant_id]

        # Create new runner
        runner = self._create_runner(tenant_id)
        self.runner_cache[tenant_id] = runner
        self.cache_timestamps[tenant_id] = current_time

        return runner
```
{{< /tab >}}

{{< tab "Monitoring" >}}
**Monitor tenant usage and health:**

```python
import logging
from dataclasses import dataclass
from typing import Dict

@dataclass
class TenantMetrics:
    tenant_id: str
    workflows_executed: int
    total_execution_time: float
    last_activity: datetime
    error_count: int

class MonitoredTenantManager:
    def __init__(self, database_url: str):
        self.database_url = database_url
        self.metrics: Dict[str, TenantMetrics] = {}
        self.logger = logging.getLogger(__name__)

    def execute_for_tenant(self, tenant_id: str, workflow_name: str, context: cloaca.Context):
        """Execute workflow with monitoring."""
        start_time = time.time()

        try:
            runner = self.get_runner(tenant_id)
            result = runner.execute(workflow_name, context)

            # Update metrics
            execution_time = time.time() - start_time
            self._update_metrics(tenant_id, execution_time, success=True)

            self.logger.info(f"Workflow executed for {tenant_id}: {workflow_name} ({execution_time:.2f}s)")
            return result

        except Exception as e:
            self._update_metrics(tenant_id, 0, success=False)
            self.logger.error(f"Workflow failed for {tenant_id}: {e}")
            raise

    def _update_metrics(self, tenant_id: str, execution_time: float, success: bool):
        """Update tenant metrics."""
        if tenant_id not in self.metrics:
            self.metrics[tenant_id] = TenantMetrics(
                tenant_id=tenant_id,
                workflows_executed=0,
                total_execution_time=0.0,
                last_activity=datetime.now(),
                error_count=0
            )

        metrics = self.metrics[tenant_id]
        if success:
            metrics.workflows_executed += 1
            metrics.total_execution_time += execution_time
        else:
            metrics.error_count += 1

        metrics.last_activity = datetime.now()
```
{{< /tab >}}
{{< /tabs >}}

## Deployment Considerations

### Environment Configuration

```python
import os
from typing import Optional

class ProductionTenantManager:
    """Production-ready tenant manager with environment configuration."""

    def __init__(self):
        self.database_url = self._get_database_url()
        self.runners = {}

    def _get_database_url(self) -> str:
        """Get database URL from environment with fallbacks."""

        # Try different environment variable names
        env_vars = [
            'DATABASE_URL',
            'POSTGRESQL_URL',
            'CLOACINA_DATABASE_URL'
        ]

        for var in env_vars:
            url = os.getenv(var)
            if url:
                return url

        # Development fallback
        if os.getenv('ENVIRONMENT') == 'development':
            return "postgresql://cloacina:cloacina@localhost:5432/cloacina"

        raise ValueError("No database URL configured. Set DATABASE_URL environment variable.")

    def create_tenant_from_env(self, tenant_prefix: str) -> str:
        """Create tenant ID from environment and prefix."""

        environment = os.getenv('ENVIRONMENT', 'development')
        region = os.getenv('REGION', 'us_east_1')

        # Create environment-specific tenant ID
        tenant_id = f"{tenant_prefix}_{environment}_{region}"

        # Validate and sanitize
        tenant_id = tenant_id.replace('-', '_').lower()

        if not self._is_valid_tenant_id(tenant_id):
            raise ValueError(f"Generated tenant ID '{tenant_id}' is invalid")

        return tenant_id

# Example deployment configuration
if __name__ == "__main__":
    # Set environment variables (normally done by deployment system)
    os.environ['DATABASE_URL'] = 'postgresql://user:pass@db.example.com:5432/cloacina'
    os.environ['ENVIRONMENT'] = 'production'
    os.environ['REGION'] = 'us-west-2'

    manager = ProductionTenantManager()
    tenant_id = manager.create_tenant_from_env("client_acme")
    print(f"Created production tenant: {tenant_id}")
```

## What You've Learned

Congratulations! You've completed the comprehensive Cloacina Python tutorial series. You now understand:

- **Multi-tenant architecture** with PostgreSQL schema isolation
- **Tenant management patterns** for SaaS applications
- **Security and validation** for tenant data
- **Recovery and persistence** across tenant lifecycles
- **Performance optimization** for multi-tenant systems
- **Production deployment** considerations

## Next Steps

With this foundation, you're ready to:

{{< button href="/python-bindings/how-to-guides/" >}}Explore How-to Guides{{< /button >}}
{{< button href="/python-bindings/api-reference/" >}}Browse API Reference{{< /button >}}
{{< button href="/python-bindings/examples/" >}}See More Examples{{< /button >}}

### Recommended Learning Path

1. **[How-to: Backend Selection](/python-bindings/how-to-guides/backend-selection/)** - Choose between SQLite and PostgreSQL
2. **[How-to: Performance Optimization](/python-bindings/how-to-guides/performance-optimization/)** - Scale your workflows
3. **[Examples: Real-world Scenarios](/python-bindings/examples/real-world-scenarios/)** - Production patterns

## Related Resources

- [Explanation: Multi-tenancy](/explanation/multi-tenancy/) - Deep dive into architecture
- [How-to: Multi-tenant Setup](/how-to-guides/multi-tenant-setup/) - Detailed setup guide
- [How-to: Multi-tenant Recovery](/how-to-guides/multi-tenant-recovery/) - Recovery strategies

{{< hint type="info" title="Reference Implementation" >}}
This tutorial demonstrates patterns from the [`examples/multi_tenant/`](https://github.com/dstorey/cloacina/tree/main/examples/multi_tenant) and [`examples/per_tenant_credentials/`](https://github.com/dstorey/cloacina/tree/main/examples/per_tenant_credentials) directories in the Cloacina repository.
{{< /hint >}}
