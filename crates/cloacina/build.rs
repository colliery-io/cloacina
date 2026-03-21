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

//! Build script for cloacina.
//!
//! Sets up Python configuration (cfg flags) and rpath for finding the
//! Python shared library at runtime. The rpath is needed for test binaries
//! and downstream binaries that embed Python.

fn main() {
    // Emit PyO3 cfg flags (e.g., Py_3_9, etc.)
    pyo3_build_config::use_pyo3_cfgs();

    // Set rpath so test binaries and downstream binaries can find
    // the Python shared library at runtime.
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
