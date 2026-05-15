/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 */

//! # Conditional Retries — `retry_condition`
//!
//! Demonstrates per-task retry-condition policies (CLOACI-T-0042).
//! Tasks fail with one of two error flavors:
//!
//! - `flaky_api_call` simulates a transient network failure twice and
//!   succeeds on the third attempt. It uses `retry_condition = "transient"`,
//!   so the retry policy's substring matcher catches "connection refused"
//!   and lets the task run again.
//!
//! - `validation_check` returns a hard validation error and uses
//!   `retry_condition = "never"`. The runner records the failure and
//!   does not retry, even though `retry_attempts = 3`.
//!
//! Vocabulary (from `cloacina_workflow::RetryCondition`):
//!
//! | string             | RetryCondition          |
//! |--------------------|-------------------------|
//! | "all" (or omitted) | AllErrors (default)     |
//! | "never"            | Never                   |
//! | "transient"        | TransientOnly           |
//! | comma list, e.g.   | ErrorPattern { ... }    |
//! | "rate limit,5xx"   | match-substring on msg  |
//!
//! Run me:
//!     export DATABASE_URL=sqlite::memory:
//!     cargo run -p conditional-retries-example

use cloacina::executor::WorkflowExecutor;
use cloacina::runner::DefaultRunner;
use cloacina::{task, workflow, Context, TaskError};
use serde_json::json;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::{info, warn};

// Per-process attempt counters — read at the end so the example
// prints "needed 3 attempts" / "stopped after 1" headlines.
static FLAKY_ATTEMPTS: AtomicU32 = AtomicU32::new(0);
static VALIDATION_ATTEMPTS: AtomicU32 = AtomicU32::new(0);

#[workflow(
    name = "conditional_retries_pipeline",
    description = "Demonstrates retry_condition: transient retries, validation does not"
)]
pub mod conditional_retries_pipeline {
    use super::*;

    /// Flaky API call. Fails twice with a "connection refused" message —
    /// matches the TransientOnly pattern — then succeeds on attempt 3.
    #[task(
        id = "flaky_api_call",
        dependencies = [],
        retry_attempts = 3,
        retry_delay_ms = 100,
        retry_max_delay_ms = 500,
        retry_jitter = false,
        retry_condition = "transient"
    )]
    pub async fn flaky_api_call(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let attempt = FLAKY_ATTEMPTS.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt < 3 {
            warn!(attempt, "flaky_api_call: simulated connection refused");
            return Err(TaskError::ExecutionFailed {
                message: "connection refused (simulated transient outage)".into(),
                task_id: "flaky_api_call".into(),
                timestamp: chrono::Utc::now(),
            });
        }
        info!(attempt, "flaky_api_call: succeeded");
        context.insert("api_result", json!({"ok": true, "attempt": attempt}))?;
        Ok(())
    }

    /// Validation check. Fails with a hard validation error.
    /// `retry_condition = "never"` prevents retries even though
    /// `retry_attempts = 3` is configured.
    #[task(
        id = "validation_check",
        dependencies = [],
        retry_attempts = 3,
        retry_delay_ms = 100,
        retry_max_delay_ms = 500,
        retry_jitter = false,
        retry_condition = "never"
    )]
    pub async fn validation_check(
        _context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let attempt = VALIDATION_ATTEMPTS.fetch_add(1, Ordering::SeqCst) + 1;
        warn!(attempt, "validation_check: input rejected — will NOT retry");
        Err(TaskError::ExecutionFailed {
            message: "input failed schema validation".into(),
            task_id: "validation_check".into(),
            timestamp: chrono::Utc::now(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());

    info!(
        "Starting conditional-retries example against {}",
        cloacina::logging::mask_db_url(&database_url)
    );
    let runner = DefaultRunner::new(&database_url).await?;

    // -------- 1. Transient retries succeed eventually --------
    info!("--- run 1: flaky_api_call (retry_condition = transient) ---");
    let flaky_workflow = Workflow::builder("conditional_retries_pipeline_flaky")
        .description("flaky API workflow")
        .add_task(std::sync::Arc::new(
            conditional_retries_pipeline::flaky_api_call_task(),
        ))
        .unwrap()
        .build()
        .unwrap();
    let _ = runner
        .execute("conditional_retries_pipeline_flaky", Context::new())
        .await;
    info!(
        "flaky_api_call attempts: {}",
        FLAKY_ATTEMPTS.load(Ordering::SeqCst)
    );

    // -------- 2. Validation errors do NOT retry --------
    info!("--- run 2: validation_check (retry_condition = never) ---");
    let validation_workflow = Workflow::builder("conditional_retries_pipeline_validation")
        .description("validation workflow — never retries")
        .add_task(std::sync::Arc::new(
            conditional_retries_pipeline::validation_check_task(),
        ))
        .unwrap()
        .build()
        .unwrap();
    let _ = runner
        .execute("conditional_retries_pipeline_validation", Context::new())
        .await;
    info!(
        "validation_check attempts: {}",
        VALIDATION_ATTEMPTS.load(Ordering::SeqCst)
    );

    // Final summary so the example tells a story without DB inspection.
    let flaky = FLAKY_ATTEMPTS.load(Ordering::SeqCst);
    let validation = VALIDATION_ATTEMPTS.load(Ordering::SeqCst);
    println!();
    println!("Summary:");
    println!(
        "  flaky_api_call:    attempted {} times → succeeded (transient retries enabled)",
        flaky
    );
    println!(
        "  validation_check:  attempted {} times → failed   (retry_condition = never)",
        validation
    );

    runner.shutdown().await?;
    let _ = flaky_workflow;
    let _ = validation_workflow;
    Ok(())
}

// `Workflow` is re-exported from `cloacina` as a top-level name.
use cloacina::Workflow;
