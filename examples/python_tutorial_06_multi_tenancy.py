#!/usr/bin/env python3
"""
Cloacina Python Tutorial 06: Multi-Tenancy

This example demonstrates deploying isolated workflows for multiple tenants
using PostgreSQL schema-based multi-tenancy, automated tenant provisioning,
security considerations, and scaling strategies.

Learning objectives:
- Understand schema-based multi-tenancy architecture
- Use admin API for automated tenant provisioning
- Implement tenant-specific workflow runners with dedicated credentials
- Manage complete tenant isolation and security
- Handle tenant lifecycle and recovery
- Design scalable multi-tenant systems
- Apply best practices for SaaS deployments

Usage:
    python python_tutorial_06_multi_tenancy.py

Prerequisites:
    pip install cloaca[postgres]
    PostgreSQL database running (see docker-compose.yaml in project root)
"""

import sys
import cloaca
import random
from datetime import datetime
from typing import Dict, Optional

# Multi-tenant workflow definitions using workflow-scoped pattern
# Create tenant onboarding workflow
with cloaca.WorkflowBuilder("tenant_onboarding") as builder:
    builder.description("New tenant onboarding workflow")
    
    # Tasks are automatically registered when defined within WorkflowBuilder context
    @cloaca.task(id="tenant_onboarding")
    def tenant_onboarding(context):
        """Handle new tenant onboarding workflow."""
        print("Starting tenant onboarding...")

        tenant_info = context.get("tenant_info", {})
        tenant_id = tenant_info.get("tenant_id", "unknown")

        # Simulate onboarding steps
        onboarding_steps = [
            "account_creation",
            "initial_configuration",
            "data_migration",
            "user_setup",
            "integration_configuration"
        ]

        completed_steps = []
        for step in onboarding_steps:
            # Simulate step execution (would be real operations in production)
            print(f"  Executing step: {step}")
            completed_steps.append({
                "step": step,
                "completed_at": datetime.now().isoformat(),
                "status": "completed"
            })

        onboarding_result = {
            "tenant_id": tenant_id,
            "onboarding_started": context.get("onboarding_started", datetime.now().isoformat()),
            "onboarding_completed": datetime.now().isoformat(),
            "steps_completed": completed_steps,
            "total_steps": len(onboarding_steps),
            "status": "completed"
        }

        context.set("onboarding_result", onboarding_result)
        print(f"Tenant {tenant_id} onboarding completed")
        return context

# Create tenant data processing workflow
with cloaca.WorkflowBuilder("tenant_data_processing") as builder:
    builder.description("Tenant-specific data processing and reporting")
    
    @cloaca.task(id="process_tenant_data")
    def process_tenant_data(context):
        """Process tenant-specific data."""
        print("Processing tenant data...")

        tenant_info = context.get("tenant_info", {})
        tenant_id = tenant_info.get("tenant_id", "unknown")

        # Simulate tenant data processing
        data_volume = random.randint(100, 1000)
        processing_time = data_volume * 0.01  # Simulate processing time

        processed_data = {
            "tenant_id": tenant_id,
            "records_processed": data_volume,
            "processing_time_seconds": processing_time,
            "processed_at": datetime.now().isoformat(),
            "data_categories": {
                "user_data": random.randint(10, 100),
                "transaction_data": random.randint(50, 500),
                "configuration_data": random.randint(5, 50)
            }
        }

        context.set("processed_data", processed_data)
        print(f"Processed {data_volume} records for tenant {tenant_id}")
        return context

    @cloaca.task(id="generate_tenant_report", dependencies=["process_tenant_data"])
    def generate_tenant_report(context):
        """Generate tenant-specific analytics report."""
        print("Generating tenant report...")

        tenant_info = context.get("tenant_info", {})
        processed_data = context.get("processed_data", {})
        tenant_id = tenant_info.get("tenant_id", "unknown")

        # Generate tenant-specific insights
        total_records = processed_data.get("records_processed", 0)
        categories = processed_data.get("data_categories", {})

        analytics = {
            "summary": {
                "tenant_id": tenant_id,
                "total_records": total_records,
                "processing_efficiency": random.uniform(0.85, 0.98),
                "data_quality_score": random.uniform(0.90, 1.0)
            },
            "breakdown": categories,
            "insights": [],
            "recommendations": [],
            "generated_at": datetime.now().isoformat()
        }

        # Generate insights
        insights = []
        if categories.get("transaction_data", 0) > 300:
            insights.append("High transaction volume detected - consider premium features")
        if categories.get("user_data", 0) > 80:
            insights.append("Growing user base - monitor performance metrics")

        # Generate recommendations
        recommendations = []
        if total_records > 800:
            recommendations.append("Consider upgrading to higher performance tier")
        if analytics["summary"]["data_quality_score"] < 0.95:
            recommendations.append("Review data validation processes")

        analytics["insights"] = insights
        analytics["recommendations"] = recommendations

        tenant_report = {
            "tenant_id": tenant_id,
            "report_type": "analytics",
            "analytics": analytics,
            "report_generated_at": datetime.now().isoformat()
        }

        context.set("tenant_report", tenant_report)
        print(f"Report generated for tenant {tenant_id}")
        return context

