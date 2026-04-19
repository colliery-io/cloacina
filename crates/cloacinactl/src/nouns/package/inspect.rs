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

pub async fn run(globals: &GlobalOpts, id: &str) -> Result<(), CliError> {
    let config = CloacinaConfig::load(&globals.home.join("config.toml"));
    let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
    let output = ctx.output;
    let client = CliClient::new(ctx)?;

    let tenant = client.ctx().tenant_segment().to_string();
    let body: Value = client
        .get(&format!("/v1/tenants/{tenant}/workflows/{id}"))
        .await?;

    match output {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&body)
                    .map_err(|e| CliError::UserError(e.to_string()))?
            );
        }
        OutputFormat::Yaml => {
            print!(
                "{}",
                serde_yaml::to_string(&body).map_err(|e| CliError::UserError(e.to_string()))?
            );
        }
        OutputFormat::Id => {
            if let Some(id) = body.get("id").and_then(|v| v.as_str()) {
                println!("{id}");
            }
        }
        OutputFormat::Table => {
            // Human-ish summary.
            println!("ID:           {}", json_str(&body, "id"));
            println!("Name:         {}", json_str(&body, "package_name"));
            println!("Version:      {}", json_str(&body, "version"));
            println!("Tenant:       {}", json_str(&body, "tenant_id"));
            println!("Created:      {}", json_str(&body, "created_at"));
            println!("Build status: {}", json_str(&body, "build_status"));
            if let Some(err) = body.get("build_error").and_then(|v| v.as_str()) {
                println!("Build error:  {}", err);
            }
        }
    }
    Ok(())
}

fn json_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("?")
        .to_string()
}
