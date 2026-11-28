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

//! `cloacinactl backfill` — load history for one workflow (CLOACINA-T-1020).
//!
//! A backfill runs a workflow once for each fire time of a cron expression
//! from a start to an end, oldest first, with its own limit on active runs.
//! Each run gets `interval_start`, `interval_end` and `backfill_id` in its
//! context.

use clap::{Args, Subcommand};

use crate::GlobalOpts;
use crate::commands::config::CloacinaConfig;
use crate::nouns::instance::merge_params;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;

#[derive(Args)]
pub struct BackfillCmd {
    #[command(subcommand)]
    verb: BackfillVerb,
}

#[derive(Subcommand)]
enum BackfillVerb {
    /// Create a backfill: one run for each fire time from --start to --end.
    Create {
        /// Workflow name.
        workflow: String,
        /// Start of the range, an RFC 3339 timestamp
        /// (`2026-01-01T00:00:00Z`). The backfill takes each fire time from
        /// the start to the end, both included.
        #[arg(
            long,
            value_name = "TIME",
            required_unless_present = "interval",
            requires = "end"
        )]
        start: Option<String>,
        /// End of the range, an RFC 3339 timestamp.
        #[arg(
            long,
            value_name = "TIME",
            required_unless_present = "interval",
            requires = "start"
        )]
        end: Option<String>,
        /// One interval only: its fire time (the end of the interval), an
        /// RFC 3339 timestamp. The same as --start and --end with this value.
        #[arg(long, value_name = "TIME", conflicts_with_all = ["start", "end"])]
        interval: Option<String>,
        /// Cron expression that gives the intervals. Omit it to use the cron
        /// schedule of the workflow, when it has exactly one.
        #[arg(long)]
        cron: Option<String>,
        /// IANA timezone for the cron expression (default: UTC, or the
        /// timezone of the workflow's schedule when --cron is omitted).
        #[arg(long)]
        timezone: Option<String>,
        /// The most runs of the backfill that may be active at once
        /// (default: 1).
        #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
        max_active_runs: Option<u32>,
        /// Set one parameter of each run as `key=value`. Repeatable. The
        /// value is parsed as JSON when it parses and taken as a string
        /// otherwise.
        #[arg(long = "param", value_name = "KEY=VALUE")]
        params: Vec<String>,
        /// Read all parameters from a JSON object file, or `-` for stdin.
        /// Merged under `--param`.
        #[arg(long = "params", value_name = "FILE")]
        params_file: Option<String>,
        /// The workflow priority of each run, 0 (the highest) to 10
        /// (CLOACINA-T-1123). Each run is held to your limit when it starts.
        /// Not set: the priority the workflow declares.
        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=10))]
        priority: Option<u8>,
    },
    /// List the backfills of a workflow, newest first, with their progress.
    List {
        workflow: String,
        #[arg(long, default_value = "100")]
        limit: u32,
        #[arg(long, default_value = "0")]
        offset: u32,
    },
    /// Show one backfill: its progress and its failed intervals.
    Inspect { workflow: String, id: String },
    /// Stop a backfill. No new interval gets a run; active runs end on
    /// their own.
    Cancel { workflow: String, id: String },
    /// Create a new backfill from the failed intervals of an earlier one.
    Retry {
        workflow: String,
        /// Id of the backfill whose failed intervals run again.
        id: String,
        /// The limit on active runs of the new backfill (default: the limit
        /// of the earlier one).
        #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
        max_active_runs: Option<u32>,
    },
}

