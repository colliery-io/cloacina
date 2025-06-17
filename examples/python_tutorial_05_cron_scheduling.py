#!/usr/bin/env python3
"""
Cloaca Tutorial 05: Cron Scheduling
Complete example demonstrating time-based workflow execution.
"""

import cloaca
from datetime import datetime
import time

# Create all workflow definitions using workflow-scoped pattern
# Daily report workflow
with cloaca.WorkflowBuilder("daily_report") as builder:
    builder.description("Daily business analytics")

    # Tasks are automatically registered when defined within WorkflowBuilder context
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

        print(f"Daily Report Generated at {current_time}")
        print(f"   Orders: {report_data['total_orders']}")
        print(f"   Revenue: ${report_data['revenue']}")
        print(f"   Users: {report_data['active_users']}")

        context.set("report_data", report_data)
        return context

# Backup workflow
with cloaca.WorkflowBuilder("system_backup") as builder:
    builder.description("System data backup")

    @cloaca.task(id="system_backup")
    def system_backup(context):
        """Perform system backup."""
        backup_type = context.get("backup_type", "incremental")
        timestamp = datetime.now()

        print(f"{backup_type.title()} backup at {timestamp}")

        context.set("backup_completed", timestamp.isoformat())
        context.set("backup_type", backup_type)
        return context

# Cleanup workflow
with cloaca.WorkflowBuilder("data_cleanup") as builder:
    builder.description("Data cleanup and maintenance")

    @cloaca.task(id="data_cleanup")
    def data_cleanup(context):
        """Clean up old data."""
        retention_days = context.get("retention_days", 30)
        timestamp = datetime.now()

        print(f"Cleaning data older than {retention_days} days at {timestamp}")

        # Simulate cleanup
        files_removed = 47
        space_freed = "1.2GB"

        context.set("cleanup_completed_at", timestamp.isoformat())
        context.set("files_removed", files_removed)
        context.set("space_freed", space_freed)

        return context

# Health check workflow
with cloaca.WorkflowBuilder("health_check") as builder:
    builder.description("System health monitoring")

    @cloaca.task(id="health_check")
    def health_check(context):
        """Perform system health check."""
        timestamp = datetime.now()

        print(f"System health check at {timestamp}")

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

def get_workflow_names():
    """Get all registered workflow names."""
    # With the workflow-scoped pattern, workflows are already registered
    # when defined within WorkflowBuilder context
    return ["daily_report", "system_backup", "data_cleanup", "health_check"]

def cron_demo():
    """Demonstrate advanced cron scheduling patterns."""
    print("\n=== Advanced Multi-Schedule Demo ===")

    # Create runner
    runner = cloaca.DefaultRunner(":memory:")

    try:
        # Workflows are already registered with the workflow-scoped pattern
        workflow_names = get_workflow_names()

        # Production-style schedules (commented) with demo equivalents
        # production_schedules = [
        #     ("0 9 * * *", "Daily report at 9 AM"),
        #     ("0 2 * * SUN", "Weekly backup on Sunday at 2 AM"),
        #     ("0 1 * * SAT", "Weekly cleanup on Saturday at 1 AM"),
        #     ("*/15 9-17 * * MON-FRI", "Health check every 15 min during business hours"),
        # ]

        # Demo schedule configurations - simplified for quick demo
        demo_schedules = [
            ("daily_report", "*/8 * * * * *"),       # Every 8 seconds
            ("health_check", "*/5 * * * * *")        # Every 5 seconds
        ]

        print("Production Schedule Patterns:")
        production_patterns = [
            ("daily_report", "0 9 * * *", "Daily at 9:00 AM"),
            ("system_backup", "0 2 * * SUN", "Weekly on Sunday at 2:00 AM"),
            ("data_cleanup", "0 1 * * SAT", "Weekly on Saturday at 1:00 AM"),
            ("health_check", "*/15 9-17 * * MON-FRI", "Every 15 min, business hours")
        ]

        for workflow, expression, description in production_patterns:
            print(f"   {workflow}: {expression} ({description})")

        print("\nStarting demo schedules...")

        # Register demo schedules using runner's cron functionality
        schedule_ids = []
        for workflow_name, cron_expression in demo_schedules:
            schedule_id = runner.register_cron_workflow(workflow_name, cron_expression, "UTC")
            schedule_ids.append(schedule_id)
            print(f"✓ {workflow_name}: {cron_expression} (ID: {schedule_id})")

        print("\nRunning multiple schedules for 30 seconds...")
        print("   (Different workflows will execute at different intervals)")
        print("   Watch for automatic cron executions below...")
        time.sleep(30)

    finally:
        runner.shutdown()

def main():
    """Main tutorial demonstration."""
    print("Cloaca Cron Scheduling Tutorial")
    print("=" * 50)

    # Run advanced demo
    cron_demo()

    print("\nTutorial completed successfully!")
    print("\nWhat you learned:")
    print("- Creating cron schedules with expressions")
    print("- Registering schedules with workflows")
    print("- Configuring timezone and context")
    print("- Managing multiple concurrent schedules")
    print("- Production scheduling patterns")

    print("\nNext Steps:")
    print("- Explore the API reference for advanced scheduling options")
    print("- Check out real-world examples in the examples directory")
    print("- Learn about production deployment considerations")

if __name__ == "__main__":
    main()
