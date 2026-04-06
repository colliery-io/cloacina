/*
 *  Copyright 2026 Colliery Software
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

//! Minimal test: load a packaged .dylib/.so via dlopen within the test process.
//! This isolates the SIGSEGV issue without needing Docker/Postgres.

#[test]
fn test_dlopen_packaged_workflow_library() {
    // Find the pre-built library
    let cargo_manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::PathBuf::from(&cargo_manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let project_path = workspace_root.join("examples/features/workflows/packaged-workflows");

    // Find .dylib or .so
    let target_dir = project_path.join("target/release");
    let lib_path = if cfg!(target_os = "macos") {
        target_dir.join("libpackaged_workflow_example.dylib")
    } else {
        target_dir.join("libpackaged_workflow_example.so")
    };

    if !lib_path.exists() {
        eprintln!(
            "Skipping test: pre-built library not found at {:?}. \
            Run `cargo build --release` in examples/features/workflows/packaged-workflows first.",
            lib_path
        );
        return;
    }

    println!("Loading library: {:?}", lib_path);

    // This is where the SIGSEGV happens
    let lib = unsafe { libloading::Library::new(&lib_path) };

    match lib {
        Ok(lib) => {
            println!("Library loaded successfully");

            // Try to get the metadata function
            let func: Result<
                libloading::Symbol<unsafe extern "C" fn() -> *const std::os::raw::c_void>,
                _,
            > = unsafe { lib.get(b"cloacina_get_task_metadata") };

            match func {
                Ok(f) => {
                    println!("Symbol found, calling cloacina_get_task_metadata...");
                    let ptr = unsafe { f() };
                    assert!(!ptr.is_null(), "Metadata pointer should not be null");
                    println!("Returned non-null pointer: {:?}", ptr);
                }
                Err(e) => {
                    panic!("Failed to find cloacina_get_task_metadata symbol: {}", e);
                }
            }
        }
        Err(e) => {
            panic!("Failed to load library: {}", e);
        }
    }
}
