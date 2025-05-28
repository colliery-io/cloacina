/*
 *  Copyright 2025 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use tracing::Level;

#[test]
fn test_structured_logging() {
    // Test structured logging with fields (without setting global subscriber)
    let span = tracing::info_span!("test_operation", operation = "unit_test");
    let _guard = span.enter();

    // These will work regardless of global subscriber state
    tracing::info!(
        task_id = "test-task",
        duration_ms = 100,
        success = true,
        "Task completed successfully"
    );
}

#[test]
fn test_logging_with_context() {
    // Create a span with context (works without global subscriber)
    let span = tracing::span!(
        Level::INFO,
        "workflow_execution",
        workflow_name = "test-workflow",
        task_count = 5
    );

    let _guard = span.enter();

    // Log within the span context
    tracing::info!("Starting Workflow execution");
    tracing::debug!(current_task = "task-1", "Processing task");
    tracing::info!("Workflow execution completed");
}

#[test]
fn test_span_creation() {
    // Test that we can create and enter spans
    let span = tracing::debug_span!("test_span", field1 = "value1");
    let guard = span.enter();

    // Test nested spans
    let nested_span = tracing::trace_span!("nested", field2 = 42);
    let _nested_guard = nested_span.enter();

    drop(_nested_guard);
    drop(guard);

    // If we get here, span creation/dropping works
    assert!(true);
}

#[test]
fn test_event_creation() {
    // Test that we can create events at different levels
    tracing::trace!("Trace event");
    tracing::debug!("Debug event");
    tracing::info!("Info event");
    tracing::warn!("Warning event");
    tracing::error!("Error event");

    // Test events with fields
    tracing::info!(field1 = "value", field2 = 123, "Event with fields");

    // If we get here without panicking, event creation works
    assert!(true);
}
