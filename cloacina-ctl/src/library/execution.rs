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

use anyhow::{bail, Context, Result};
use libloading::{Library, Symbol};
use std::path::PathBuf;

use crate::cli::Cli;
use crate::manifest::{PackageManifest, EXECUTE_TASK_SYMBOL};
use crate::utils::{should_print, LogLevel};

pub fn execute_task_from_library(
    library_path: &PathBuf,
    task_name: &str,
    context_json: &str,
    cli: &Cli,
) -> Result<()> {
    if should_print(cli, LogLevel::Debug) {
        println!("Loading library: {:?}", library_path);
    }

    // Load the dynamic library
    let lib = unsafe {
        Library::new(library_path)
            .with_context(|| format!("Failed to load library: {:?}", library_path))?
    };

    // Get the cloacina_execute_task symbol
    let execute_task: Symbol<
        unsafe extern "C" fn(
            task_name: *const u8,
            task_name_len: u32,
            context_json: *const u8,
            context_len: u32,
            result_buffer: *mut u8,
            result_capacity: u32,
            result_len: *mut u32,
        ) -> i32,
    > = unsafe {
        lib.get(EXECUTE_TASK_SYMBOL.as_bytes())
            .context("Symbol 'cloacina_execute_task' not found in library")?
    };

    // Prepare input parameters
    let task_name_bytes = task_name.as_bytes();
    let context_bytes = context_json.as_bytes();
    const RESULT_BUFFER_SIZE: usize = 10 * 1024 * 1024; // 10MB buffer for result (matches database limit)
    let mut result_buffer = vec![0u8; RESULT_BUFFER_SIZE];
    let mut result_len: u32 = 0;

    if should_print(cli, LogLevel::Debug) {
        println!(
            "Calling cloacina_execute_task with task_name={}, context_len={}",
            task_name,
            context_bytes.len()
        );
    }

    // Call the function
    let return_code = unsafe {
        execute_task(
            task_name_bytes.as_ptr(),
            task_name_bytes.len() as u32,
            context_bytes.as_ptr(),
            context_bytes.len() as u32,
            result_buffer.as_mut_ptr(),
            result_buffer.len() as u32,
            &mut result_len,
        )
    };

    // Handle result
    if return_code == 0 {
        // Success
        if result_len > 0 && result_len <= result_buffer.len() as u32 {
            let result_json = String::from_utf8_lossy(&result_buffer[..result_len as usize]);
            if should_print(cli, LogLevel::Info) {
                println!("Task execution successful!");
                println!("Result: {}", result_json);
            }
        } else if result_len > result_buffer.len() as u32 {
            bail!(
                "Task execution result too large: {} bytes exceeds maximum buffer size of {} bytes. \
                This indicates the task context has grown beyond the database storage limit.",
                result_len,
                result_buffer.len()
            );
        } else {
            if should_print(cli, LogLevel::Info) {
                println!("Task execution successful! (no result data)");
            }
        }
    } else {
        // Error
        let error_msg = if result_len > 0 && result_len <= result_buffer.len() as u32 {
            String::from_utf8_lossy(&result_buffer[..result_len as usize]).to_string()
        } else if result_len > result_buffer.len() as u32 {
            format!(
                "Task execution failed (code: {}) with oversized error message: {} bytes exceeds buffer size of {} bytes",
                return_code, result_len, result_buffer.len()
            )
        } else {
            format!("Unknown error (code: {})", return_code)
        };

        bail!("Task execution failed: {}", error_msg);
    }

    Ok(())
}

pub fn resolve_task_name(manifest: &PackageManifest, task_identifier: &str) -> Result<String> {
    // Try to parse as index first - if successful, convert to task name
    if let Ok(index) = task_identifier.parse::<u32>() {
        let index = index as usize;
        if index < manifest.tasks.len() {
            return Ok(manifest.tasks[index].id.clone());
        } else {
            bail!(
                "Task index {} is out of range. Available tasks: 0-{}",
                index,
                manifest.tasks.len().saturating_sub(1)
            );
        }
    }

    // Check if it's already a valid task name
    for task in &manifest.tasks {
        if task.id == task_identifier {
            return Ok(task.id.clone());
        }
    }

    bail!(
        "Task '{}' not found. Available tasks: {:?}",
        task_identifier,
        manifest.tasks.iter().map(|t| &t.id).collect::<Vec<_>>()
    );
}
