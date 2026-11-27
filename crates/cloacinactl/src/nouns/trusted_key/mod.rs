/*
 *  Copyright 2025-2026 Colliery Software
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

//! `cloacinactl trusted-key` — the keys a tenant trusts for package signing
//! (CLOACINA-A-0018, CLOACINA-T-1061).
//!
//! With `--require-signatures` on the server, an upload to a tenant passes
//! when its signer is a live trusted key of the tenant. A tenant admin
//! manages the keys of its own tenant; a platform admin names the tenant
//! with `--tenant`.

use clap::{ArgGroup, Args, Subcommand};

use crate::GlobalOpts;
use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;

#[derive(Args)]
pub struct TrustedKeyCmd {
    #[command(subcommand)]
    verb: TrustedKeyVerb,
}

#[derive(Subcommand)]
enum TrustedKeyVerb {
    /// Add an Ed25519 public key to the tenant's trusted keys.
    #[command(group(ArgGroup::new("key").required(true).args(["pem_file", "public_key"])))]
    Add {
        /// A PEM file with the public key (`-----BEGIN PUBLIC KEY-----`).
        #[arg(long, value_name = "PATH")]
        pem_file: Option<std::path::PathBuf>,
        /// The raw 32-byte public key as 64 hexadecimal characters.
        #[arg(long, value_name = "HEX")]
        public_key: Option<String>,
        /// A name for the key, to tell the keys of the list apart.
        #[arg(long)]
        name: Option<String>,
    },
    /// List the tenant's trusted keys. A revoked key stays in the list, marked.
    List,
    /// Revoke a trusted key: no upload passes with it after.
    Revoke {
        /// The id of the key, from `add` or `list`.
        id: String,
        #[arg(long)]
        force: bool,
    },
}

impl TrustedKeyCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        let tenant = client.ctx().tenant_segment().to_string();

        match self.verb {
            TrustedKeyVerb::Add {
                pem_file,
                public_key,
                name,
            } => {
                let body = add_body(pem_file.as_deref(), public_key.as_deref(), name.as_deref())?;
                let resp: serde_json::Value = client
                    .post(&format!("/v1/tenants/{tenant}/trusted-keys"), &body)
                    .await?;
                render::object(&resp, output)
            }
            TrustedKeyVerb::List => {
                let body: serde_json::Value = client
                    .get(&format!("/v1/tenants/{tenant}/trusted-keys"))
                    .await?;
                render::list(&body, output)
            }
            TrustedKeyVerb::Revoke { id, force } => {
                if !force {
                    crate::shared::client::confirm_destructive(&format!(
                        "revoke trusted key {id}"
                    ))?;
                }
                client
                    .delete(&format!("/v1/tenants/{tenant}/trusted-keys/{id}"))
                    .await?;
                println!("revoked {id}");
                Ok(())
            }
        }
    }
}

/// The body of the add: the PEM text of the file, or the hex key as given.
/// The server checks that the key is an Ed25519 public key.
fn add_body(
    pem_file: Option<&std::path::Path>,
    public_key: Option<&str>,
    name: Option<&str>,
) -> Result<serde_json::Value, CliError> {
    let mut body = serde_json::Map::new();
    match (pem_file, public_key) {
        (Some(path), None) => {
            let pem = std::fs::read_to_string(path).map_err(|e| {
                CliError::UserError(format!("failed to read PEM file '{}': {e}", path.display()))
            })?;
            body.insert("public_key_pem".into(), pem.into());
        }
        (None, Some(hex_key)) => {
            body.insert("public_key".into(), hex_key.trim().into());
        }
        _ => {
            return Err(CliError::UserError(
                "give exactly one of --pem-file and --public-key".into(),
            ));
        }
    }
    if let Some(name) = name {
        body.insert("name".into(), name.into());
    }
    Ok(serde_json::Value::Object(body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Harness {
        #[command(subcommand)]
        verb: TrustedKeyVerb,
    }

    #[test]
    fn add_takes_exactly_one_form_of_the_key() {
        assert!(Harness::try_parse_from(["x", "add", "--public-key", "ab"]).is_ok());
        assert!(Harness::try_parse_from(["x", "add", "--pem-file", "key.pem"]).is_ok());
        assert!(Harness::try_parse_from(["x", "add"]).is_err());
        assert!(
            Harness::try_parse_from(["x", "add", "--public-key", "ab", "--pem-file", "key.pem"])
                .is_err()
        );
    }

    #[test]
    fn add_body_reads_the_pem_file_or_passes_the_hex_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("key.pem");
        let pem = "-----BEGIN PUBLIC KEY-----\nAAAA\n-----END PUBLIC KEY-----\n";
        std::fs::write(&path, pem).unwrap();

        let body = add_body(Some(&path), None, Some("ci")).unwrap();
        assert_eq!(body["public_key_pem"], pem);
        assert_eq!(body["name"], "ci");
        assert!(body.get("public_key").is_none());

        let body = add_body(None, Some(" abcd \n"), None).unwrap();
        assert_eq!(body["public_key"], "abcd");
        assert!(body.get("name").is_none());
        assert!(body.get("public_key_pem").is_none());

        assert!(add_body(None, None, None).is_err());
        assert!(add_body(Some(&dir.path().join("missing.pem")), None, None).is_err());
    }
}
