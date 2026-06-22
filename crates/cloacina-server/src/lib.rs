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

//! Cloacina HTTP API server library.
//!
//! Extracted from `cloacinactl serve` into its own crate (T-0510). Exposes a
//! single `run()` entrypoint that boots the axum HTTP server with auth, tenant
//! management, workflow upload, and execution APIs.

pub mod agent_registry;
pub mod delivery_sink;
pub mod fleet_coordinator;
pub mod fleet_executor;
pub mod openapi;
pub mod ops_metrics;
pub mod routes;
pub mod tenant_runner_cache;

/// CORS configuration (CLOACI-T-0643 / REQ-009). Disabled unless at least
/// one allowed origin is configured — browser consumers (the TS SDK / UI)
/// require an explicit operator opt-in.
#[derive(Debug, Clone, Default)]
pub struct CorsConfig {
    /// Exact origins allowed (e.g. `https://ops.example.com`), or `*` for any.
    pub allowed_origins: Vec<String>,
    /// HTTP methods allowed in preflight. Empty = GET, POST, DELETE, OPTIONS.
    pub allowed_methods: Vec<String>,
    /// Request headers allowed in preflight. Empty = authorization, content-type.
    pub allowed_headers: Vec<String>,
}

impl CorsConfig {
    /// Build the tower-http layer, or `None` when CORS is disabled.
    /// Invalid origin/method/header values fail fast at boot.
    pub fn layer(&self) -> Result<Option<tower_http::cors::CorsLayer>> {
        use tower_http::cors::{AllowOrigin, CorsLayer};

        if self.allowed_origins.is_empty() {
            return Ok(None);
        }

        let origins = if self.allowed_origins.iter().any(|o| o == "*") {
            AllowOrigin::any()
        } else {
            let parsed: Vec<axum::http::HeaderValue> = self
                .allowed_origins
                .iter()
                .map(|o| {
                    o.parse()
                        .with_context(|| format!("invalid CORS origin: {o}"))
                })
                .collect::<Result<_>>()?;
            AllowOrigin::list(parsed)
        };

        let methods: Vec<axum::http::Method> = if self.allowed_methods.is_empty() {
            vec![
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ]
        } else {
            self.allowed_methods
                .iter()
                .map(|m| {
                    m.to_uppercase()
                        .parse()
                        .with_context(|| format!("invalid CORS method: {m}"))
                })
                .collect::<Result<_>>()?
        };

        let headers: Vec<axum::http::HeaderName> = if self.allowed_headers.is_empty() {
            vec![
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
            ]
        } else {
            self.allowed_headers
                .iter()
                .map(|h| {
                    h.parse()
                        .with_context(|| format!("invalid CORS header: {h}"))
                })
                .collect::<Result<_>>()?
        };

        Ok(Some(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods(methods)
                .allow_headers(headers),
        ))
    }
}

use anyhow::{Context, Result};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cloacina::computation_graph::registry::EndpointRegistry;
use cloacina::computation_graph::scheduler::ComputationGraphScheduler;
use cloacina::database::Database;
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::security::SecurityConfig;

/// Cached per-tenant database connections for schema isolation.
///
/// Each tenant gets a small connection pool scoped to their PostgreSQL schema.
/// Lazily populated on first request for a given tenant.
pub struct TenantDatabaseCache {
    databases: tokio::sync::RwLock<std::collections::HashMap<String, Database>>,
    database_url: String,
}

impl TenantDatabaseCache {
    pub fn new(database_url: String) -> Self {
        Self {
            databases: tokio::sync::RwLock::new(std::collections::HashMap::new()),
            database_url,
        }
    }

    /// Get or create a schema-scoped Database for the given tenant.
    ///
    /// Returns the admin (public schema) database if tenant_id is "public".
    pub async fn resolve(
        &self,
        tenant_id: &str,
        admin_db: &Database,
    ) -> Result<Database, cloacina::database::connection::DatabaseError> {
        if tenant_id == "public" {
            return Ok(admin_db.clone());
        }

        // Fast path: check read lock
        {
            let cache = self.databases.read().await;
            if let Some(db) = cache.get(tenant_id) {
                return Ok(db.clone());
            }
        }

        // Slow path: create and cache
        let db = Database::try_new_with_schema(
            &self.database_url,
            "cloacina",
            2, // small pool per tenant
            Some(tenant_id),
        )?;

        let mut cache = self.databases.write().await;
        // Double-check after acquiring write lock
        if let Some(existing) = cache.get(tenant_id) {
            return Ok(existing.clone());
        }
        cache.insert(tenant_id.to_string(), db.clone());
        Ok(db)
    }

    /// CLOACI-T-0581: drop the cached `Database` for a tenant. Used by
    /// tenant teardown — once the schema is dropped, the cached
    /// connection pool is stale and must not be reused. Returns `true`
    /// if an entry was evicted.
    pub async fn evict(&self, tenant_id: &str) -> bool {
        let mut cache = self.databases.write().await;
        cache.remove(tenant_id).is_some()
    }
}

/// Shared application state accessible from all route handlers.
#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub runner: Arc<DefaultRunner>,
    pub key_cache: Arc<crate::routes::auth::KeyCache>,
    pub endpoint_registry: EndpointRegistry,
    pub graph_scheduler: Arc<ComputationGraphScheduler>,
    /// Substrate (CLOACI-I-0115): in-process registry of currently-connected
    /// WS recipients. Plugged into the per-replica `DeliveryRelay` as its sink.
    pub delivery_sink: Arc<crate::delivery_sink::WsDeliverySink>,
    /// Substrate (CLOACI-I-0115): wake handle for the local `DeliveryRelay`.
    /// Producers (CLI/agent handlers) wake this after enqueue / on reconnect
    /// resync, bypassing the LISTEN/NOTIFY round-trip for same-replica work.
    pub delivery_wake: cloacina::delivery::WakeHandle,
    /// Execution-agent fleet (CLOACI-I-0114 / T-0631): in-memory roster of
    /// registered agents. Consulted by the `FleetExecutor` (T-0633) for
    /// capacity-aware selection.
    pub agent_registry: Arc<crate::agent_registry::AgentRegistry>,
    /// Fleet (CLOACI-T-0633): rendezvous between the `FleetExecutor` and the
    /// agent's `POST /v1/agent/result` route. The executor registers a oneshot
    /// before enqueueing a work packet; `report_result` forwards the incoming
    /// `AgentResultRequest` to the waiting executor.
    pub fleet_coordinator: Arc<crate::fleet_coordinator::FleetCoordinator>,
    pub security_config: SecurityConfig,
    /// Short-lived WebSocket auth tickets (single-use, TTL-based).
    pub ws_tickets: Arc<crate::routes::auth::WsTicketStore>,
    /// Prometheus metrics handle for rendering /metrics endpoint.
    pub metrics_handle: metrics_exporter_prometheus::PrometheusHandle,
    /// Per-tenant database connection cache for schema isolation.
    pub tenant_databases: Arc<TenantDatabaseCache>,
    /// CLOACI-T-0580: per-tenant `DefaultRunner` cache for tenant-scoped
    /// workflow execution. Each cached runner shares the same inventory
    /// `Runtime` via `Arc`; per-tenant runners differ only in their
    /// underlying `Database` (schema scope) and lifecycle state.
    pub tenant_runners: Arc<crate::tenant_runner_cache::TenantRunnerCache>,
    /// CLOACI-T-0581: max wall-clock for the tenant-teardown runner-evict
    /// step. Past this, the runner is hard-dropped from the cache without
    /// awaiting graceful shutdown completion — any task that ignored
    /// cooperative cancellation will error on its next DB write once the
    /// schema is dropped.
    pub tenant_deletion_drain_timeout: std::time::Duration,
    /// CLOACI-T-0639: heartbeat interval (seconds) advertised to fleet agents
    /// on register. Agents heartbeat at this rate; the liveness sweeper uses the
    /// same value as its tick + dead-after basis. Operator-configurable via
    /// `--agent-heartbeat-interval-s`.
    pub agent_heartbeat_interval_seconds: u32,
}

/// CLOACI-T-0580: build the base `DefaultRunnerConfig` used by every
/// per-tenant runner in `TenantRunnerCache`. Mirrors the admin runner
/// config (`registry_storage_backend` is the same; reconcile interval
/// honors the operator override).
fn runner_config_for_tenant_cache(
    reconcile_interval: Option<std::time::Duration>,
    default_executor: &str,
) -> cloacina::DefaultRunnerConfig {
    let mut builder = cloacina::DefaultRunnerConfig::builder();
    builder = builder.registry_storage_backend("database");
    if let Some(interval) = reconcile_interval {
        builder = builder.registry_reconcile_interval(interval);
    }
    // CLOACI-T-0640: carry the configured default executor onto per-tenant
    // runners so their dispatcher sends tenant tasks to the same executor (and
    // a dispatcher exists at all, which `register_executor` requires).
    builder = builder.default_executor(default_executor);
    builder
        .build()
        .expect("default tenant runner config builds cleanly")
}

/// Validate security-related CLI args at server boot.
///
/// Extracted from `run()` so it's unit-testable without spinning up the
/// full server. Currently enforces: `--require-signatures` is only meaningful
/// paired with `--verification-org-id`; reject the combo at boot rather than
/// surface a 403 on first upload (CLOACI-I-0103 / T-0567).
fn validate_security_args(
    require_signatures: bool,
    verification_org_id: Option<&uuid::Uuid>,
) -> Result<()> {
    if require_signatures && verification_org_id.is_none() {
        anyhow::bail!(
            "--require-signatures requires --verification-org-id <UUID> \
             (or set CLOACINA_VERIFICATION_ORG_ID env var). Without a trusted \
             org_id the server has no way to verify uploaded signatures."
        );
    }
    Ok(())
}

