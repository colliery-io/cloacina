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

use clap::{Args, Subcommand};

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;
use crate::GlobalOpts;

#[derive(Args)]
pub struct TriggerCmd {
    #[command(subcommand)]
    verb: TriggerVerb,
}

#[derive(Subcommand)]
enum TriggerVerb {
    List {
        /// Maximum number of rows to return (server-side cap: 1000).
        /// CLOACI-T-0596 / API-10.
        #[arg(long, default_value = "100")]
        limit: u32,
        /// Offset into the result set for pagination.
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    Inspect {
        name: String,
    },
}

impl TriggerCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        let tenant = client.ctx().tenant_segment().to_string();
        match self.verb {
            TriggerVerb::List { limit, offset } => {
                let body: serde_json::Value = client
                    .get(&format!(
                        "/v1/tenants/{tenant}/triggers?limit={limit}&offset={offset}"
                    ))
                    .await?;
                render::list(&body, output)
            }
            TriggerVerb::Inspect { name } => {
                let body: serde_json::Value = client
                    .get(&format!("/v1/tenants/{tenant}/triggers/{name}"))
                    .await?;
                render::object(&body, output)
            }
        }
    }
}
