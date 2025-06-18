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

/*!
# Package Workflow Demo

This example demonstrates how to compile a packaged workflow to a shared library
and create a .cloacina package file for distribution.

Run with: `cargo run --example package_workflow`
*/

use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  Packaged Workflow Build Demo");
    println!("================================\n");

    // Step 1: Build as shared library
    println!("Step 1: Building workflow as shared library...");
    let output = Command::new("cargo")
        .args(&["build", "--release", "--lib"])
        .current_dir(".")
        .output()?;

    if !output.status.success() {
        eprintln!("Build failed:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err("Build failed".into());
    }

    println!("✅ Build completed successfully");

    // Step 2: Show the generated library
    let library_name = if cfg!(target_os = "macos") {
        "libsimple_packaged_demo.dylib"
    } else if cfg!(target_os = "windows") {
        "simple_packaged_demo.dll"
    } else {
        "libsimple_packaged_demo.so"
    };

    let library_path = format!("target/release/{}", library_name);
    println!("📦 Generated library: {}", library_path);

    // Step 3: Check if library exists and show size
    if let Ok(metadata) = std::fs::metadata(&library_path) {
        println!("📏 Library size: {} bytes", metadata.len());
        println!("🔧 Library contains FFI exports for dynamic loading");
    } else {
        println!("⚠️  Library not found at expected path");
    }

    println!("\n🎯 Next Steps:");
    println!("   1. The .dylib/.so/.dll file can be loaded dynamically");
    println!("   2. Run 'cargo run --example end_to_end_demo' to see it in action");
    println!("   3. In production, create .cloacina archive with cloacina-ctl");

    println!("\n💡 Example cloacina-ctl usage:");
    println!("   cloacina-ctl package build .");
    println!("   # Creates simple_packaged_demo.cloacina");

    Ok(())
}
