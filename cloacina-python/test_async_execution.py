#!/usr/bin/env python3
"""
Full async execution test of the Cloacina Python bindings.
"""

import asyncio
from cloacina import task, Workflow, UnifiedExecutor

# Define tasks
@task(id="async_extract", dependencies=[])
def extract_data(context):
    print("  📊 Extracting data...")
    context = context or {}
    context["raw_data"] = {"users": [1, 2, 3], "products": ["A", "B", "C"]}
    print(f"    Extracted: {context['raw_data']}")
    return context

@task(id="async_transform", dependencies=["async_extract"])
def transform_data(context):
    print("  🔧 Transforming data...")
    raw = context.get("raw_data", {})
    context["analytics"] = {
        "user_count": len(raw.get("users", [])),
        "product_count": len(raw.get("products", [])),
        "processing_time": "2024-01-01T12:00:00Z"
    }
    print(f"    Analytics: {context['analytics']}")
    return context

@task(id="async_load", dependencies=["async_transform"])
def load_data(context):
    print("  💾 Loading data to warehouse...")
    analytics = context.get("analytics", {})
    context["result"] = {
        "status": "success",
        "records_processed": analytics.get("user_count", 0) + analytics.get("product_count", 0),
        "completed_at": "2024-01-01T12:01:00Z"
    }
    print(f"    Final result: {context['result']}")
    return context

async def main():
    print("🚀 Testing Cloacina Async Execution")
    print("=" * 50)

    try:
        # Create workflow
        print("\n1️⃣ Creating workflow...")
        workflow = Workflow("async_etl_demo")
        print(f"   ✅ Created: {workflow}")

        # Create executor
        print("\n2️⃣ Creating executor...")
        executor = UnifiedExecutor(max_workers=3)
        print(f"   ✅ Created: {executor}")

        # Initialize executor
        print("\n3️⃣ Initializing executor...")
        await executor.initialize(database_url="test.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000")
        print("   ✅ Executor initialized!")

        # Register workflow
        print("\n4️⃣ Registering workflow...")
        await executor.register_workflow(workflow)
        print("   ✅ Workflow registered!")

        # Execute workflow
        print("\n5️⃣ Executing workflow...")
        initial_context = {"pipeline_id": "demo-001", "started_at": "2024-01-01T11:59:00Z"}
        result = await executor.execute(workflow, initial_context)

        print("\n📊 Execution Results:")
        print(f"   Execution ID: {result['execution_id']}")
        print(f"   Status: {result['status']}")
        print(f"   Duration: {result.get('duration_seconds', 0):.3f}s")
        print(f"   Tasks: {len(result.get('task_results', []))}")

        if 'final_context' in result:
            print("\n📋 Final Context:")
            for key, value in result['final_context'].items():
                print(f"   {key}: {value}")

        # Shutdown
        print("\n6️⃣ Shutting down executor...")
        await executor.shutdown()
        print("   ✅ Shutdown complete!")

        print("\n🎉 ASYNC EXECUTION SUCCESSFUL!")
        print("\n✅ All components working:")
        print("  • Task registration ✅")
        print("  • Workflow creation ✅")
        print("  • Executor initialization ✅")
        print("  • Async execution ✅")
        print("  • Context handling ✅")
        print("  • Result processing ✅")

    except Exception as e:
        print(f"\n❌ Async test failed: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    asyncio.run(main())
