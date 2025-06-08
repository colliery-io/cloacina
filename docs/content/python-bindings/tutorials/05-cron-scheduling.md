---
title: "05 - Cron Scheduling"
description: "Schedule automated workflow execution with time-based triggers"
weight: 15
reviewer: "automation"
review_date: "2025-06-08"
---

# Cron Scheduling

Welcome to the cron scheduling tutorial! In this tutorial, you'll learn how to create time-based triggers for your workflows using Cloaca's built-in cron scheduling capabilities. This is essential for building automated data pipelines, periodic maintenance tasks, and scheduled business processes.

## Learning Objectives

- Understand cron scheduling in Cloaca
- Create time-based workflow triggers
- Configure schedule policies and recovery
- Monitor scheduled executions
- Handle timezone considerations
- Implement robust scheduling patterns

## Prerequisites

- Completion of [Tutorial 4](/python-bindings/tutorials/04-error-handling/)
- Understanding of cron expression syntax
- Basic knowledge of timezone handling
- Familiarity with Python datetime operations

## Time Estimate
25-30 minutes

## Cron Scheduling Overview

Cloaca provides built-in cron scheduling that runs within your application process, eliminating the need for external schedulers like crontab or task queues for time-based workflows.

### Key Features

- **Cron expressions** for flexible scheduling
- **Timezone support** for global applications
- **Missed execution recovery** when applications restart
- **Execution monitoring** and logging
- **Per-schedule configuration** for different policies

## Basic Cron Scheduling

Let's start with a simple scheduled workflow:

```python
import cloaca
from datetime import datetime
import time

# Define a scheduled task
@cloaca.task(id="daily_report")
def daily_report(context):
    """Generate daily business report."""
    current_time = datetime.now()

    # Simulate report generation
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
    context.set("report_type", "daily")

    return context

def create_daily_report_workflow():
    """Create the daily report workflow."""
    builder = cloaca.WorkflowBuilder("daily_report")
    builder.description("Daily business analytics report")
    builder.add_task("daily_report")
    return builder.build()

def basic_cron_scheduling():
    """Demonstrate basic cron scheduling."""
    print("=== Basic Cron Scheduling Demo ===")

    # Create runner with cron scheduling enabled
    runner = cloaca.DefaultRunner("sqlite:///:memory:")

    # Register workflow
    cloaca.register_workflow_constructor("daily_report", create_daily_report_workflow)

    # Create cron schedule
    schedule = cloaca.CronSchedule(
        workflow_name="daily_report",
        cron_expression="0 9 * * *",  # Daily at 9:00 AM
        timezone="UTC",
        enabled=True
    )

    try:
        # Register the schedule
        runner.add_cron_schedule(schedule)
        print("✓ Cron schedule registered: Daily at 9:00 AM UTC")

        # For demo purposes, we'll use a frequent schedule
        demo_schedule = cloaca.CronSchedule(
            workflow_name="daily_report",
            cron_expression="*/30 * * * * *",  # Every 30 seconds for demo
            timezone="UTC",
            enabled=True
        )

        runner.add_cron_schedule(demo_schedule)
        print("✓ Demo schedule added: Every 30 seconds")

        # Run for a short time to see executions
        print("⏰ Waiting for scheduled executions...")
        time.sleep(65)  # Wait for at least 2 executions

    finally:
        runner.shutdown()

if __name__ == "__main__":
    basic_cron_scheduling()
```

## Advanced Scheduling Patterns

### Multi-Schedule Workflows