/// Run the API server.
///
/// Argument count is over clippy's default threshold; the right long-term fix
/// is a `RunConfig` struct, tracked as a follow-up. T-0567 added the
/// `verification_org_id` parameter and pushed us to 8.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    home: std::path::PathBuf,
    bind: SocketAddr,
    database_url: String,
    verbose: bool,
    bootstrap_key: Option<String>,
    require_signatures: bool,
    verification_org_id: Option<uuid::Uuid>,
    reconcile_interval: Option<std::time::Duration>,
    tenant_runner_cache_size: usize,
    tenant_deletion_drain_timeout: std::time::Duration,
    log_retention_days: u64,
    // Executor every task is dispatched to (CLOACI-T-0640). `"default"` runs all
    // work on the in-process thread executor; `"fleet"` sends it to the
    // execution-agent fleet. Validated against the registered executor keys at
    // boot — an unknown key fails fast.
    default_executor: String,
    // Fleet agent liveness tuning (CLOACI-T-0639). `agent_heartbeat_interval_s`
    // is advertised to agents (their heartbeat cadence) and is the sweeper's
    // tick rate; the sweeper marks an agent dead after `agent_liveness_misses`
    // missed beats (dead-after = interval × misses). Defaults (15, 3) reproduce
    // the prior hardcoded 15s/45s.
    agent_heartbeat_interval_s: u32,
    agent_liveness_misses: u32,
    // CORS opt-in (CLOACI-T-0643 / REQ-009). Disabled unless at least one
    // allowed origin is configured.
    cors: CorsConfig,
) -> Result<()> {
    // Validate CORS config before any heavy startup work.
    let cors_layer = cors.layer()?;
    // Fail fast at boot rather than 403 at first upload (CLOACI-I-0103 / T-0567).
    validate_security_args(require_signatures, verification_org_id.as_ref())?;

    // CLOACI-T-0582: enable strict search_path checking on the server.
    // Adds a `current_schema()` round-trip on every tenant-scoped
    // connection acquire, catching silent search_path drift. The daemon
    // (single-tenant per ADR-0005) leaves this off to avoid the cost.
    cloacina::database::connection::set_strict_search_path(true);

    // Register the Python runtime in cloacina core's dispatch slot so the
    // reconciler can load uploaded Python-language packages. The compiler
    // service deliberately never does this — it has no business touching
    // Python. See CLOACI-T-0529.
    cloacina_python::install();

    // Set up logging (file + stderr, same as daemon)
    std::fs::create_dir_all(&home)
        .with_context(|| format!("Failed to create home: {}", home.display()))?;

    let logs_dir = home.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .with_context(|| format!("Failed to create logs dir: {}", logs_dir.display()))?;

    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    // Daily-rotated file appender with optional retention via
    // `max_log_files`. `log_retention_days == 0` disables pruning per
    // operator opt-out; otherwise the appender keeps the most recent N
    // files. CLOACI-T-0592.
    let mut log_builder = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("cloacina-server")
        .filename_suffix("log");
    if log_retention_days > 0 {
        log_builder = log_builder.max_log_files(log_retention_days as usize);
    }
    let file_appender = log_builder.build(&logs_dir).with_context(|| {
        format!(
            "Failed to build rolling log appender in {}",
            logs_dir.display()
        )
    })?;
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Build the base subscriber with stderr + file layers
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(fmt::layer().json().with_writer(non_blocking));

    // Conditionally add OpenTelemetry tracing layer
    #[cfg(feature = "telemetry")]
    {
        if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
            use opentelemetry::trace::TracerProvider;

            let service_name =
                std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "cloacina".to_string());

            let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .build()
                .context("Failed to create OTLP exporter")?;

            let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                .with_batch_exporter(otlp_exporter)
                .with_resource(
                    opentelemetry_sdk::Resource::builder()
                        .with_service_name(service_name)
                        .build(),
                )
                .build();

            let tracer = provider.tracer("cloacina");
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            subscriber.with(otel_layer).init();
            // Provider is kept alive by the global registry
            opentelemetry::global::set_tracer_provider(provider);
        } else {
            subscriber.init();
        }
    }

    #[cfg(not(feature = "telemetry"))]
    {
        subscriber.init();
    }

    info!("Starting API server");
    info!("  Bind:     {}", bind);
    info!("  Database: {}", mask_db_url(&database_url));
    info!("  Home:     {}", home.display());
    warn!("Server running without TLS -- use a TLS-terminating reverse proxy (nginx, Caddy, Envoy) in production");

    // Initialize Prometheus metrics recorder
    let metrics_builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    let metrics_handle = metrics_builder
        .install_recorder()
        .context("Failed to install Prometheus metrics recorder")?;

    // Register metric descriptions
    metrics::describe_counter!(
        "cloacina_workflows_total",
        "Total workflow executions by status and reason. \
         reason is `ok` on success, or a bounded failure category \
         (currently `dependency_failed`) on failure."
    );
    metrics::describe_counter!(
        "cloacina_tasks_total",
        "Total task executions by status and reason. \
         reason is `ok` on success, or one of the bounded failure categories: \
         `task_error`, `timeout`, `validation_failed`, `infrastructure`, \
         `task_not_found`, `claim_lost`, `unknown`."
    );
    metrics::describe_counter!(
        "cloacina_api_requests_total",
        "Total API requests by HTTP method and response status code"
    );
    metrics::describe_histogram!(
        "cloacina_api_request_duration_seconds",
        "API request handler duration by HTTP method and response status code"
    );
    metrics::describe_histogram!(
        "cloacina_workflow_duration_seconds",
        "Workflow execution duration"
    );
    metrics::describe_histogram!("cloacina_task_duration_seconds", "Task execution duration");
    metrics::describe_gauge!("cloacina_active_workflows", "Currently active workflows");
    metrics::describe_gauge!("cloacina_active_tasks", "Currently active tasks");
    metrics::describe_counter!(
        "cloacina_scheduler_claim_attempts_total",
        "Total task claim attempts by the executor. outcome is `claimed` (claim succeeded), \
         `contended` (another runner already held the claim), or `empty` (scheduler tick \
         found no ready tasks to dispatch)."
    );
    metrics::describe_counter!(
        "cloacina_scheduler_heartbeat_writes_total",
        "Total successful heartbeat writes by the executor's per-task heartbeat loop. \
         Failed heartbeats are recorded only in logs."
    );
    metrics::describe_counter!(
        "cloacina_scheduler_stale_claims_swept_total",
        "Total stale claims released by the stale-claim sweeper. Each increment \
         corresponds to one task whose runner heartbeat had expired and was reset to Ready."
    );
    metrics::describe_counter!(
        "cloacina_supervisor_restarts_total",
        "Total computation-graph supervisor restarts. Labels: graph (graph name), \
         component (`reactor` or accumulator name), reason \
         (`panic` | `error` | `shutdown_timeout`). `shutdown_timeout` is emitted \
         from the graceful-shutdown path; the supervision loop only observes \
         `panic` (JoinError::is_panic) and `error` (any other terminated handle)."
    );
    metrics::describe_gauge!(
        "cloacina_component_health",
        "Current computation-graph component health, expressed as a one-of \
         indicator. For each (graph, component) tuple the gauge is `1` on the \
         component's current state and `0` on every other state. State label is \
         bounded: `healthy | degraded | starting | stopped | crashed`. \
         Re-emitted every supervisor tick from the existing ReactorHealth / \
         AccumulatorHealth watch channels (projected via `as_state_label()`)."
    );
    metrics::describe_counter!(
        "cloacina_accumulator_events_total",
        "Total events processed by computation-graph accumulators. \
         `kind` is bounded to `passthrough | stream | polling | batch`; \
         `graph` is the deployed graph name (or `embedded` for runtimes \
         without a DAL); `accumulator` is the declared accumulator name."
    );
    metrics::describe_histogram!(
        "cloacina_accumulator_emit_duration_seconds",
        "End-to-end emit latency for each accumulator event: time from the \
         event arriving on the merge channel through `process()` + boundary \
         send + checkpoint persistence."
    );
    metrics::describe_gauge!(
        "cloacina_accumulator_buffer_depth",
        "Current internal buffer size for buffered accumulators. Meaningful \
         for `batch` and stateful `stream` kinds; `passthrough` and `polling` \
         emit `0` from runtime startup so dashboards see a stable series \
         per (graph, accumulator)."
    );
    metrics::describe_counter!(
        "cloacina_accumulator_checkpoint_writes_total",
        "Total successful checkpoint writes via `CheckpointHandle::save` or \
         `persist_boundary`. Failed writes are recorded in logs and do not \
         increment this counter."
    );
    metrics::describe_counter!(
        "cloacina_reactor_fires_total",
        "Total reactor fires (graph executions). `strategy` ∈ \
         `when_any | when_all | sequential` — projects the two-axis \
         (criteria × input_strategy) design onto a single bounded label."
    );
    metrics::describe_counter!(
        "cloacina_reactor_firings_total",
        "Total `reactor_firings` rows written by the reactor on each \
         fire (CLOACI-I-0100 / T-0599). One row per fire feeds the \
         subscription poller; this counter mirrors successful row \
         writes only — DAL failures land in logs."
    );
    metrics::describe_counter!(
        "cloacina_reactor_firings_pruned_total",
        "Total `reactor_firings` rows deleted by the unified scheduler's \
         TTL prune sweep (CLOACI-I-0100 / T-0601). Unlabeled — single \
         global counter is sufficient."
    );
    metrics::describe_histogram!(
        "cloacina_reactor_fire_duration_seconds",
        "Wall-clock duration of the user's compiled graph body (the time \
         inside `(graph)(snapshot).await`). Excludes cache lookup and \
         persistence overhead."
    );
    metrics::describe_gauge!(
        "cloacina_reactor_cache_age_seconds",
        "Age in seconds of the most-recent emission per source held in the \
         reactor's input cache. Refreshed on every boundary arrival (every \
         known source is re-emitted, so sources that fall silent show \
         increasing staleness). Sources that have never emitted are absent \
         from the gauge until their first boundary."
    );
    metrics::describe_counter!(
        "cloacina_reactor_deduped_events_total",
        "Total boundary events the reactor rejected as duplicates of an \
         already-seen emission sequence. Reserved for the reactor-side dedup \
         path that follows T-0413's persistence work — see I-0099 / T-0587 \
         for the rollout plan."
    );
    metrics::describe_gauge!(
        "cloacina_ws_connections_active",
        "Currently open WebSocket connections by endpoint. `endpoint` is \
         bounded `{accumulator, reactor}`. RAII-guarded so a panic inside a \
         handler still decrements on Drop — defends against the leak shape \
         that motivated T-0534."
    );
    metrics::describe_counter!(
        "cloacina_ws_messages_total",
        "Total WebSocket messages by `endpoint` (`accumulator` | `reactor`) \
         and `direction` (`in` | `out`). Counts framed messages — ping/pong \
         heartbeats handled by axum are excluded."
    );
    metrics::describe_counter!(
        "cloacina_ws_auth_failures_total",
        "Total rejected WebSocket upgrade requests by bounded `reason`: \
         `ticket_expired`, `invalid_signature`, `tenant_mismatch`, \
         `not_authorized`."
    );
    // Delivery substrate (CLOACI-I-0115 / T-0628). Backstop sweeper signals.
    metrics::describe_counter!(
        "cloacina_delivery_outbox_sweep_runs_total",
        "Total delivery-outbox sweep ticks. Sustained zero (with non-zero \
         outbox depth) means the sweeper is wedged; sustained high cadence is \
         expected and cheap (one scan per `sweep_interval`)."
    );
    metrics::describe_counter!(
        "cloacina_delivery_outbox_sweep_redeliveries_total",
        "Rows the sweeper reset from `delivered` back to `pending` because \
         their ack didn't arrive within `stuck_threshold` — i.e. rows the \
         relay had to retry via the backstop, not the normal NOTIFY path. \
         Sustained non-zero means recipients are disconnecting before \
         acking, or NOTIFY is being missed; chase via outbox-depth and the \
         WS connection metric."
    );
    metrics::describe_gauge!(
        "cloacina_delivery_outbox_open",
        "Current count of non-`acked` rows in `delivery_outbox` (sum of \
         `pending` + `delivered`). Sustained growth means delivery is wedged \
         — the analog of the compiler `sweep_resets_total` signal."
    );
    // Execution-agent fleet (CLOACI-I-0114 / T-0634).
    metrics::describe_counter!(
        "cloacina_fleet_agents_evicted_total",
        "Agents removed by the fleet heartbeat sweeper after their heartbeat \
         went stale (older than 3× the advertised interval). Sustained \
         non-zero means agents are dying or losing connectivity."
    );
    metrics::describe_counter!(
        "cloacina_fleet_work_reassigned_total",
        "In-flight delivery_outbox rows re-targeted from an evicted (dead) \
         agent to a live agent by the heartbeat sweeper's reclaim path \
         (CLOACI-T-0634). Tracks how much work crashed agents shed onto the \
         rest of the fleet."
    );
    metrics::describe_counter!(
        "cloacina_reactor_persist_failures_total",
        "Total `persist_reactor_state` failures, broken down by branch. \
         `kind` ∈ `cache_serialize`, `dirty_serialize`, `seq_serialize`, \
         `save`. The reactor downgrades to `ReactorHealth::Degraded` after \
         5 consecutive failures (any kind) and recovers on the next success. \
         See CLOACI-I-0108 / T-0590."
    );
    metrics::describe_counter!(
        "cloacina_accumulator_persist_failures_total",
        "Total accumulator persist failures, broken down by call site. \
         `kind` ∈ `checkpoint` (polling-accumulator save), `boundary` \
         (persist_boundary), `batch_buffer` (batch accumulator buffer save). \
         Replaces the silent `let _ = persist_*` patterns flagged as OPS-15."
    );
    metrics::describe_counter!(
        "cloacina_context_merge_failures_total",
        "Total failures merging dependency contexts into a task's input \
         context. `kind` ∈ `parse` (dependency context JSON failed to \
         deserialize — fails the task as `ContextLoadFailed`), `merge` \
         (Context API rejected an insert/update — counted but does not \
         fail the task). Closes COR-11."
    );

    // Connect to Postgres with DB-backed registry (so uploaded packages get compiled + loaded)
    let mut runner_builder = DefaultRunnerConfig::builder();
    runner_builder = runner_builder.registry_storage_backend("database");
    if let Some(interval) = reconcile_interval {
        runner_builder = runner_builder.registry_reconcile_interval(interval);
    }
    // CLOACI-T-0571: forward the verification config into the runner so the
    // reconciler's defense-in-depth signature-existence check fires even
    // when packages reach `workflow_packages` via paths other than the
    // upload route.
    runner_builder = runner_builder
        .require_signatures(require_signatures)
        .verification_org_id(verification_org_id.map(cloacina::UniversalUuid::from));
    // Default executor (CLOACI-T-0640): every task dispatches to this one
    // executor key. `"default"` keeps all work on the in-process thread
    // executor; `"fleet"` sends it to the execution-agent fleet. Applied to BOTH
    // the global runner and every per-tenant runner — tenant workflows execute
    // on per-tenant runners (TenantRunnerCache, T-0580), so the setting must
    // reach them too or fleet dispatch never fires for tenant tasks (the only
    // kind the server runs). The key is validated against the registered
    // executors after registration below.
    let use_fleet = default_executor == "fleet";
    info!(default_executor = %default_executor, "Default executor configured");
    runner_builder = runner_builder.default_executor(default_executor.clone());
    let runner_config = runner_builder
        .build()
        .context("Invalid runner configuration")?;

    let runner = DefaultRunner::with_config(&database_url, runner_config)
        .await
        .context("Failed to connect to database")?;

    info!("Connected to Postgres, migrations applied");

    let endpoint_registry = EndpointRegistry::new();
    let unified_dal = cloacina::dal::unified::DAL::new(runner.database().clone());
    let graph_scheduler = Arc::new(ComputationGraphScheduler::with_dal(
        endpoint_registry.clone(),
        unified_dal.clone(),
    ));

    // Substrate startup (CLOACI-I-0115, tasks T-0626/T-0627): construct the
    // WS delivery sink and the per-replica DeliveryRelay; spawn the relay's
    // event-driven drain loop and (on Postgres) the LISTEN task that forwards
    // NOTIFYs to the relay's wake handle. The `_substrate_shutdown_tx` is
    // held for the lifetime of `run()`; when it drops on shutdown, the spawned
    // tasks observe the receiver-side error and exit cleanly.
    let delivery_sink = Arc::new(crate::delivery_sink::WsDeliverySink::new());
    let delivery_relay =
        cloacina::delivery::DeliveryRelay::new(unified_dal.clone(), delivery_sink.clone());
    let delivery_wake = delivery_relay.wake_handle();
    let (_substrate_shutdown_tx, substrate_shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(delivery_relay.run(substrate_shutdown_rx.clone()));
    #[cfg(feature = "postgres")]
    tokio::spawn(cloacina::delivery::run_pg_listener(
        database_url.clone(),
        "delivery_outbox".to_string(),
        delivery_wake.clone(),
        substrate_shutdown_rx.clone(),
    ));
    // Safety-net sweeper (CLOACI-T-0628): periodic backstop for missed NOTIFYs,
    // disconnects, and replica crashes between commit and ack. Default
    // 30s/60s cadence/threshold; backend-agnostic.
    let delivery_sweeper =
        cloacina::delivery::DeliverySweeper::new(unified_dal.clone(), delivery_wake.clone());
    tokio::spawn(delivery_sweeper.run(substrate_shutdown_rx.clone()));

    // Wire graph scheduler into the runner so the reconciler can route CG packages
    runner.set_graph_scheduler(graph_scheduler.clone()).await;

    // Fleet (CLOACI-I-0114 / T-0633): shared roster + result-rendezvous so the
    // FleetExecutor (registered on the runner's dispatcher below) and the agent
    // HTTP routes (which read the same AppState) operate on one instance each.
    let agent_registry = Arc::new(crate::agent_registry::AgentRegistry::new());
    let fleet_coordinator = Arc::new(crate::fleet_coordinator::FleetCoordinator::new());

    // CLOACI-T-0634: per-tenant runner cache. Build it here (rather than inline
    // in the struct literal) so we can attach a fleet registrar — each per-tenant
    // runner needs its OWN FleetExecutor on its OWN dispatcher (tenant workflows
    // run on per-tenant runners, not the global one). The registrar builds a
    // tenant-scoped DAL + result handler (so status/context writes hit the
    // tenant's schema) and shares the process-global roster/coordinator/wake.
    let tenant_runner_cache = {
        let cache = crate::tenant_runner_cache::TenantRunnerCache::new(
            std::num::NonZeroUsize::new(tenant_runner_cache_size.max(1))
                .expect("max(1) is non-zero"),
            runner_config_for_tenant_cache(reconcile_interval, &default_executor),
        )
        // CLOACI-T-0581 follow-up: per-tenant runners share the global graph
        // scheduler so their reconcilers can route packaged CGs. The scheduler
        // stores tenant_id per graph (T-0579), so health-endpoint filtering
        // still works.
        .with_graph_scheduler(graph_scheduler.clone());
        if use_fleet {
            let reg_registry = agent_registry.clone();
            let reg_coordinator = fleet_coordinator.clone();
            let reg_wake = delivery_wake.clone();
            let registrar: crate::tenant_runner_cache::FleetRegistrar =
                Arc::new(move |runner: &cloacina::DefaultRunner| {
                    let dal = cloacina::dal::unified::DAL::new(runner.database().clone());
                    let result_handler = cloacina::executor::TaskResultHandler::new(
                        dal.clone(),
                        Arc::new(std::sync::atomic::AtomicU64::new(0)),
                        Arc::new(std::sync::atomic::AtomicU64::new(0)),
                        None,
                    );
                    let fleet_executor = crate::fleet_executor::FleetExecutor::new(
                        dal,
                        reg_registry.clone(),
                        reg_coordinator.clone(),
                        reg_wake.clone(),
                        result_handler,
                        runner.runtime(),
                    );
                    if runner.register_executor("fleet", Arc::new(fleet_executor)) {
                        info!("Fleet executor registered on per-tenant runner dispatcher");
                    } else {
                        warn!(
                            "per-tenant runner has no dispatcher — fleet routing will \
                             not fire for this tenant"
                        );
                    }
                });
            cache.with_fleet_registrar(registrar)
        } else {
            cache
        }
    };

    let state = AppState {
        database: runner.database().clone(),
        runner: Arc::new(runner),
        key_cache: Arc::new(crate::routes::auth::KeyCache::default_cache()),
        endpoint_registry,
        graph_scheduler: graph_scheduler.clone(),
        delivery_sink: delivery_sink.clone(),
        delivery_wake: delivery_wake.clone(),
        agent_registry: agent_registry.clone(),
        fleet_coordinator: fleet_coordinator.clone(),
        security_config: SecurityConfig {
            require_signatures,
            verification_org_id: verification_org_id.map(cloacina::UniversalUuid::from),
            ..SecurityConfig::default()
        },
        ws_tickets: Arc::new(crate::routes::auth::WsTicketStore::new(
            std::time::Duration::from_secs(60),
        )),
        metrics_handle,
        tenant_databases: Arc::new(TenantDatabaseCache::new(database_url.clone())),
        // CLOACI-T-0580: per-tenant runner cache. Capacity is operator-
        // tunable; 256 default. If the configured cap is zero we fall
        // back to 1 (LruCache requires NonZeroUsize) so misconfiguration
        // doesn't panic the server at boot.
        tenant_runners: Arc::new(tenant_runner_cache),
        tenant_deletion_drain_timeout,
        // Clamp to >=1 so the advertised interval matches the sweeper's
        // clamped cadence (the agent also clamps, but keep server-side parity).
        agent_heartbeat_interval_seconds: agent_heartbeat_interval_s.max(1),
    };

    // Bootstrap: create initial admin key if none exist
    bootstrap_admin_key(&state, &home, bootstrap_key.as_deref()).await?;

    // Operational-metrics publisher (CLOACI-T-0718): pushes server/compiler/
    // fleet/reconciler snapshots to a subscribed Operations UI over the WS
    // substrate (direct sink push, not the durable outbox). No-op when nothing
    // is subscribed.
    crate::ops_metrics::spawn(state.clone(), substrate_shutdown_rx.clone());

    // Fleet executor registration (CLOACI-I-0114 / T-0633, T-0640). Only wired
    // up when the operator opts into the fleet via `--default-executor fleet`;
    // otherwise all work stays on the in-process thread executor and there's no
    // reason to build it. Build the FleetExecutor over the shared roster +
    // coordinator + substrate wake, and register it on the runner's dispatcher
    // under the "fleet" key. Result reconciliation flows through the shared
    // TaskResultHandler so a fleet-run task's status/retry/context writes are
    // identical to a thread run. `runner_id = None`: the fleet relies on
    // substrate at-least-once + reconciliation for dedup rather than the thread
    // executor's per-task runner claim.
    if use_fleet {
        let fleet_result_handler = cloacina::executor::TaskResultHandler::new(
            unified_dal.clone(),
            Arc::new(std::sync::atomic::AtomicU64::new(0)),
            Arc::new(std::sync::atomic::AtomicU64::new(0)),
            None,
        );
        let fleet_executor = crate::fleet_executor::FleetExecutor::new(
            unified_dal.clone(),
            agent_registry.clone(),
            fleet_coordinator.clone(),
            delivery_wake.clone(),
            fleet_result_handler,
            state.runner.runtime(),
        );
        if state
            .runner
            .register_executor("fleet", Arc::new(fleet_executor))
        {
            info!("Fleet executor registered on dispatcher under key 'fleet'");
        } else {
            warn!(
                "Fleet executor NOT registered — runner has no dispatcher \
                 (push-based execution disabled); fleet dispatch will not fire"
            );
        }
    }

    // CLOACI-T-0640: hard-match the configured default executor against the
    // registered executor keys. The thread runner always registers "default";
    // "fleet" is registered just above only when opted in. A typo'd key would
    // otherwise silently send ALL work to a nonexistent executor — every task
    // would dispatch-error and the server would look hung — so fail fast at boot
    // instead.
    if !state.runner.has_executor(&default_executor) {
        let mut available = vec!["default"];
        if use_fleet && state.runner.has_executor("fleet") {
            available.push("fleet");
        }
        anyhow::bail!(
            "--default-executor '{default_executor}' is not a registered executor \
             (available: {}). Set a valid key via --default-executor / \
             CLOACINA_DEFAULT_EXECUTOR, or [server].default_executor in config.toml. \
             Use 'fleet' only with the execution-agent fleet deployed.",
            available.join(", ")
        );
    }

    // Fleet heartbeat sweeper (CLOACI-I-0114 / T-0634). Periodically evicts
    // agents whose heartbeat has gone stale (so selection + `has_capacity()`
    // stop counting dead agents) AND reclaims their in-flight work by
    // re-targeting its delivery_outbox rows to a live agent (otherwise the
    // rows stay pinned to a dead recipient and spin on NoRoute). Shares the
    // substrate shutdown channel.
    {
        let sweep_registry = agent_registry.clone();
        let sweep_dal = unified_dal.clone();
        let sweep_wake = delivery_wake.clone();
        let mut sweep_shutdown = substrate_shutdown_rx.clone();
        // Heartbeat interval the server advertises to agents + sweep cadence;
        // evict after `agent_liveness_misses` missed beats (CLOACI-T-0639).
        // Both clamped to >=1 so a 0 from the CLI can't wedge the ticker.
        let beat = std::time::Duration::from_secs(agent_heartbeat_interval_s.max(1) as u64);
        let dead_after = beat * agent_liveness_misses.max(1);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(beat);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let dead = sweep_registry.sweep_dead(dead_after);
                        if !dead.is_empty() {
                            info!(
                                evicted = ?dead,
                                "fleet: heartbeat sweeper evicted stale agent(s)"
                            );
                            metrics::counter!("cloacina_fleet_agents_evicted_total")
                                .increment(dead.len() as u64);

                            // Dead-agent reclaim (CLOACI-T-0634): re-target each
                            // dead agent's in-flight work to a live agent so it
                            // isn't pinned to a recipient whose connection is
                            // gone (which would spin on NoRoute). The work keeps
                            // its task_execution_id, so the FleetExecutor's
                            // awaiting rendezvous receives the new agent's result
                            // unchanged. If no live agent exists, the rows stay
                            // put and the FleetExecutor's timeout → retry path
                            // still recovers the task (degraded, not lost).
                            for dead_rec in &dead {
                                let dead_id = &dead_rec.agent_id;
                                let snap = sweep_registry.snapshot();
                                // Reclaim only to a live agent in the SAME tenant
                                // as the dead one (REQ-008 tenant isolation),
                                // greedy on most-free-capacity.
                                let target = snap
                                    .iter()
                                    .filter(|a| {
                                        a.available_capacity > 0
                                            && a.tenant_id == dead_rec.tenant_id
                                    })
                                    .max_by_key(|a| a.available_capacity);
                                match target {
                                    Some(t) => {
                                        match sweep_dal
                                            .delivery_outbox()
                                            .reassign_open_rows(
                                                &format!("agent:{dead_id}"),
                                                &format!("agent:{}", t.agent_id),
                                            )
                                            .await
                                        {
                                            Ok(n) if n > 0 => {
                                                info!(
                                                    dead_agent = %dead_id,
                                                    target_agent = %t.agent_id,
                                                    reassigned = n,
                                                    "fleet: reclaimed dead agent's in-flight work"
                                                );
                                                metrics::counter!(
                                                    "cloacina_fleet_work_reassigned_total"
                                                )
                                                .increment(n as u64);
                                                sweep_wake.wake();
                                            }
                                            Ok(_) => {}
                                            Err(e) => warn!(
                                                dead_agent = %dead_id,
                                                error = %e,
                                                "fleet: reclaim reassign failed"
                                            ),
                                        }
                                    }
                                    None => warn!(
                                        dead_agent = %dead_id,
                                        "fleet: no live agent to reclaim work; \
                                         FleetExecutor timeout will retry"
                                    ),
                                }
                            }
                        }
                    }
                    res = sweep_shutdown.changed() => {
                        if res.is_err() || *sweep_shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        });
    }

    // Keep references for shutdown
    let scheduler_for_shutdown = state.graph_scheduler.clone();
    let runner_for_shutdown = state.runner.clone();
    let tenant_runners_for_shutdown = state.tenant_runners.clone();

    // Build router; CORS (when enabled) wraps everything, including
    // preflight OPTIONS for the /v1 nest.
    let app = build_router(state);
    let app = match cors_layer {
        Some(layer) => {
            info!("CORS enabled for origins: {:?}", cors.allowed_origins);
            app.layer(layer)
        }
        None => app,
    };

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("Failed to bind to {}", bind))?;

    info!("");
    info!("API server is running on http://{}", bind);
    info!("  GET  /health     — liveness check");
    info!("  GET  /ready      — readiness check");
    info!("  GET  /metrics    — Prometheus metrics");
    info!("  POST /v1/auth/keys      — create API key (auth required)");
    info!("  GET  /v1/auth/keys      — list API keys (auth required)");
    info!("  DEL  /v1/auth/keys/:id  — revoke key (auth required)");
    info!("");

    // Shared shutdown signal — used by supervision loop and graceful shutdown.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Start supervision loop — auto-restart crashed accumulator/reactor tasks
    let _supervision_handle = scheduler_for_shutdown
        .start_supervision(shutdown_rx.clone(), std::time::Duration::from_secs(5));

    let scheduler_handle = {
        let scheduler = scheduler_for_shutdown.clone();
        let mut rx = shutdown_rx; // move, not clone — only consumer
        tokio::spawn(async move {
            let _ = rx.changed().await;
            info!("Shutting down graph scheduler...");
            scheduler.shutdown_all().await;
            info!("Computation graph scheduler shutdown complete");
        })
    };

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        shutdown_signal().await;
        // Signal the graph scheduler to shut down first
        let _ = shutdown_tx.send(true);
        // Wait for graph scheduler to finish flushing/persisting
        let _ = scheduler_handle.await;
        // Shut down the workflow runner (scheduler loop, executor, stale claim sweeper)
        info!("Shutting down workflow runner...");
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            runner_for_shutdown.shutdown(),
        )
        .await
        {
            Ok(Ok(())) => info!("Workflow runner shutdown complete"),
            Ok(Err(e)) => warn!("Workflow runner shutdown error: {}", e),
            Err(_) => warn!("Workflow runner shutdown timed out after 30s"),
        }
        // CLOACI-T-0580: shut down every cached per-tenant runner.
        info!("Shutting down tenant runner cache...");
        let results = tenant_runners_for_shutdown.shutdown_all().await;
        let total = results.len();
        let failed = results.values().filter(|r| r.is_err()).count();
        if failed == 0 {
            info!(
                tenant_runners = total,
                "Tenant runner cache shutdown complete"
            );
        } else {
            warn!(
                tenant_runners = total,
                failed, "Tenant runner cache shutdown completed with errors"
            );
            for (tenant, result) in results {
                if let Err(e) = result {
                    warn!(tenant_id = %tenant, error = %e, "tenant runner shutdown failed");
                }
            }
        }
    })
    .await
    .context("Server error")?;

    info!("API server shutdown complete");
    Ok(())
}

