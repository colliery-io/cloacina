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

use std::io::{self, BufRead, IsTerminal, Write};

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::GlobalOpts;

pub async fn run(globals: &GlobalOpts, id: &str, force: bool) -> Result<(), CliError> {
    if !force && io::stdin().is_terminal() {
        print!("delete package {id}? [y/N] ");
        io::stdout().flush().ok();
        let mut line = String::new();
        io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(CliError::Io)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            return Err(CliError::UserError("cancelled".into()));
        }
    }

    let config = CloacinaConfig::load(&globals.home.join("config.toml"));
    let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
    let client = CliClient::new(ctx)?;
    let tenant = client.ctx().tenant_segment().to_string();

    // Server's delete handler is keyed on (name, version). Look up the row
    // first so users can pass a UUID or a package name.
    let body: serde_json::Value = client
        .get(&format!("/tenants/{tenant}/workflows/{id}"))
        .await?;
    let name = body
        .get("package_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::UserError("workflow response missing package_name".to_string()))?;
    let version = body
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::UserError("workflow response missing version".to_string()))?;

    client
        .delete(&format!("/tenants/{tenant}/workflows/{name}/{version}"))
        .await?;
    println!("deleted {id}");
    Ok(())
}