```python
import cloaca
from datetime import datetime, timezone
import json

@cloaca.task(id="data_backup")
def data_backup(context):
    """Perform database backup."""
    backup_type = context.get("backup_type", "incremental")
    timestamp = datetime.now().isoformat()

    print(f"💾 Performing {backup_type} backup at {timestamp}")

    # Simulate backup process
    if backup_type == "full":
        print("   Full backup: All tables exported")
        backup_size = "2.5GB"
    else:
        print("   Incremental backup: Changed records only")
        backup_size = "150MB"

    context.set("backup_completed_at", timestamp)
    context.set("backup_size", backup_size)
    context.set("backup_type", backup_type)

    return context

@cloaca.task(id="cleanup_logs")
def cleanup_logs(context):
    """Clean up old log files."""
    retention_days = context.get("retention_days", 30)
    timestamp = datetime.now().isoformat()

    print(f"🧹 Cleaning logs older than {retention_days} days at {timestamp}")

    # Simulate cleanup
    files_removed = 47
    space_freed = "1.2GB"

    context.set("cleanup_completed_at", timestamp)
    context.set("files_removed", files_removed)
    context.set("space_freed", space_freed)

    return context

@cloaca.task(id="system_health_check")
def system_health_check(context):
    """Perform system health monitoring."""
    timestamp = datetime.now().isoformat()

    print(f"🏥 System health check at {timestamp}")

    # Simulate health checks
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

    context.set("health_check_at", timestamp)
    context.set("health_status", health_status)
    context.set("overall_status", overall_status)

    return context

def create_maintenance_workflows():
    """Create various maintenance workflows."""

    # Full backup workflow
    full_backup_builder = cloaca.WorkflowBuilder("full_backup")
    full_backup_builder.description("Weekly full database backup")
    full_backup_builder.add_task("data_backup")

    # Incremental backup workflow
    incremental_backup_builder = cloaca.WorkflowBuilder("incremental_backup")
    incremental_backup_builder.description("Daily incremental backup")
    incremental_backup_builder.add_task("data_backup")

    # Log cleanup workflow
    cleanup_builder = cloaca.WorkflowBuilder("log_cleanup")
    cleanup_builder.description("Weekly log file cleanup")
    cleanup_builder.add_task("cleanup_logs")

    # Health check workflow
    health_builder = cloaca.WorkflowBuilder("health_check")
    health_builder.description("Hourly system health monitoring")
    health_builder.add_task("system_health_check")

    return {
        "full_backup": full_backup_builder.build(),
        "incremental_backup": incremental_backup_builder.build(),
        "log_cleanup": cleanup_builder.build(),
        "health_check": health_builder.build()
    }

def demonstrate_multiple_schedules():
    """Demonstrate multiple cron schedules for different maintenance tasks."""
    print("=== Multiple Schedule Management ===")

    # Create runner
    runner = cloaca.DefaultRunner("sqlite:///:memory:")

    # Register workflows
    workflows = create_maintenance_workflows()
    for name, workflow in workflows.items():
        cloaca.register_workflow_constructor(name, lambda w=workflow: w)

    # Define schedules with different patterns
    schedules = [
        # Full backup: Weekly on Sunday at 2:00 AM
        cloaca.CronSchedule(
            workflow_name="full_backup",
            cron_expression="0 2 * * SUN",
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({"backup_type": "full"})
        ),

        # Incremental backup: Daily at 3:00 AM
        cloaca.CronSchedule(
            workflow_name="incremental_backup",
            cron_expression="0 3 * * *",
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({"backup_type": "incremental"})
        ),

        # Log cleanup: Weekly on Saturday at 1:00 AM
        cloaca.CronSchedule(
            workflow_name="log_cleanup",
            cron_expression="0 1 * * SAT",
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({"retention_days": 30})
        ),

        # Health check: Every 15 minutes during business hours
        cloaca.CronSchedule(
            workflow_name="health_check",
            cron_expression="*/15 9-17 * * MON-FRI",
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({})
        )
    ]

    # For demo, use frequent schedules
    demo_schedules = [
        cloaca.CronSchedule(
            workflow_name="full_backup",
            cron_expression="*/45 * * * * *",  # Every 45 seconds
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({"backup_type": "full"})
        ),
        cloaca.CronSchedule(
            workflow_name="health_check",
            cron_expression="*/20 * * * * *",  # Every 20 seconds
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({})
        )
    ]

    try:
        # Register production schedules (commented out for demo)
        for schedule in schedules:
            # runner.add_cron_schedule(schedule)
            print(f"📅 Would register: {schedule.workflow_name} - {schedule.cron_expression}")

        # Register demo schedules
        for schedule in demo_schedules:
            runner.add_cron_schedule(schedule)
            print(f"✓ Demo schedule: {schedule.workflow_name} - {schedule.cron_expression}")

        print("\n⏰ Running demo schedules...")
        time.sleep(70)  # Let schedules run

    finally:
        runner.shutdown()

if __name__ == "__main__":
    demonstrate_multiple_schedules()
```