/// Middleware that generates a UUID request ID, creates a tracing span,
/// and adds the X-Request-Id response header.
async fn request_id_middleware(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let id = uuid::Uuid::new_v4().to_string();

    // CLOACI-T-0578: pre-declare auth-derived fields as Empty so the auth
    // middleware can `record(...)` them onto this span after extraction.
    // `tracing::field::Empty` reserves the field; a recording call later
    // attaches the actual value, and the JSON/OTLP subscriber renders it
    // in every event nested under this span (handler logs, audit emits).
    let span = tracing::info_span!(
        "request",
        request_id = %id,
        method = %request.method(),
        path = %request.uri().path(),
        tenant_id = tracing::field::Empty,
        key_id = tracing::field::Empty,
        role = tracing::field::Empty,
    );
    let mut response = {
        use tracing::Instrument;
        next.run(request).instrument(span).await
    };
    if let Ok(val) = id.parse() {
        response.headers_mut().insert("x-request-id", val);
    }
    response
}

fn build_router(state: AppState) -> Router {
    use axum::{extract::DefaultBodyLimit, middleware, routing::delete, routing::post};

    // Authenticated routes — behind auth middleware
    let auth_routes = Router::new()
        // Key management
        .route("/auth/keys", post(crate::routes::keys::create_key))
        .route("/auth/keys", get(crate::routes::keys::list_keys))
        .route(
            "/auth/keys/{key_id}",
            delete(crate::routes::keys::revoke_key),
        )
        // WebSocket ticket exchange (single-use, short-lived)
        .route(
            "/auth/ws-ticket",
            post(crate::routes::keys::create_ws_ticket),
        )
        // Execution-agent fleet roster (admin, operator-facing) — CLOACI-I-0124
        .route("/agents", get(crate::routes::agent::list_agents))
        // Compiler / build-pipeline status (admin) — CLOACI-I-0124
        .route(
            "/compiler/status",
            get(crate::routes::compiler::compiler_status),
        )
        // Tenant management
        .route("/tenants", post(crate::routes::tenants::create_tenant))
        .route("/tenants", get(crate::routes::tenants::list_tenants))
        .route(
            "/tenants/{schema_name}",
            delete(crate::routes::tenants::remove_tenant),
        )
        // Tenant-scoped key creation (admin-only)
        .route(
            "/tenants/{tenant_id}/keys",
            post(crate::routes::keys::create_tenant_key),
        )
        // Workflow packages (tenant-scoped)
        .route(
            "/tenants/{tenant_id}/workflows",
            post(crate::routes::workflows::upload_workflow),
        )
        .route(
            "/tenants/{tenant_id}/workflows",
            get(crate::routes::workflows::list_workflows),
        )
        .route(
            "/tenants/{tenant_id}/workflows/{name}",
            get(crate::routes::workflows::get_workflow),
        )
        .route(
            "/tenants/{tenant_id}/workflows/{name}/source",
            get(crate::routes::workflows::get_workflow_source),
        )
        // Pause / resume a workflow (block new executions) — CLOACI-T-0749.
        .route(
            "/tenants/{tenant_id}/workflows/{name}/pause",
            post(crate::routes::workflows::pause_workflow),
        )
        .route(
            "/tenants/{tenant_id}/workflows/{name}/resume",
            post(crate::routes::workflows::resume_workflow),
        )
        .route(
            "/tenants/{tenant_id}/workflows/{name}/{version}",
            delete(crate::routes::workflows::delete_workflow),
        )
        // Trigger schedules (tenant-scoped, read-only)
        .route(
            "/tenants/{tenant_id}/triggers",
            get(crate::routes::triggers::list_triggers),
        )
        .route(
            "/tenants/{tenant_id}/triggers/{name}",
            get(crate::routes::triggers::get_trigger),
        )
        // Pause / resume a schedule (cron or trigger) — CLOACI-T-0749.
        .route(
            "/tenants/{tenant_id}/triggers/{name}/pause",
            post(crate::routes::triggers::pause_trigger),
        )
        .route(
            "/tenants/{tenant_id}/triggers/{name}/resume",
            post(crate::routes::triggers::resume_trigger),
        )
        .route(
            "/tenants/{tenant_id}/triggers/{name}/fire",
            post(crate::routes::triggers::fire_trigger),
        )
        .route(
            "/tenants/{tenant_id}/triggers/{name}/interface",
            get(crate::routes::triggers::get_trigger_interface),
        )
        // Executions (tenant-scoped)
        .route(
            "/tenants/{tenant_id}/workflows/{name}/execute",
            post(crate::routes::executions::execute_workflow),
        )
        .route(
            "/tenants/{tenant_id}/executions",
            get(crate::routes::executions::list_executions),
        )
        .route(
            "/tenants/{tenant_id}/executions/{exec_id}",
            get(crate::routes::executions::get_execution),
        )
        .route(
            "/tenants/{tenant_id}/executions/{exec_id}/events",
            get(crate::routes::executions::get_execution_events),
        )
        .route(
            "/tenants/{tenant_id}/executions/{exec_id}/tasks",
            get(crate::routes::executions::get_execution_tasks),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::routes::auth::require_auth,
        ));

    // Computation graph health routes — behind auth.
    // CLOACI-T-0595 / API-08: paths are relative to the `/v1` nest below
    // (previously these used absolute `/v1/...` and were merged at the
    // top level, bypassing the `/v1` nest's middleware contract).
    let graph_health_routes = Router::new()
        .route(
            "/health/accumulators",
            get(crate::routes::health_graphs::list_accumulators),
        )
        .route(
            "/health/reactors",
            get(crate::routes::health_graphs::list_reactors),
        )
        // Operator manual reactor fire (CLOACI-T-0751) — REST surface over the
        // existing reactor ForceFire/FireWith mechanics. Per-op authZ + audit
        // live in the handler; the route-layer auth below supplies the key.
        .route(
            "/health/reactors/{name}/fire",
            post(crate::routes::health_graphs::fire_reactor),
        )
        // Observable fires (CLOACI-T-0766): recent fires + per-minute cadence.
        .route(
            "/health/reactors/{name}/fires",
            get(crate::routes::health_graphs::list_reactor_fires),
        )
        .route(
            "/health/reactors/{name}/fires/timeseries",
            get(crate::routes::health_graphs::reactor_fire_timeseries),
        )
        // Operator manual accumulator inject (CLOACI-T-0753) — REST analogue of
        // the WS accumulator-push path; typed JSON event, server-side encoding.
        .route(
            "/health/accumulators/{name}/inject",
            post(crate::routes::health_graphs::inject_accumulator),
        )
        // Declared input-interface discovery (CLOACI-I-0128 / T-0758) — the typed
        // slots an operator supplies to fire/inject; backs UI typed forms.
        .route(
            "/health/reactors/{name}/interface",
            get(crate::routes::health_graphs::get_reactor_interface),
        )
        .route(
            "/health/accumulators/{name}/interface",
            get(crate::routes::health_graphs::get_accumulator_interface),
        )
        .route(
            "/health/graphs",
            get(crate::routes::health_graphs::list_graphs),
        )
        .route(
            "/health/graphs/{name}",
            get(crate::routes::health_graphs::get_graph),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::routes::auth::require_auth,
        ));

    // WebSocket routes — auth handled in the handler (before upgrade).
    // Relative paths so they nest cleanly under `/v1` per API-08.
    let ws_routes = Router::new()
        .route(
            "/ws/accumulator/{name}",
            get(crate::routes::ws::accumulator_ws),
        )
        .route("/ws/reactor/{name}", get(crate::routes::ws::reactor_ws))
        // Substrate delivery (CLOACI-I-0115 / T-0627). Auth handled in the
        // handler (header bearer or single-use ticket); tenant inferred from
        // the authenticated key and enforced against delivery_outbox.tenant_id.
        .route(
            "/ws/delivery/{recipient}",
            get(crate::routes::delivery_ws::delivery_ws),
        );

    // Execution-agent fleet (CLOACI-I-0114 / T-0631). All endpoints share
    // the standard authed-route middleware; tenant + admin checks live in
    // each handler. T-0633 will wire the FleetExecutor into these routes;
    // for now register/heartbeat mutate the in-memory roster and result is
    // a stub.
    let agent_routes = axum::routing::Router::new()
        .route(
            "/agent/register",
            axum::routing::post(crate::routes::agent::register_agent),
        )
        .route(
            "/agent/heartbeat",
            axum::routing::post(crate::routes::agent::heartbeat_agent),
        )
        .route(
            "/agent/result",
            axum::routing::post(crate::routes::agent::report_result),
        )
        .route(
            "/agent/artifact/{digest}",
            axum::routing::get(crate::routes::agent::fetch_artifact),
        )
        .route(
            "/agent/source/{digest}",
            axum::routing::get(crate::routes::agent::fetch_source),
        )
        // All four agent endpoints authenticate with an API key (bearer) and
        // extract `Extension<AuthenticatedKey>`. The layer MUST be attached
        // here, before the merge into `auth_routes` below: `.route_layer` only
        // covers routes already in the router, so `auth_routes`' own layer does
        // NOT reach these merged routes. Without this, `register_agent` panics
        // on the missing extension → 500 → the agent exits on register. The
        // delivery WS (ws_routes) is deliberately NOT key-authed — it uses a
        // single-use ws-ticket — so auth stays per-sub-router, not nest-wide.
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::routes::auth::require_auth,
        ));

    // All of v1 in a single nest — API-08 invariant: anything served
    // under `/v1/*` shares the same middleware stack.
    let v1 = auth_routes
        .merge(graph_health_routes)
        .merge(ws_routes)
        .merge(agent_routes);

    // Public routes — no auth
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
        // Machine-readable API contract (CLOACI-T-0643 / REQ-001). Same
        // document the `emit-openapi` subcommand writes to disk.
        .route(
            "/openapi.json",
            get(|| async {
                (
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    crate::openapi::openapi_json(),
                )
            }),
        )
        // All v1 routes (auth + graph health + ws) under one nest.
        .nest("/v1", v1)
        .fallback(fallback_404)
        // Body size limit: 100MB (matches PackageValidator)
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        // API request metrics (counts by method and status)
        .layer(middleware::from_fn(api_request_metrics))
        // Request ID + tracing span (outermost — wraps everything)
        .layer(middleware::from_fn(request_id_middleware))
        .with_state(state)
}

