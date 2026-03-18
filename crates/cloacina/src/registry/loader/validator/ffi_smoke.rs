/*
 *  Copyright 2025-2026 Colliery Software
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

//! FFI smoke testing for packaged workflows.
//!
//! After symbol validation confirms the required FFI symbols exist, this module
//! actually *calls* them to verify the FFI boundary works. This catches issues
//! like missing tokio runtimes, ABI mismatches, and serialisation failures that
//! are invisible to symbol-only validation.

use libloading::Library;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::Path;
use tracing::{debug, warn};

use super::types::ValidationResult;
use super::PackageValidator;

/// C-compatible structure returned by cloacina_get_task_metadata
#[repr(C)]
struct CTaskMetadata {
    index: u32,
    local_id: *const c_char,
    _namespaced_id_template: *const c_char,
    _dependencies_json: *const c_char,
    _description: *const c_char,
    _source_location: *const c_char,
}

/// C-compatible collection returned by cloacina_get_task_metadata
#[repr(C)]
struct CPackageTasks {
    task_count: u32,
    tasks: *const CTaskMetadata,
    _package_name: *const c_char,
    _graph_data_json: *const c_char,
}

/// Type alias for the execute task FFI function
type ExecuteTaskFn = unsafe extern "C" fn(
    task_name: *const c_char,
    task_name_len: u32,
    context_json: *const c_char,
    context_len: u32,
    result_buffer: *mut u8,
    result_capacity: u32,
    result_len: *mut u32,
) -> i32;

/// Type alias for the get metadata FFI function
type GetMetadataFn = unsafe extern "C" fn() -> *const CPackageTasks;

impl PackageValidator {
    /// Run FFI smoke tests on a package library.
    ///
    /// This loads the library, extracts task names from metadata, and calls
    /// `cloacina_execute_task` for each task with an empty context. We don't
    /// expect tasks to succeed (they may need real context data), but we verify
    /// the FFI boundary doesn't panic or crash.
    ///
    /// A return code of 0 (success) or -1 (task error) are both acceptable —
    /// what we're catching is process-level failures: panics, aborts, segfaults.
    pub(super) async fn smoke_test_ffi(&self, package_path: &Path, result: &mut ValidationResult) {
        // Load the library
        let lib = match unsafe { Library::new(package_path) } {
            Ok(lib) => lib,
            Err(e) => {
                // Library can't be loaded — skip FFI smoke test (symbol validation
                // already reported this error)
                debug!("Skipping FFI smoke test: library load failed: {}", e);
                return;
            }
        };

        // Get task names from metadata
        let task_names = match self.extract_task_names(&lib) {
            Ok(names) => names,
            Err(e) => {
                result.warnings.push(format!(
                    "FFI smoke test: could not extract task names from metadata: {}",
                    e
                ));
                return;
            }
        };

        if task_names.is_empty() {
            debug!("FFI smoke test: no tasks found, skipping");
            return;
        }

        // Get the execute function
        let execute_fn = match unsafe { lib.get::<ExecuteTaskFn>(b"cloacina_execute_task") } {
            Ok(func) => func,
            Err(e) => {
                // Symbol validation already reported this
                debug!(
                    "Skipping FFI smoke test: cloacina_execute_task not found: {}",
                    e
                );
                return;
            }
        };

        // Smoke test each task
        let empty_context = b"{}";
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB result buffer

        for task_name in &task_names {
            let mut result_len: u32 = 0;

            debug!("FFI smoke test: calling task '{}'", task_name);

            // Use catch_unwind to contain Rust panics from the loaded library.
            // Note: this won't catch abort() — for that we'd need a subprocess.
            // But it covers the most common case (panic! in task code).
            let task_name_bytes = task_name.as_bytes();
            let context_bytes = empty_context;

            let call_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                (execute_fn)(
                    task_name_bytes.as_ptr() as *const c_char,
                    task_name_bytes.len() as u32,
                    context_bytes.as_ptr() as *const c_char,
                    context_bytes.len() as u32,
                    buffer.as_mut_ptr(),
                    buffer.len() as u32,
                    &mut result_len,
                )
            }));

            match call_result {
                Ok(rc) => {
                    // rc 0 = success, rc -1 = task error (expected with empty context)
                    // Both are fine — the FFI boundary worked.
                    if rc == 0 {
                        debug!("FFI smoke test: task '{}' succeeded (rc=0)", task_name);
                    } else {
                        // Extract error message for debug logging
                        let error_msg = if result_len > 0 {
                            String::from_utf8_lossy(&buffer[..result_len as usize]).to_string()
                        } else {
                            "(no output)".to_string()
                        };
                        debug!(
                            "FFI smoke test: task '{}' returned error (rc={}, expected with empty context): {}",
                            task_name, rc, error_msg
                        );
                    }
                }
                Err(panic_info) => {
                    // The task panicked across the FFI boundary — this is a real problem.
                    let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };

                    let error = format!(
                        "FFI validation failed: task '{}' panicked during smoke test: {}. \
                         This means the task will crash at runtime when loaded by the server or daemon. \
                         Common causes: missing tokio runtime (use cloacina_workflow's re-exported tokio), \
                         incompatible library architecture, or unhandled dependency in FFI context.",
                        task_name, panic_msg
                    );

                    warn!("{}", error);
                    result.errors.push(error);
                }
            }
        }
    }

    /// Extract task names from the library's metadata symbol.
    fn extract_task_names(&self, lib: &Library) -> Result<Vec<String>, String> {
        let get_metadata = unsafe {
            lib.get::<GetMetadataFn>(b"cloacina_get_task_metadata")
                .map_err(|e| format!("cloacina_get_task_metadata not found: {}", e))?
        };

        let c_package_tasks = unsafe { (get_metadata)() };
        if c_package_tasks.is_null() {
            return Err("cloacina_get_task_metadata returned null".to_string());
        }

        let package = unsafe { &*c_package_tasks };
        let mut names = Vec::new();

        if package.task_count > 0 && !package.tasks.is_null() {
            let tasks =
                unsafe { std::slice::from_raw_parts(package.tasks, package.task_count as usize) };
            for task in tasks {
                if !task.local_id.is_null() {
                    let name = unsafe {
                        CStr::from_ptr(task.local_id)
                            .to_str()
                            .map_err(|e| format!("Invalid UTF-8 in task name: {}", e))?
                            .to_string()
                    };
                    names.push(name);
                }
            }
        }

        Ok(names)
    }
}
