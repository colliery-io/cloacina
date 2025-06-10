#!/usr/bin/env python3
"""
Cloaca Tutorial 05: Cron Scheduling
Complete example demonstrating time-based workflow execution.
"""

import cloaca
from datetime import datetime
import time
import os

# Task definitions
@cloaca.task(id="daily_report")
def daily_report(context):
    """Generate daily business report."""
    current_time = datetime.now()
    report_data = {
        "generated_at": current_time.isoformat(),
        "total_orders": 150,
        "revenue": 12500.50,
        "active_users": 89
    }

    print(f"📊 Daily Report Generated at {current_time}")
    print(f"   Orders: {report_data['total_orders']}")
    print(f"   Revenue: ${report_data['revenue']}")
    print(f"   Users: {report_data['active_users']}")

    context.set("report_data", report_data)
    return context

@cloaca.task(id="system_backup")
def system_backup(context):
    """Perform system backup."""
    backup_type = context.get("backup_type", "incremental")
    timestamp = datetime.now()

    print(f"💾 {backup_type.title()} backup at {timestamp}")

    context.set("backup_completed", timestamp.isoformat())
    context.set("backup_type", backup_type)
    return context

@cloaca.task(id="data_cleanup")
def data_cleanup(context):
    """Clean up old data."""
    retention_days = context.get("retention_days", 30)
    timestamp = datetime.now()

    print(f"🧹 Cleaning data older than {retention_days} days at {timestamp}")

    # Simulate cleanup
    files_removed = 47
    space_freed = "1.2GB"

    context.set("cleanup_completed_at", timestamp.isoformat())
    context.set("files_removed", files_removed)
    context.set("space_freed", space_freed)

    return context

@cloaca.task(id="health_check")
def health_check(context):
    """Perform system health check."""
    timestamp = datetime.now()

    print(f"🏥 System health check at {timestamp}")

    # Simulate health monitoring
    health_status = {
        "cpu_usage": 45.2,
        "memory_usage": 62.8,
        "disk_usage": 73.1,
        "active_connections": 234,
        "response_time_ms": 145
    }

    # Determine overall health
    if health_status["cpu_usage"] > 80 or health_status["memory_usage"] > 90:
        overall_status = "warning"
    elif health_status["cpu_usage"] > 95 or health_status["memory_usage"] > 95:
        overall_status = "critical"
    else:
        overall_status = "healthy"

    print(f"   Status: {overall_status.upper()}")
    print(f"   CPU: {health_status['cpu_usage']}%")
    print(f"   Memory: {health_status['memory_usage']}%")

    context.set("health_check_at", timestamp.isoformat())
    context.set("health_status", health_status)
    context.set("overall_status", overall_status)

    return context

def create_workflows():
    """Create all workflow definitions."""

    # Daily report workflow
    report_builder = cloaca.WorkflowBuilder("daily_report")
    report_builder.description("Daily business analytics")
    report_builder.add_task("daily_report")

    # Backup workflow
    backup_builder = cloaca.WorkflowBuilder("system_backup")
    backup_builder.description("System data backup")
    backup_builder.add_task("system_backup")

    # Cleanup workflow
    cleanup_builder = cloaca.WorkflowBuilder("data_cleanup")
    cleanup_builder.description("Data cleanup and maintenance")
    cleanup_builder.add_task("data_cleanup")

    # Health check workflow
    health_builder = cloaca.WorkflowBuilder("health_check")
    health_builder.description("System health monitoring")
    health_builder.add_task("health_check")

    return {
        "daily_report": report_builder.build(),
        "system_backup": backup_builder.build(),
        "data_cleanup": cleanup_builder.build(),
        "health_check": health_builder.build()
    }

