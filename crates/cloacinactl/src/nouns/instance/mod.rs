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

//! `cloacinactl instance` — named, param-bound workflow instances
//! (CLOACI-T-0894).
//!
//! An instance binds a set of parameter values to a workflow under a name,
//! optionally on a cron schedule. Before this noun existed, the capability was
//! reachable only from the embedded runner, so server users could bind params
//! per run but never create a durable named instance.

use clap::{Args, Subcommand};
use std::path::PathBuf;

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;
use crate::GlobalOpts;

#[derive(Args)]
pub struct InstanceCmd {
    #[command(subcommand)]
    verb: InstanceVerb,
}

#[derive(Subcommand)]
enum InstanceVerb {
    /// Create a named instance of a workflow.
    Create {
        /// Workflow name.
        workflow: String,
        /// Instance name, unique per workflow.
        instance: String,
        /// Bind one parameter as `key=value`. Repeatable. The value is parsed
        /// as JSON when it parses (`count=5` binds the number 5) and taken as a
        /// string otherwise (`mode=copy`).
        #[arg(long = "param", value_name = "KEY=VALUE")]
        params: Vec<String>,
        /// Read all parameters from a JSON object file, or `-` for stdin.
        /// Merged UNDER `--param`, so an explicit flag wins on conflict.
        #[arg(long = "params", value_name = "FILE")]
        params_file: Option<String>,
        /// Cron expression. Omit to create an unscheduled binding that never
        /// fires on its own.
        #[arg(long)]
        cron: Option<String>,
        /// IANA timezone for `--cron` (default: UTC).
        #[arg(long)]
        timezone: Option<String>,
        /// Create the schedule disabled.
        #[arg(long)]
        disabled: bool,
    },
    /// List a workflow's named instances.
    List {
        workflow: String,
        #[arg(long, default_value = "100")]
        limit: u32,
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    /// Show one instance.
    Inspect { workflow: String, instance: String },
    /// Delete an instance. In-flight executions are unaffected.
    Delete { workflow: String, instance: String },
}

impl InstanceCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        let tenant = client.ctx().tenant_segment().to_string();

        match self.verb {
            InstanceVerb::Create {
                workflow,
                instance,
                params,
                params_file,
                cron,
                timezone,
                disabled,
            } => {
                let merged = merge_params(&params, params_file.as_deref())?;
                let mut body = serde_json::json!({
                    "instance_name": instance,
                    "params": merged,
                });
                if let Some(c) = cron {
                    body["cron"] = serde_json::json!(c);
                }
                if let Some(tz) = timezone {
                    body["timezone"] = serde_json::json!(tz);
                }
                if disabled {
                    body["enabled"] = serde_json::json!(false);
                }
                let resp: serde_json::Value = client
                    .post(
                        &format!("/v1/tenants/{tenant}/workflows/{workflow}/instances"),
                        &body,
                    )
                    .await?;
                render::object(&resp, output)
            }
            InstanceVerb::List {
                workflow,
                limit,
                offset,
            } => {
                let body: serde_json::Value = client
                    .get(&format!(
                        "/v1/tenants/{tenant}/workflows/{workflow}/instances?limit={limit}&offset={offset}"
                    ))
                    .await?;
                render::list(&body, output)
            }
            InstanceVerb::Inspect { workflow, instance } => {
                let body: serde_json::Value = client
                    .get(&format!(
                        "/v1/tenants/{tenant}/workflows/{workflow}/instances/{instance}"
                    ))
                    .await?;
                render::object(&body, output)
            }
            InstanceVerb::Delete { workflow, instance } => {
                client
                    .delete(&format!(
                        "/v1/tenants/{tenant}/workflows/{workflow}/instances/{instance}"
                    ))
                    .await?;
                println!("Deleted instance '{}' of '{}'", instance, workflow);
                Ok(())
            }
        }
    }
}

/// Combine `--params <file>` with repeated `--param k=v`.
///
/// The file provides the base object and explicit `--param` flags override it,
/// so a shared params file can be reused with a couple of per-instance tweaks.
fn merge_params(
    pairs: &[String],
    file: Option<&str>,
) -> Result<serde_json::Map<String, serde_json::Value>, CliError> {
    let mut out = match file {
        None => serde_json::Map::new(),
        Some(src) => {
            let buf = if src == "-" {
                let mut b = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut b)
                    .map_err(CliError::Io)?;
                b
            } else {
                std::fs::read_to_string(PathBuf::from(src)).map_err(CliError::Io)?
            };
            match serde_json::from_str::<serde_json::Value>(&buf)
                .map_err(|e| CliError::UserError(e.to_string()))?
            {
                serde_json::Value::Object(m) => m,
                _ => {
                    return Err(CliError::UserError(
                        "--params file must contain a JSON object".to_string(),
                    ))
                }
            }
        }
    };

    for pair in pairs {
        let (k, v) = pair.split_once('=').ok_or_else(|| {
            CliError::UserError(format!("--param must be KEY=VALUE, got '{}'", pair))
        })?;
        if k.is_empty() {
            return Err(CliError::UserError(format!(
                "--param key must not be empty in '{}'",
                pair
            )));
        }
        // A value that parses as JSON is bound as that JSON type, so numbers
        // and booleans reach a typed slot correctly; anything else is a string.
        let value = serde_json::from_str::<serde_json::Value>(v)
            .unwrap_or_else(|_| serde_json::Value::String(v.to_string()));
        out.insert(k.to_string(), value);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_values_are_typed_when_they_parse_as_json() {
        let m = merge_params(
            &[
                "count=5".to_string(),
                "ratio=1.5".to_string(),
                "on=true".to_string(),
                "mode=copy".to_string(),
            ],
            None,
        )
        .unwrap();
        assert_eq!(m["count"], serde_json::json!(5));
        assert_eq!(m["ratio"], serde_json::json!(1.5));
        assert_eq!(m["on"], serde_json::json!(true));
        // Bare text is not valid JSON, so it stays a string rather than erroring.
        assert_eq!(m["mode"], serde_json::json!("copy"));
    }

    #[test]
    fn a_value_containing_equals_is_kept_whole() {
        let m = merge_params(&["expr=a=b".to_string()], None).unwrap();
        assert_eq!(m["expr"], serde_json::json!("a=b"));
    }

    #[test]
    fn malformed_pairs_are_rejected() {
        assert!(merge_params(&["novalue".to_string()], None).is_err());
        assert!(merge_params(&["=novalue".to_string()], None).is_err());
    }
}
