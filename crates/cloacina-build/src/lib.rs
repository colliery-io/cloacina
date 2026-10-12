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

//! Build-time helper for binaries that depend on cloacina.
//!
//! Any binary crate that links against `cloacina` (which embeds Python via PyO3)
//! needs the Python shared library rpath set at link time. Without this, macOS
//! framework builds will crash at launch with:
//!
//! ```text
//! dyld: Library not loaded: @rpath/Python3.framework/Versions/3.x/Python3
//! ```
//!
//! # Usage
//!
//! Add to your binary crate's `Cargo.toml`:
//!
//! ```toml
//! [build-dependencies]
//! cloacina-build = "0.3.2"
//! ```
//!
//! Then create a `build.rs`:
//!
//! ```rust,no_run
//! cloacina_build::configure();
//! ```

/// Configures the Python rpath and PyO3 cfg flags for the current binary crate.
///
/// Call this from your `build.rs` `main()` function. It:
/// 1. Emits PyO3 cfg flags (e.g., `Py_3_9`)
/// 2. Sets the rpath linker arg so the Python shared library is found at runtime
pub fn configure() {
    pyo3_build_config::use_pyo3_cfgs();

    let config = pyo3_build_config::get();
    if let Some(lib_dir) = &config.lib_dir {
        // On macOS with framework builds, the dylib is loaded as
        // @rpath/Python3.framework/... so we need the framework search path.
        let rpath = if lib_dir.contains(".framework/") {
            let parts: Vec<&str> = lib_dir.splitn(2, ".framework/").collect();
            std::path::Path::new(parts[0])
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| lib_dir.clone())
        } else {
            lib_dir.clone()
        };

        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", rpath);
    }
}