def basic_cron_demo():
    """Demonstrate basic cron scheduling."""
    print("=== Basic Cron Scheduling Demo ===")

    # Create runner
    runner = cloaca.DefaultRunner(":memory:")

    try:
        # Register workflows
        workflows = create_workflows()
        for name, workflow in workflows.items():
            cloaca.register_workflow_constructor(name, lambda w=workflow: w)

        # Create basic schedules (demo frequencies)
        schedules = [
            # Daily report every 20 seconds (demo)
            cloaca.CronSchedule(
                workflow_name="daily_report",
                cron_expression="*/20 * * * * *",
                timezone="UTC",
                enabled=True,
                context=cloaca.Context({})
            ),

            # Health check every 15 seconds (demo)
            cloaca.CronSchedule(
                workflow_name="health_check",
                cron_expression="*/15 * * * * *",
                timezone="UTC",
                enabled=True,
                context=cloaca.Context({})
            )
        ]

        # Register schedules
        for schedule in schedules:
            runner.add_cron_schedule(schedule)
            print(f"✓ Scheduled: {schedule.workflow_name} - {schedule.cron_expression}")

        print("\n⏰ Running schedules for 45 seconds...")
        time.sleep(45)

    finally:
        runner.shutdown()

def advanced_cron_demo():
    """Demonstrate advanced cron scheduling patterns."""
    print("\n=== Advanced Multi-Schedule Demo ===")

    # Create runner
    runner = cloaca.DefaultRunner(":memory:")

    try:
        # Register workflows
        workflows = create_workflows()
        for name, workflow in workflows.items():
            cloaca.register_workflow_constructor(name, lambda w=workflow: w)

        # Production-style schedules (commented) with demo equivalents
        # production_schedules = [
        #     ("0 9 * * *", "Daily report at 9 AM"),
        #     ("0 2 * * SUN", "Weekly backup on Sunday at 2 AM"),
        #     ("0 1 * * SAT", "Weekly cleanup on Saturday at 1 AM"),
        #     ("*/15 9-17 * * MON-FRI", "Health check every 15 min during business hours"),
        # ]

        demo_schedules = [
            cloaca.CronSchedule(
                workflow_name="daily_report",
                cron_expression="*/25 * * * * *",  # Every 25 seconds
                timezone="UTC",
                enabled=True,
                context=cloaca.Context({})
            ),
            cloaca.CronSchedule(
                workflow_name="system_backup",
                cron_expression="*/30 * * * * *",  # Every 30 seconds
                timezone="UTC",
                enabled=True,
                context=cloaca.Context({"backup_type": "incremental"})
            ),
            cloaca.CronSchedule(
                workflow_name="data_cleanup",
                cron_expression="*/35 * * * * *",  # Every 35 seconds
                timezone="UTC",
                enabled=True,
                context=cloaca.Context({"retention_days": 30})
            ),
            cloaca.CronSchedule(
                workflow_name="health_check",
                cron_expression="*/10 * * * * *",  # Every 10 seconds
                timezone="UTC",
                enabled=True,
                context=cloaca.Context({})
            )
        ]

        print("📅 Production Schedule Patterns:")
        production_patterns = [
            ("daily_report", "0 9 * * *", "Daily at 9:00 AM"),
            ("system_backup", "0 2 * * SUN", "Weekly on Sunday at 2:00 AM"),
            ("data_cleanup", "0 1 * * SAT", "Weekly on Saturday at 1:00 AM"),
            ("health_check", "*/15 9-17 * * MON-FRI", "Every 15 min, business hours")
        ]

        for workflow, expression, description in production_patterns:
            print(f"   {workflow}: {expression} ({description})")

        print("\n🚀 Starting demo schedules...")

        # Register demo schedules
        for schedule in demo_schedules:
            runner.add_cron_schedule(schedule)
            print(f"✓ {schedule.workflow_name}: {schedule.cron_expression}")

        print("\n⏰ Running multiple schedules for 60 seconds...")
        print("   (Different workflows will execute at different intervals)")
        time.sleep(60)

    finally:
        runner.shutdown()

def main():
    """Main tutorial demonstration."""
    print("🕐 Cloaca Cron Scheduling Tutorial")
    print("=" * 50)

    # Run basic demo
    basic_cron_demo()

    # Run advanced demo
    advanced_cron_demo()

    print("\n✅ Tutorial completed successfully!")
    print("\nWhat you learned:")
    print("- Creating cron schedules with expressions")
    print("- Registering schedules with workflows")
    print("- Configuring timezone and context")
    print("- Managing multiple concurrent schedules")
    print("- Production scheduling patterns")

    print("\n🎓 Next Steps:")
    print("- Explore the API reference for advanced scheduling options")
    print("- Check out real-world examples in the examples directory")
    print("- Learn about production deployment considerations")

if __name__ == "__main__":
    main()
