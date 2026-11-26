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

//! `cloacinactl package versions <name>` — every stored version of a package,
//! and `cloacinactl package activate <name> <version>` — make an earlier
//! built version the active one (CLOACINA-T-1018).

use serde_json::Value;

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::{GlobalOpts, OutputFormat};

/// Path of the versions list of a package.
pub(crate) fn versions_path(tenant: &str, name: &str) -> String {
    format!("/v1/tenants/{tenant}/workflows/{name}/versions")
}

/// Path of the activation of one version of a package.
pub(crate) fn activate_path(tenant: &str, name: &str, version: &str) -> String {
    format!("/v1/tenants/{tenant}/workflows/{name}/versions/{version}/activate")
}

pub async fn list(globals: &GlobalOpts, name: &str) -> Result<(), CliError> {
    let config = CloacinaConfig::load(&globals.home.join("config.toml"));
    let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
    let output = ctx.output;
    let client = CliClient::new(ctx)?;

    let tenant = client.ctx().tenant_segment().to_string();
    let body: Value = client.get(&versions_path(&tenant, name)).await?;
    let items: Vec<Value> = body
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    match output {
        OutputFormat::Json => {
            let s = serde_json::to_string_pretty(&items)
                .map_err(|e| CliError::UserError(e.to_string()))?;
            println!("{s}");
        }
        OutputFormat::Yaml => {
            let s =
                serde_yaml::to_string(&items).map_err(|e| CliError::UserError(e.to_string()))?;
            print!("{s}");
        }
        OutputFormat::Id => {
            for item in &items {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    println!("{id}");
                }
            }
        }
        OutputFormat::Table => {
            for line in table_lines(&items) {
                println!("{line}");
            }
        }
    }
    Ok(())
}

pub async fn activate(globals: &GlobalOpts, name: &str, version: &str) -> Result<(), CliError> {
    let config = CloacinaConfig::load(&globals.home.join("config.toml"));
    let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
    let client = CliClient::new(ctx)?;

    let tenant = client.ctx().tenant_segment().to_string();
    let body: Value = client
        .post(
            &activate_path(&tenant, name, version),
            &serde_json::json!({}),
        )
        .await?;
    let id = body.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    println!("{name} {version} is active ({id})");
    Ok(())
}

/// The state column of the table: what the row is to an operator.
fn state_of(item: &Value) -> &'static str {
    let flag = |k: &str| item.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    let status = item
        .get("build_status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if flag("active") {
        "active"
    } else if flag("superseded") {
        match status {
            "success" => "superseded",
            _ => "abandoned",
        }
    } else {
        match status {
            "pending" => "pending",
            "building" => "building",
            "failed" => "failed",
            _ => "?",
        }
    }
}

fn table_lines(items: &[Value]) -> Vec<String> {
    if items.is_empty() {
        return vec!["No versions.".to_string()];
    }
    let mut lines = vec![format!(
        "{:<12} {:<12} {:<11} {:<9} {:<26}",
        "VERSION", "STATE", "BUILD", "ID", "UPLOADED"
    )];
    for item in items {
        let text = |k: &str| item.get(k).and_then(|v| v.as_str()).unwrap_or("?");
        let id: String = text("id").chars().take(8).collect();
        lines.push(format!(
            "{:<12} {:<12} {:<11} {:<9} {:<26}",
            text("version"),
            state_of(item),
            text("build_status"),
            id,
            text("created_at"),
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn paths_address_the_package_by_name_and_version() {
        assert_eq!(
            versions_path("acme", "team-jobs"),
            "/v1/tenants/acme/workflows/team-jobs/versions"
        );
        assert_eq!(
            activate_path("acme", "team-jobs", "1.2.0"),
            "/v1/tenants/acme/workflows/team-jobs/versions/1.2.0/activate"
        );
    }

    #[test]
    fn state_names_each_row_of_the_lifecycle() {
        let row = |active: bool, superseded: bool, status: &str| json!({"active": active, "superseded": superseded, "build_status": status});
        assert_eq!(state_of(&row(true, false, "success")), "active");
        assert_eq!(state_of(&row(false, true, "success")), "superseded");
        assert_eq!(state_of(&row(false, true, "failed")), "abandoned");
        assert_eq!(state_of(&row(false, true, "pending")), "abandoned");
        assert_eq!(state_of(&row(false, false, "pending")), "pending");
        assert_eq!(state_of(&row(false, false, "building")), "building");
        assert_eq!(state_of(&row(false, false, "failed")), "failed");
    }

    #[test]
    fn table_has_a_header_and_one_line_per_version() {
        assert_eq!(table_lines(&[]), vec!["No versions.".to_string()]);
        let items = vec![
            json!({
                "id": "0a1b2c3d-0000-0000-0000-000000000000",
                "version": "2.0.0",
                "active": true,
                "superseded": false,
                "build_status": "success",
                "created_at": "2026-01-02T00:00:00+00:00",
            }),
            json!({
                "id": "9f8e7d6c-0000-0000-0000-000000000000",
                "version": "1.0.0",
                "active": false,
                "superseded": true,
                "build_status": "success",
                "created_at": "2026-01-01T00:00:00+00:00",
            }),
        ];
        let lines = table_lines(&items);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("VERSION"));
        assert!(lines[1].contains("2.0.0") && lines[1].contains("active"));
        assert!(lines[1].contains("0a1b2c3d") && !lines[1].contains("0a1b2c3d-"));
        assert!(lines[2].contains("1.0.0") && lines[2].contains("superseded"));
    }
}
