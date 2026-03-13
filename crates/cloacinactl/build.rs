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

//! Build script for cloacinactl.
//!
//! Ensures the embedded Python interpreter's shared library can be found
//! at runtime by setting the appropriate rpath.

fn main() {
    // Get Python configuration from pyo3-build-config
    let config = pyo3_build_config::get();

    // On macOS with framework builds, the dylib is loaded as
    // @rpath/Python3.framework/... so we need the framework search path.
    // On Linux, we need the lib directory in rpath.
    if let Some(lib_dir) = &config.lib_dir {
        // For macOS framework builds, the lib_dir is deep inside the framework.
        // We need the parent directory that contains *.framework/.
        let rpath = if lib_dir.contains(".framework/") {
            // Walk up to find the directory containing the .framework
            let parts: Vec<&str> = lib_dir.splitn(2, ".framework/").collect();
            let framework_container = std::path::Path::new(parts[0])
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| lib_dir.clone());
            framework_container
        } else {
            lib_dir.clone()
        };

        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", rpath);
    }
}
