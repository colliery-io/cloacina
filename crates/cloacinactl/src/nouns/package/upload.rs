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

use super::sign::UploadSignature;
use crate::GlobalOpts;
use crate::commands::config::CloacinaConfig;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;

/// Upload the archive. `signature` is the signature that goes with it
/// (CLOACINA-T-1062), sent as the `signature` field of the upload; `None`
/// sends none.
pub async fn run(
    globals: &GlobalOpts,
    file: &Path,
    signature: Option<UploadSignature>,
) -> Result<(), CliError> {
    let config = CloacinaConfig::load(&globals.home.join("config.toml"));
    let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;

    let bytes = std::fs::read(file).map_err(CliError::Io)?;
    let filename = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "package.cloacina".to_string());
    let signature = match signature {
        Some(signature) => signature.resolve(file, &bytes)?,
        None => None,
    };

    let part = multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str("application/octet-stream")
        .map_err(CliError::from_reqwest)?;
    let mut form = multipart::Form::new().part("file", part);
    if let Some(signature) = signature {
        form = form.text("signature", signature);
    }

    let http = reqwest::Client::new();
    let tenant = ctx.tenant_segment();
    let url = format!(
        "{}/v1/tenants/{}/workflows",
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
        // CLOACINA-T-1123: the limit that holds the runs of the package is a
        // warning on stderr, not a quiet field of the answer.
        if let Some(notice) = priority_warning(&body) {
            eprintln!("{notice}");
        }
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

/// The warning line of an upload answer: the server's `priority_notice`,
/// with a `warning:` prefix. `None` when the answer has no notice.
fn priority_warning(response: &serde_json::Value) -> Option<String> {
    response
        .get("priority_notice")
        .and_then(|notice| notice.as_str())
        .map(|notice| format!("warning: {notice}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_priority_notice_of_an_upload_is_a_warning() {
        let held = serde_json::json!({
            "package_id": "p",
            "tenant_id": "acme",
            "priority_limit": 5,
            "priority_notice": "the runs of this package are held to priority 5 or lower",
        });
        assert_eq!(
            priority_warning(&held).as_deref(),
            Some("warning: the runs of this package are held to priority 5 or lower")
        );
        let admin = serde_json::json!({"package_id": "p", "priority_limit": 0});
        assert_eq!(priority_warning(&admin), None);
    }
}