## Timezone and Recovery Configuration

```python
import cloaca
from datetime import datetime, timezone, timedelta
import time

@cloaca.task(id="global_sync")
def global_sync(context):
    """Synchronize data across global regions."""
    region = context.get("region", "unknown")
    sync_type = context.get("sync_type", "delta")
    current_time = datetime.now()

    print(f"🌍 Global sync for {region} at {current_time}")
    print(f"   Sync type: {sync_type}")

    # Simulate region-specific processing
    if region == "us-east":
        records_synced = 15420
    elif region == "eu-west":
        records_synced = 8930
    elif region == "asia-pacific":
        records_synced = 12150
    else:
        records_synced = 5000

    print(f"   Records synced: {records_synced}")

    context.set("sync_completed_at", current_time.isoformat())
    context.set("records_synced", records_synced)
    context.set("region", region)

    return context

def create_global_sync_workflow():
    """Create global synchronization workflow."""
    builder = cloaca.WorkflowBuilder("global_sync")
    builder.description("Cross-region data synchronization")
    builder.add_task("global_sync")
    return builder.build()

def demonstrate_timezone_scheduling():
    """Demonstrate timezone-aware scheduling and recovery policies."""
    print("=== Timezone & Recovery Configuration ===")

    # Create runner with recovery settings
    runner = cloaca.DefaultRunner("sqlite:///:memory:")

    # Register workflow
    cloaca.register_workflow_constructor("global_sync", create_global_sync_workflow)

    # Create timezone-specific schedules
    regional_schedules = [
        # US East Coast: 6:00 AM ET (11:00 AM UTC)
        cloaca.CronSchedule(
            workflow_name="global_sync",
            cron_expression="0 11 * * *",
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({
                "region": "us-east",
                "sync_type": "full"
            })
        ),

        # Europe: 6:00 AM CET (5:00 AM UTC)
        cloaca.CronSchedule(
            workflow_name="global_sync",
            cron_expression="0 5 * * *",
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({
                "region": "eu-west",
                "sync_type": "incremental"
            })
        ),

        # Asia Pacific: 6:00 AM JST (9:00 PM UTC previous day)
        cloaca.CronSchedule(
            workflow_name="global_sync",
            cron_expression="0 21 * * *",
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({
                "region": "asia-pacific",
                "sync_type": "delta"
            })
        )
    ]

    # Demo schedules (every 25 seconds for different regions)
    demo_schedules = [
        cloaca.CronSchedule(
            workflow_name="global_sync",
            cron_expression="*/25 * * * * *",
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({
                "region": "us-east",
                "sync_type": "demo"
            })
        ),
        cloaca.CronSchedule(
            workflow_name="global_sync",
            cron_expression="5,30,55 * * * * *",  # Offset by 5 seconds
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({
                "region": "eu-west",
                "sync_type": "demo"
            })
        )
    ]

    try:
        # Show production schedules
        print("📍 Production Regional Schedules:")
        for schedule in regional_schedules:
            print(f"   {schedule.context.get('region')}: {schedule.cron_expression} UTC")

        # Register demo schedules
        print("\n🚀 Starting demo schedules...")
        for schedule in demo_schedules:
            runner.add_cron_schedule(schedule)
            print(f"✓ {schedule.context.get('region')}: {schedule.cron_expression}")

        print("\n⏰ Watching scheduled executions...")
        time.sleep(70)

    finally:
        runner.shutdown()

if __name__ == "__main__":
    demonstrate_timezone_scheduling()
```

## Schedule Management and Monitoring

