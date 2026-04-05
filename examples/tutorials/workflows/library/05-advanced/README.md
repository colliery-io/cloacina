# Tutorial 05: Cron Scheduling

This example demonstrates how to use Cloacina's cron scheduling feature to automatically execute workflows on time-based triggers.

## Features Demonstrated

- Creating workflows with the macro system
- Setting up cron schedules programmatically
- Automatic workflow execution via cron triggers
- Multiple concurrent schedules
- Different cron expressions and timezones

## Running the Example

```bash
cd examples/tutorial-05
cargo run
```

## What You'll See

The example demonstrates three types of scheduled workflows:

1. **Data Backup** - Simulated backup process with different types
2. **Health Check** - System monitoring with health status reporting
3. **Daily Report** - Report generation with metrics

## Configuration

The example uses SQLite for simplicity. In production, you would typically:

1. Use PostgreSQL for better performance and features
2. Set up actual cron schedules instead of manual execution
3. Implement proper error handling and alerting
4. Configure appropriate logging and monitoring

## Related Documentation

- [Tutorial 05: Cron Scheduling](/tutorials/05-cron-scheduling/)
- [Cron Scheduling API Reference](/reference/cron-scheduling/)
