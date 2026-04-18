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

use std::path::Path;

use reqwest::multipart;

use crate::commands::config::CloacinaConfig;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::GlobalOpts;

pub async fn run(globals: &GlobalOpts, file: &Path) -> Result<(), CliError> {
    let config = CloacinaConfig::load(&globals.home.join("config.toml"));
    let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;

    let bytes = std::fs::read(file).map_err(CliError::Io)?;
    let filename = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "package.cloacina".to_string());

    let part = multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str("application/octet-stream")
        .map_err(CliError::from_reqwest)?;
    let form = multipart::Form::new().part("package", part);

    let http = reqwest::Client::new();
    let tenant = ctx.tenant_segment();
    let url = format!(
        "{}/tenants/{}/workflows",
        ctx.server.trim_end_matches('/'),
        tenant
    );
    let response = http
        .post(&url)
        .bearer_auth(&ctx.api_key)
        .multipart(form)
        .send()
        .await
        .map_err(CliError::from_reqwest)?;
    let status = response.status().as_u16();
    if response.status().is_success() {
        let body: serde_json::Value = response.json().await.map_err(CliError::from_reqwest)?;
        if let Some(id) = body
            .get("package_id")
            .or_else(|| body.get("id"))
            .and_then(|v| v.as_str())
        {
            println!("{id}");
        } else {
            println!("{body}");
        }
        return Ok(());
    }
    let body = response
        .json::<serde_json::Value>()
        .await
        .unwrap_or(serde_json::Value::Null);
    Err(CliError::from_status(status, body))
}
