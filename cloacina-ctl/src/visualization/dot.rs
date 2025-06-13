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

use anyhow::Result;

use crate::cli::Cli;
use crate::manifest::PackageManifest;
use crate::utils::{should_print, LogLevel};

pub fn generate_dot_visualization(manifest: &PackageManifest, cli: &Cli) -> Result<()> {
    if should_print(cli, LogLevel::Info) {
        println!("digraph \"{}\" {{", manifest.package.name);
        println!("  rankdir=LR;");
        println!("  node [shape=box];");
        println!();

        // Add nodes
        for task in &manifest.tasks {
            println!("  \"{}\" [label=\"{}\"];", task.id, task.id);
        }

        println!();

        // Add edges
        for task in &manifest.tasks {
            for dependency in &task.dependencies {
                println!("  \"{}\" -> \"{}\";", dependency, task.id);
            }
        }

        println!("}}");
    }

    Ok(())
}
