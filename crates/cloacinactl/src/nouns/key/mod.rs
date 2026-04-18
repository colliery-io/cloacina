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

use clap::{Args, Subcommand, ValueEnum};

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;
use crate::{GlobalOpts, OutputFormat};

#[derive(Args)]
pub struct KeyCmd {
    #[command(subcommand)]
    verb: KeyVerb,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Role {
    Admin,
    Write,
    Read,
}

#[derive(Subcommand)]
enum KeyVerb {
    /// Create an API key. Tenant scope inferred from the calling key unless
    /// admin + `--role admin`.
    Create {
        /// Human-readable name/label for the key.
        name: String,
        #[arg(long, value_enum, default_value_t = Role::Read)]
        role: Role,
    },
    List,
    Revoke {
        id: String,
        #[arg(long)]
        force: bool,
    },
}

impl KeyVerb {
    fn role_str(r: Role) -> &'static str {
        match r {
            Role::Admin => "admin",
            Role::Write => "write",
            Role::Read => "read",
        }
    }
}

impl KeyCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        match self.verb {
            KeyVerb::Create { name, role } => {
                let body = serde_json::json!({
                    "name": name,
                    "role": KeyVerb::role_str(role),
                });
                let resp: serde_json::Value = client.post("/auth/keys", &body).await?;

                // One-time-only warning for human output.
                if matches!(output, OutputFormat::Table) {
                    let id = resp.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    let plaintext = resp
                        .get("key")
                        .or_else(|| resp.get("secret"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("<none>");
                    println!("created key: {plaintext}");
                    println!("ID:          {id}");
                    println!("role:        {}", KeyVerb::role_str(role));
                    println!("NOTE: this is the only time the secret will be shown. Save it now.");
                    Ok(())
                } else {
                    render::object(&resp, output)
                }
            }
            KeyVerb::List => {
                let body: serde_json::Value = client.get("/auth/keys").await?;
                let keys = body.get("keys").cloned().unwrap_or(body);
                render::list(&keys, output)
            }
            KeyVerb::Revoke { id, force } => {
                if !force {
                    crate::shared::client::confirm_destructive(&format!("revoke key {id}"))?;
                }
                client.delete(&format!("/auth/keys/{id}")).await?;
                println!("revoked {id}");
                Ok(())
            }
        }
    }
}
