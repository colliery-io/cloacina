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

//! Task definitions for the ETL workflow
//!
//! This module demonstrates a simple ETL workflow with three tasks:
//! - Extract: Get numbers from input context
//! - Transform: Multiply numbers by 2
//! - Load: Store the transformed numbers

use cloacina::{task, Context, TaskError};
use serde_json::{json, Value};
use tracing::{debug, info};

/// Extract numbers from the input context
#[task(
    id = "extract_numbers",
    dependencies = [],
    retry_attempts = 2,
    retry_backoff = "fixed",
    retry_delay_ms = 1000
)]
pub async fn extract_numbers(context: &mut Context<Value>) -> Result<(), TaskError> {
    info!("Extracting numbers from input");

    let numbers = context
        .get("numbers")
        .and_then(|v| v.as_array())
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "No numbers found in context".to_string(),
        })?
        .clone(); // Clone the array to avoid borrow issues

    debug!("Found {} numbers to process", numbers.len());

    // Store the extracted numbers
    context.insert("extracted_numbers", json!(numbers))?;
    context.insert("extract_timestamp", json!(chrono::Utc::now()))?;

    info!("Successfully extracted {} numbers", numbers.len());
    Ok(())
}

/// Transform the numbers (multiply by 2)
#[task(
    id = "transform_numbers",
    dependencies = ["extract_numbers"],
    retry_attempts = 2,
    retry_backoff = "fixed",
    retry_delay_ms = 1000
)]
pub async fn transform_numbers(context: &mut Context<Value>) -> Result<(), TaskError> {
    info!("Transforming numbers");

    let numbers = context
        .get("extracted_numbers")
        .and_then(|v| v.as_array())
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "No extracted numbers found for transformation".to_string(),
        })?
        .clone(); // Clone the array to avoid borrow issues

    // Transform numbers (multiply by 2)
    let transformed: Vec<i64> = numbers
        .iter()
        .filter_map(|n| n.as_i64())
        .map(|n| n * 2)
        .collect();

    debug!("Transformed {} numbers", transformed.len());

    // Store transformed numbers
    context.insert("transformed_numbers", json!(transformed))?;
    context.insert("transform_timestamp", json!(chrono::Utc::now()))?;

    info!("Successfully transformed {} numbers", transformed.len());
    Ok(())
}

/// Load the transformed numbers
#[task(
    id = "load_numbers",
    dependencies = ["transform_numbers"],
    retry_attempts = 2,
    retry_backoff = "fixed",
    retry_delay_ms = 1000
)]
pub async fn load_numbers(context: &mut Context<Value>) -> Result<(), TaskError> {
    info!("Loading transformed numbers");

    let numbers = context
        .get("transformed_numbers")
        .and_then(|v| v.as_array())
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "No transformed numbers found for loading".to_string(),
        })?
        .clone(); // Clone the array to avoid borrow issues

    // Simulate loading the numbers (in a real scenario, this would write to a database or file)
    debug!("Loading {} numbers", numbers.len());

    // Store load results
    context.insert("loaded_numbers", json!(numbers))?;
    context.insert("load_timestamp", json!(chrono::Utc::now()))?;
    context.insert("load_status", json!("success"))?;

    info!("Successfully loaded {} numbers", numbers.len());
    Ok(())
}
