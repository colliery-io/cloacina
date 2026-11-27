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
    /// Act as an execution agent, and nothing else.
    Agent,
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
        /// Make a platform admin key: no tenant, role `admin` and the
        /// platform admin flag. Needs a platform admin key (CLOACINA-T-1065).
        #[arg(long, conflicts_with = "role")]
        platform_admin: bool,
        /// When the key expires: an RFC 3339 timestamp in the future, for
        /// example `2027-01-31T00:00:00Z`. Not set: the key does not expire.
        #[arg(long)]
        expires_at: Option<String>,
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
            Role::Agent => "agent",
        }
    }

    /// The body of `POST /v1/auth/keys`. A platform admin key has role
    /// `admin`.
    fn create_body(name: &str, role: Role, platform_admin: bool) -> serde_json::Value {
        if platform_admin {
            serde_json::json!({ "name": name, "role": "admin", "platform_admin": true })
        } else {
            serde_json::json!({ "name": name, "role": KeyVerb::role_str(role) })
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
            KeyVerb::Create {
                name,
                role,
                platform_admin,
                expires_at,
            } => {
                let mut body = KeyVerb::create_body(&name, role, platform_admin);
                if let Some(at) = &expires_at {
                    body["expires_at"] = serde_json::Value::String(at.clone());
                }
                let resp: serde_json::Value = client.post("/v1/auth/keys", &body).await?;

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
                    let role = resp
                        .get("permissions")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    println!("role:        {role}");
                    if platform_admin {
                        println!("platform admin: yes");
                    }
                    if let Some(at) = resp.get("expires_at").and_then(|v| v.as_str()) {
                        println!("expires at:  {at}");
                    }
                    println!("NOTE: this is the only time the secret will be shown. Save it now.");
                    Ok(())
                } else {
                    render::object(&resp, output)
                }
            }
            KeyVerb::List => {
                let body: serde_json::Value = client.get("/v1/auth/keys").await?;
                // T-0594 unified `{items,total}` envelope — render::list reads
                // `items` (the old `.get("keys").unwrap_or(body)` swallow is gone).
                render::list(&body, output)
            }
            KeyVerb::Revoke { id, force } => {
                if !force {
                    crate::shared::client::confirm_destructive(&format!("revoke key {id}"))?;
                }
                client.delete(&format!("/v1/auth/keys/{id}")).await?;
                println!("revoked {id}");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        key: KeyCmd,
    }

    /// CLOACINA-T-1065: `--platform-admin` asks for a platform admin key with
    /// role `admin`, and cannot be combined with `--role`.
    #[test]
    fn platform_admin_flag_sends_the_field_and_role_admin() {
        let cli = Cli::try_parse_from(["key", "create", "ops", "--platform-admin"]).unwrap();
        let KeyVerb::Create {
            name,
            role,
            platform_admin,
            ..
        } = cli.key.verb
        else {
            panic!("not a create");
        };
        assert!(platform_admin);
        assert_eq!(
            KeyVerb::create_body(&name, role, platform_admin),
            serde_json::json!({ "name": "ops", "role": "admin", "platform_admin": true })
        );

        assert!(
            Cli::try_parse_from(["key", "create", "ops", "--platform-admin", "--role", "read"])
                .is_err()
        );

        let cli = Cli::try_parse_from(["key", "create", "ci", "--role", "write"]).unwrap();
        let KeyVerb::Create {
            name,
            role,
            platform_admin,
            ..
        } = cli.key.verb
        else {
            panic!("not a create");
        };
        assert_eq!(
            KeyVerb::create_body(&name, role, platform_admin),
            serde_json::json!({ "name": "ci", "role": "write" })
        );
    }
}
