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

//! `cloacinactl reactor <verb>` — operator actions on reactors loaded in the
//! server's graph scheduler (CLOACI-T-0751).
//!
//! Backed by `POST /v1/health/reactors/{name}/fire`. Lets an operator inject a
//! single event into a running computation graph for a manual operational
//! check, without hand-crafting raw boundary bytes. Typed JSON inputs are
//! serialized to the boundary encoding server-side.

use std::collections::HashMap;

use clap::{Args, Subcommand};

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;
use crate::GlobalOpts;

#[derive(Args)]
pub struct ReactorCmd {
    #[command(subcommand)]
    verb: ReactorVerb,
}

#[derive(Subcommand)]
enum ReactorVerb {
    /// Force-fire a reactor with its CURRENT cache (no input injected).
    ForceFire {
        /// Reactor name.
        name: String,
    },
    /// Fire a reactor after replacing its cache with typed inputs.
    ///
    /// Each `--input` is `source=<json>`; the JSON value is serialized to the
    /// boundary encoding server-side. Full-replace only (no partial merge).
    Fire {
        /// Reactor name.
        name: String,
        /// Per-source typed input as `source=<json>` (repeatable). Example:
        /// `--input prices='{"sym":"ABC","px":12.5}'`.
        #[arg(long = "input", value_name = "SOURCE=JSON", required = true)]
        inputs: Vec<String>,
    },
}

impl ReactorCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;

        match self.verb {
            ReactorVerb::ForceFire { name } => {
                let body = serde_json::json!({ "mode": "force_fire" });
                let resp: serde_json::Value = client
                    .post(&format!("/v1/health/reactors/{name}/fire"), &body)
                    .await?;
                render::object(&resp, output)
            }
            ReactorVerb::Fire { name, inputs } => {
                let parsed = parse_inputs(&inputs)?;
                let body = serde_json::json!({
                    "mode": "fire_with",
                    "inputs": parsed,
                });
                let resp: serde_json::Value = client
                    .post(&format!("/v1/health/reactors/{name}/fire"), &body)
                    .await?;
                render::object(&resp, output)
            }
        }
    }
}

/// Parse `source=<json>` pairs into a JSON map. The value after the first `=`
/// is parsed as JSON; if it isn't valid JSON it is treated as a JSON string so
/// `--input note=hello` works without quoting.
fn parse_inputs(pairs: &[String]) -> Result<HashMap<String, serde_json::Value>, CliError> {
    let mut map = HashMap::with_capacity(pairs.len());
    for pair in pairs {
        let (source, raw) = pair.split_once('=').ok_or_else(|| {
            CliError::Other(anyhow::anyhow!(
                "invalid --input '{pair}': expected SOURCE=JSON"
            ))
        })?;
        if source.is_empty() {
            return Err(CliError::Other(anyhow::anyhow!(
                "invalid --input '{pair}': empty source name"
            )));
        }
        let value = serde_json::from_str::<serde_json::Value>(raw)
            .unwrap_or_else(|_| serde_json::Value::String(raw.to_string()));
        map.insert(source.to_string(), value);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_inputs_parses_json_and_falls_back_to_string() {
        let pairs = vec![
            "a={\"x\":1}".to_string(),
            "b=42".to_string(),
            "c=hello".to_string(),
        ];
        let map = parse_inputs(&pairs).unwrap();
        assert_eq!(map["a"], serde_json::json!({"x": 1}));
        assert_eq!(map["b"], serde_json::json!(42));
        assert_eq!(map["c"], serde_json::json!("hello"));
    }

    #[test]
    fn parse_inputs_rejects_missing_equals() {
        let pairs = vec!["bad".to_string()];
        assert!(parse_inputs(&pairs).is_err());
    }
}