# Tenant Management System
class TenantManager:
    """Manages multi-tenant workflow execution."""

    def __init__(self, admin_postgres_url: str):
        """Initialize with PostgreSQL admin connection URL."""
        self.admin_postgres_url = admin_postgres_url
        self.tenant_runners: Dict[str, cloaca.DefaultRunner] = {}
        self.tenant_credentials: Dict[str, cloaca.TenantCredentials] = {}

        # Initialize database admin for tenant provisioning
        self.db_admin = cloaca.DatabaseAdmin(admin_postgres_url)

    def provision_tenant(self, tenant_id: str) -> cloaca.TenantCredentials:
        """Provision a new tenant with dedicated schema and credentials."""
        print(f"Provisioning tenant: {tenant_id}")

        # Create tenant configuration
        tenant_config = cloaca.TenantConfig(
            schema_name=f"tenant_{tenant_id}",
            username=f"{tenant_id}_user",
            password=""  # Auto-generate secure password
        )

        # Create tenant using admin API
        credentials = self.db_admin.create_tenant(tenant_config)
        self.tenant_credentials[tenant_id] = credentials

        print(f"Tenant {tenant_id} provisioned:")
        print(f"  Schema: {credentials.schema_name}")
        print(f"  Username: {credentials.username}")
        print(f"  Password: {credentials.password}")
        print(f"  Connection string: {credentials.connection_string}")
        print("  Dedicated database connection ready")

        return credentials

    def create_tenant_runner(self, tenant_id: str) -> cloaca.DefaultRunner:
        """Create a tenant-specific runner with schema isolation."""
        print(f"Creating runner for tenant: {tenant_id}")

        # Get or create tenant credentials
        if tenant_id not in self.tenant_credentials:
            credentials = self.provision_tenant(tenant_id)
        else:
            credentials = self.tenant_credentials[tenant_id]

        # Create runner with schema-based isolation
        # This uses the admin connection with tenant-specific schema
        runner = cloaca.DefaultRunner.with_schema(self.admin_postgres_url, credentials.schema_name)
        self.tenant_runners[tenant_id] = runner

        print(f"Runner created for tenant {tenant_id} with dedicated credentials")
        return runner

    def get_tenant_runner(self, tenant_id: str) -> Optional[cloaca.DefaultRunner]:
        """Get existing runner for tenant."""
        return self.tenant_runners.get(tenant_id)

    def execute_for_tenant(self, tenant_id: str, workflow_name: str, context: cloaca.Context):
        """Execute workflow for specific tenant."""
        runner = self.get_tenant_runner(tenant_id)
        if not runner:
            runner = self.create_tenant_runner(tenant_id)

        print(f"Executing {workflow_name} for tenant {tenant_id}")
        return runner.execute(workflow_name, context)

    def onboard_new_tenant(self, tenant_id: str, tenant_info: Dict) -> Dict:
        """Complete onboarding workflow for new tenant."""
        print(f"Starting onboarding for tenant: {tenant_id}")

        # Create context with tenant information
        context = cloaca.Context({
            "tenant_info": {
                "tenant_id": tenant_id,
                **tenant_info
            },
            "onboarding_started": datetime.now().isoformat()
        })

        # Execute onboarding workflow
        result = self.execute_for_tenant(tenant_id, "tenant_onboarding", context)

        if result.status == "Completed":
            onboarding_result = result.final_context.get("onboarding_result", {})
            print(f"Tenant {tenant_id} onboarded successfully")
            return {
                "status": "success",
                "tenant_id": tenant_id,
                "onboarding_result": onboarding_result
            }
        else:
            print(f"Tenant {tenant_id} onboarding failed: {result.status}")
            return {
                "status": "failed",
                "tenant_id": tenant_id,
                "error": getattr(result, 'error', 'Unknown error')
            }

    def process_tenant_data(self, tenant_id: str) -> Dict:
        """Process data for specific tenant."""
        print(f"Processing data for tenant: {tenant_id}")

        # Create context with tenant information
        context = cloaca.Context({
            "tenant_info": {
                "tenant_id": tenant_id
            },
            "processing_started": datetime.now().isoformat()
        })

        # Execute data processing workflow
        result = self.execute_for_tenant(tenant_id, "tenant_data_processing", context)

        if result.status == "Completed":
            final_context = result.final_context
            return {
                "status": "success",
                "tenant_id": tenant_id,
                "processed_data": final_context.get("processed_data", {}),
                "tenant_report": final_context.get("tenant_report", {})
            }
        else:
            return {
                "status": "failed",
                "tenant_id": tenant_id,
                "error": getattr(result, 'error', 'Unknown error')
            }

    def remove_tenant(self, tenant_id: str):
        """Remove tenant completely including schema and credentials."""
        print(f"Removing tenant: {tenant_id}")

        # Cleanup runner first
        if tenant_id in self.tenant_runners:
            runner = self.tenant_runners[tenant_id]
            runner.shutdown()
            del self.tenant_runners[tenant_id]

        # Remove tenant credentials and schema
        if tenant_id in self.tenant_credentials:
            credentials = self.tenant_credentials[tenant_id]
            self.db_admin.remove_tenant(credentials.schema_name, credentials.username)
            del self.tenant_credentials[tenant_id]
            print(f"Tenant {tenant_id} removed completely")

    def cleanup_tenant_resources(self, tenant_id: str):
        """Clean up runtime resources for tenant (keeps schema)."""
        if tenant_id in self.tenant_runners:
            print(f"Cleaning up runtime resources for tenant: {tenant_id}")
            runner = self.tenant_runners[tenant_id]
            runner.shutdown()
            del self.tenant_runners[tenant_id]

    def shutdown_all(self):
        """Shutdown all tenant runners."""
        print("Shutting down all tenant runners...")
        for tenant_id, runner in self.tenant_runners.items():
            print(f"  Shutting down runner for tenant: {tenant_id}")
            runner.shutdown()
        self.tenant_runners.clear()