```python
import cloaca
from datetime import datetime
import time
import json

@cloaca.task(id="schedule_monitor")
def schedule_monitor(context):
    """Monitor and report on schedule execution."""
    monitor_type = context.get("monitor_type", "health")
    timestamp = datetime.now()

    print(f"📊 Schedule monitoring ({monitor_type}) at {timestamp}")

    # Simulate monitoring data
    if monitor_type == "health":
        metrics = {
            "active_schedules": 12,
            "successful_executions_24h": 87,
            "failed_executions_24h": 2,
            "average_execution_time_ms": 1250,
            "next_execution": (datetime.now() + timedelta(minutes=30)).isoformat()
        }
    else:  # performance
        metrics = {
            "peak_concurrent_workflows": 8,
            "resource_utilization": 34.5,
            "queue_length": 3,
            "avg_wait_time_ms": 150
        }

    print(f"   Metrics: {json.dumps(metrics, indent=2)}")

    context.set("monitoring_data", metrics)
    context.set("monitor_type", monitor_type)
    context.set("measured_at", timestamp.isoformat())

    return context

class ScheduleManager:
    """Advanced schedule management with monitoring."""

    def __init__(self, database_url: str):
        self.runner = cloaca.DefaultRunner(database_url)
        self.active_schedules = {}

    def register_workflow(self, name: str, constructor):
        """Register a workflow constructor."""
        cloaca.register_workflow_constructor(name, constructor)

    def add_schedule(self, schedule_id: str, schedule: cloaca.CronSchedule):
        """Add a named schedule for management."""
        self.runner.add_cron_schedule(schedule)
        self.active_schedules[schedule_id] = schedule
        print(f"✓ Added schedule '{schedule_id}': {schedule.workflow_name} - {schedule.cron_expression}")

    def list_schedules(self):
        """List all active schedules."""
        print(f"\n📋 Active Schedules ({len(self.active_schedules)}):")
        for schedule_id, schedule in self.active_schedules.items():
            status = "🟢 Enabled" if schedule.enabled else "🔴 Disabled"
            print(f"   {schedule_id}: {schedule.workflow_name}")
            print(f"      Expression: {schedule.cron_expression}")
            print(f"      Timezone: {schedule.timezone}")
            print(f"      Status: {status}")
            print()

    def get_schedule_info(self, schedule_id: str):
        """Get detailed information about a specific schedule."""
        if schedule_id not in self.active_schedules:
            print(f"❌ Schedule '{schedule_id}' not found")
            return None

        schedule = self.active_schedules[schedule_id]
        return {
            "id": schedule_id,
            "workflow_name": schedule.workflow_name,
            "cron_expression": schedule.cron_expression,
            "timezone": schedule.timezone,
            "enabled": schedule.enabled,
            "context": dict(schedule.context.data) if schedule.context else {}
        }

    def shutdown(self):
        """Clean shutdown of the schedule manager."""
        print("\n🛑 Shutting down schedule manager...")
        self.runner.shutdown()

def demonstrate_schedule_management():
    """Demonstrate advanced schedule management capabilities."""
    print("=== Schedule Management & Monitoring ===")

    # Create schedule manager
    manager = ScheduleManager("sqlite:///:memory:")

    # Create monitoring workflow
    def create_monitor_workflow():
        builder = cloaca.WorkflowBuilder("schedule_monitor")
        builder.description("Schedule execution monitoring")
        builder.add_task("schedule_monitor")
        return builder.build()

    # Register workflows
    manager.register_workflow("schedule_monitor", create_monitor_workflow)

    # Add various schedules
    schedules = {
        "health_monitor": cloaca.CronSchedule(
            workflow_name="schedule_monitor",
            cron_expression="*/30 * * * * *",  # Every 30 seconds
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({"monitor_type": "health"})
        ),

        "performance_monitor": cloaca.CronSchedule(
            workflow_name="schedule_monitor",
            cron_expression="*/45 * * * * *",  # Every 45 seconds
            timezone="UTC",
            enabled=True,
            context=cloaca.Context({"monitor_type": "performance"})
        )
    }

    try:
        # Add schedules
        for schedule_id, schedule in schedules.items():
            manager.add_schedule(schedule_id, schedule)

        # List active schedules
        manager.list_schedules()

        # Show schedule details
        print("🔍 Schedule Details:")
        for schedule_id in schedules.keys():
            info = manager.get_schedule_info(schedule_id)
            print(f"   {json.dumps(info, indent=2)}")

        print("\n⏰ Running monitoring schedules...")
        time.sleep(60)

    finally:
        manager.shutdown()

if __name__ == "__main__":
    demonstrate_schedule_management()
```

## Running the Complete Example

Save this as `python_cron_tutorial.py`:

