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

use anyhow::Result;
use std::time::Duration;

use crate::commands::config::CloacinaConfig;
use crate::{GlobalOpts, OutputFormat};

pub async fn run(globals: &GlobalOpts) -> Result<()> {
    let config = CloacinaConfig::load(&globals.home.join("config.toml"));
    let base = compiler_base_url(&config.compiler.local_addr);
    let url = format!("{}/v1/status", base);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            match globals.effective_output() {
                OutputFormat::Json => println!(
                    "{}",
                    serde_json::json!({
                        "reachable": false,
                        "endpoint": url,
                        "error": format!("{e:#}"),
                    })
                ),
                _ => {
                    println!("compiler:   {base}");
                    println!("  reachable: no ({e:#})");
                }
            }
            std::process::exit(2);
        }
    };

    if !response.status().is_success() {
        println!("compiler:   {base}");
        println!(
            "  reachable: yes, but status HTTP {}",
            response.status().as_u16()
        );
        std::process::exit(2);
    }

    let body: serde_json::Value = response.json().await.unwrap_or(serde_json::json!({}));

    match globals.effective_output() {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
        _ => {
            println!("compiler:   {base}");
            println!("  endpoint:  {url}");
            println!(
                "  pending:   {}",
                body.get("pending")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".into())
            );
            println!(
                "  building:  {}",
                body.get("building")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".into())
            );
            println!("  last OK:   {}", fmt_ts(&body, "last_success_at"));
            println!("  last FAIL: {}", fmt_ts(&body, "last_failure_at"));
            println!("  heartbeat: {}", fmt_ts(&body, "heartbeat_at"));
        }
    }
    Ok(())
}

fn fmt_ts(body: &serde_json::Value, key: &str) -> String {
    match body.get(key).and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => "—".to_string(),
    }
}

pub(crate) fn compiler_base_url(local_addr: &str) -> String {
    if local_addr.starts_with("http://") || local_addr.starts_with("https://") {
        local_addr.trim_end_matches('/').to_string()
    } else {
        format!("http://{}", local_addr)
    }
}
