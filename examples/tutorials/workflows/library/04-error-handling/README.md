# Tutorial 4: Error Handling and Retries

This example demonstrates how to implement robust error handling, retry policies, and recovery mechanisms in Cloacina pipelines. It shows various patterns for handling different types of errors and making your workflows resilient to failures.

## Features Demonstrated

- Different types of errors and handling strategies
- Retry policy configuration and customization
- Graceful degradation and fallback patterns
- Non-critical task failure handling
- Simulated failures for testing error handling
- Callback hooks for task success and failure notifications

## Running the Example

To run this example:

```bash
cargo run
```

## What to Expect

When you run this example, you'll see:

1. Simulated API calls that may fail (70% success rate)
2. Fallback mechanisms when external data is unavailable
3. Critical operations with retry policies
4. Non-critical cleanup operations that can fail without stopping the pipeline

The example includes simulated failures to demonstrate the error handling mechanisms. You'll see different outcomes each time you run it due to the random nature of the simulated failures.

## Key Concepts

- **Retry Policies**: Different retry configurations for different types of operations
- **Error Types**: External, Transient, and Non-Critical errors
- **Fallback Mechanisms**: Graceful degradation when primary operations fail
- **Failure Modes**: Configuring how task failures affect the overall pipeline
- **Callbacks**: Hooks for task success and failure notifications

## Callbacks

Cloacina supports callback functions that are invoked when tasks succeed or fail. These are useful for:

- Alerting (send notifications to Slack, PagerDuty, etc.)
- Monitoring (update dashboards, record metrics)
- Cleanup operations
- Logging and audit trails

### Callback Signatures

```rust
// Success callback - called when task completes successfully
async fn on_task_success(
    task_id: &str,
    context: &Context<Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Handle success
    Ok(())
}

// Failure callback - called when task fails (after all retries exhausted)
async fn on_task_failure(
    task_id: &str,
    error: &TaskError,
    context: &Context<Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Handle failure
    Ok(())
}
```

### Usage

Add callbacks to your task definitions using the `on_success` and `on_failure` parameters:

```rust
#[task(
    id = "critical_task",
    dependencies = [],
    on_success = log_completion,
    on_failure = alert_team
)]
async fn critical_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // Task implementation
}
```

Key points:
- Callbacks are resolved at compile time - just reference any async function in scope
- Callback failures do not fail the task or workflow (errors are logged but isolated)
- Use `on_failure` for critical tasks that need immediate attention when they fail
- Use `on_success` for audit logging or triggering downstream processes

## Next Steps

After running this example, try:

1. Modifying the retry policies
2. Adding new error handling patterns
3. Implementing your own fallback mechanisms
4. Testing different failure scenarios
