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

//! `cloacinactl secret` — tenant secrets (CLOACI-I-0133 / T-0862).
//!
//! Metadata-only reads: `list`/`get` never show a value. Create/rotate read
//! field VALUES from a file (`k=@path`), stdin (`k=-`), or an interactive prompt
//! (`k` / `k=?`) — **never** from an argv literal (which would land in shell
//! history / the process table) and never echoed back.

use std::collections::BTreeMap;
use std::io::{Read, Write};

use clap::{Args, Subcommand};

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;
use crate::GlobalOpts;

#[derive(Args)]
pub struct SecretCmd {
    #[command(subcommand)]
    verb: SecretVerb,
}

#[derive(Subcommand)]
enum SecretVerb {
    /// Create a secret. Field VALUES come from a file (`k=@path`), stdin
    /// (`k=-`), or a prompt (`k` / `k=?`) — never an argv literal.
    Create {
        /// Secret name (unique within the tenant).
        name: String,
        /// Repeatable field source: `NAME=@path` (file), `NAME=-` (stdin), or
        /// `NAME` / `NAME=?` (prompt). A literal `NAME=value` is rejected so a
        /// secret never lands on the command line.
        #[arg(
            long = "field",
            short = 'f',
            value_name = "NAME=@path|NAME=-|NAME",
            required = true
        )]
        field: Vec<String>,
    },
    /// Rotate a secret's values in place (the next fire sees the new value).
    Rotate {
        name: String,
        #[arg(
            long = "field",
            short = 'f',
            value_name = "NAME=@path|NAME=-|NAME",
            required = true
        )]
        field: Vec<String>,
    },
    /// List secret metadata (names, field names, timestamps). Never shows values.
    List,
    /// Show one secret's metadata. Never shows values.
    Get { name: String },
    /// Delete a secret.
    Delete {
        name: String,
        #[arg(long)]
        force: bool,
    },
}

impl SecretCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        let tenant = client.ctx().tenant_segment().to_string();

        match self.verb {
            SecretVerb::Create { name, field } => {
                let fields = collect_fields(&field)?;
                let body = serde_json::json!({ "name": name, "fields": fields });
                let resp: serde_json::Value = client
                    .post(&format!("/v1/tenants/{tenant}/secrets"), &body)
                    .await?;
                render::object(&resp, output)
            }
            SecretVerb::Rotate { name, field } => {
                let fields = collect_fields(&field)?;
                let body = serde_json::json!({ "fields": fields });
                let resp: serde_json::Value = client
                    .put(&format!("/v1/tenants/{tenant}/secrets/{name}"), &body)
                    .await?;
                render::object(&resp, output)
            }
            SecretVerb::List => {
                let body: serde_json::Value =
                    client.get(&format!("/v1/tenants/{tenant}/secrets")).await?;
                render::list(&body, output)
            }
            SecretVerb::Get { name } => {
                let body: serde_json::Value = client
                    .get(&format!("/v1/tenants/{tenant}/secrets/{name}"))
                    .await?;
                render::object(&body, output)
            }
            SecretVerb::Delete { name, force } => {
                if !force {
                    crate::shared::client::confirm_destructive(&format!("delete secret {name}"))?;
                }
                client
                    .delete(&format!("/v1/tenants/{tenant}/secrets/{name}"))
                    .await?;
                println!("deleted {name}");
                Ok(())
            }
        }
    }
}

/// Parse `--field` specs into a `{name: value}` map, reading each value from its
/// source. A literal `NAME=value` is rejected — values must never be on argv.
fn collect_fields(specs: &[String]) -> Result<BTreeMap<String, String>, CliError> {
    let mut out = BTreeMap::new();
    let mut stdin_used = false;
    for spec in specs {
        let (name, source) = match spec.split_once('=') {
            Some((n, s)) => (n.trim().to_string(), s.to_string()),
            // Bare `NAME` → prompt.
            None => (spec.trim().to_string(), "?".to_string()),
        };
        if name.is_empty() {
            return Err(CliError::UserError(format!(
                "invalid --field '{spec}': empty field name"
            )));
        }

        let value = if let Some(path) = source.strip_prefix('@') {
            read_file_value(path)?
        } else if source == "-" {
            if stdin_used {
                return Err(CliError::UserError(
                    "only one --field may read from stdin ('-')".into(),
                ));
            }
            stdin_used = true;
            read_stdin_value()?
        } else if source == "?" {
            prompt_value(&name)?
        } else {
            // A literal value on the command line — refuse it.
            return Err(CliError::UserError(format!(
                "field '{name}': refusing a literal value on the command line (it would leak \
                 into shell history / the process table). Use '{name}=@<file>', '{name}=-' \
                 (stdin), or '{name}' (prompt)."
            )));
        };

        out.insert(name, value);
    }
    Ok(out)
}

/// One trailing newline is stripped (files/heredocs usually add one); interior
/// content is preserved verbatim.
fn strip_one_trailing_newline(mut s: String) -> String {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
    s
}

fn read_file_value(path: &str) -> Result<String, CliError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| CliError::UserError(format!("failed to read field file '{path}': {e}")))?;
    Ok(strip_one_trailing_newline(raw))
}

fn read_stdin_value() -> Result<String, CliError> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(CliError::Io)?;
    Ok(strip_one_trailing_newline(buf))
}

/// Interactive prompt. The prompt text goes to stderr (so piping stdout stays
/// clean); the value is read from stdin and never re-printed.
fn prompt_value(name: &str) -> Result<String, CliError> {
    eprint!("value for '{name}': ");
    std::io::stderr().flush().ok();
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(CliError::Io)?;
    Ok(strip_one_trailing_newline(line))
}