/// The body of the create request.
#[allow(clippy::too_many_arguments)]
fn create_body(
    start: Option<String>,
    end: Option<String>,
    interval: Option<String>,
    cron: Option<String>,
    timezone: Option<String>,
    max_active_runs: Option<u32>,
    params: serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Value, CliError> {
    let (start, end) = match (interval, start, end) {
        (Some(fire), _, _) => (fire.clone(), fire),
        (None, Some(start), Some(end)) => (start, end),
        _ => {
            return Err(CliError::UserError(
                "give --start and --end, or --interval".to_string(),
            ));
        }
    };
    let mut body = serde_json::json!({ "start": start, "end": end });
    if let Some(cron) = cron {
        body["cron"] = serde_json::json!(cron);
    }
    if let Some(timezone) = timezone {
        body["timezone"] = serde_json::json!(timezone);
    }
    if let Some(limit) = max_active_runs {
        body["max_active_runs"] = serde_json::json!(limit);
    }
    if !params.is_empty() {
        body["params"] = serde_json::Value::Object(params);
    }
    Ok(body)
}

impl BackfillCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        let tenant = client.ctx().tenant_segment().to_string();

        match self.verb {
            BackfillVerb::Create {
                workflow,
                start,
                end,
                interval,
                cron,
                timezone,
                max_active_runs,
                params,
                params_file,
                priority,
            } => {
                let merged = merge_params(&params, params_file.as_deref())?;
                let mut body = create_body(
                    start,
                    end,
                    interval,
                    cron,
                    timezone,
                    max_active_runs,
                    merged,
                )?;
                if let Some(priority) = priority {
                    body["priority"] = serde_json::json!(priority);
                }
                let resp: serde_json::Value = client
                    .post(
                        &format!("/v1/tenants/{tenant}/workflows/{workflow}/backfills"),
                        &body,
                    )
                    .await?;
                // CLOACINA-T-1123: a priority above the creator's limit is a
                // warning on stderr.
                if let Some(notice) = resp.get("priority_notice").and_then(|n| n.as_str()) {
                    eprintln!("warning: {notice}");
                }
                render::object(&resp, output)
            }
            BackfillVerb::List {
                workflow,
                limit,
                offset,
            } => {
                let body: serde_json::Value = client
                    .get(&format!(
                        "/v1/tenants/{tenant}/workflows/{workflow}/backfills?limit={limit}&offset={offset}"
                    ))
                    .await?;
                render::list(&body, output)
            }
            BackfillVerb::Inspect { workflow, id } => {
                let body: serde_json::Value = client
                    .get(&format!(
                        "/v1/tenants/{tenant}/workflows/{workflow}/backfills/{id}"
                    ))
                    .await?;
                render::object(&body, output)
            }
            BackfillVerb::Cancel { workflow, id } => {
                let resp: serde_json::Value = client
                    .post(
                        &format!("/v1/tenants/{tenant}/workflows/{workflow}/backfills/{id}/cancel"),
                        &serde_json::json!({}),
                    )
                    .await?;
                render::object(&resp, output)
            }
            BackfillVerb::Retry {
                workflow,
                id,
                max_active_runs,
            } => {
                let mut body = serde_json::json!({});
                if let Some(limit) = max_active_runs {
                    body["max_active_runs"] = serde_json::json!(limit);
                }
                let resp: serde_json::Value = client
                    .post(
                        &format!("/v1/tenants/{tenant}/workflows/{workflow}/backfills/{id}/retry"),
                        &body,
                    )
                    .await?;
                render::object(&resp, output)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(flatten)]
        cmd: BackfillCmd,
    }

    fn parse(args: &[&str]) -> Result<BackfillVerb, clap::Error> {
        let mut full = vec!["backfill"];
        full.extend_from_slice(args);
        TestCli::try_parse_from(full).map(|cli| cli.cmd.verb)
    }

    #[test]
    fn backfill_create_builds_the_request_body() {
        let Ok(BackfillVerb::Create {
            workflow,
            start,
            end,
            interval,
            cron,
            timezone,
            max_active_runs,
            params,
            params_file,
            priority,
        }) = parse(&[
            "create",
            "hourly_build",
            "--start",
            "2026-01-01T00:00:00Z",
            "--end",
            "2026-08-18T23:00:00Z",
            "--cron",
            "0 * * * *",
            "--max-active-runs",
            "8",
            "--param",
            "region=eu",
            "--priority",
            "9",
        ])
        else {
            panic!("create must parse");
        };
        assert_eq!(workflow, "hourly_build");
        assert_eq!(priority, Some(9), "CLOACINA-T-1123");
        assert!(
            parse(&[
                "create",
                "w",
                "--interval",
                "2026-01-01T00:00:00Z",
                "--priority",
                "11"
            ])
            .is_err()
        );
        let merged = merge_params(&params, params_file.as_deref()).unwrap();
        let body = create_body(
            start,
            end,
            interval,
            cron,
            timezone,
            max_active_runs,
            merged,
        )
        .unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "start": "2026-01-01T00:00:00Z",
                "end": "2026-08-18T23:00:00Z",
                "cron": "0 * * * *",
                "max_active_runs": 8,
                "params": {"region": "eu"},
            })
        );
    }

    #[test]
    fn backfill_create_of_one_interval_sets_start_and_end() {
        let Ok(BackfillVerb::Create {
            start,
            end,
            interval,
            ..
        }) = parse(&["create", "wf", "--interval", "2026-03-04T07:00:00Z"])
        else {
            panic!("create must parse");
        };
        let body = create_body(
            start,
            end,
            interval,
            None,
            None,
            None,
            serde_json::Map::new(),
        )
        .unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "start": "2026-03-04T07:00:00Z",
                "end": "2026-03-04T07:00:00Z",
            })
        );
    }

    #[test]
    fn backfill_create_needs_a_range_or_one_interval() {
        assert!(parse(&["create", "wf"]).is_err());
        assert!(parse(&["create", "wf", "--start", "2026-01-01T00:00:00Z"]).is_err());
        assert!(
            parse(&[
                "create",
                "wf",
                "--interval",
                "2026-01-01T00:00:00Z",
                "--end",
                "2026-01-02T00:00:00Z"
            ])
            .is_err()
        );
        assert!(
            parse(&[
                "create",
                "wf",
                "--interval",
                "2026-01-01T00:00:00Z",
                "--max-active-runs",
                "0"
            ])
            .is_err()
        );
    }

    #[test]
    fn backfill_verbs_parse() {
        assert!(matches!(
            parse(&["list", "wf"]),
            Ok(BackfillVerb::List {
                limit: 100,
                offset: 0,
                ..
            })
        ));
        assert!(matches!(
            parse(&["inspect", "wf", "an-id"]),
            Ok(BackfillVerb::Inspect { .. })
        ));
        assert!(matches!(
            parse(&["cancel", "wf", "an-id"]),
            Ok(BackfillVerb::Cancel { .. })
        ));
        assert!(matches!(
            parse(&["retry", "wf", "an-id", "--max-active-runs", "4"]),
            Ok(BackfillVerb::Retry {
                max_active_runs: Some(4),
                ..
            })
        ));
        assert!(parse(&["retry", "wf"]).is_err());
    }
}
