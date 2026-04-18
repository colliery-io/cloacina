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

//! Noun-verb subcommand dispatch. Each noun is its own module; verbs are
//! methods on the noun's `Cmd` struct.

use anyhow::Result;

use crate::GlobalOpts;

pub mod daemon;
pub mod server;

/// Composite status — runs daemon status + server status and prints both.
/// The one documented exception to the strict noun-verb rule.
pub async fn top_level_status(globals: &GlobalOpts) -> Result<()> {
    println!("=== daemon ===");
    if let Err(e) = daemon::status::run(globals).await {
        println!("daemon unreachable: {e:#}");
    }

    println!();
    println!("=== server ===");
    if let Err(e) = server::status::run(globals).await {
        println!("server unreachable: {e:#}");
    }

    Ok(())
}
