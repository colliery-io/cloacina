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
pub struct TenantCmd {
    #[command(subcommand)]
    verb: TenantVerb,
}

#[derive(Subcommand)]
enum TenantVerb {
    Create {
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    List,
    Delete {
        name: String,
        #[arg(long)]
        force: bool,
    },
}

impl TenantCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        match self.verb {
            TenantVerb::Create { name, description } => {
                let body = serde_json::json!({
                    "name": name,
                    "description": description,
                });
                let resp: serde_json::Value = client.post("/tenants", &body).await?;
                render::object(&resp, output)
            }
            TenantVerb::List => {
                let body: serde_json::Value = client.get("/tenants").await?;
                render::list(&body, output)
            }
            TenantVerb::Delete { name, force } => {
                if !force {
                    crate::shared::client::confirm_destructive(&format!("delete tenant {name}"))?;
                }
                client.delete(&format!("/tenants/{name}")).await?;
                println!("deleted tenant {name}");
                Ok(())
            }
        }
    }
}