# Simulation function to demonstrate multi-tenancy
def simulate_multi_tenant_operations():
    """Simulate multi-tenant SaaS operations."""
    print("=== Multi-Tenant SaaS Simulation ===")

    # PostgreSQL admin connection URL (modify as needed for your setup)
    admin_postgres_url = "postgresql://cloacina:cloacina@localhost:5432/cloacina"

    # Create tenant manager with admin credentials
    tenant_manager = TenantManager(admin_postgres_url)

    # Define multiple tenants
    tenants = [
        {
            "tenant_id": "acme_corp",
            "company_name": "Acme Corporation",
            "industry": "Technology",
            "size": "enterprise"
        },
        {
            "tenant_id": "beta_inc",
            "company_name": "Beta Inc",
            "industry": "Healthcare",
            "size": "medium"
        },
        {
            "tenant_id": "gamma_ltd",
            "company_name": "Gamma Ltd",
            "industry": "Finance",
            "size": "small"
        }
    ]

    print(f"Managing {len(tenants)} tenants with schema isolation")
    print()

    # Phase 1: Onboard all tenants
    print("Phase 1: Tenant Onboarding")
    print("-" * 30)

    onboarding_results = []
    for tenant in tenants:
        result = tenant_manager.onboard_new_tenant(
            tenant["tenant_id"],
            tenant
        )
        onboarding_results.append(result)
        print()

    # Phase 2: Process data for each tenant
    print("Phase 2: Tenant Data Processing")
    print("-" * 35)

    processing_results = []
    for tenant in tenants:
        result = tenant_manager.process_tenant_data(tenant["tenant_id"])
        processing_results.append(result)
        print()

    # Phase 3: Display results summary
    print("Phase 3: Multi-Tenant Results Summary")
    print("-" * 40)

    successful_onboarding = len([r for r in onboarding_results if r["status"] == "success"])
    successful_processing = len([r for r in processing_results if r["status"] == "success"])

    print(f"Onboarding Results: {successful_onboarding}/{len(tenants)} successful")
    print(f"Processing Results: {successful_processing}/{len(tenants)} successful")
    print()

    # Show tenant-specific results
    for i, tenant in enumerate(tenants):
        tenant_id = tenant["tenant_id"]
        processing_result = processing_results[i]

        print(f"Tenant: {tenant['company_name']} ({tenant_id})")

        if processing_result["status"] == "success":
            processed_data = processing_result["processed_data"]
            tenant_report = processing_result["tenant_report"]

            print(f"  Records processed: {processed_data.get('records_processed', 0)}")
            print(f"  Processing time: {processed_data.get('processing_time_seconds', 0):.2f}s")

            analytics = tenant_report.get("analytics", {})
            summary = analytics.get("summary", {})
            print(f"  Data quality score: {summary.get('data_quality_score', 0):.1%}")
            print(f"  Processing efficiency: {summary.get('processing_efficiency', 0):.1%}")

            insights = analytics.get("insights", [])
            if insights:
                print(f"  Key insights: {len(insights)} generated")
                for insight in insights[:2]:  # Show first 2 insights
                    print(f"    • {insight}")
        else:
            print(f"  Status: Failed - {processing_result.get('error', 'Unknown error')}")

        print()

    # Cleanup
    print("Cleaning up tenant resources...")
    tenant_manager.shutdown_all()

    return {
        "tenants_processed": len(tenants),
        "onboarding_success_rate": successful_onboarding / len(tenants),
        "processing_success_rate": successful_processing / len(tenants),
        "onboarding_results": onboarding_results,
        "processing_results": processing_results
    }

