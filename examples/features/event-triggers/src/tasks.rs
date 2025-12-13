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

//! # Task Definitions
//!
//! This module defines the tasks used by the event-triggered workflows.
//! Tasks are grouped by the workflow they belong to.

use cloacina::prelude::*;
use cloacina::task;
use tracing::info;

// ============================================================================
// File Processing Workflow Tasks
// ============================================================================

/// Validates and parses an incoming file.
#[task(id = "validate_file", dependencies = [])]
pub async fn validate_file(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Clone string value to avoid borrow issues
    let filename = context
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    info!("Validating file: {}", filename);

    // Simulate validation
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    context.insert("validated", serde_json::json!(true))?;
    context.insert("file_size_bytes", serde_json::json!(1024))?;
    context.insert("record_count", serde_json::json!(100))?;

    info!("File '{}' validated successfully", filename);
    Ok(())
}

/// Processes the validated file data.
#[task(id = "process_file", dependencies = ["validate_file"])]
pub async fn process_file(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let filename = context
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let record_count = context
        .get("record_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    info!("Processing {} records from '{}'", record_count, filename);

    // Simulate processing
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    context.insert("processed_records", serde_json::json!(record_count))?;
    context.insert("processing_status", serde_json::json!("completed"))?;

    info!("Successfully processed {} records", record_count);
    Ok(())
}

/// Archives the processed file.
#[task(id = "archive_file", dependencies = ["process_file"])]
pub async fn archive_file(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let filename = context
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    info!("Archiving file: {}", filename);

    // Simulate archiving
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let archive_path = format!("/data/archive/{}", filename);
    context.insert("archive_path", serde_json::json!(archive_path.clone()))?;

    info!("File archived to: {}", archive_path);
    Ok(())
}

// ============================================================================
// Queue Processing Workflow Tasks
// ============================================================================

/// Drains messages from the queue.
#[task(id = "drain_queue", dependencies = [])]
pub async fn drain_queue(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let queue_name = context
        .get("queue_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let queue_depth = context
        .get("queue_depth")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    info!(
        "Draining {} messages from queue '{}'",
        queue_depth, queue_name
    );

    // Simulate draining
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    context.insert("messages_drained", serde_json::json!(queue_depth))?;

    info!("Drained {} messages from '{}'", queue_depth, queue_name);
    Ok(())
}

/// Processes the drained messages.
#[task(id = "process_messages", dependencies = ["drain_queue"])]
pub async fn process_messages(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let messages_drained = context
        .get("messages_drained")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    info!("Processing {} messages", messages_drained);

    // Simulate processing
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    context.insert("messages_processed", serde_json::json!(messages_drained))?;
    context.insert("processing_complete", serde_json::json!(true))?;

    info!("Processed {} messages successfully", messages_drained);
    Ok(())
}

/// Acknowledges processed messages.
#[task(id = "ack_messages", dependencies = ["process_messages"])]
pub async fn ack_messages(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let messages_processed = context
        .get("messages_processed")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    info!("Acknowledging {} messages", messages_processed);

    // Simulate acknowledgment
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    context.insert(
        "messages_acknowledged",
        serde_json::json!(messages_processed),
    )?;

    info!("Acknowledged {} messages", messages_processed);
    Ok(())
}

// ============================================================================
// Service Recovery Workflow Tasks
// ============================================================================

/// Diagnoses the service failure.
#[task(id = "diagnose_failure", dependencies = [])]
pub async fn diagnose_failure(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let service_name = context
        .get("service_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let failures = context
        .get("consecutive_failures")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    info!(
        "Diagnosing failure for service '{}' ({} consecutive failures)",
        service_name, failures
    );

    // Simulate diagnosis
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    context.insert("diagnosis", serde_json::json!("connection_timeout"))?;
    context.insert("severity", serde_json::json!("medium"))?;

    info!("Diagnosis complete: connection_timeout (severity: medium)");
    Ok(())
}

/// Attempts to restart the service.
#[task(id = "restart_service", dependencies = ["diagnose_failure"])]
pub async fn restart_service(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let service_name = context
        .get("service_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    info!("Attempting to restart service '{}'", service_name);

    // Simulate restart
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    context.insert("restart_attempted", serde_json::json!(true))?;
    context.insert("restart_success", serde_json::json!(true))?;

    info!("Service '{}' restarted successfully", service_name);
    Ok(())
}

/// Verifies service health after restart.
#[task(id = "verify_recovery", dependencies = ["restart_service"])]
pub async fn verify_recovery(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let service_name = context
        .get("service_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    info!("Verifying recovery of service '{}'", service_name);

    // Simulate verification
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    context.insert("recovery_verified", serde_json::json!(true))?;
    context.insert("health_status", serde_json::json!("healthy"))?;

    info!(
        "Service '{}' recovery verified - status: healthy",
        service_name
    );
    Ok(())
}

/// Sends notification about the incident.
#[task(id = "notify_incident", dependencies = ["verify_recovery"])]
pub async fn notify_incident(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let service_name = context
        .get("service_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let health_status = context
        .get("health_status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    info!(
        "Sending incident notification for '{}' (current status: {})",
        service_name, health_status
    );

    // Simulate notification
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    context.insert("notification_sent", serde_json::json!(true))?;

    info!("Incident notification sent for '{}'", service_name);
    Ok(())
}