/// Middleware that counts API requests by method and status code, and records
/// handler duration as a histogram.
async fn api_request_metrics(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = request.method().to_string();
    let started = std::time::Instant::now();
    let response = next.run(request).await;
    let elapsed = started.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();
    metrics::counter!(
        "cloacina_api_requests_total",
        "method" => method.clone(),
        "status" => status.clone(),
    )
    .increment(1);
    metrics::histogram!(
        "cloacina_api_request_duration_seconds",
        "method" => method,
        "status" => status,
    )
    .record(elapsed);
    response
}

/// GET /health — liveness check (no auth, no DB)
#[utoipa::path(
    get,
    path = "/health",
    tag = "operational",
    responses((status = 200, description = "Server process is alive — `{\"status\": \"ok\"}`"))
)]
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}

/// GET /ready — readiness check (verifies DB connection pool is healthy)
#[utoipa::path(
    get,
    path = "/ready",
    tag = "operational",
    responses(
        (status = 200, description = "Ready — DB reachable, no crashed graphs"),
        (status = 503, description = "Not ready — `reason` field explains (database unreachable / crashed computation graphs)"),
    )
)]
async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    // Verify we can acquire a connection from the pool
    let db_ready = state.database.get_postgres_connection().await.is_ok();

    // Check if any computation graphs have crashed
    let graphs = state.graph_scheduler.list_graphs().await;
    let crashed_graphs: Vec<&str> = graphs
        .iter()
        .filter(|g| !g.running)
        .map(|g| g.name.as_str())
        .collect();

    if db_ready && crashed_graphs.is_empty() {
        (StatusCode::OK, Json(serde_json::json!({"status": "ready"})))
    } else if !db_ready {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "not ready", "reason": "database unreachable"})),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "not ready",
                "reason": "crashed computation graphs",
                "crashed_graphs": crashed_graphs,
            })),
        )
    }
}

