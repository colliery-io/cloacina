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

use serde_json::Value;

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::{GlobalOpts, OutputFormat};

pub async fn run(globals: &GlobalOpts, filter: Option<&str>) -> Result<(), CliError> {
    let config = CloacinaConfig::load(&globals.home.join("config.toml"));
    let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
    let output = ctx.output;
    let client = CliClient::new(ctx)?;

    let tenant = client.ctx().tenant_segment().to_string();
    let body: Value = client.get(&format!("/tenants/{tenant}/workflows")).await?;
    let items: Vec<Value> = body
        .get("workflows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let items: Vec<Value> = match filter {
        Some(pat) => items
            .into_iter()
            .filter(|v| {
                v.get("package_name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.contains(pat))
                    .unwrap_or(false)
            })
            .collect(),
        None => items,
    };

    render_list(&items, output)
}

fn render_list(items: &[Value], format: OutputFormat) -> Result<(), CliError> {
    match format {
        OutputFormat::Json => {
            let s = serde_json::to_string_pretty(items)
                .map_err(|e| CliError::UserError(e.to_string()))?;
            println!("{s}");
        }
        OutputFormat::Yaml => {
            let s = serde_yaml::to_string(items).map_err(|e| CliError::UserError(e.to_string()))?;
            print!("{s}");
        }
        OutputFormat::Id => {
            for item in items {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    println!("{id}");
                }
            }
        }
        OutputFormat::Table => {
            if items.is_empty() {
                println!("No items.");
                return Ok(());
            }
            println!(
                "{:<12} {:<30} {:<10} {:<20}",
                "ID", "NAME", "VERSION", "CREATED"
            );
            for item in items {
                let id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(truncate_id)
                    .unwrap_or_else(|| "?".into());
                let name = item
                    .get("package_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let version = item.get("version").and_then(|v| v.as_str()).unwrap_or("?");
                let created = item
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                println!("{:<12} {:<30} {:<10} {:<20}", id, name, version, created);
            }
        }
    }
    Ok(())
}

fn truncate_id(id: &str) -> String {
    if id.len() <= 8 {
        id.to_string()
    } else {
        format!("{}…", &id[..8])
    }
}
