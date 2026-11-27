/*
 *  Copyright 2025-2026 Colliery Software
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

//! Alert webhook for finished runs (CLOACINA-T-1024).
//!
//! When a run ends, the engine writes a `delivery_outbox` row addressed to
//! [`cloacina::run_alerts::RECIPIENT`] in the transaction that changes the
//! status (see `cloacina::run_alerts`). The sender here looks for open rows in
//! each tenant schema on a short tick, claims a row with the outbox's
//! compare-and-set `pending → delivered`, builds the payload from the run's
//! own rows, POSTs it and acks the row.
//!
//! - One replica sends: only one claim of a row succeeds.
//! - A failed send leaves the row claimed. Any replica claims it again after
//!   [`retry_delay`]. After `max_attempts` sends the row is acked and counted
//!   as dropped. A row older than [`MAX_NOTICE_AGE`] is dropped the same way.
//! - Delivery is at least once: the receiver can get an alert twice when its
//!   answer is lost, or when a replica stops between the send and the ack.
//! - The scheduler loop never waits for the receiver: it only inserts a row.
//!
//! The webhook URL can hold a token (a Slack or PagerDuty URL does), and the
//! secret signs the body: neither is ever logged. Log lines name the host.

use std::sync::OnceLock;
use std::time::Duration;

use chrono::{DateTime, Utc};
use cloacina::dal::DAL;
use cloacina::database::Database;
use cloacina::models::delivery_outbox::DeliveryOutbox;
use cloacina::models::task_execution::TaskExecution;
use cloacina::models::workflow_execution::WorkflowExecutionRecord;
use cloacina::run_alerts::{RECIPIENT, RunFinishedNotice, RunOutcome, RunSchedule};
use hmac::{Hmac, KeyInit, Mac};
use openidconnect::reqwest;
use serde::Serialize;
use sha2::Sha256;
use tokio::sync::watch;
use tracing::{debug, info, warn};

use crate::TenantDatabaseCache;

/// Version of the payload shape. It changes only when a field is removed or
/// changes meaning; new fields do not change it.
pub const PAYLOAD_SCHEMA_VERSION: u32 = 1;

/// Header that names the event, for example `workflow_execution.failed`.
pub const EVENT_HEADER: &str = "X-Cloacina-Event";
/// Header that holds the execution id. The same for each retry of an alert.
pub const DELIVERY_HEADER: &str = "X-Cloacina-Delivery";
/// Header that holds `t=<unix seconds>,v1=<hex HMAC-SHA256>`.
pub const SIGNATURE_HEADER: &str = "X-Cloacina-Signature";

/// How often the sender looks for open notice rows.
const POLL_INTERVAL: Duration = Duration::from_secs(5);
/// The most rows of one tenant handled in one pass.
const BATCH_LIMIT: i64 = 50;
/// A notice older than this is dropped, not sent.
pub const MAX_NOTICE_AGE: Duration = Duration::from_secs(24 * 60 * 60);
/// The most failed tasks listed in one payload.
const MAX_FAILED_TASKS: usize = 20;
/// The most characters of an error text in a payload.
const MAX_ERROR_CHARS: usize = 500;

/// Settings of the alert webhook.
#[derive(Clone)]
pub struct AlertWebhookConfig {
    url: reqwest::Url,
    /// The outcomes that send an alert.
    pub outcomes: Vec<RunOutcome>,
    secret: Option<String>,
    /// Time allowed for one request.
    pub timeout: Duration,
    /// The most sends of one alert.
    pub max_attempts: u32,
    /// The public URL of this server, with no trailing slash. For links.
    pub public_base_url: Option<String>,
}

impl std::fmt::Debug for AlertWebhookConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The URL path and the secret stay out of every log line.
        f.debug_struct("AlertWebhookConfig")
            .field("host", &self.host())
            .field("outcomes", &self.outcomes)
            .field("signed", &self.secret.is_some())
            .field("timeout", &self.timeout)
            .field("max_attempts", &self.max_attempts)
            .field("public_base_url", &self.public_base_url)
            .finish()
    }
}

impl AlertWebhookConfig {
    /// Checks the settings. `events` holds `failed`, `completed`, `cancelled`
    /// or `all`; an empty list means `failed`.
    pub fn new(
        url: &str,
        events: &[String],
        secret: Option<String>,
        timeout: Duration,
        max_attempts: u32,
        public_base_url: Option<String>,
    ) -> Result<Self, String> {
        // The parse error can echo the input, so it is not passed on.
        let url = reqwest::Url::parse(url.trim())
            .map_err(|_| "the alert webhook URL is not a valid URL".to_string())?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err("the alert webhook URL must be an http or https URL".to_string());
        }
        if timeout.is_zero() {
            return Err("the alert webhook timeout must be at least 1 second".to_string());
        }
        if max_attempts == 0 {
            return Err("the alert webhook max attempts must be at least 1".to_string());
        }
        Ok(Self {
            url,
            outcomes: parse_outcomes(events)?,
            secret: secret.filter(|s| !s.is_empty()),
            timeout,
            max_attempts,
            public_base_url: public_base_url
                .map(|b| b.trim().trim_end_matches('/').to_string())
                .filter(|b| !b.is_empty()),
        })
    }

    /// The host of the webhook URL, for log lines.
    pub fn host(&self) -> &str {
        self.url.host_str().unwrap_or("")
    }
}

/// Reads the list of outcomes that send an alert.
pub fn parse_outcomes(events: &[String]) -> Result<Vec<RunOutcome>, String> {
    let mut outcomes = Vec::new();
    for raw in events {
        let name = raw.trim().to_ascii_lowercase();
        if name.is_empty() {
            continue;
        }
        let found: Vec<RunOutcome> = if name == "all" {
            RunOutcome::ALL.to_vec()
        } else {
            vec![RunOutcome::parse(&name).ok_or_else(|| {
                format!(
                    "unknown alert webhook event '{name}': use failed, completed, cancelled or all"
                )
            })?]
        };
        for outcome in found {
            if !outcomes.contains(&outcome) {
                outcomes.push(outcome);
            }
        }
    }
    if outcomes.is_empty() {
        outcomes.push(RunOutcome::Failed);
    }
    Ok(outcomes)
}

static INSTALLED: OnceLock<AlertWebhookConfig> = OnceLock::new();

/// Gives the settings to the server. `main` calls it before `run`; `run`
/// starts the sender when it finds them. The first call wins.
///
/// It also switches on the notice rows in the engine, here and not when the
/// sender starts, so that no run can end before the switch is on.
pub fn install(config: AlertWebhookConfig) {
    if INSTALLED.set(config.clone()).is_ok() {
        cloacina::run_alerts::enable(&config.outcomes);
    }
}

/// The settings given with [`install`]. `None` = the webhook is off.
pub fn installed() -> Option<&'static AlertWebhookConfig> {
    INSTALLED.get()
}

/// Describes the metrics of the sender.
pub fn describe_metrics() {
    metrics::describe_counter!(
        "cloacina_alert_webhook_sent_total",
        "Alert webhook requests that the receiver answered with a 2xx status. \
         `event` ∈ `failed`, `completed`, `cancelled`. CLOACINA-T-1024."
    );
    metrics::describe_counter!(
        "cloacina_alert_webhook_failed_total",
        "Alert webhook sends that failed and stay due for a retry. `reason` ∈ \
         `timeout`, `status` (a status outside 2xx), `transport` (no connection, \
         TLS, DNS), `load` (the run could not be read). CLOACINA-T-1024."
    );
    metrics::describe_counter!(
        "cloacina_alert_webhook_dropped_total",
        "Alerts given up without a 2xx answer. `reason` ∈ `attempts` (the \
         configured number of sends failed), `expired` (the notice was older \
         than 24 hours), `invalid` (the notice could not be read). CLOACINA-T-1024."
    );
}

// ---------------------------------------------------------------------------
// Payload
// ---------------------------------------------------------------------------

/// The JSON body of an alert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AlertPayload {
    pub schema_version: u32,
    /// `workflow_execution.failed`, `.completed` or `.cancelled`.
    pub event: String,
    pub tenant: String,
    pub workflow: String,
    pub workflow_version: String,
    /// The package of the workflow. `null` for a workflow compiled into the
    /// binary, and when the run has no task to read it from.
    pub package: Option<String>,
    /// The package version the run is pinned to. `null` when the run has no
    /// pin (no package, or no task of it was dispatched).
    pub package_version: Option<String>,
    pub execution_id: String,
    /// The status of the run in the database: `Failed`, `Completed`, `Cancelled`.
    pub status: String,
    /// `schedule`, `trigger`, `manual`, or `null` when not known.
    pub trigger_origin: Option<String>,
    /// The time a cron schedule asked for. RFC 3339.
    pub scheduled_time: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    /// The reason stored on the run, cut to 500 characters.
    pub error: Option<String>,
    /// The number of failed tasks. `failed_tasks` lists 20 at most.
    pub failed_task_count: usize,
    pub failed_tasks: Vec<FailedTask>,
    pub links: Links,
}

/// One failed task of the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FailedTask {
    pub name: String,
    pub attempt: i32,
    pub max_attempts: i32,
    /// The last error text, cut to 500 characters.
    pub error: Option<String>,
}

/// Where to read the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Links {
    pub api_path: String,
    pub ui_path: String,
    /// Set when the server knows its public base URL.
    pub api_url: Option<String>,
    pub ui_url: Option<String>,
}

/// The event name of an outcome.
pub fn event_name(outcome: RunOutcome) -> String {
    format!("workflow_execution.{}", outcome.as_str())
}

/// Cuts a text to `max` characters and marks the cut.
fn truncate_text(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(max).collect();
    cut.push('…');
    cut
}

/// The package of a run, read from the names of its tasks. `None` for a
/// workflow compiled into the binary, and for a run with no task.
pub fn package_of(tasks: &[TaskExecution]) -> Option<String> {
    tasks
        .iter()
        .filter_map(|t| cloacina::TaskNamespace::from_string(&t.task_name).ok())
        .map(|ns| ns.package_name)
        .find(|p| p != "embedded")
}

/// Builds the alert body from the rows of the run. No context and no secret
/// is read here.
pub fn build_payload(
    tenant: &str,
    outcome: RunOutcome,
    run: &WorkflowExecutionRecord,
    tasks: &[TaskExecution],
    package_version: Option<String>,
    schedule: Option<&RunSchedule>,
    public_base_url: Option<&str>,
) -> AlertPayload {
    let namespaces: Vec<Option<cloacina::TaskNamespace>> = tasks
        .iter()
        .map(|t| cloacina::TaskNamespace::from_string(&t.task_name).ok())
        .collect();
    let package = package_of(tasks);

    let mut failed: Vec<(&TaskExecution, Option<&cloacina::TaskNamespace>)> = tasks
        .iter()
        .zip(namespaces.iter())
        .filter(|(t, _)| t.status == "Failed")
        .map(|(t, ns)| (t, ns.as_ref()))
        .collect();
    failed.sort_by(|a, b| a.0.task_name.cmp(&b.0.task_name));
    let failed_task_count = failed.len();
    let failed_tasks = failed
        .into_iter()
        .take(MAX_FAILED_TASKS)
        .map(|(t, ns)| FailedTask {
            name: ns
                .map(|ns| ns.task_id.clone())
                .unwrap_or_else(|| t.task_name.clone()),
            attempt: t.attempt,
            max_attempts: t.max_attempts,
            error: t
                .last_error
                .as_deref()
                .or(t.error_details.as_deref())
                .map(|e| truncate_text(e, MAX_ERROR_CHARS)),
        })
        .collect();

    let trigger_origin = match (run.trigger_origin.as_deref(), schedule) {
        (Some(origin), _) => Some(origin.to_string()),
        (None, Some(s)) if s.schedule_type == "cron" => Some("schedule".to_string()),
        (None, Some(s)) => Some(s.schedule_type.clone()),
        (None, None) => None,
    };

    let execution_id = run.id.0.to_string();
    let api_path = format!("/v1/tenants/{tenant}/executions/{execution_id}");
    let ui_path = format!("/executions/{execution_id}");
    let links = Links {
        api_url: public_base_url.map(|base| format!("{base}{api_path}")),
        ui_url: public_base_url.map(|base| format!("{base}{ui_path}")),
        api_path,
        ui_path,
    };

    AlertPayload {
        schema_version: PAYLOAD_SCHEMA_VERSION,
        event: event_name(outcome),
        tenant: tenant.to_string(),
        workflow: run.workflow_name.clone(),
        workflow_version: run.workflow_version.clone(),
        package_version: package.as_ref().and(package_version),
        package,
        execution_id,
        status: run.status.clone(),
        trigger_origin,
        scheduled_time: schedule
            .and_then(|s| s.scheduled_time)
            .map(|t| t.to_rfc3339()),
        started_at: run.started_at.0.to_rfc3339(),
        finished_at: run.completed_at.map(|t| t.0.to_rfc3339()),
        error: run
            .error_details
            .as_deref()
            .map(|e| truncate_text(e, MAX_ERROR_CHARS)),
        failed_task_count,
        failed_tasks,
        links,
    }
}

// ---------------------------------------------------------------------------
// Signature
// ---------------------------------------------------------------------------

/// The hex HMAC-SHA256 of `"<timestamp>.<body>"` with the shared secret.
pub fn sign(secret: &[u8], timestamp: i64, body: &[u8]) -> String {
    // HMAC takes a key of any length, so this cannot fail.
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC takes a key of any length");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

/// The value of the signature header: `t=<timestamp>,v1=<signature>`.
pub fn signature_header(secret: &[u8], timestamp: i64, body: &[u8]) -> String {
    format!("t={timestamp},v1={}", sign(secret, timestamp, body))
}

// ---------------------------------------------------------------------------
// Sender
// ---------------------------------------------------------------------------

/// Why one request failed. Holds no URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    Timeout,
    /// The receiver answered with a status outside 2xx.
    Status(u16),
    Transport(String),
}

impl SendError {
    fn reason(&self) -> &'static str {
        match self {
            SendError::Timeout => "timeout",
            SendError::Status(_) => "status",
            SendError::Transport(_) => "transport",
        }
    }
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SendError::Timeout => write!(f, "timed out"),
            SendError::Status(code) => write!(f, "status {code}"),
            SendError::Transport(e) => write!(f, "transport error: {e}"),
        }
    }
}

/// What one pass over a tenant did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PassStats {
    pub sent: usize,
    pub failed: usize,
    pub dropped: usize,
}

impl PassStats {
    fn add(&mut self, other: PassStats) {
        self.sent += other.sent;
        self.failed += other.failed;
        self.dropped += other.dropped;
    }
}

/// The wait before send number `attempts + 1`, counted from the last send.
/// It doubles from 30 s to 15 min, and is never under twice the timeout, so a
/// second replica does not claim a row while the first still waits for the
/// receiver.
pub fn retry_delay(attempts: i32, timeout: Duration) -> Duration {
    let doublings = attempts.clamp(1, 6) as u32 - 1;
    let backoff = Duration::from_secs(30 * 2u64.pow(doublings)).min(Duration::from_secs(900));
    backoff.max(timeout * 2)
}

/// Sends the alerts.
pub struct AlertSender {
    config: AlertWebhookConfig,
    client: reqwest::Client,
}

impl AlertSender {
    pub fn new(config: AlertWebhookConfig) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            // A redirect would send the signed body to another place.
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("cloacina-server/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| format!("alert webhook http client: {e}"))?;
        Ok(Self { config, client })
    }

    /// One POST of `body`. `Ok` only for a 2xx answer.
    pub async fn send(
        &self,
        event: &str,
        execution_id: &str,
        body: Vec<u8>,
    ) -> Result<(), SendError> {
        let mut request = self
            .client
            .post(self.config.url.clone())
            .header("Content-Type", "application/json")
            .header(EVENT_HEADER, event)
            .header(DELIVERY_HEADER, execution_id);
        if let Some(secret) = &self.config.secret {
            request = request.header(
                SIGNATURE_HEADER,
                signature_header(secret.as_bytes(), Utc::now().timestamp(), &body),
            );
        }
        match request.body(body).send().await {
            Ok(response) if response.status().is_success() => Ok(()),
            Ok(response) => Err(SendError::Status(response.status().as_u16())),
            Err(e) if e.is_timeout() => Err(SendError::Timeout),
            // The error text of reqwest names the URL: take it out.
            Err(e) => Err(SendError::Transport(e.without_url().to_string())),
        }
    }

    /// Handles the open notice rows of one tenant: drops the old ones, sends
    /// the ones that are due, leaves the rest. `now` is a parameter so a test
    /// can move the clock.
    pub async fn process_tenant(&self, dal: &DAL, tenant: &str, now: DateTime<Utc>) -> PassStats {
        let mut stats = PassStats::default();
        let rows = match dal
            .delivery_outbox()
            .list_open_for_recipient(RECIPIENT, BATCH_LIMIT)
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                debug!(tenant, error = %e, "alert webhook: cannot list the notice rows");
                return stats;
            }
        };
        for row in rows {
            self.process_row(dal, tenant, &row, now, &mut stats).await;
        }
        stats
    }

    async fn process_row(
        &self,
        dal: &DAL,
        tenant: &str,
        row: &DeliveryOutbox,
        now: DateTime<Utc>,
        stats: &mut PassStats,
    ) {
        let outbox = dal.delivery_outbox();
        let age = now.signed_duration_since(row.created_at.0);
        if age.to_std().is_ok_and(|age| age > MAX_NOTICE_AGE) {
            if outbox.mark_acked(row.id).await.is_ok() {
                self.dropped(tenant, row, "expired", stats);
            }
            return;
        }
        let Some((notice, outcome)) = RunFinishedNotice::from_bytes(&row.payload)
            .and_then(|n| n.run_outcome().map(|o| (n, o)))
        else {
            if outbox.mark_acked(row.id).await.is_ok() {
                self.dropped(tenant, row, "invalid", stats);
            }
            return;
        };
        if !self.config.outcomes.contains(&outcome) {
            // Written under an earlier setting. Not an alert now.
            let _ = outbox.mark_acked(row.id).await;
            return;
        }

        // Not due while the last send is in flight or its retry wait runs.
        if row.delivery_attempts > 0
            && let Some(last) = row.delivered_at
        {
            let waited = now
                .signed_duration_since(last.0)
                .to_std()
                .unwrap_or_default();
            if waited < retry_delay(row.delivery_attempts, self.config.timeout) {
                return;
            }
        }
        // Due again with no send left: the last one failed, or its replica
        // stopped before it could ack or drop the row.
        if row.delivery_attempts as i64 >= self.config.max_attempts as i64 {
            if outbox.mark_acked(row.id).await.is_ok() {
                self.dropped(tenant, row, "attempts", stats);
            }
            return;
        }

        // Claim. Each step is a compare-and-set, so one replica wins.
        if row.delivery_state == "delivered" && outbox.reset_to_pending(row.id).await.is_err() {
            return;
        }
        if outbox.mark_delivered(row.id).await.is_err() {
            return;
        }
        let attempt = row.delivery_attempts + 1;
        let last_attempt = attempt as i64 >= self.config.max_attempts as i64;
        let execution_id = notice.execution_id.to_string();

        let result = match self.load_payload(dal, tenant, &notice, outcome).await {
            Ok(body) => self
                .send(&event_name(outcome), &execution_id, body)
                .await
                .map_err(|e| (e.reason(), e.to_string())),
            Err(e) => Err(("load", e)),
        };
        match result {
            Ok(()) => {
                if let Err(e) = outbox.mark_acked(row.id).await {
                    warn!(
                        tenant, execution_id = %execution_id, error = %e,
                        "alert webhook: sent, but the row could not be acked; it will be sent again"
                    );
                }
                stats.sent += 1;
                metrics::counter!("cloacina_alert_webhook_sent_total", "event" => outcome.as_str())
                    .increment(1);
                debug!(tenant, execution_id = %execution_id, event = outcome.as_str(), "alert webhook: sent");
            }
            Err((reason, error)) => {
                stats.failed += 1;
                metrics::counter!("cloacina_alert_webhook_failed_total", "reason" => reason)
                    .increment(1);
                warn!(
                    tenant, execution_id = %execution_id, host = self.config.host(),
                    attempt, max_attempts = self.config.max_attempts, error = %error,
                    "alert webhook: send failed"
                );
                if last_attempt && outbox.mark_acked(row.id).await.is_ok() {
                    self.dropped(tenant, row, "attempts", stats);
                }
            }
        }
    }

    fn dropped(
        &self,
        tenant: &str,
        row: &DeliveryOutbox,
        reason: &'static str,
        stats: &mut PassStats,
    ) {
        stats.dropped += 1;
        metrics::counter!("cloacina_alert_webhook_dropped_total", "reason" => reason).increment(1);
        warn!(
            tenant,
            outbox_id = row.id,
            reason,
            attempts = row.delivery_attempts,
            "alert webhook: alert dropped"
        );
    }

    async fn load_payload(
        &self,
        dal: &DAL,
        tenant: &str,
        notice: &RunFinishedNotice,
        outcome: RunOutcome,
    ) -> Result<Vec<u8>, String> {
        let id = cloacina::UniversalUuid::from(notice.execution_id);
        let run = dal
            .workflow_execution()
            .get_by_id(id)
            .await
            .map_err(|e| format!("cannot read the run: {e}"))?;
        let tasks = dal
            .task_execution()
            .get_all_tasks_for_workflow(id)
            .await
            .map_err(|e| format!("cannot read the tasks: {e}"))?;
        // The schedule and the version pin only fill optional fields: an
        // error here does not hold the alert back.
        let schedule = cloacina::run_alerts::schedule_of_run(dal, id)
            .await
            .unwrap_or_default();
        let package_version = match package_of(&tasks) {
            Some(package) => dal
                .workflow_packages()
                .get_execution_pin(id, &package)
                .await
                .ok()
                .flatten()
                .map(|pin| pin.version),
            None => None,
        };
        let payload = build_payload(
            tenant,
            outcome,
            &run,
            &tasks,
            package_version,
            schedule.as_ref(),
            self.config.public_base_url.as_deref(),
        );
        serde_json::to_vec(&payload).map_err(|e| format!("cannot write the payload: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Background task
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
#[derive(diesel::QueryableByName)]
struct OpenRow {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    open: bool,
}

/// `"name"` with inner quotes doubled, for use as an SQL identifier.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// The tenants (schemas) that hold an open notice row. One small indexed
/// query per schema on one admin connection, so a tenant with no alert costs
/// no connection pool of its own. A schema with no `delivery_outbox` table is
/// not a tenant and is skipped.
#[cfg(feature = "postgres")]
async fn tenants_with_open_notices(admin_db: &Database) -> Vec<String> {
    let mut schemas = vec!["public".to_string()];
    match cloacina::database::DatabaseAdmin::new(admin_db.clone())
        .list_tenant_schemas()
        .await
    {
        Ok(found) => schemas.extend(found),
        Err(e) => debug!(error = %e, "alert webhook: cannot list the tenant schemas"),
    }
    let conn = match admin_db.get_postgres_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            debug!(error = %e, "alert webhook: no database connection for this pass");
            return Vec::new();
        }
    };
    conn.interact(move |conn| {
        use diesel::RunQueryDsl;
        schemas
            .into_iter()
            .filter(|schema| {
                let sql = format!(
                    "SELECT EXISTS (SELECT 1 FROM {}.delivery_outbox \
                     WHERE recipient = $1 AND delivery_state <> 'acked') AS open",
                    quote_ident(schema)
                );
                diesel::sql_query(sql)
                    .bind::<diesel::sql_types::Text, _>(RECIPIENT)
                    .get_result::<OpenRow>(conn)
                    .map(|row| row.open)
                    .unwrap_or(false)
            })
            .collect::<Vec<String>>()
    })
    .await
    .unwrap_or_default()
}

#[cfg(not(feature = "postgres"))]
async fn tenants_with_open_notices(_admin_db: &Database) -> Vec<String> {
    vec!["public".to_string()]
}

/// Starts the sender. Call it once, only when a webhook is configured.
pub fn spawn(
    config: AlertWebhookConfig,
    admin_db: Database,
    tenant_databases: std::sync::Arc<TenantDatabaseCache>,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), String> {
    let sender = AlertSender::new(config.clone())?;
    info!(
        host = config.host(),
        events = ?config.outcomes.iter().map(|o| o.as_str()).collect::<Vec<_>>(),
        signed = config.secret.is_some(),
        timeout_s = config.timeout.as_secs(),
        max_attempts = config.max_attempts,
        "alert webhook enabled"
    );
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let mut total = PassStats::default();
                    for tenant in tenants_with_open_notices(&admin_db).await {
                        let db = match tenant_databases.resolve(&tenant, &admin_db).await {
                            Ok(db) => db,
                            Err(e) => {
                                debug!(tenant = %tenant, error = %e, "alert webhook: no tenant database");
                                continue;
                            }
                        };
                        total.add(sender.process_tenant(&DAL::new(db), &tenant, Utc::now()).await);
                    }
                    if total != PassStats::default() {
                        debug!(sent = total.sent, failed = total.failed, dropped = total.dropped, "alert webhook pass");
                    }
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
        debug!("alert webhook sender stopped");
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;
    use cloacina::UniversalTimestamp;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    // ── a local receiver ──────────────────────────────────────────────

    /// The headers and the body of one request.
    type Request = (HeaderMap, Vec<u8>);

    #[derive(Clone, Default)]
    struct Receiver {
        /// The statuses to answer with, in order. Empty = 200.
        statuses: Arc<Mutex<VecDeque<u16>>>,
        /// Wait this long before each answer.
        delay: Duration,
        requests: Arc<Mutex<Vec<Request>>>,
    }

    impl Receiver {
        fn answering(statuses: &[u16]) -> Self {
            Self {
                statuses: Arc::new(Mutex::new(statuses.iter().copied().collect())),
                ..Self::default()
            }
        }

        fn count(&self) -> usize {
            self.requests.lock().unwrap().len()
        }

        /// Listens on a free local port and returns the URL of the hook.
        async fn start(&self) -> String {
            async fn hook(
                axum::extract::State(receiver): axum::extract::State<Receiver>,
                headers: HeaderMap,
                body: Bytes,
            ) -> StatusCode {
                receiver
                    .requests
                    .lock()
                    .unwrap()
                    .push((headers, body.to_vec()));
                let status = receiver.statuses.lock().unwrap().pop_front().unwrap_or(200);
                tokio::time::sleep(receiver.delay).await;
                StatusCode::from_u16(status).unwrap()
            }
            let app = axum::Router::new()
                .route("/hook", post(hook))
                .with_state(self.clone());
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                let _ = axum::serve(listener, app).await;
            });
            format!("http://{addr}/hook")
        }
    }

    fn config(
        url: &str,
        secret: Option<&str>,
        timeout: Duration,
        max_attempts: u32,
    ) -> AlertWebhookConfig {
        AlertWebhookConfig::new(
            url,
            &[],
            secret.map(str::to_string),
            timeout,
            max_attempts,
            Some("https://cloacina.example.com/".to_string()),
        )
        .unwrap()
    }

    fn sender(url: &str, secret: Option<&str>) -> AlertSender {
        AlertSender::new(config(url, secret, Duration::from_secs(5), 3)).unwrap()
    }

    // ── settings ──────────────────────────────────────────────────────

    #[test]
    fn alert_events_default_to_failed_and_all_means_every_outcome() {
        assert_eq!(parse_outcomes(&[]).unwrap(), vec![RunOutcome::Failed]);
        assert_eq!(
            parse_outcomes(&["all".to_string()]).unwrap(),
            RunOutcome::ALL.to_vec()
        );
        assert_eq!(
            parse_outcomes(&[
                " Cancelled".to_string(),
                "failed".to_string(),
                "failed".to_string()
            ])
            .unwrap(),
            vec![RunOutcome::Cancelled, RunOutcome::Failed]
        );
        assert!(parse_outcomes(&["paused".to_string()]).is_err());
    }

    #[test]
    fn alert_config_rejects_bad_settings() {
        let ok = |url: &str, timeout: u64, attempts: u32| {
            AlertWebhookConfig::new(url, &[], None, Duration::from_secs(timeout), attempts, None)
        };
        assert!(ok("https://alerts.example.com/hook", 10, 8).is_ok());
        assert!(ok("not a url", 10, 8).is_err());
        assert!(ok("ftp://alerts.example.com/hook", 10, 8).is_err());
        assert!(ok("https://alerts.example.com/hook", 0, 8).is_err());
        assert!(ok("https://alerts.example.com/hook", 10, 0).is_err());
    }

    #[test]
    fn alert_config_debug_and_errors_hide_the_url_path_and_the_secret() {
        let cfg = AlertWebhookConfig::new(
            "https://alerts.example.com/hook/token-in-path",
            &[],
            Some("signing-secret-value".to_string()),
            Duration::from_secs(10),
            8,
            None,
        )
        .unwrap();
        let shown = format!("{cfg:?}");
        assert!(shown.contains("alerts.example.com"));
        assert!(!shown.contains("token-in-path"));
        assert!(!shown.contains("signing-secret-value"));

        let err = AlertWebhookConfig::new(
            "ftp://alerts.example.com/token-in-path",
            &[],
            None,
            Duration::from_secs(10),
            8,
            None,
        )
        .unwrap_err();
        assert!(!err.contains("token-in-path"));
    }

    #[test]
    fn alert_webhook_is_off_when_no_url_is_given() {
        // Nothing in this test process calls `install`, as in a server with
        // no URL: no settings, so `run` starts no sender, and the engine
        // writes no notice row.
        assert!(installed().is_none());
        for outcome in RunOutcome::ALL {
            assert!(!cloacina::run_alerts::is_enabled(outcome));
        }
    }

    // ── payload ───────────────────────────────────────────────────────

    fn at(seconds: i64) -> UniversalTimestamp {
        UniversalTimestamp(DateTime::from_timestamp(1_700_000_000 + seconds, 0).unwrap())
    }

    fn run(status: &str, error: Option<&str>) -> WorkflowExecutionRecord {
        WorkflowExecutionRecord {
            id: cloacina::UniversalUuid::from(
                uuid::Uuid::parse_str("7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11").unwrap(),
            ),
            workflow_name: "nightly_etl".to_string(),
            workflow_version: "3".to_string(),
            status: status.to_string(),
            context_id: None,
            started_at: at(0),
            completed_at: Some(at(95)),
            error_details: error.map(str::to_string),
            recovery_attempts: 0,
            last_recovery_at: None,
            paused_at: None,
            pause_reason: None,
            created_at: at(0),
            updated_at: at(95),
            trigger_origin: None,
            priority_asked: None,
            priority: 5,
        }
    }

    fn task(name: &str, status: &str, attempt: i32, error: Option<&str>) -> TaskExecution {
        TaskExecution {
            id: cloacina::UniversalUuid::new_v4(),
            workflow_execution_id: cloacina::UniversalUuid::new_v4(),
            task_name: name.to_string(),
            status: status.to_string(),
            started_at: Some(at(1)),
            completed_at: Some(at(90)),
            attempt,
            max_attempts: 3,
            error_details: None,
            trigger_rules: "{}".to_string(),
            task_configuration: "{}".to_string(),
            retry_at: None,
            last_error: error.map(str::to_string),
            recovery_attempts: 0,
            last_recovery_at: None,
            sub_status: None,
            claimed_by: None,
            heartbeat_at: None,
            waiting_since: None,
            artifact_fetch_wait_since: None,
            pool: None,
            cpu_request_millis: None,
            memory_request_bytes: None,
            fleet_agent_id: None,
            fleet_dispatch_id: None,
            fleet_packet_row_id: None,
            created_at: at(0),
            updated_at: at(90),
            workflow_priority: 5,
            task_priority: 5,
            fleet_dispatch_claimed_by: None,
        }
    }

    #[test]
    fn alert_payload_for_a_failed_scheduled_run() {
        let tasks = vec![
            task("acme::etl_pkg::nightly_etl::extract", "Completed", 1, None),
            task(
                "acme::etl_pkg::nightly_etl::load",
                "Failed",
                3,
                Some("connection refused"),
            ),
            task("acme::etl_pkg::nightly_etl::report", "Skipped", 1, None),
        ];
        let schedule = RunSchedule {
            schedule_type: "cron".to_string(),
            scheduled_time: Some(at(-5).0),
        };
        let payload = build_payload(
            "acme",
            RunOutcome::Failed,
            &run("Failed", Some("1 task(s) failed, 1 completed, 1 skipped")),
            &tasks,
            Some("1.4.2".to_string()),
            Some(&schedule),
            Some("https://cloacina.example.com"),
        );

        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "schema_version": 1,
                "event": "workflow_execution.failed",
                "tenant": "acme",
                "workflow": "nightly_etl",
                "workflow_version": "3",
                "package": "etl_pkg",
                "package_version": "1.4.2",
                "execution_id": "7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11",
                "status": "Failed",
                "trigger_origin": "schedule",
                "scheduled_time": "2023-11-14T22:13:15+00:00",
                "started_at": "2023-11-14T22:13:20+00:00",
                "finished_at": "2023-11-14T22:14:55+00:00",
                "error": "1 task(s) failed, 1 completed, 1 skipped",
                "failed_task_count": 1,
                "failed_tasks": [{
                    "name": "load",
                    "attempt": 3,
                    "max_attempts": 3,
                    "error": "connection refused"
                }],
                "links": {
                    "api_path": "/v1/tenants/acme/executions/7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11",
                    "ui_path": "/executions/7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11",
                    "api_url": "https://cloacina.example.com/v1/tenants/acme/executions/7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11",
                    "ui_url": "https://cloacina.example.com/executions/7f1c4a52-0d3e-4a52-9b7e-2f6d1c0a9e11"
                }
            })
        );
    }

    #[test]
    fn alert_payload_origin_links_and_package_when_little_is_known() {
        let mut manual = run("Cancelled", None);
        manual.trigger_origin = Some("manual".to_string());
        let payload = build_payload(
            "public",
            RunOutcome::Cancelled,
            &manual,
            &[],
            Some("9.9.9".to_string()),
            None,
            None,
        );
        assert_eq!(payload.event, "workflow_execution.cancelled");
        assert_eq!(payload.trigger_origin.as_deref(), Some("manual"));
        assert_eq!(payload.package, None);
        assert_eq!(payload.package_version, None, "no package, so no version");
        assert_eq!(payload.scheduled_time, None);
        assert_eq!(payload.links.api_url, None);
        assert_eq!(payload.links.ui_url, None);
        assert!(payload.failed_tasks.is_empty());

        let trigger = RunSchedule {
            schedule_type: "trigger".to_string(),
            scheduled_time: None,
        };
        let embedded = [task("public::embedded::wf::only", "Completed", 1, None)];
        let payload = build_payload(
            "public",
            RunOutcome::Completed,
            &run("Completed", None),
            &embedded,
            None,
            Some(&trigger),
            None,
        );
        assert_eq!(payload.trigger_origin.as_deref(), Some("trigger"));
        assert_eq!(payload.package, None);

        let unknown = build_payload(
            "public",
            RunOutcome::Completed,
            &run("Completed", None),
            &[],
            None,
            None,
            None,
        );
        assert_eq!(unknown.trigger_origin, None);
    }

    #[test]
    fn alert_payload_cuts_long_errors_and_caps_the_task_list() {
        let long = "é".repeat(MAX_ERROR_CHARS + 40);
        let tasks: Vec<TaskExecution> = (0..MAX_FAILED_TASKS + 5)
            .map(|i| {
                task(
                    &format!("acme::pkg::wf::task_{i:02}"),
                    "Failed",
                    1,
                    Some(&long),
                )
            })
            .collect();
        let payload = build_payload(
            "acme",
            RunOutcome::Failed,
            &run("Failed", Some(&long)),
            &tasks,
            None,
            None,
            None,
        );
        assert_eq!(payload.failed_task_count, MAX_FAILED_TASKS + 5);
        assert_eq!(payload.failed_tasks.len(), MAX_FAILED_TASKS);
        assert_eq!(payload.failed_tasks[0].name, "task_00");
        let cut = payload.failed_tasks[0].error.as_deref().unwrap();
        assert_eq!(cut.chars().count(), MAX_ERROR_CHARS + 1);
        assert!(cut.ends_with('…'));
        assert_eq!(
            payload.error.as_deref().unwrap().chars().count(),
            MAX_ERROR_CHARS + 1
        );
    }

    // ── signature ─────────────────────────────────────────────────────

    #[test]
    fn alert_signature_matches_a_known_value() {
        // The value from Python:
        // hmac.new(b"test-secret", b'1700000000.{"a":1}', hashlib.sha256).hexdigest()
        let expected = "8cb2c3355fca388e9ac2caec004f4d5d7045d74937ab5faad61dc11682247a9f";
        assert_eq!(sign(b"test-secret", 1_700_000_000, br#"{"a":1}"#), expected);
        assert_eq!(
            signature_header(b"test-secret", 1_700_000_000, br#"{"a":1}"#),
            format!("t=1700000000,v1={expected}")
        );
        assert_ne!(
            sign(b"other-secret", 1_700_000_000, br#"{"a":1}"#),
            expected
        );
        assert_ne!(sign(b"test-secret", 1_700_000_001, br#"{"a":1}"#), expected);
        assert_ne!(sign(b"test-secret", 1_700_000_000, br#"{"a":2}"#), expected);
    }

    #[test]
    fn alert_retry_delay_doubles_and_stays_above_twice_the_timeout() {
        let t = Duration::from_secs(10);
        assert_eq!(retry_delay(1, t), Duration::from_secs(30));
        assert_eq!(retry_delay(2, t), Duration::from_secs(60));
        assert_eq!(retry_delay(3, t), Duration::from_secs(120));
        assert_eq!(retry_delay(6, t), Duration::from_secs(900));
        assert_eq!(retry_delay(40, t), Duration::from_secs(900));
        assert_eq!(
            retry_delay(1, Duration::from_secs(60)),
            Duration::from_secs(120)
        );
    }

    // ── one request ───────────────────────────────────────────────────

    #[tokio::test]
    async fn alert_send_success_carries_the_body_the_event_and_a_valid_signature() {
        let receiver = Receiver::default();
        let url = receiver.start().await;
        let body = br#"{"event":"workflow_execution.failed"}"#.to_vec();

        sender(&url, Some("test-secret"))
            .send("workflow_execution.failed", "exec-1", body.clone())
            .await
            .unwrap();

        let requests = receiver.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let (headers, got) = &requests[0];
        assert_eq!(got, &body);
        assert_eq!(headers["content-type"], "application/json");
        assert_eq!(headers[EVENT_HEADER], "workflow_execution.failed");
        assert_eq!(headers[DELIVERY_HEADER], "exec-1");
        // Check the signature the way a receiver does.
        let header = headers[SIGNATURE_HEADER].to_str().unwrap();
        let (t, v1) = header.split_once(",v1=").unwrap();
        let timestamp: i64 = t.strip_prefix("t=").unwrap().parse().unwrap();
        assert!((Utc::now().timestamp() - timestamp).abs() < 60);
        assert_eq!(v1, sign(b"test-secret", timestamp, got));
    }

    #[tokio::test]
    async fn alert_send_without_a_secret_has_no_signature_header() {
        let receiver = Receiver::default();
        let url = receiver.start().await;
        sender(&url, None)
            .send("workflow_execution.failed", "exec-1", b"{}".to_vec())
            .await
            .unwrap();
        let requests = receiver.requests.lock().unwrap();
        assert!(!requests[0].0.contains_key(SIGNATURE_HEADER));
    }

    #[tokio::test]
    async fn alert_send_reports_a_500_and_then_succeeds() {
        let receiver = Receiver::answering(&[500]);
        let url = receiver.start().await;
        let sender = sender(&url, None);

        let first = sender
            .send("workflow_execution.failed", "exec-1", b"{}".to_vec())
            .await;
        assert_eq!(first, Err(SendError::Status(500)));
        let second = sender
            .send("workflow_execution.failed", "exec-1", b"{}".to_vec())
            .await;
        assert_eq!(second, Ok(()));
        assert_eq!(receiver.count(), 2);
    }

    #[tokio::test]
    async fn alert_send_times_out_on_a_slow_receiver() {
        let receiver = Receiver {
            delay: Duration::from_secs(5),
            ..Receiver::default()
        };
        let url = receiver.start().await;
        let sender = AlertSender::new(config(&url, None, Duration::from_millis(200), 3)).unwrap();

        let started = std::time::Instant::now();
        let result = sender
            .send("workflow_execution.failed", "exec-1", b"{}".to_vec())
            .await;
        assert_eq!(result, Err(SendError::Timeout));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[tokio::test]
    async fn alert_send_reports_a_down_receiver_without_the_url() {
        // Bind a port, then free it: nothing listens there.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let sender = sender(&format!("http://{addr}/hook/token-in-path"), None);

        let result = sender
            .send("workflow_execution.failed", "exec-1", b"{}".to_vec())
            .await;
        match result {
            Err(SendError::Transport(text)) => assert!(!text.contains("token-in-path")),
            other => panic!("expected a transport error, got {other:?}"),
        }
    }

    // ── notice rows on SQLite in memory ───────────────────────────────

    #[cfg(feature = "sqlite")]
    mod with_database {
        use super::*;
        use cloacina::models::delivery_outbox::NewDeliveryOutbox;
        use cloacina::models::schedule::{NewSchedule, NewScheduleExecution};
        use cloacina::models::task_execution::NewTaskExecution;
        use cloacina::models::workflow_execution::NewWorkflowExecution;
        use cloacina::run_alerts::KIND;

        async fn unique_dal() -> DAL {
            let url = format!(
                "file:alert_webhook_test_{}?mode=memory&cache=shared",
                uuid::Uuid::new_v4()
            );
            let db = Database::new(&url, "", 5);
            db.run_migrations()
                .await
                .expect("migrations should succeed");
            DAL::new(db)
        }

        /// A failed run with one failed task, started by a cron schedule, and
        /// its notice row. Returns the execution id.
        async fn failed_run_with_notice(dal: &DAL) -> cloacina::UniversalUuid {
            let run = dal
                .workflow_execution()
                .create(NewWorkflowExecution {
                    workflow_name: "nightly_etl".into(),
                    workflow_version: "3".into(),
                    status: "Running".into(),
                    context_id: None,
                })
                .await
                .unwrap();
            let task = dal
                .task_execution()
                .create(NewTaskExecution {
                    workflow_execution_id: run.id,
                    task_name: "public::etl_pkg::nightly_etl::load".into(),
                    status: "Running".into(),
                    attempt: 2,
                    max_attempts: 2,
                    trigger_rules: "{}".into(),
                    task_configuration: "{}".into(),
                })
                .await
                .unwrap();
            dal.task_execution()
                .mark_failed(task.id, "connection refused", None)
                .await
                .unwrap();
            dal.workflow_execution()
                .mark_failed(run.id, "1 task(s) failed, 0 completed, 0 skipped")
                .await
                .unwrap();
            let schedule = dal
                .schedule()
                .create(NewSchedule::cron(
                    "nightly_etl",
                    "0 2 * * *",
                    UniversalTimestamp::now(),
                ))
                .await
                .unwrap();
            dal.schedule_execution()
                .create(NewScheduleExecution {
                    schedule_id: schedule.id,
                    workflow_execution_id: Some(run.id),
                    scheduled_time: Some(at(0)),
                    claimed_at: None,
                    context_hash: None,
                })
                .await
                .unwrap();
            enqueue_notice(dal, run.id.0, "failed").await;
            run.id
        }

        async fn enqueue_notice(dal: &DAL, execution_id: uuid::Uuid, outcome: &str) -> i64 {
            dal.delivery_outbox()
                .enqueue(NewDeliveryOutbox {
                    recipient: RECIPIENT.to_string(),
                    kind: KIND.to_string(),
                    tenant_id: None,
                    payload: serde_json::to_vec(&serde_json::json!({
                        "execution_id": execution_id,
                        "outcome": outcome,
                    }))
                    .unwrap(),
                })
                .await
                .unwrap()
                .id
        }

        async fn open_rows(dal: &DAL) -> Vec<DeliveryOutbox> {
            dal.delivery_outbox()
                .list_open_for_recipient(RECIPIENT, 100)
                .await
                .unwrap()
        }

        #[tokio::test]
        async fn alert_notice_is_sent_once_with_the_payload_of_the_run() {
            let dal = unique_dal().await;
            let id = failed_run_with_notice(&dal).await;
            let receiver = Receiver::default();
            let sender = sender(&receiver.start().await, Some("test-secret"));

            let stats = sender.process_tenant(&dal, "public", Utc::now()).await;
            assert_eq!(
                stats,
                PassStats {
                    sent: 1,
                    failed: 0,
                    dropped: 0
                }
            );
            assert!(open_rows(&dal).await.is_empty(), "the row is acked");

            // A second pass, as from a second replica, finds nothing to send.
            let again = sender.process_tenant(&dal, "public", Utc::now()).await;
            assert_eq!(again, PassStats::default());
            assert_eq!(receiver.count(), 1);

            let requests = receiver.requests.lock().unwrap();
            let (headers, body) = &requests[0];
            assert_eq!(headers[EVENT_HEADER], "workflow_execution.failed");
            assert_eq!(headers[DELIVERY_HEADER], id.0.to_string().as_str());
            let json: serde_json::Value = serde_json::from_slice(body).unwrap();
            assert_eq!(json["schema_version"], 1);
            assert_eq!(json["event"], "workflow_execution.failed");
            assert_eq!(json["tenant"], "public");
            assert_eq!(json["workflow"], "nightly_etl");
            assert_eq!(json["package"], "etl_pkg");
            assert_eq!(json["package_version"], serde_json::Value::Null);
            assert_eq!(json["execution_id"], id.0.to_string());
            assert_eq!(json["status"], "Failed");
            assert_eq!(json["trigger_origin"], "schedule");
            assert_eq!(json["scheduled_time"], "2023-11-14T22:13:20+00:00");
            assert_eq!(json["error"], "1 task(s) failed, 0 completed, 0 skipped");
            assert_eq!(json["failed_task_count"], 1);
            assert_eq!(json["failed_tasks"][0]["name"], "load");
            assert_eq!(json["failed_tasks"][0]["error"], "connection refused");
            assert_eq!(
                json["links"]["ui_url"],
                format!("https://cloacina.example.com/executions/{}", id.0)
            );
        }

        #[tokio::test]
        async fn alert_notice_survives_a_500_and_is_sent_after_the_retry_wait() {
            let dal = unique_dal().await;
            failed_run_with_notice(&dal).await;
            let receiver = Receiver::answering(&[500]);
            let sender = sender(&receiver.start().await, None);
            let start = Utc::now();

            let first = sender.process_tenant(&dal, "public", start).await;
            assert_eq!(
                first,
                PassStats {
                    sent: 0,
                    failed: 1,
                    dropped: 0
                }
            );
            let rows = open_rows(&dal).await;
            assert_eq!(rows.len(), 1, "the row stays open for a retry");
            assert_eq!(rows[0].delivery_attempts, 1);

            // Inside the retry wait no replica sends again.
            let early = sender
                .process_tenant(&dal, "public", start + chrono::Duration::seconds(10))
                .await;
            assert_eq!(early, PassStats::default());
            assert_eq!(receiver.count(), 1);

            let later = sender
                .process_tenant(&dal, "public", start + chrono::Duration::seconds(40))
                .await;
            assert_eq!(
                later,
                PassStats {
                    sent: 1,
                    failed: 0,
                    dropped: 0
                }
            );
            assert_eq!(receiver.count(), 2);
            assert!(open_rows(&dal).await.is_empty());
        }

        #[tokio::test]
        async fn alert_notice_is_dropped_after_the_last_attempt() {
            let dal = unique_dal().await;
            failed_run_with_notice(&dal).await;
            let receiver = Receiver::answering(&[500, 503, 500, 500]);
            // 3 attempts at most.
            let sender = sender(&receiver.start().await, None);
            let start = Utc::now();

            let mut total = PassStats::default();
            for minutes in [0, 20, 40, 60] {
                total.add(
                    sender
                        .process_tenant(&dal, "public", start + chrono::Duration::minutes(minutes))
                        .await,
                );
            }
            assert_eq!(
                total,
                PassStats {
                    sent: 0,
                    failed: 3,
                    dropped: 1
                }
            );
            assert_eq!(receiver.count(), 3, "no send after the last attempt");
            assert!(open_rows(&dal).await.is_empty());
        }

        #[tokio::test]
        async fn alert_notice_claimed_by_a_replica_that_stopped_is_sent_after_the_wait() {
            let dal = unique_dal().await;
            failed_run_with_notice(&dal).await;
            let receiver = Receiver::default();
            let sender = sender(&receiver.start().await, None);
            // Another replica claimed the row and stopped before the ack.
            let row = open_rows(&dal).await.remove(0);
            dal.delivery_outbox().mark_delivered(row.id).await.unwrap();
            let claimed_at = Utc::now();

            let during = sender.process_tenant(&dal, "public", claimed_at).await;
            assert_eq!(during, PassStats::default());
            assert_eq!(
                receiver.count(),
                0,
                "a claimed row is not sent by a second replica"
            );

            let after = sender
                .process_tenant(&dal, "public", claimed_at + chrono::Duration::seconds(40))
                .await;
            assert_eq!(
                after,
                PassStats {
                    sent: 1,
                    failed: 0,
                    dropped: 0
                }
            );
            assert_eq!(receiver.count(), 1);
        }

        #[tokio::test]
        async fn alert_notice_that_is_old_unreadable_or_not_selected_is_not_sent() {
            let dal = unique_dal().await;
            let receiver = Receiver::default();
            // Failed runs only.
            let sender = sender(&receiver.start().await, None);

            // Not selected: a completed run. Acked with no send and no drop.
            enqueue_notice(&dal, uuid::Uuid::new_v4(), "completed").await;
            // Unreadable outcome.
            enqueue_notice(&dal, uuid::Uuid::new_v4(), "paused").await;
            let stats = sender.process_tenant(&dal, "public", Utc::now()).await;
            assert_eq!(
                stats,
                PassStats {
                    sent: 0,
                    failed: 0,
                    dropped: 1
                }
            );
            assert!(open_rows(&dal).await.is_empty());

            // Old: a real failed run, seen a day and an hour later.
            failed_run_with_notice(&dal).await;
            let stats = sender
                .process_tenant(&dal, "public", Utc::now() + chrono::Duration::hours(25))
                .await;
            assert_eq!(
                stats,
                PassStats {
                    sent: 0,
                    failed: 0,
                    dropped: 1
                }
            );
            assert!(open_rows(&dal).await.is_empty());
            assert_eq!(receiver.count(), 0);
        }

        #[tokio::test]
        async fn alert_notice_of_a_deleted_run_fails_and_is_dropped_at_the_last_attempt() {
            let dal = unique_dal().await;
            let receiver = Receiver::default();
            let sender = AlertSender::new(config(
                &receiver.start().await,
                None,
                Duration::from_secs(5),
                1,
            ))
            .unwrap();
            enqueue_notice(&dal, uuid::Uuid::new_v4(), "failed").await;

            let stats = sender.process_tenant(&dal, "public", Utc::now()).await;
            assert_eq!(
                stats,
                PassStats {
                    sent: 0,
                    failed: 1,
                    dropped: 1
                }
            );
            assert_eq!(receiver.count(), 0);
            assert!(open_rows(&dal).await.is_empty());
        }
    }
}