/// GET /metrics — Prometheus metrics rendered from the recorder installed at startup.
async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let body = state.metrics_handle.render();
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

/// Fallback for unmatched routes — returns the canonical `ApiError`
/// envelope (CLOACI-T-0595 / API-06) so every server error matches the
/// same shape regardless of which handler (or no handler) produced it.
async fn fallback_404() -> impl IntoResponse {
    crate::routes::error::ApiError::not_found("not_found", "no route matches this request")
}

/// Wait for shutdown signal (SIGINT or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl+c");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received SIGINT — shutting down"),
        _ = terminate => info!("Received SIGTERM — shutting down"),
    }
}

/// Bootstrap: create an admin API key on first startup if none exist.
///
/// Writes the plaintext key to `~/.cloacina/bootstrap-key` with mode 0600.
/// The key is never logged.
async fn bootstrap_admin_key(
    state: &AppState,
    home: &std::path::Path,
    provided_key: Option<&str>,
) -> Result<()> {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let has_keys = dal.api_keys().has_any_keys().await.unwrap_or(false);

    if has_keys {
        info!("API keys exist — skipping bootstrap");
        return Ok(());
    }

    info!("No API keys found — creating bootstrap admin key");

    let (plaintext, hash) = if let Some(key) = provided_key {
        // Use provided key
        let hash = cloacina::security::api_keys::hash_api_key(key);
        (key.to_string(), hash)
    } else {
        // Auto-generate
        cloacina::security::api_keys::generate_api_key()
    };

    dal.api_keys()
        .create_key(&hash, "bootstrap-admin", None, true, "admin")
        .await
        .context("Failed to create bootstrap admin key")?;

    // Write plaintext to file (never log it)
    let key_path = home.join("bootstrap-key");
    std::fs::write(&key_path, &plaintext)
        .with_context(|| format!("Failed to write bootstrap key to {}", key_path.display()))?;

    // Set file permissions to owner-only (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Failed to set permissions on {}", key_path.display()))?;
    }

    info!(
        "Bootstrap admin key written to {} (mode 0600)",
        key_path.display()
    );

    Ok(())
}