```python
#!/usr/bin/env python3
"""
Cloaca Cron Scheduling Tutorial
Complete example demonstrating time-based workflow execution.
"""

import cloaca
from datetime import datetime, timedelta
import time
import json

# Import all our task definitions
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

    return {
        "daily_report": report_builder.build(),
        "system_backup": backup_builder.build()
    }

def main():
    """Main tutorial demonstration."""
    print("🕐 Cloaca Cron Scheduling Tutorial")
    print("=" * 50)

    # Create runner
    runner = cloaca.DefaultRunner("sqlite:///:memory:")

    try:
        # Register workflows
        workflows = create_workflows()
        for name, workflow in workflows.items():
            cloaca.register_workflow_constructor(name, lambda w=workflow: w)

        # Create schedules
        schedules = [
            # Daily report every 20 seconds (demo)
            cloaca.CronSchedule(
                workflow_name="daily_report",
                cron_expression="*/20 * * * * *",
                timezone="UTC",
                enabled=True,
                context=cloaca.Context({})
            ),

            # Backup every 35 seconds (demo)
            cloaca.CronSchedule(
                workflow_name="system_backup",
                cron_expression="*/35 * * * * *",
                timezone="UTC",
                enabled=True,
                context=cloaca.Context({"backup_type": "incremental"})
            )
        ]

        # Register schedules
        for schedule in schedules:
            runner.add_cron_schedule(schedule)
            print(f"✓ Scheduled: {schedule.workflow_name} - {schedule.cron_expression}")

        print(f"\n⏰ Running schedules for 75 seconds...")
        print("   (You should see executions approximately every 20 and 35 seconds)")

        time.sleep(75)

        print("\n✅ Tutorial completed successfully!")
        print("\nWhat you learned:")
        print("- Creating cron schedules with expressions")
        print("- Registering schedules with workflows")
        print("- Configuring timezone and context")
        print("- Managing multiple concurrent schedules")

    finally:
        print("\n🛑 Shutting down...")
        runner.shutdown()

if __name__ == "__main__":
    main()
```

Run the tutorial:

```bash
python python_cron_tutorial.py
```

## Best Practices

{{< tabs "cron-best-practices" >}}
{{< tab "Schedule Design" >}}
**Design robust schedules:**
- Use appropriate intervals for your use case
- Consider business hours and maintenance windows
- Plan for timezone differences in global applications
- Avoid overlapping long-running schedules
{{< /tab >}}

{{< tab "Error Handling" >}}
**Handle failures gracefully:**
- Implement retry logic in tasks
- Monitor for missed executions
- Log schedule execution outcomes
- Set up alerting for critical scheduled workflows
{{< /tab >}}

{{< tab "Performance" >}}
**Optimize performance:**
- Limit concurrent scheduled executions
- Use appropriate database backends for scale
- Monitor resource usage during peak schedule times
- Consider using separate runners for different schedule types
{{< /tab >}}
{{< /tabs >}}

## Production Considerations

### Cron Expression Examples

```bash
# Common patterns
"0 0 * * *"        # Daily at midnight
"0 */6 * * *"      # Every 6 hours
"0 9 * * MON-FRI"  # Weekdays at 9 AM
"0 0 1 * *"        # First day of month
"0 2 * * SUN"      # Weekly on Sunday at 2 AM
```

### Deployment Tips

1. **Environment Configuration**: Use environment variables for schedule expressions
2. **Health Monitoring**: Monitor schedule execution in production
3. **Graceful Shutdown**: Ensure runners shut down cleanly
4. **Resource Limits**: Set appropriate connection and memory limits

## What You've Learned

Congratulations! You now understand:

- **Cron expressions** and how to use them with Cloaca
- **Schedule registration** and workflow management
- **Timezone handling** for global applications
- **Recovery and monitoring** capabilities
- **Production considerations** for scheduled workflows

## Next Steps

With cron scheduling mastered, you're ready to:

1. **[API Reference](/python-bindings/api-reference/)** - Explore advanced scheduling options
2. **[Examples](/python-bindings/examples/)** - See real-world scheduling patterns
3. **[Performance Guide](/python-bindings/how-to-guides/)** - Optimize scheduled workflows

## Related Resources

- [Explanation: Cron Scheduling Architecture](/explanation/cron-scheduling/) - Deep dive into scheduling
- [How-to: Production Scheduling](/how-to-guides/production-scheduling/) - Deployment best practices
- [Rust Tutorial: Cron Scheduling](/tutorials/05-cron-scheduling/) - Rust equivalent
