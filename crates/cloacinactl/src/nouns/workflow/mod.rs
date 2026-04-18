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

//! `cloacinactl workflow <verb>`.

use clap::{Args, Subcommand};
use std::path::PathBuf;

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;
use crate::GlobalOpts;

#[derive(Args)]
pub struct WorkflowCmd {
    #[command(subcommand)]
    verb: WorkflowVerb,
}

#[derive(Subcommand)]
enum WorkflowVerb {
    /// List all registered workflows.
    List {
        #[arg(long)]
        package: Option<String>,
    },
    /// Show tasks, deps, trigger rules, and schedules for a workflow.
    Inspect { name: String },
    /// Kick off an execution.
    Run {
        name: String,
        /// Context JSON file path, or `-` for stdin.
        #[arg(long)]
        context: Option<String>,
    },
    /// Allow new scheduled runs.
    Enable { name: String },
    /// Stop new scheduled runs (does not cancel in-flight).
    Disable { name: String },
}

impl WorkflowCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        match self.verb {
            WorkflowVerb::List { package } => {
                let path = match package {
                    Some(p) => format!("/v1/workflows/index?package={p}"),
                    None => "/v1/workflows/index".into(),
                };
                let body: serde_json::Value = client.get(&path).await?;
                render::list(&body, output)
            }
            WorkflowVerb::Inspect { name } => {
                let body: serde_json::Value =
                    client.get(&format!("/v1/workflows/by-name/{name}")).await?;
                render::object(&body, output)
            }
            WorkflowVerb::Run { name, context } => {
                let body = load_context(context.as_deref())?;
                let resp: serde_json::Value = client
                    .post(&format!("/v1/workflows/{name}/run"), &body)
                    .await?;
                if let Some(id) = resp.get("execution_id").and_then(|v| v.as_str()) {
                    println!("{id}");
                } else {
                    render::object(&resp, output)?;
                }
                Ok(())
            }
            WorkflowVerb::Enable { name } => {
                let _: serde_json::Value = client
                    .post(
                        &format!("/v1/workflows/{name}/enable"),
                        &serde_json::json!({}),
                    )
                    .await?;
                println!("enabled {name}");
                Ok(())
            }
            WorkflowVerb::Disable { name } => {
                let _: serde_json::Value = client
                    .post(
                        &format!("/v1/workflows/{name}/disable"),
                        &serde_json::json!({}),
                    )
                    .await?;
                println!("disabled {name}");
                Ok(())
            }
        }
    }
}

fn load_context(source: Option<&str>) -> Result<serde_json::Value, CliError> {
    match source {
        None => Ok(serde_json::json!({})),
        Some("-") => {
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf).map_err(CliError::Io)?;
            serde_json::from_str(&buf).map_err(|e| CliError::UserError(e.to_string()))
        }
        Some(path) => {
            let buf = std::fs::read_to_string(PathBuf::from(path)).map_err(CliError::Io)?;
            serde_json::from_str(&buf).map_err(|e| CliError::UserError(e.to_string()))
        }
    }
}
