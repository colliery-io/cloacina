# Cron Scheduling Example

This example demonstrates Cloacina's cron scheduling capabilities for automated workflow execution. It shows how to create time-based schedules that automatically trigger workflows at specified intervals.

## Features Demonstrated

- **Multiple Cron Schedules**: Three different workflows with different timing patterns
- **Workflow Automation**: Zero-touch execution based on time triggers
- **Recovery Service**: Automatic detection and retry of missed executions
- **Monitoring**: Real-time execution statistics and logging
- **Graceful Shutdown**: Clean termination with proper resource cleanup

## Workflows

### 1. Data Backup Workflow (`data_backup_workflow`)
**Schedule**: Every 30 minutes
**Purpose**: Automated data backup with integrity verification

**Tasks**:
- `check_backup_prerequisites` - Verify disk space and permissions
- `create_backup_snapshot` - Create incremental backup
- `verify_backup_integrity` - Validate backup checksum
- `cleanup_old_backups` - Remove outdated backup files

### 2. Health Check Workflow (`health_check_workflow`)
**Schedule**: Every 5 minutes
**Purpose**: Continuous system monitoring and health assessment

**Tasks**:
- `check_system_resources` - Monitor CPU, memory, and disk usage
- `check_database_connectivity` - Verify database health
- `check_external_services` - Test external service availability
- `update_health_metrics` - Aggregate overall health score

### 3. Daily Report Workflow (`daily_report_workflow`)
**Schedule**: Daily at 6:00 AM UTC
**Purpose**: Generate and distribute daily usage reports

**Tasks**:
- `collect_daily_metrics` - Gather metrics from various sources
- `generate_usage_report` - Create formatted report
- `send_report_notification` - Email report to stakeholders

## Running the Example

```bash
# Navigate to the example directory
cd examples/cron-scheduling

# Run the example
cargo run

# The example will:
# 1. Create and register all workflows
# 2. Set up cron schedules for each workflow
# 3. Run for 5 minutes showing automatic execution
# 4. Display execution statistics
# 5. Shutdown gracefully
```

## Expected Output

When running, you'll see logs similar to:

```
🚀 Starting Cron Scheduling Example
📊 DefaultRunner initialized with cron scheduling enabled
📝 Workflows registered successfully
📅 Created backup schedule (ID: ...) - runs every 30 minutes
🏥 Created health check schedule (ID: ...) - runs every 5 minutes
📊 Created daily report schedule (ID: ...) - runs daily at 6 AM UTC
⏰ Cron schedules created, workflows will execute automatically
🔍 You can monitor execution in the logs below...

# Automatic workflow executions will appear as:
🖥️  Checking system resources...
✅ System resource check completed
🗄️  Checking database connectivity...
✅ Database connectivity check passed (23ms)
# ... and so on
```

## Key Concepts

### Cron Expressions
- `*/30 * * * *` - Every 30 minutes
- `*/5 * * * *` - Every 5 minutes
- `0 6 * * *` - Daily at 6:00 AM

### Recovery Service
The example enables automatic recovery for missed executions:
- Detects executions that were started but never completed
- Automatically retries failed executions
- Configurable retry limits and timeouts

### Catchup Policies
- `Skip` - Ignore missed executions (used in this example)
- `RunAll` - Execute all missed schedules when system recovers

## Customization

You can modify the example to:

1. **Change schedules**: Update cron expressions in `create_cron_schedules()`
2. **Add workflows**: Create new workflow functions and register them
3. **Adjust timing**: Modify poll intervals for faster/slower checking
4. **Add tasks**: Extend workflows with additional task dependencies

## Database

The example uses SQLite with WAL mode for better concurrency:
```
cron-example.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000
```

You can examine the database contents to see:
- Cron schedules in the `cron_schedules` table
- Execution history in the `cron_executions` table
- Pipeline executions in the `pipeline_executions` table

## Production Considerations

When using cron scheduling in production:

1. **Timezone handling**: Always specify explicit timezones
2. **Resource limits**: Set appropriate `max_concurrent_tasks`
3. **Monitoring**: Implement alerting on failed executions
4. **Database**: Use PostgreSQL for better concurrent performance
5. **Recovery settings**: Tune recovery intervals based on your SLA requirements
