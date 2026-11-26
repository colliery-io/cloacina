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

//! `cloacinactl audit <verb>` (CLOACINA-T-1024): read the audit record —
//! who did what, when, in which tenant.

use clap::{Args, Subcommand};

use crate::GlobalOpts;
use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;

#[derive(Args)]
pub struct AuditCmd {
    #[command(subcommand)]
    verb: AuditVerb,
}

#[derive(Subcommand)]
enum AuditVerb {
    /// Audit rows, newest first. Reads the tenant of the profile (tenant
    /// admin key); `--all-tenants` reads every tenant (platform admin key).
    List {
        /// Read every tenant and platform actions (platform admin key).
        #[arg(long)]
        all_tenants: bool,
        /// Only rows with this action, for example `package.upload`.
        #[arg(long)]
        action: Option<String>,
        /// Only rows whose key id, key name or identity equals this.
        #[arg(long)]
        actor: Option<String>,
        /// Only rows at or after this RFC 3339 time.
        #[arg(long)]
        since: Option<String>,
        /// Only rows before this RFC 3339 time.
        #[arg(long)]
        until: Option<String>,
        /// Maximum number of rows (server-side cap: 200).
        #[arg(long, default_value = "50")]
        limit: u32,
        /// Read rows older than this row id (the cursor of the page before).
        #[arg(long)]
        cursor: Option<i64>,
    },
}

/// The query string of an audit list, with each value percent-encoded (an
/// identity such as `oidc:https://idp.example:sub` and a time with `+` need
/// it).
fn list_query(
    action: Option<String>,
    actor: Option<String>,
    since: Option<String>,
    until: Option<String>,
    limit: u32,
    cursor: Option<i64>,
) -> String {
    let mut pairs: Vec<(&str, String)> = vec![("limit", limit.to_string())];
    for (name, value) in [
        ("action", action),
        ("actor", actor),
        ("since", since),
        ("until", until),
        ("cursor", cursor.map(|c| c.to_string())),
    ] {
        if let Some(v) = value {
            pairs.push((name, v));
        }
    }
    reqwest::Url::parse_with_params("http://localhost/", &pairs)
        .ok()
        .and_then(|u| u.query().map(|q| format!("?{q}")))
        .unwrap_or_default()
}

impl AuditCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        match self.verb {
            AuditVerb::List {
                all_tenants,
                action,
                actor,
                since,
                until,
                limit,
                cursor,
            } => {
                let query = list_query(action, actor, since, until, limit, cursor);
                let path = if all_tenants {
                    format!("/v1/audit{query}")
                } else {
                    let tenant = client.ctx().tenant_segment().to_string();
                    format!("/v1/tenants/{tenant}/audit{query}")
                };
                let body: serde_json::Value = client.get(&path).await?;
                render::list(&body, output)?;
                if let Some(next) = body.get("next_cursor").and_then(|v| v.as_i64()) {
                    eprintln!("More rows: run again with --cursor {next}");
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_list_query_encodes_values() {
        let q = list_query(
            Some("package.upload".into()),
            Some("oidc:https://idp.example:sub 1".into()),
            Some("2026-01-02T03:04:05+00:00".into()),
            None,
            25,
            Some(7),
        );
        assert!(q.starts_with("?limit=25&action=package.upload&actor="));
        assert!(!q.contains(' '));
        assert!(!q.contains("+00"), "a `+` must be encoded: {q}");
        assert!(q.contains("since=2026-01-02T03%3A04%3A05%2B00%3A00"));
        assert!(q.ends_with("&cursor=7"));
        assert!(!q.contains("until"));
    }

    #[test]
    fn audit_list_query_with_no_filter_has_only_the_limit() {
        assert_eq!(list_query(None, None, None, None, 50, None), "?limit=50");
    }
}