/// Mask password in database URL for logging
/// Re-export from cloacina::logging for backward compat in tests.
fn mask_db_url(url: &str) -> String {
    cloacina::logging::mask_db_url(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use serial_test::serial;
    use std::io::Write;
    use tower::ServiceExt;

    const TEST_DB_URL: &str = "postgres://cloacina:cloacina@localhost:5432/cloacina";

    /// One Prometheus recorder per test process, shared by every `test_state()`.
    ///
    /// `install_recorder()` installs the GLOBAL recorder and only succeeds once
    /// per process. The metric macros (`metrics::counter!`, etc.) always emit to
    /// that global recorder, so a test that scrapes a *different* handle sees
    /// nothing. Previously each `test_state()` tried to install and, after the
    /// first, fell back to a fresh unconnected handle — so every metrics test
    /// that didn't run first failed. Installing once and handing out clones of
    /// the same global handle keeps emit + scrape on the same recorder.
    fn shared_test_metrics_handle() -> metrics_exporter_prometheus::PrometheusHandle {
        static HANDLE: std::sync::OnceLock<metrics_exporter_prometheus::PrometheusHandle> =
            std::sync::OnceLock::new();
        HANDLE
            .get_or_init(|| {
                metrics_exporter_prometheus::PrometheusBuilder::new()
                    .install_recorder()
                    .expect("install global test Prometheus recorder")
            })
            .clone()
    }

    /// One shared `DefaultRunner` (+ its connection pool) for the whole test
    /// process. Each `test_state()` previously built its own runner via
    /// `with_config`, which opens a connection pool plus background services
    /// that are never torn down — so ~90 serial tests accumulated ~90 pools and
    /// exhausted postgres `max_connections` ("too many clients") on hosts with a
    /// modest cap (macos default ~100; the docker postgres bumps it to 500).
    /// Every test already targets the same `TEST_DB_URL` database with no
    /// per-test reset, so sharing the runner changes only the connection count,
    /// not isolation (and tests are `#[serial]`). CLOACI-T-0639 follow-up.
    async fn shared_test_runner() -> Arc<cloacina::runner::DefaultRunner> {
        static RUNNER: tokio::sync::OnceCell<Arc<cloacina::runner::DefaultRunner>> =
            tokio::sync::OnceCell::const_new();
        RUNNER
            .get_or_init(|| async {
                let runner_config = cloacina::runner::DefaultRunnerConfig::builder()
                    .registry_storage_backend("database")
                    .build()
                    .expect("test config must be valid");
                Arc::new(
                    cloacina::runner::DefaultRunner::with_config(TEST_DB_URL, runner_config)
                        .await
                        .expect("Failed to connect to test database"),
                )
            })
            .await
            .clone()
    }

    /// Create a test AppState with a real Postgres connection.
    async fn test_state() -> AppState {
        let runner = shared_test_runner().await;

        // Share the one globally-installed recorder so emitted metrics are
        // visible when this state's `/metrics` handle is scraped.
        let test_metrics_handle = shared_test_metrics_handle();

        // Substrate components for the test AppState. Spawned tasks are
        // intentionally omitted here — test fixtures that exercise delivery
        // construct their own relay if needed; the sink and a wake handle are
        // enough for handler-side code to compile and run.
        let _test_delivery_sink = Arc::new(crate::delivery_sink::WsDeliverySink::new());
        let _test_delivery_relay = cloacina::delivery::DeliveryRelay::new(
            cloacina::dal::unified::DAL::new(runner.database().clone()),
            _test_delivery_sink.clone(),
        );
        let _test_delivery_wake = _test_delivery_relay.wake_handle();
        AppState {
            database: runner.database().clone(),
            runner,
            key_cache: Arc::new(crate::routes::auth::KeyCache::default_cache()),
            endpoint_registry: EndpointRegistry::new(),
            graph_scheduler: Arc::new(ComputationGraphScheduler::new(EndpointRegistry::new())),
            delivery_sink: _test_delivery_sink,
            delivery_wake: _test_delivery_wake,
            agent_registry: Arc::new(crate::agent_registry::AgentRegistry::new()),
            fleet_coordinator: Arc::new(crate::fleet_coordinator::FleetCoordinator::new()),
            security_config: SecurityConfig::default(),
            ws_tickets: Arc::new(crate::routes::auth::WsTicketStore::new(
                std::time::Duration::from_secs(60),
            )),
            metrics_handle: test_metrics_handle,
            tenant_databases: Arc::new(TenantDatabaseCache::new(TEST_DB_URL.to_string())),
            tenant_runners: Arc::new(crate::tenant_runner_cache::TenantRunnerCache::new(
                std::num::NonZeroUsize::new(8).expect("test cap"),
                runner_config_for_tenant_cache(None, "default"),
            )),
            tenant_deletion_drain_timeout: std::time::Duration::from_secs(5),
            agent_heartbeat_interval_seconds: cloacina::fleet::DEFAULT_HEARTBEAT_INTERVAL_SECONDS,
        }
    }

    /// Create a test AppState with `require_signatures = true` and a known
    /// `verification_org_id`. Used by the T-0570 signature-contract tests
    /// to drive the upload route through its verification gate.
    async fn test_state_with_signature_required(
        verification_org_id: cloacina::UniversalUuid,
    ) -> AppState {
        let mut state = test_state().await;
        state.security_config = SecurityConfig {
            require_signatures: true,
            verification_org_id: Some(verification_org_id),
            ..SecurityConfig::default()
        };
        state
    }

    /// Create a bootstrap API key and return the plaintext token.
    async fn create_test_api_key(state: &AppState) -> String {
        let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();
        let dal = cloacina::dal::DAL::new(state.database.clone());
        dal.api_keys()
            .create_key(&hash, "test-key", None, true, "admin")
            .await
            .expect("Failed to create test API key");
        plaintext
    }

    /// Send a request to the router and return (status, body as serde_json::Value).
    async fn send_request(
        app: Router,
        request: axum::http::Request<Body>,
    ) -> (StatusCode, serde_json::Value) {
        let response = app.oneshot(request).await.expect("request failed");
        let status = response.status();
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let body: serde_json::Value =
            serde_json::from_slice(&body_bytes).unwrap_or(serde_json::json!({}));
        (status, body)
    }

    // ── Request ID middleware ─────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_request_id_header_present() {
        let state = test_state().await;
        let app = build_router(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);
        let request_id = response.headers().get("x-request-id");
        assert!(
            request_id.is_some(),
            "Response should include X-Request-Id header"
        );
        let id_str = request_id.unwrap().to_str().unwrap();
        assert!(
            uuid::Uuid::parse_str(id_str).is_ok(),
            "X-Request-Id should be a valid UUID, got: {}",
            id_str
        );
    }

    // ── Health / Ready / Metrics ──────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_health_returns_200() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    #[serial]
    async fn test_ready_returns_200_with_db() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/ready")
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ready");
    }

    #[tokio::test]
    #[serial]
    async fn test_metrics_returns_prometheus_format() {
        let state = test_state().await;

        // Record some test metrics so the output isn't empty
        metrics::counter!(
            "cloacina_workflows_total",
            "status" => "completed",
            "reason" => "ok",
        )
        .increment(3);
        metrics::counter!(
            "cloacina_tasks_total",
            "status" => "completed",
            "reason" => "ok",
        )
        .increment(10);
        metrics::counter!(
            "cloacina_tasks_total",
            "status" => "failed",
            "reason" => "task_error",
        )
        .increment(2);

        let app = build_router(state);

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);

        let content_type = response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            content_type.contains("text/plain"),
            "Content-Type should be text/plain for Prometheus, got: {}",
            content_type
        );

        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let text = String::from_utf8_lossy(&body_bytes);

        // Verify Prometheus text format: HELP, TYPE, and metric lines
        assert!(
            text.contains("cloacina_workflows_total"),
            "Metrics should contain workflow counters. Got:\n{}",
            text
        );
        assert!(
            text.contains("cloacina_tasks_total"),
            "Metrics should contain task counters. Got:\n{}",
            text
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_scheduler_loop_metrics_emit() {
        let state = test_state().await;

        // Emit each new scheduler-loop metric exactly as the production code does.
        // Mirrors:
        //   - executor/thread_task_executor.rs claim_for_runner → {claimed, contended}
        //   - execution_planner/scheduler_loop.rs dispatch_ready_tasks (no work) → empty
        //   - executor/thread_task_executor.rs heartbeat loop on Ok
        //   - execution_planner/stale_claim_sweeper.rs after mark_ready succeeds
        metrics::counter!(
            "cloacina_scheduler_claim_attempts_total",
            "outcome" => "claimed",
        )
        .increment(1);
        metrics::counter!(
            "cloacina_scheduler_claim_attempts_total",
            "outcome" => "contended",
        )
        .increment(1);
        metrics::counter!(
            "cloacina_scheduler_claim_attempts_total",
            "outcome" => "empty",
        )
        .increment(1);
        metrics::counter!("cloacina_scheduler_heartbeat_writes_total").increment(1);
        metrics::counter!("cloacina_scheduler_stale_claims_swept_total").increment(1);

        let app = build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let text = String::from_utf8_lossy(&body_bytes);

        // Each new metric registered with describe_counter! and emitted at least once
        // must appear in the exposition output.
        assert!(
            text.contains("cloacina_scheduler_claim_attempts_total"),
            "Missing claim_attempts_total in /metrics output:\n{}",
            text
        );
        // All three outcome label values must be present.
        for outcome in ["claimed", "contended", "empty"] {
            assert!(
                text.contains(&format!(
                    "cloacina_scheduler_claim_attempts_total{{outcome=\"{}\"}}",
                    outcome
                )),
                "Missing outcome={} label in claim_attempts_total. Got:\n{}",
                outcome,
                text
            );
        }
        assert!(
            text.contains("cloacina_scheduler_heartbeat_writes_total"),
            "Missing heartbeat_writes_total in /metrics output:\n{}",
            text
        );
        assert!(
            text.contains("cloacina_scheduler_stale_claims_swept_total"),
            "Missing stale_claims_swept_total in /metrics output:\n{}",
            text
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_supervisor_health_metrics_emit() {
        let state = test_state().await;

        // Emit each new supervisor / health metric exactly as the production
        // code does. Mirrors:
        //   - computation_graph/scheduler.rs check_and_restart_failed →
        //     supervisor_restarts_total at reactor + accumulator restart paths
        //   - computation_graph/scheduler.rs emit_health_metrics →
        //     component_health gauge from supervision tick
        metrics::counter!(
            "cloacina_supervisor_restarts_total",
            "graph" => "test_graph",
            "component" => "reactor",
            "reason" => "panic",
        )
        .increment(1);
        metrics::counter!(
            "cloacina_supervisor_restarts_total",
            "graph" => "test_graph",
            "component" => "acc_a",
            "reason" => "error",
        )
        .increment(1);

        for state_label in ["healthy", "degraded", "starting", "stopped", "crashed"] {
            let value = if state_label == "healthy" { 1.0 } else { 0.0 };
            metrics::gauge!(
                "cloacina_component_health",
                "graph" => "test_graph",
                "component" => "reactor",
                "state" => state_label,
            )
            .set(value);
        }

        let app = build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let text = String::from_utf8_lossy(&body_bytes);

        assert!(
            text.contains("cloacina_supervisor_restarts_total"),
            "Missing supervisor_restarts_total in /metrics output:\n{}",
            text
        );
        // Both bounded reasons must round-trip through the exporter.
        for reason in ["panic", "error"] {
            assert!(
                text.contains(&format!("reason=\"{}\"", reason)),
                "Missing reason={} in supervisor restarts. Got:\n{}",
                reason,
                text
            );
        }
        assert!(
            text.contains("cloacina_component_health"),
            "Missing component_health gauge in /metrics output:\n{}",
            text
        );
        // Exactly one state==1 invariant: assert all five state values appear
        // (one as 1, four as 0).
        for state_label in ["healthy", "degraded", "starting", "stopped", "crashed"] {
            assert!(
                text.contains(&format!("state=\"{}\"", state_label)),
                "Missing state={} in component_health. Got:\n{}",
                state_label,
                text
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_accumulator_metrics_emit() {
        let state = test_state().await;

        // Emit each accumulator metric exactly as the production runtimes do.
        for kind in ["passthrough", "stream", "polling", "batch"] {
            metrics::counter!(
                "cloacina_accumulator_events_total",
                "graph" => "test_graph",
                "accumulator" => format!("acc_{}", kind),
                "kind" => kind,
            )
            .increment(1);
            metrics::histogram!(
                "cloacina_accumulator_emit_duration_seconds",
                "graph" => "test_graph",
                "accumulator" => format!("acc_{}", kind),
            )
            .record(0.001);
        }
        metrics::gauge!(
            "cloacina_accumulator_buffer_depth",
            "graph" => "test_graph",
            "accumulator" => "acc_batch",
        )
        .set(42.0);
        metrics::counter!(
            "cloacina_accumulator_checkpoint_writes_total",
            "graph" => "test_graph",
            "accumulator" => "acc_passthrough",
        )
        .increment(1);

        let app = build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let text = String::from_utf8_lossy(&body_bytes);

        // All four kind label values must round-trip
        for kind in ["passthrough", "stream", "polling", "batch"] {
            assert!(
                text.contains(&format!("kind=\"{}\"", kind)),
                "Missing kind={} in accumulator_events_total. Got:\n{}",
                kind,
                text
            );
        }
        assert!(
            text.contains("cloacina_accumulator_emit_duration_seconds"),
            "Missing emit_duration histogram in /metrics output:\n{}",
            text
        );
        assert!(
            text.contains("cloacina_accumulator_buffer_depth"),
            "Missing buffer_depth gauge in /metrics output:\n{}",
            text
        );
        assert!(
            text.contains("cloacina_accumulator_checkpoint_writes_total"),
            "Missing checkpoint_writes_total in /metrics output:\n{}",
            text
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_reactor_metrics_emit() {
        let state = test_state().await;

        for strategy in ["when_any", "when_all", "sequential"] {
            metrics::counter!(
                "cloacina_reactor_fires_total",
                "graph" => "test_graph",
                "reactor" => "test_reactor",
                "strategy" => strategy,
            )
            .increment(1);
        }
        metrics::histogram!(
            "cloacina_reactor_fire_duration_seconds",
            "graph" => "test_graph",
            "reactor" => "test_reactor",
        )
        .record(0.002);
        for source in ["src_a", "src_b"] {
            metrics::gauge!(
                "cloacina_reactor_cache_age_seconds",
                "graph" => "test_graph",
                "reactor" => "test_reactor",
                "source" => source,
            )
            .set(1.5);
        }
        metrics::counter!(
            "cloacina_reactor_deduped_events_total",
            "graph" => "test_graph",
            "reactor" => "test_reactor",
            "source" => "src_a",
        )
        .increment(1);

        let app = build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let text = String::from_utf8_lossy(&body_bytes);

        for strategy in ["when_any", "when_all", "sequential"] {
            assert!(
                text.contains(&format!("strategy=\"{}\"", strategy)),
                "Missing strategy={} in reactor_fires_total. Got:\n{}",
                strategy,
                text
            );
        }
        assert!(
            text.contains("cloacina_reactor_fire_duration_seconds"),
            "Missing fire_duration histogram in /metrics output:\n{}",
            text
        );
        assert!(
            text.contains("cloacina_reactor_cache_age_seconds"),
            "Missing cache_age gauge in /metrics output:\n{}",
            text
        );
        assert!(
            text.contains("cloacina_reactor_deduped_events_total"),
            "Missing deduped_events counter in /metrics output:\n{}",
            text
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_ws_metrics_emit() {
        let state = test_state().await;

        for endpoint in ["accumulator", "reactor"] {
            metrics::gauge!(
                "cloacina_ws_connections_active",
                "endpoint" => endpoint,
            )
            .set(1.0);
            for direction in ["in", "out"] {
                metrics::counter!(
                    "cloacina_ws_messages_total",
                    "endpoint" => endpoint,
                    "direction" => direction,
                )
                .increment(1);
            }
        }
        for reason in [
            "ticket_expired",
            "invalid_signature",
            "tenant_mismatch",
            "not_authorized",
        ] {
            metrics::counter!(
                "cloacina_ws_auth_failures_total",
                "reason" => reason,
            )
            .increment(1);
        }

        let app = build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let text = String::from_utf8_lossy(&body_bytes);

        for endpoint in ["accumulator", "reactor"] {
            assert!(
                text.contains(&format!("endpoint=\"{}\"", endpoint)),
                "Missing endpoint={} label. Got:\n{}",
                endpoint,
                text
            );
        }
        for direction in ["in", "out"] {
            assert!(
                text.contains(&format!("direction=\"{}\"", direction)),
                "Missing direction={} label in ws_messages_total. Got:\n{}",
                direction,
                text
            );
        }
        for reason in [
            "ticket_expired",
            "invalid_signature",
            "tenant_mismatch",
            "not_authorized",
        ] {
            assert!(
                text.contains(&format!("reason=\"{}\"", reason)),
                "Missing reason={} in ws_auth_failures_total. Got:\n{}",
                reason,
                text
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_persist_failure_metrics_emit() {
        let state = test_state().await;

        for kind in [
            "cache_serialize",
            "dirty_serialize",
            "seq_serialize",
            "save",
        ] {
            metrics::counter!(
                "cloacina_reactor_persist_failures_total",
                "graph" => "test_graph",
                "reactor" => "test_reactor",
                "kind" => kind,
            )
            .increment(1);
        }
        for kind in ["checkpoint", "boundary", "batch_buffer"] {
            metrics::counter!(
                "cloacina_accumulator_persist_failures_total",
                "graph" => "test_graph",
                "accumulator" => "acc0",
                "kind" => kind,
            )
            .increment(1);
        }

        let app = build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");
        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let text = String::from_utf8_lossy(&body_bytes);

        for kind in [
            "cache_serialize",
            "dirty_serialize",
            "seq_serialize",
            "save",
        ] {
            assert!(
                text.contains(&format!(
                    "cloacina_reactor_persist_failures_total{{graph=\"test_graph\",kind=\"{}\"",
                    kind
                )) || text.contains(&format!("kind=\"{}\"", kind)),
                "Missing kind={} in reactor_persist_failures_total. Got:\n{}",
                kind,
                text
            );
        }
        for kind in ["checkpoint", "boundary", "batch_buffer"] {
            assert!(
                text.contains(&format!("kind=\"{}\"", kind)),
                "Missing kind={} in accumulator_persist_failures_total. Got:\n{}",
                kind,
                text
            );
        }
    }

    /// I-0099 cardinality guard — assert that every `cloacina_*` metric
    /// introduced by the initiative stays under a fixed series ceiling.
    ///
    /// Emits each metric exactly as the production code does, scrapes
    /// `/metrics`, then groups all exposition lines by metric name and
    /// asserts each metric's distinct label-set count is below the
    /// documented limit. Caught here, this protects against the
    /// "someone added `tenant_id` as a label" failure mode that motivates
    /// the cardinality discipline in I-0099.
    #[tokio::test]
    #[serial]
    async fn test_i0099_cardinality_within_ceiling() {
        use std::collections::HashMap;

        let state = test_state().await;

        // Exercise every I-0099 metric at least once across the full label
        // domain so /metrics reflects realistic cardinality.
        for outcome in ["claimed", "contended", "empty"] {
            metrics::counter!(
                "cloacina_scheduler_claim_attempts_total",
                "outcome" => outcome,
            )
            .increment(1);
        }
        metrics::counter!("cloacina_scheduler_heartbeat_writes_total").increment(1);
        metrics::counter!("cloacina_scheduler_stale_claims_swept_total").increment(1);

        for reason in ["panic", "error", "shutdown_timeout"] {
            metrics::counter!(
                "cloacina_supervisor_restarts_total",
                "graph" => "g0",
                "component" => "reactor",
                "reason" => reason,
            )
            .increment(1);
        }
        for state_label in ["healthy", "degraded", "starting", "stopped", "crashed"] {
            metrics::gauge!(
                "cloacina_component_health",
                "graph" => "g0",
                "component" => "reactor",
                "state" => state_label,
            )
            .set(if state_label == "healthy" { 1.0 } else { 0.0 });
        }

        for kind in ["passthrough", "stream", "polling", "batch"] {
            metrics::counter!(
                "cloacina_accumulator_events_total",
                "graph" => "g0",
                "accumulator" => "acc0",
                "kind" => kind,
            )
            .increment(1);
        }
        metrics::histogram!(
            "cloacina_accumulator_emit_duration_seconds",
            "graph" => "g0",
            "accumulator" => "acc0",
        )
        .record(0.001);
        metrics::gauge!(
            "cloacina_accumulator_buffer_depth",
            "graph" => "g0",
            "accumulator" => "acc0",
        )
        .set(0.0);
        metrics::counter!(
            "cloacina_accumulator_checkpoint_writes_total",
            "graph" => "g0",
            "accumulator" => "acc0",
        )
        .increment(1);

        for strategy in ["when_any", "when_all", "sequential"] {
            metrics::counter!(
                "cloacina_reactor_fires_total",
                "graph" => "g0",
                "reactor" => "r0",
                "strategy" => strategy,
            )
            .increment(1);
        }
        metrics::histogram!(
            "cloacina_reactor_fire_duration_seconds",
            "graph" => "g0",
            "reactor" => "r0",
        )
        .record(0.001);
        metrics::gauge!(
            "cloacina_reactor_cache_age_seconds",
            "graph" => "g0",
            "reactor" => "r0",
            "source" => "src_a",
        )
        .set(1.0);
        metrics::counter!(
            "cloacina_reactor_deduped_events_total",
            "graph" => "g0",
            "reactor" => "r0",
            "source" => "src_a",
        )
        .increment(1);

        for endpoint in ["accumulator", "reactor"] {
            metrics::gauge!(
                "cloacina_ws_connections_active",
                "endpoint" => endpoint,
            )
            .set(1.0);
            for direction in ["in", "out"] {
                metrics::counter!(
                    "cloacina_ws_messages_total",
                    "endpoint" => endpoint,
                    "direction" => direction,
                )
                .increment(1);
            }
        }
        for reason in [
            "ticket_expired",
            "invalid_signature",
            "tenant_mismatch",
            "not_authorized",
        ] {
            metrics::counter!(
                "cloacina_ws_auth_failures_total",
                "reason" => reason,
            )
            .increment(1);
        }

        // I-0108 persist-failure counters — included so the cardinality
        // guard covers them as well.
        for kind in [
            "cache_serialize",
            "dirty_serialize",
            "seq_serialize",
            "save",
        ] {
            metrics::counter!(
                "cloacina_reactor_persist_failures_total",
                "graph" => "g0",
                "reactor" => "r0",
                "kind" => kind,
            )
            .increment(1);
        }
        for kind in ["checkpoint", "boundary", "batch_buffer"] {
            metrics::counter!(
                "cloacina_accumulator_persist_failures_total",
                "graph" => "g0",
                "accumulator" => "acc0",
                "kind" => kind,
            )
            .increment(1);
        }
        // I-0110 / COR-11: context-merge failure counter — bounded
        // `kind ∈ parse | merge`. Included in the cardinality guard so
        // future additions don't sneak in an unbounded label.
        for kind in ["parse", "merge"] {
            metrics::counter!(
                "cloacina_context_merge_failures_total",
                "kind" => kind,
            )
            .increment(1);
        }
        // I-0100 / T-0599: reactor firings counter — labels are
        // `graph | reactor` (typically the same value, derived from the
        // package's reactor name). Bounded by the loaded reactor set,
        // not by request-time data.
        metrics::counter!(
            "cloacina_reactor_firings_total",
            "graph" => "g0",
            "reactor" => "r0",
        )
        .increment(1);
        // I-0100 / T-0601: TTL prune counter — unlabeled.
        metrics::counter!("cloacina_reactor_firings_pruned_total").increment(1);

        // Scrape and parse
        let app = build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let text = String::from_utf8_lossy(&body_bytes);

        // Documented ceilings — per-metric distinct label-set count.
        // Derived from the bounded enum products in I-0099:
        //   scheduler_claim_attempts_total = 3 outcomes
        //   scheduler_heartbeat_writes_total = 1
        //   scheduler_stale_claims_swept_total = 1
        //   supervisor_restarts_total ≤ 3 reasons × small graph/component fan-out
        //   component_health = 5 states × small graph/component fan-out
        //   accumulator_events_total ≤ 4 kinds × small graph/accumulator fan-out
        //   reactor_fires_total ≤ 3 strategies × small graph/reactor fan-out
        //   ws_connections_active = 2 endpoints
        //   ws_messages_total = 2 endpoints × 2 directions = 4
        //   ws_auth_failures_total = 4 reasons
        //
        // The ceiling below accommodates the per-test cardinality (one
        // graph/reactor/accumulator name) plus a safety margin. Inflated
        // labels (e.g., tenant_id, event keys) would push any of these
        // metrics far above the ceiling.
        // Count DISTINCT LABEL SETS per canonical metric — not raw series.
        // A histogram emits one `_bucket` series per `le` boundary plus `_sum`
        // and `_count`; those are a fixed Prometheus artifact, not cardinality
        // risk. Collapsing the `le` label so all buckets of one label set count
        // once measures the thing the guard actually cares about (unbounded
        // labels like tenant_id / raw paths still blow up the distinct-set
        // count). This also keeps the count stable when many tests share one
        // process-wide recorder (e.g. the request-duration histogram, which the
        // middleware feeds on every test request).
        let mut label_sets: HashMap<&str, std::collections::HashSet<String>> = HashMap::new();
        for line in text.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            // Capture metric name up to '{' or whitespace.
            let name_end = line
                .find('{')
                .or_else(|| line.find(' '))
                .unwrap_or(line.len());
            let name = &line[..name_end];
            // Strip Prometheus histogram suffixes so all buckets of one
            // histogram count as the same metric for our purposes.
            let canonical: &str = if let Some(stripped) = name.strip_suffix("_bucket") {
                stripped
            } else if let Some(stripped) = name.strip_suffix("_sum") {
                stripped
            } else if let Some(stripped) = name.strip_suffix("_count") {
                stripped
            } else {
                name
            };
            if !canonical.starts_with("cloacina_") {
                continue;
            }
            // Extract the label block between `{` and `}`, dropping the
            // histogram `le` bucket boundary so buckets collapse to one set.
            let labels = match (line.find('{'), line.rfind('}')) {
                (Some(a), Some(b)) if b > a => &line[a + 1..b],
                _ => "",
            };
            let canonical_labels: Vec<&str> = labels
                .split(',')
                .map(|kv| kv.trim())
                .filter(|kv| !kv.is_empty() && !kv.starts_with("le="))
                .collect();
            label_sets
                .entry(canonical)
                .or_default()
                .insert(canonical_labels.join(","));
        }
        let series_count: HashMap<&str, usize> =
            label_sets.iter().map(|(k, v)| (*k, v.len())).collect();

        // Generous per-metric ceiling — every I-0099 metric should be far
        // below this. If a regression inflates labels (tenant_id, event
        // keys, raw paths), this assertion fails loudly.
        let ceiling = 64usize;
        for (metric, count) in &series_count {
            assert!(
                *count <= ceiling,
                "I-0099 cardinality guard: {} has {} distinct label sets, \
                 exceeds ceiling {}. A new label may be unbounded — check \
                 docs/operations/metrics.md and ensure all labels are \
                 enum-bounded or derived from package metadata.",
                metric,
                count,
                ceiling
            );
        }

        // Sanity: every I-0099 metric we just emitted should appear in the
        // scrape — if a metric is missing the cardinality assertion was
        // vacuous for it.
        for expected in [
            "cloacina_scheduler_claim_attempts_total",
            "cloacina_scheduler_heartbeat_writes_total",
            "cloacina_scheduler_stale_claims_swept_total",
            "cloacina_supervisor_restarts_total",
            "cloacina_component_health",
            "cloacina_accumulator_events_total",
            "cloacina_accumulator_emit_duration_seconds",
            "cloacina_accumulator_buffer_depth",
            "cloacina_accumulator_checkpoint_writes_total",
            "cloacina_reactor_fires_total",
            "cloacina_reactor_fire_duration_seconds",
            "cloacina_reactor_cache_age_seconds",
            "cloacina_reactor_deduped_events_total",
            "cloacina_ws_connections_active",
            "cloacina_ws_messages_total",
            "cloacina_ws_auth_failures_total",
            "cloacina_reactor_persist_failures_total",
            "cloacina_accumulator_persist_failures_total",
            "cloacina_context_merge_failures_total",
            "cloacina_reactor_firings_total",
            "cloacina_reactor_firings_pruned_total",
        ] {
            assert!(
                series_count.contains_key(expected),
                "I-0099 metric {} missing from /metrics scrape — \
                 cardinality assertion was vacuous for it. \
                 Series found: {:?}",
                expected,
                series_count.keys().collect::<Vec<_>>()
            );
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_api_request_duration_histogram_emitted() {
        let state = test_state().await;
        let app = build_router(state);

        // Fire a request through the middleware stack so the histogram records.
        let _ = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request failed");

        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("failed to read body")
            .to_bytes();
        let text = String::from_utf8_lossy(&body_bytes);

        assert!(
            text.contains("cloacina_api_request_duration_seconds"),
            "Metrics output should include the API request duration histogram. Got:\n{}",
            text
        );
        assert!(
            text.contains("cloacina_api_requests_total"),
            "Metrics output should include the API request counter. Got:\n{}",
            text
        );
    }

    // ── Routing guard ─────────────────────────────────────────────────

    /// Regression for T-0557 Bug 1: T-0449 nested every authenticated
    /// route under `/v1/`, but the test suite kept hitting bare paths
    /// for ~7 months without anyone noticing because the suite was
    /// silently failing on a missing Postgres connection. This test
    /// asserts the `/v1/` prefix is mandatory — a request to the
    /// unprefixed path must hit the 404 fallback, not be silently
    /// fall-through-routed somewhere else.
    #[tokio::test]
    #[serial]
    async fn test_unprefixed_auth_route_returns_404() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/auth/keys")
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "/auth/keys (without /v1/ prefix) must 404 — production paths are /v1/-prefixed"
        );
    }

    // ── Auth middleware ───────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_auth_no_token_returns_401() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/auth/keys")
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body["error"].as_str().unwrap().contains("Authorization"));
    }

    #[tokio::test]
    #[serial]
    async fn test_auth_invalid_token_returns_401() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/auth/keys")
            .header("Authorization", "Bearer clk_totally_invalid_key_12345678")
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body["error"].as_str().unwrap().contains("invalid"));
    }

    #[tokio::test]
    #[serial]
    async fn test_auth_valid_token_passes() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/auth/keys")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    #[serial]
    async fn test_auth_malformed_header_returns_401() {
        let state = test_state().await;
        let app = build_router(state);

        // "Basic" instead of "Bearer"
        let req = axum::http::Request::builder()
            .uri("/v1/auth/keys")
            .header("Authorization", "Basic abc123")
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    // ── Key management ───────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_create_key_returns_201() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/auth/keys")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"name": "new-test-key"}"#))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(body["key"].as_str().unwrap().starts_with("clk_"));
        assert_eq!(body["name"], "new-test-key");
        assert!(body["id"].as_str().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_create_key_missing_name_returns_422() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/auth/keys")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{}"#))
            .unwrap();

        let (status, _) = send_request(app, req).await;
        // axum returns 422 for deserialization failures
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    #[serial]
    async fn test_list_keys_returns_list() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/auth/keys")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["items"].as_array().is_some());
        assert!(!body["items"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_key_valid() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;

        // Create a second key to revoke
        let (_, hash2) = cloacina::security::api_keys::generate_api_key();
        let dal = cloacina::dal::DAL::new(state.database.clone());
        let info2 = dal
            .api_keys()
            .create_key(&hash2, "to-revoke", None, false, "admin")
            .await
            .expect("create key");

        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/v1/auth/keys/{}", info2.id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "revoked");
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_key_nonexistent_returns_404() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let fake_id = uuid::Uuid::new_v4();
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/v1/auth/keys/{}", fake_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_key_invalid_uuid_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri("/v1/auth/keys/not-a-uuid")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    // ── Tenants ──────────────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_create_tenant_returns_201() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let name = format!(
            "test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );
        // CLOACI-T-0594 / API-01: request body now uses `{name, description?, password?}`.
        let body_json = serde_json::json!({
            "name": name,
            "description": "integration test tenant",
            "password": "testpass123",
        });

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body_json).unwrap()))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::CREATED, "body: {:?}", body);
        // Response keys reflect the new shape — `name` (the canonical
        // identifier) replaces `schema_name`.
        assert_eq!(body["name"], name);
    }

    #[tokio::test]
    #[serial]
    async fn test_list_tenants() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/tenants")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        // CLOACI-T-0594 / API-03: unified `{items, total}` envelope.
        assert!(
            body["items"].as_array().is_some(),
            "list_tenants must return items envelope; got {:?}",
            body
        );
        assert!(body["total"].as_u64().is_some());
    }

    /// CLOACI-T-0580: LRU eviction. With a cache cap of 2, cycling
    /// through 3 tenant runners must evict the least-recently-used.
    /// Requires Postgres (creates real tenant schemas + runners).
    ///
    /// Uses a freshly-built `TenantRunnerCache` with cap=2 rather than
    /// the default test cap of 8.
    #[tokio::test]
    #[serial]
    async fn test_tenant_runner_cache_lru_evicts_oldest() {
        let mut state = test_state().await;
        let token = create_test_api_key(&state).await;

        // Override the cache with a small cap for this test.
        state.tenant_runners = Arc::new(crate::tenant_runner_cache::TenantRunnerCache::new(
            std::num::NonZeroUsize::new(2).expect("cap=2"),
            runner_config_for_tenant_cache(None, "default"),
        ));

        let schema_a = format!(
            "test_lru_a_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );
        let schema_b = format!(
            "test_lru_b_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );
        let schema_c = format!(
            "test_lru_c_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );
        for s in [&schema_a, &schema_b, &schema_c] {
            let req = axum::http::Request::builder()
                .method("POST")
                .uri("/v1/tenants")
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "name": s,
                        "password": "testpass123",
                    })
                    .to_string(),
                ))
                .unwrap();
            let (status, _) = send_request(build_router(state.clone()), req).await;
            assert_eq!(status, StatusCode::CREATED, "create tenant {}", s);
        }

        // Acquire A, then B → cache full (cap=2).
        let db_a = state
            .tenant_databases
            .resolve(&schema_a, &state.database)
            .await
            .expect("resolve A");
        let _ = state
            .tenant_runners
            .get_or_create(&schema_a, db_a)
            .await
            .expect("A");
        let db_b = state
            .tenant_databases
            .resolve(&schema_b, &state.database)
            .await
            .expect("resolve B");
        let _ = state
            .tenant_runners
            .get_or_create(&schema_b, db_b)
            .await
            .expect("B");
        assert_eq!(state.tenant_runners.len().await, 2);

        // Acquire C → evicts A (least recently used).
        let db_c = state
            .tenant_databases
            .resolve(&schema_c, &state.database)
            .await
            .expect("resolve C");
        let _ = state
            .tenant_runners
            .get_or_create(&schema_c, db_c)
            .await
            .expect("C");
        assert_eq!(
            state.tenant_runners.len().await,
            2,
            "cache must stay bounded at cap=2"
        );

        // Cleanup.
        for s in [&schema_a, &schema_b, &schema_c] {
            let req = axum::http::Request::builder()
                .method("DELETE")
                .uri(format!("/v1/tenants/{}", s))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap();
            let _ = send_request(build_router(state.clone()), req).await;
        }
    }

    /// CLOACI-T-0581: re-running `remove_tenant` on the same tenant is
    /// idempotent. First call drops the schema; second call sees no
    /// runner/DB to evict and `DROP SCHEMA IF EXISTS` is a no-op.
    /// Returns success both times.
    #[tokio::test]
    #[serial]
    async fn test_remove_tenant_idempotent_retry() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;

        let schema = format!(
            "test_idem_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );

        // Create.
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants")
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "name": schema,
                    "password": "testpass123",
                })
                .to_string(),
            ))
            .unwrap();
        let (status, _) = send_request(build_router(state.clone()), req).await;
        assert_eq!(status, StatusCode::CREATED);

        // First delete.
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/v1/tenants/{}", schema))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let (status_1, body_1) = send_request(build_router(state.clone()), req).await;
        assert_eq!(status_1, StatusCode::OK);
        assert_eq!(body_1["status"], "removed");

        // Second delete (idempotent).
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/v1/tenants/{}", schema))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let (status_2, body_2) = send_request(build_router(state.clone()), req).await;
        assert_eq!(
            status_2,
            StatusCode::OK,
            "idempotent retry must succeed: {body_2:?}"
        );
        assert_eq!(body_2["status"], "removed");
        // Step counts: no runner, no cached DB.
        assert_eq!(body_2["runner_evicted"], false);
        assert_eq!(body_2["db_cache_evicted"], false);
    }

    /// CLOACI-T-0580: two per-tenant runners constructed through the
    /// `TenantRunnerCache` share the same `Arc<Runtime>` allocation —
    /// inventory isn't duplicated per tenant. Requires a live Postgres
    /// (admin schema + per-tenant schemas created via `create_tenant`).
    #[tokio::test]
    #[serial]
    async fn test_tenant_runners_share_inventory_arc() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;

        // Create two distinct tenant schemas.
        let schema_a = format!(
            "test_arc_a_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );
        let schema_b = format!(
            "test_arc_b_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );
        for s in [&schema_a, &schema_b] {
            let req = axum::http::Request::builder()
                .method("POST")
                .uri("/v1/tenants")
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "name": s,
                        "password": "testpass123",
                    })
                    .to_string(),
                ))
                .unwrap();
            let (status, _body) = send_request(build_router(state.clone()), req).await;
            assert_eq!(
                status,
                StatusCode::CREATED,
                "create tenant {} should succeed",
                s
            );
        }

        // Construct runners for both tenants via the cache.
        let db_a = state
            .tenant_databases
            .resolve(&schema_a, &state.database)
            .await
            .expect("resolve A");
        let db_b = state
            .tenant_databases
            .resolve(&schema_b, &state.database)
            .await
            .expect("resolve B");
        let runner_a = state
            .tenant_runners
            .get_or_create(&schema_a, db_a)
            .await
            .expect("runner A");
        let runner_b = state
            .tenant_runners
            .get_or_create(&schema_b, db_b)
            .await
            .expect("runner B");

        // The cache's `shared_runtime` accessor should match what each
        // runner reports via its `runtime()` accessor.
        let cache_rt = state.tenant_runners.shared_runtime();
        assert!(
            std::sync::Arc::ptr_eq(&cache_rt, &runner_a.runtime()),
            "runner A's Runtime Arc should match the cache's shared_runtime"
        );
        assert!(
            std::sync::Arc::ptr_eq(&cache_rt, &runner_b.runtime()),
            "runner B's Runtime Arc should match the cache's shared_runtime"
        );
        // Transitively, both runners share the same Arc.
        assert!(
            std::sync::Arc::ptr_eq(&runner_a.runtime(), &runner_b.runtime()),
            "two tenant runners must share the same Runtime Arc (inventory not duplicated)"
        );

        // Clean up: drop both tenants.
        for s in [&schema_a, &schema_b] {
            let req = axum::http::Request::builder()
                .method("DELETE")
                .uri(format!("/v1/tenants/{}", s))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap();
            let _ = send_request(build_router(state.clone()), req).await;
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_remove_tenant_nonexistent_succeeds() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri("/v1/tenants/nonexistent_schema_xyz")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        // DROP SCHEMA IF EXISTS is idempotent — succeeds even if schema doesn't exist
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "removed");
    }

    #[tokio::test]
    #[serial]
    async fn test_create_then_delete_tenant() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;

        let schema = format!(
            "test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "_")
        );
        let body_json = serde_json::json!({
            "name": schema,
            "password": "testpass123",
        });

        // Create
        let app = build_router(state.clone());
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body_json).unwrap()))
            .unwrap();
        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::CREATED);

        // Delete
        let app = build_router(state);
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/v1/tenants/{}", schema))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "removed");
    }

    #[tokio::test]
    #[serial]
    async fn test_create_tenant_missing_fields_returns_422() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        // Missing required schema_name and username
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{}"#))
            .unwrap();
        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    // ── Workflows ────────────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_list_workflows_returns_list() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/tenants/public/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["items"].as_array().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_workflow_nonexistent_returns_404() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/tenants/public/workflows/nonexistent_workflow")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_workflow_empty_file_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        // Build a multipart body with an empty file
        let boundary = "----testboundary";
        let multipart_body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.cloacina\"\r\nContent-Type: application/octet-stream\r\n\r\n\r\n--{boundary}--\r\n"
        );

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(multipart_body))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "body: {:?}", body);
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_workflow_no_file_field_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        // Multipart body with wrong field name
        let boundary = "----testboundary";
        let multipart_body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"data\"; filename=\"test.txt\"\r\nContent-Type: application/octet-stream\r\n\r\nsome data\r\n--{boundary}--\r\n"
        );

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(multipart_body))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "body: {:?}", body);
    }

    /// Path to test fixture directory (relative to workspace root).
    fn fixture_path(name: &str) -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::PathBuf::from(manifest_dir)
            .join("test-fixtures")
            .join(name)
    }

    /// Build a multipart request body with a file field.
    fn multipart_file_body(data: &[u8]) -> (String, Vec<u8>) {
        let boundary = "----TestBoundary9876543210";
        let mut body = Vec::new();
        write!(
            body,
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"pkg.cloacina\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .unwrap();
        body.extend_from_slice(data);
        write!(body, "\r\n--{boundary}--\r\n").unwrap();
        (boundary.to_string(), body)
    }

    /// Delete a workflow by name/version if it exists (cleanup for idempotent tests).
    async fn delete_workflow_if_exists(state: &AppState, token: &str, name: &str, version: &str) {
        let app = build_router(state.clone());
        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/v1/tenants/public/workflows/{}/{}", name, version))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        // Ignore result — may 404 on first run
        let _ = app.oneshot(req).await;
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_valid_python_workflow_returns_201() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;

        // Clean up from any prior run
        delete_workflow_if_exists(&state, &token, "test-fixture-python", "1.0.0").await;

        let app = build_router(state);
        let package_data = std::fs::read(fixture_path("python-workflow.cloacina"))
            .expect("fixture file not found");
        let (boundary, body_bytes) = multipart_file_body(&package_data);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(body_bytes))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::CREATED, "body: {:?}", body);
        assert!(body["package_id"].as_str().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_valid_rust_workflow_returns_201() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;

        // Clean up from any prior run
        delete_workflow_if_exists(&state, &token, "test-fixture-rust", "1.0.0").await;

        let app = build_router(state);
        let package_data =
            std::fs::read(fixture_path("rust-workflow.cloacina")).expect("fixture file not found");
        let (boundary, body_bytes) = multipart_file_body(&package_data);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(body_bytes))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::CREATED, "body: {:?}", body);
        assert!(body["package_id"].as_str().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_corrupt_package_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let (boundary, body_bytes) = multipart_file_body(b"this is not a valid bzip2 archive");

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(body_bytes))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "body: {:?}", body);
    }

    // ── Executions ───────────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_list_executions_returns_list() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/tenants/public/executions")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["items"].as_array().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_execution_invalid_uuid_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/tenants/public/executions/not-a-uuid")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_execution_nonexistent_returns_404() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let fake_id = uuid::Uuid::new_v4();
        let req = axum::http::Request::builder()
            .uri(format!("/v1/tenants/public/executions/{}", fake_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_execution_events_invalid_uuid_returns_400() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/tenants/public/executions/not-a-uuid/events")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_nonexistent_workflow_returns_error() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/workflows/nonexistent_wf/execute")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"context": {}}"#))
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_execution_events_valid_uuid_no_events() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        // Valid UUID format but no matching execution — should return
        // an empty events list (the DAL returns Ok([]) for missing executions)
        let fake_id = uuid::Uuid::new_v4();
        let req = axum::http::Request::builder()
            .uri(format!("/v1/tenants/public/executions/{}/events", fake_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["events"].as_array().is_some());
        assert!(body["events"].as_array().unwrap().is_empty());
    }

    // ── Triggers ─────────────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_list_triggers_returns_list() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/tenants/public/triggers")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["items"].as_array().is_some());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_trigger_nonexistent_returns_404() {
        let state = test_state().await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/v1/tenants/public/triggers/nonexistent_trigger")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let (status, _) = send_request(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // ── Fallback / 404 ──────────────────────────────────────────────

    #[tokio::test]
    #[serial]
    async fn test_unknown_route_returns_404() {
        let state = test_state().await;
        let app = build_router(state);

        let req = axum::http::Request::builder()
            .uri("/nonexistent/route")
            .body(Body::empty())
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "no route matches this request");
    }

    // ── Signature contract tests (CLOACI-I-0103 / T-0570) ─────────────────
    //
    // These exercise the full upload-route verification gate end-to-end:
    // a signed payload must pass the gate; an unsigned payload must be
    // rejected with `signature_not_found`. They depend on TEST_DB_URL
    // being reachable (same constraint as the rest of this test module)
    // and use random per-test org_ids to stay isolated.

    #[tokio::test]
    #[serial]
    async fn test_upload_unsigned_with_require_signatures_returns_403() {
        let org_id = cloacina::UniversalUuid::new_v4();
        let state = test_state_with_signature_required(org_id).await;
        let token = create_test_api_key(&state).await;
        let app = build_router(state);

        let pkg_bytes = b"unsigned bogus package bytes";
        let (boundary, body) = multipart_file_body(pkg_bytes);

        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(body))
            .unwrap();

        let (status, body) = send_request(app, req).await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "expected 403 from verification gate; body: {:?}",
            body
        );
        let code = body.get("code").and_then(|v| v.as_str()).unwrap_or("");
        assert_eq!(
            code, "signature_not_found",
            "expected signature_not_found code; got: {} body: {:?}",
            code, body
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_upload_signed_with_require_signatures_passes_verification() {
        use cloacina::dal::DAL;
        use cloacina::security::{DbKeyManager, DbPackageSigner, KeyManager, PackageSigner};
        use std::io::Write as _;
        use tempfile::NamedTempFile;

        let org_id = cloacina::UniversalUuid::new_v4();
        let state = test_state_with_signature_required(org_id).await;
        let token = create_test_api_key(&state).await;

        // Provision a signing key for the test org and self-trust it so the
        // verifier accepts signatures from this key.
        let dal = DAL::new(state.database.clone());
        let km = DbKeyManager::new(dal.clone());
        let signer = DbPackageSigner::new(dal);
        let master_key = [0u8; 32];

        let key_info = km
            .create_signing_key(org_id, "contract-test-key", &master_key)
            .await
            .expect("create_signing_key");
        km.trust_public_key(org_id, &key_info.public_key, Some("self"))
            .await
            .expect("trust_public_key");

        // Sign the bytes and store the `package_signatures` row before upload.
        // Make the payload unique per test run so prior runs' signatures don't
        // collide on package_hash (the test DB persists across runs and
        // `find_signature(hash)` returns one row — picking up a stale row from
        // a previous run yields `untrusted_signer` because the old key
        // fingerprint isn't trusted for *this* run's random org_id).
        let pkg_bytes = format!(
            "signed contract-test payload {}",
            cloacina::UniversalUuid::new_v4()
        )
        .into_bytes();
        let pkg_bytes = pkg_bytes.as_slice();
        let tf = NamedTempFile::new().unwrap();
        tf.as_file()
            .write_all(pkg_bytes)
            .expect("write tempfile bytes");
        signer
            .sign_package_with_db_key(
                tf.path(),
                key_info.id,
                &master_key,
                /* store_signature= */ true,
            )
            .await
            .expect("sign_package_with_db_key");

        // Upload the same bytes through the HTTP handler.
        let app = build_router(state);
        let (boundary, body) = multipart_file_body(pkg_bytes);
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/tenants/public/workflows")
            .header("Authorization", format!("Bearer {}", token))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={}", boundary),
            )
            .body(Body::from(body))
            .unwrap();

        let (status, body) = send_request(app, req).await;

        // Contract: a valid signature must pass the verification gate.
        // The bytes aren't a real .cloacina archive so the request will fail
        // at the registration step downstream — that's fine; we're asserting
        // verification specifically. If the response is 403, it must NOT be
        // from a verification-related code.
        if status == StatusCode::FORBIDDEN {
            let code = body.get("code").and_then(|v| v.as_str()).unwrap_or("");
            let verification_codes = [
                "signature_not_found",
                "package_tampered",
                "untrusted_signer",
                "invalid_signature",
                "signature_verification_unconfigured",
                "signature_verification_error",
            ];
            assert!(
                !verification_codes.contains(&code),
                "expected verification to pass; got 403 with verification code: {} body: {:?}",
                code,
                body
            );
        }
        // If status is anything other than 403, the verification gate already
        // accepted it (failure further downstream is out of scope here).
    }

    // ── Security args validation (CLOACI-I-0103 / T-0567) ─────────────────

    #[test]
    fn validate_security_args_default_passes() {
        // Default config: no signature requirement, no org_id. Most common deployment.
        assert!(validate_security_args(false, None).is_ok());
    }

    #[test]
    fn validate_security_args_org_without_require_passes() {
        // Configuring an org_id without enabling require_signatures is allowed
        // (operator may want it pre-staged before flipping the flag).
        let uuid = uuid::Uuid::new_v4();
        assert!(validate_security_args(false, Some(&uuid)).is_ok());
    }

    #[test]
    fn validate_security_args_require_with_org_passes() {
        // The fully-configured opt-in posture.
        let uuid = uuid::Uuid::new_v4();
        assert!(validate_security_args(true, Some(&uuid)).is_ok());
    }

    #[test]
    fn validate_security_args_require_without_org_fails() {
        // The bad combo we want to catch at boot, not at first upload.
        let result = validate_security_args(true, None);
        let err = result.expect_err("expected validation failure");
        let msg = err.to_string();
        assert!(
            msg.contains("verification-org-id"),
            "error must name the missing flag for the operator; got: {}",
            msg
        );
        assert!(
            msg.contains("CLOACINA_VERIFICATION_ORG_ID"),
            "error must also name the env var alternative; got: {}",
            msg
        );
    }
}