# Main execution
if __name__ == "__main__":
    print("=== Cloacina Python Tutorial 06: Multi-Tenancy ===")
    print()
    print("This tutorial demonstrates:")
    print("- Schema-based multi-tenancy with PostgreSQL")
    print("- Tenant-specific workflow runners")
    print("- Complete data isolation between tenants")
    print("- Tenant lifecycle management")
    print("- Scalable SaaS architecture patterns")
    print()

    try:
        # Run the multi-tenant simulation
        results = simulate_multi_tenant_operations()

        print("=== Tutorial Completed Successfully ===")
        print(f"Tenants processed: {results['tenants_processed']}")
        print(f"Onboarding success rate: {results['onboarding_success_rate']:.1%}")
        print(f"Processing success rate: {results['processing_success_rate']:.1%}")

    except Exception as e:
        print(f"Tutorial failed: {e}")
        print()
        print("Common issues:")
        print("- PostgreSQL not running (try: docker-compose up -d)")
        print("- Wrong connection URL (check postgres_url variable)")
        print("- Missing cloaca[postgres] installation")
        print("- Database permissions issues")
        sys.exit(1)

    print()
    print("Key concepts demonstrated:")
    print("- PostgreSQL schema-based tenant isolation")
    print("- Automated tenant provisioning with admin API")
    print("- Dedicated tenant credentials and database users")
    print("- Tenant-specific DefaultRunner instances")
    print("- Complete schema and user lifecycle management")
    print("- Independent workflow execution per tenant")
    print("- Resource cleanup and tenant removal")
    print()
    print("Multi-tenancy benefits:")
    print("- Complete data isolation (no cross-tenant access possible)")
    print("- Dedicated database credentials per tenant")
    print("- Native PostgreSQL performance (no application filtering)")
    print("- Enterprise-grade security boundaries")
    print("- Zero code changes required for existing workflows")
    print("- Automated tenant provisioning and removal")
    print("- Easy scaling and tenant management")
    print()
    print("Next steps:")
    print("- Deploy to production with proper PostgreSQL setup")
    print("- Configure monitoring and analytics per tenant")
    print("- Implement tenant-specific backup and recovery")
    print("- Add tenant usage tracking and billing")
    print("- Explore advanced multi-tenant patterns")
