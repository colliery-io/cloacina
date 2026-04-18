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

use anyhow::{Context, Result};
use std::time::Duration;

use crate::GlobalOpts;

pub async fn run(globals: &GlobalOpts) -> Result<()> {
    let Some(server_url) = globals.server.clone() else {
        println!("no server configured (pass --server or set a profile in T-0512)");
        std::process::exit(2);
    };

    let health_url = format!("{}/health", server_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client
        .get(&health_url)
        .send()
        .await
        .with_context(|| format!("failed to reach {health_url}"));

    match response {
        Ok(r) if r.status().is_success() => {
            println!("server:     {server_url}");
            println!("  reachable: yes (HTTP {})", r.status().as_u16());
            println!("  endpoint:  {health_url}");
            Ok(())
        }
        Ok(r) => {
            println!("server:     {server_url}");
            println!("  reachable: yes, but status HTTP {}", r.status().as_u16());
            std::process::exit(2);
        }
        Err(e) => {
            println!("server:     {server_url}");
            println!("  reachable: no ({e:#})");
            std::process::exit(2);
        }
    }
}
