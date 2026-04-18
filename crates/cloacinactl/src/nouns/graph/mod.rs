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

//! `cloacinactl graph <verb>` — computation graphs loaded in the server's
//! reactive scheduler.
//!
//! Backed by the server's `/v1/health/reactors*` + `/v1/health/accumulators`
//! endpoints. Computation graphs are called "reactors" internally; the CLI
//! surfaces the operator-facing term.

use clap::{Args, Subcommand};

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;
use crate::GlobalOpts;

#[derive(Args)]
pub struct GraphCmd {
    #[command(subcommand)]
    verb: GraphVerb,
}

#[derive(Subcommand)]
enum GraphVerb {
    /// List loaded computation graphs with health + pause state.
    List,
    /// Show a single graph's health, accumulators, and pause state.
    Status { name: String },
    /// List accumulators across all loaded graphs with health.
    Accumulators,
}

impl GraphCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        match self.verb {
            GraphVerb::List => {
                let body: serde_json::Value = client.get("/v1/health/reactors").await?;
                let reactors = body.get("reactors").cloned().unwrap_or(body);
                render::list(&reactors, output)
            }
            GraphVerb::Status { name } => {
                let body: serde_json::Value =
                    client.get(&format!("/v1/health/reactors/{name}")).await?;
                render::object(&body, output)
            }
            GraphVerb::Accumulators => {
                let body: serde_json::Value = client.get("/v1/health/accumulators").await?;
                let accs = body.get("accumulators").cloned().unwrap_or(body);
                render::list(&accs, output)
            }
        }
    }
}
