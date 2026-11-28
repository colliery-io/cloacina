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

use serde_json::Value;

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::{GlobalOpts, OutputFormat};

/// Which packages `package list` shows (CLOACINA-T-1085).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Only {
    /// Every package of the tenant.
    All,
    /// The tenant's packages that need a new publish.
    NeedsNewBuild,
    /// The packages of every tenant that need a new publish (platform
    /// admin).
    NeedsNewBuildAllTenants,
}

pub async fn run(globals: &GlobalOpts, filter: Option<&str>, only: Only) -> Result<(), CliError> {
    let config = CloacinaConfig::load(&globals.home.join("config.toml"));
    let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
    let output = ctx.output;
    let client = CliClient::new(ctx)?;

    let path = match only {
        Only::NeedsNewBuildAllTenants => "/v1/packages/needs-new-build".to_string(),
        _ => {
            let tenant = client.ctx().tenant_segment().to_string();
            format!("/v1/tenants/{tenant}/workflows")
        }
    };
    let body: Value = client.get(&path).await?;
    // The server returns the unified `TenantListResponse` envelope
    // (`{tenant_id, items, total}`, CLOACI-T-0594). Read `items`; fall back to
    // the legacy `workflows` key for older servers.
    let items: Vec<Value> = body
        .get("items")
        .or_else(|| body.get("workflows"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let items = select(items, filter, only);
    match only {
        Only::All => render_list(&items, output),
        _ => render_needs_new_build(&items, output, only == Only::NeedsNewBuildAllTenants),
    }
}

/// The items `package list` shows: by name filter, and for
/// `--needs-new-build` only the packages that carry the mark.
fn select(items: Vec<Value>, filter: Option<&str>, only: Only) -> Vec<Value> {
    items
        .into_iter()
        .filter(|v| match filter {
            Some(pat) => v
                .get("package_name")
                .and_then(|n| n.as_str())
                .is_some_and(|n| n.contains(pat)),
            None => true,
        })
        .filter(|v| match only {
            Only::NeedsNewBuild => v.get("needs_new_build").is_some_and(|m| !m.is_null()),
            _ => true,
        })
        .collect()
}

/// One row of the needs-new-build table: tenant, name, version, the two
/// interface versions and the reason. The tenant list carries the mark in
/// `needs_new_build`; the list across tenants carries it flat.
fn needs_new_build_row(item: &Value) -> [String; 6] {
    let text = |v: Option<&Value>| match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => "?".to_string(),
    };
    let mark = item.get("needs_new_build").unwrap_or(item);
    [
        text(item.get("tenant_id")),
        text(item.get("package_name")),
        text(item.get("version")),
        text(mark.get("built_for")),
        text(mark.get("current")),
        text(mark.get("reason")),
    ]
}

fn render_needs_new_build(
    items: &[Value],
    format: OutputFormat,
    with_tenant: bool,
) -> Result<(), CliError> {
    match format {
        OutputFormat::Table => {
            if items.is_empty() {
                println!("No package needs a new publish.");
                return Ok(());
            }
            if with_tenant {
                println!(
                    "{:<16} {:<30} {:<10} {:<10} {:<8}",
                    "TENANT", "NAME", "VERSION", "BUILT FOR", "LOADS"
                );
            } else {
                println!(
                    "{:<30} {:<10} {:<10} {:<8}",
                    "NAME", "VERSION", "BUILT FOR", "LOADS"
                );
            }
            for item in items {
                let [tenant, name, version, built_for, current, reason] = needs_new_build_row(item);
                if with_tenant {
                    println!("{tenant:<16} {name:<30} {version:<10} {built_for:<10} {current:<8}");
                } else {
                    println!("{name:<30} {version:<10} {built_for:<10} {current:<8}");
                }
                println!("  {reason}");
            }
            Ok(())
        }
        other => render_list(items, other),
    }
}

fn render_list(items: &[Value], format: OutputFormat) -> Result<(), CliError> {
    match format {
        OutputFormat::Json => {
            let s = serde_json::to_string_pretty(items)
                .map_err(|e| CliError::UserError(e.to_string()))?;
            println!("{s}");
        }
        OutputFormat::Yaml => {
            let s = serde_yaml::to_string(items).map_err(|e| CliError::UserError(e.to_string()))?;
            print!("{s}");
        }
        OutputFormat::Id => {
            for item in items {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    println!("{id}");
                }
            }
        }
        OutputFormat::Table => {
            if items.is_empty() {
                println!("No items.");
                return Ok(());
            }
            println!(
                "{:<12} {:<30} {:<10} {:<20}",
                "ID", "NAME", "VERSION", "CREATED"
            );
            for item in items {
                let id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(truncate_id)
                    .unwrap_or_else(|| "?".into());
                let name = item
                    .get("package_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let version = item.get("version").and_then(|v| v.as_str()).unwrap_or("?");
                let created = item
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                println!("{:<12} {:<30} {:<10} {:<20}", id, name, version, created);
            }
        }
    }
    Ok(())
}

fn truncate_id(id: &str) -> String {
    if id.len() <= 8 {
        id.to_string()
    } else {
        format!("{}…", &id[..8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// CLOACINA-T-1085: `--needs-new-build` keeps the marked packages, and a
    /// row names the versions and the reason, from the tenant list and from
    /// the list across tenants.
    #[test]
    fn needs_new_build_keeps_the_marked_packages_with_both_versions() {
        let items = vec![
            json!({"package_name": "fine", "version": "1.0.0"}),
            json!({
                "package_name": "old-pin",
                "version": "2.0.0",
                "plugin_interface_version": 7,
                "needs_new_build": {"reason": "change the pin", "built_for": 7, "current": 8},
            }),
        ];
        let kept = select(items.clone(), None, Only::NeedsNewBuild);
        assert_eq!(kept.len(), 1);
        let row = needs_new_build_row(&kept[0]);
        assert_eq!(row[1], "old-pin");
        assert_eq!(row[3], "7");
        assert_eq!(row[4], "8");
        assert_eq!(row[5], "change the pin");
        assert_eq!(select(items, None, Only::All).len(), 2);

        let across = json!({
            "tenant_id": "acme",
            "package_name": "old-pin",
            "version": "2.0.0",
            "built_for": 7,
            "current": 8,
            "reason": "change the pin",
        });
        let row = needs_new_build_row(&across);
        assert_eq!(row[0], "acme");
        assert_eq!(row[3], "7");
        assert_eq!(row[4], "8");
    }
}
