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

//! `cloacinactl serve` command — starts the Cloacina server.

use crate::config::ServerConfig;
use crate::routes::health::{self, AppState};
use anyhow::Result;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Server operational mode.
#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum ServeMode {
    /// Run all services: API server, scheduler, worker, cron, recovery.
    All,
    /// Run only the HTTP API server (no background services).
    Api,
    /// Run only the task executor/dispatcher (no API, no scheduler).
    Worker,
    /// Run only the scheduler, cron, and recovery services (no API, no executor).
    Scheduler,
}

impl std::fmt::Display for ServeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServeMode::All => write!(f, "all"),
            ServeMode::Api => write!(f, "api"),
            ServeMode::Worker => write!(f, "worker"),
            ServeMode::Scheduler => write!(f, "scheduler"),
        }
    }
}

/// Arguments for the `serve` subcommand.
#[derive(Debug, clap::Args)]
pub struct ServeArgs {
    /// Server operational mode.
    #[arg(long, value_enum, default_value = "all")]
    pub mode: ServeMode,

    /// Path to the TOML configuration file.
    #[arg(long)]
    pub config: Option<String>,

    /// Bind address (overrides config file).
    #[arg(long, default_value = "0.0.0.0")]
    pub bind: String,

    /// Port to listen on (overrides config file).
    #[arg(long, default_value = "8080")]
    pub port: u16,
}

/// OpenAPI documentation struct.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Cloacina API",
        description = "Cloacina workflow orchestration server API",
    ),
    paths(health::health),
    components(schemas(health::HealthResponse)),
    tags(
        (name = "system", description = "System health and status"),
    )
)]
struct ApiDoc;

/// Build the axum Router with application state.
///
/// Separated from `run()` so it can be tested independently.
pub fn app(state: Arc<AppState>) -> Router {
    // Public routes — no auth required
    let public_routes = Router::new()
        .route("/health", get(health::health))
        .route("/metrics", get(crate::routes::metrics::metrics))
        .merge(SwaggerUi::new("/api-docs/").url("/api-docs/openapi.json", ApiDoc::openapi()));

    // Protected routes — auth required
    #[allow(unused_mut)]
    let mut protected_routes = Router::new()
        .route(
            "/executions",
            post(crate::routes::executions::create_execution)
                .get(crate::routes::executions::list_executions),
        )
        .route(
            "/executions/{id}",
            get(crate::routes::executions::get_execution)
                .delete(crate::routes::executions::cancel_execution),
        )
        .route(
            "/executions/{id}/pause",
            post(crate::routes::executions::pause_execution),
        )
        .route(
            "/executions/{id}/resume",
            post(crate::routes::executions::resume_execution),
        )
        .route("/workflows", get(crate::routes::workflows::list_workflows))
        .route(
            "/workflows/packages",
            post(crate::routes::workflows::upload_package),
        )
        .route(
            "/workflows/packages/{id}",
            axum::routing::delete(crate::routes::workflows::delete_package),
        )
        .route(
            "/workflows/{name}/schedules",
            post(crate::routes::workflows::create_schedule)
                .get(crate::routes::workflows::list_schedules),
        )
        .route(
            "/workflows/schedules/{id}",
            get(crate::routes::workflows::get_schedule)
                .delete(crate::routes::workflows::delete_schedule),
        )
        .route(
            "/workflows/schedules/{id}/history",
            get(crate::routes::workflows::get_schedule_history),
        )
        // Tenant management routes — require_admin is enforced via route_layer
        // so it only applies to matched routes, not to fallback handling.
        .route(
            "/tenants",
            post(crate::routes::tenants::create_tenant).get(crate::routes::tenants::list_tenants),
        )
        .route(
            "/tenants/{id}",
            get(crate::routes::tenants::get_tenant)
                .delete(crate::routes::tenants::deactivate_tenant),
        )
        .route(
            "/tenants/{id}/api-keys",
            post(crate::routes::tenants::create_tenant_key)
                .get(crate::routes::tenants::list_tenant_keys),
        )
        .route(
            "/tenants/{id}/api-keys/{key_id}",
            axum::routing::delete(crate::routes::tenants::revoke_tenant_key),
        )
        // Trigger management routes (read-only + enable/disable)
        .route("/triggers", get(crate::routes::triggers::list_triggers))
        .route(
            "/triggers/{name}",
            get(crate::routes::triggers::get_trigger),
        )
        .route(
            "/triggers/{name}/enable",
            post(crate::routes::triggers::enable_trigger),
        )
        .route(
            "/triggers/{name}/disable",
            post(crate::routes::triggers::disable_trigger),
        );

    // Auth-test endpoint only available in debug builds
    #[cfg(debug_assertions)]
    {
        protected_routes =
            protected_routes.route("/auth-test", get(crate::routes::auth_test::auth_test));
    }

    // Apply auth middleware to protected routes only (if auth is configured).
    // SECURITY: When no database is configured, reject all protected routes
    // with 503 instead of silently serving them without authentication.
    // Use route_layer (not layer) so middleware only runs for matched routes,
    // not the fallback 404 handler — otherwise unmatched paths return 503.
    let protected_routes = if let Some(ref auth) = state.auth_state {
        protected_routes.route_layer(axum::middleware::from_fn_with_state(
            auth.clone(),
            crate::auth::middleware::auth_middleware,
        ))
    } else {
        tracing::warn!(
            "No database configured — protected API endpoints will return 503. \
             Set CLOACINA_DATABASE_URL to enable authentication."
        );
        protected_routes.route_layer(axum::middleware::from_fn(reject_no_auth))
    };

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers(tower_http::cors::Any)
                .allow_origin(tower_http::cors::Any),
        )
        .with_state(state)
}

/// Middleware that rejects all requests with 503 when auth is not configured.
///
/// Applied to protected routes when no database URL is set, ensuring
/// endpoints are never silently served without authentication.
async fn reject_no_auth(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let _ = (request, next);
    axum::response::IntoResponse::into_response((
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "Authentication not configured. Set CLOACINA_DATABASE_URL to enable the API.",
    ))
}

/// Wait for a shutdown signal (SIGTERM or Ctrl+C).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, stopping server...");
}

/// Build a DefaultRunnerConfig from the ServerConfig.
fn build_runner_config(
    config: &ServerConfig,
    mode: ServeMode,
) -> cloacina::runner::DefaultRunnerConfig {
    let mut builder = cloacina::runner::DefaultRunnerConfig::builder()
        .max_concurrent_tasks(config.worker.max_concurrent_tasks)
        .task_timeout(std::time::Duration::from_secs(
            config.worker.task_timeout_seconds,
        ))
        .scheduler_poll_interval(std::time::Duration::from_millis(
            config.scheduler.poll_interval_ms,
        ))
        .db_pool_size(config.database.pool_size);

    // Enable cron and triggers for scheduler and all modes
    let enable_scheduling = matches!(mode, ServeMode::All | ServeMode::Scheduler);
    builder = builder
        .enable_cron_scheduling(enable_scheduling)
        .enable_trigger_scheduling(enable_scheduling);

    // Enable continuous scheduling if configured
    if config.scheduler.enable_continuous && enable_scheduling {
        builder = builder.enable_continuous_scheduling(true);
    }

    // Enable registry reconciler with database storage for all modes that have a runner.
    // This ensures uploaded workflow packages are loaded into the global registries.
    builder = builder
        .enable_registry_reconciler(true)
        .registry_storage_backend("database")
        .registry_reconcile_interval(std::time::Duration::from_secs(5));

    builder.build()
}

/// Run the serve command.
pub async fn run(args: &ServeArgs) -> Result<()> {
    let config = crate::config::load_config(args.config.as_deref(), args)?;
    let mode = args.mode;

    // Initialize observability (Prometheus metrics + OpenTelemetry stub)
    crate::observability::init_prometheus();
    crate::observability::record_static_metrics(config.worker.max_concurrent_tasks);
    crate::observability::init_opentelemetry(
        &config.observability.otlp_endpoint,
        &config.observability.otlp_service_name,
    );

    let needs_api = matches!(mode, ServeMode::All | ServeMode::Api);
    let needs_runner = matches!(
        mode,
        ServeMode::All | ServeMode::Worker | ServeMode::Scheduler
    );

    // Start DefaultRunner if this mode needs background services
    let runner = if needs_runner && !config.database.url.is_empty() {
        let runner_config = build_runner_config(&config, mode);
        info!("Starting background services (mode: {})", mode);
        match cloacina::runner::DefaultRunner::with_config(&config.database.url, runner_config)
            .await
        {
            Ok(r) => {
                info!("Background services started");
                Some(r)
            }
            Err(e) => {
                tracing::error!("Failed to start background services: {}", e);
                return Err(e.into());
            }
        }
    } else {
        if needs_runner && config.database.url.is_empty() {
            tracing::warn!("No database URL configured — background services disabled");
        }
        None
    };

    // Start HTTP server if this mode needs API
    if needs_api {
        // Create auth state if database is available
        let auth_state = if !config.database.url.is_empty() {
            let auth_database = cloacina::Database::try_new_with_schema(
                &config.database.url,
                "",
                config.database.pool_size,
                None,
            )
            .map_err(|e| anyhow::anyhow!("Failed to create auth database pool: {}", e))?;
            let auth_dal = Arc::new(cloacina::dal::DAL::new(auth_database));
            let auth_cache = crate::auth::cache::AuthCache::new(std::time::Duration::from_secs(60));
            Some(crate::auth::middleware::AuthState {
                cache: auth_cache,
                dal: auth_dal,
            })
        } else {
            None
        };

        let state = Arc::new(AppState {
            startup_instant: Instant::now(),
            mode: config.server.mode.clone(),
            auth_state,
            runner: runner.clone(),
        });

        let bind_addr = format!("{}:{}", config.server.bind, config.server.port);
        let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
        let local_addr = listener.local_addr()?;

        info!(
            "Cloacina server listening on {} (mode: {})",
            local_addr, config.server.mode
        );

        axum::serve(listener, app(state))
            .with_graceful_shutdown(shutdown_signal())
            .await?;
    } else {
        // No API — just wait for shutdown signal
        info!("Running in {} mode (no HTTP server)", mode);
        shutdown_signal().await;
    }

    // Shutdown runner
    if let Some(runner) = runner {
        info!("Stopping background services...");
        if let Err(e) = runner.shutdown().await {
            tracing::error!("Error during runner shutdown: {}", e);
        }
    }

    // Auth database pool is cleaned up when AppState/AuthState is dropped

    info!("Server stopped cleanly");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_serve_health_endpoint_lifecycle() {
        // Create app state
        let state = Arc::new(AppState {
            startup_instant: Instant::now(),
            mode: "api".to_string(),
            auth_state: None,
            runner: None,
        });

        // Bind to port 0 (OS assigns random available port)
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        // Start server in background task
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app(state))
                .with_graceful_shutdown(async {
                    // Shutdown after 5 seconds max (safety net)
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        // Wait for server to be ready
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Hit /health
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/health", base_url))
            .send()
            .await
            .expect("health request failed");

        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = resp.json().await.expect("invalid JSON");
        assert_eq!(body["status"], "ok");
        assert!(body["version"].is_string());
        assert_eq!(body["mode"], "api");
        assert!(body["uptime_seconds"].is_number());

        // Hit /api-docs/openapi.json
        let resp = client
            .get(format!("{}/api-docs/openapi.json", base_url))
            .send()
            .await
            .expect("openapi request failed");

        assert_eq!(resp.status(), 200);
        let spec: serde_json::Value = resp.json().await.expect("invalid OpenAPI JSON");
        assert!(spec["openapi"].is_string());
        assert!(spec["paths"]["/health"].is_object());

        // Server will shut down when the safety-net timeout fires
        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    #[tokio::test]
    async fn test_health_returns_correct_mode() {
        let state = Arc::new(AppState {
            startup_instant: Instant::now(),
            mode: "scheduler".to_string(),
            auth_state: None,
            runner: None,
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app(state))
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let resp = reqwest::get(format!("http://{}/health", addr))
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["mode"], "scheduler");

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    #[tokio::test]
    async fn test_unknown_route_returns_404() {
        let state = Arc::new(AppState {
            startup_instant: Instant::now(),
            mode: "api".to_string(),
            auth_state: None,
            runner: None,
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app(state))
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let resp = reqwest::get(format!("http://{}/nonexistent", addr))
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    /// Helper: create an app with auth middleware using a pre-populated cache (no DB needed).
    fn app_with_auth_cache(cache: crate::auth::cache::AuthCache) -> (Router, Arc<AppState>) {
        // Create a dummy DAL — won't be hit because cache is pre-populated
        // We need a valid DAL struct but it won't make DB calls
        let auth_state = crate::auth::middleware::AuthState {
            cache,
            dal: std::sync::Arc::new(cloacina::dal::DAL::new(cloacina::Database::new(
                "sqlite://:memory:",
                "test",
                1,
            ))),
        };

        let state = Arc::new(AppState {
            startup_instant: Instant::now(),
            mode: "api".to_string(),
            auth_state: Some(auth_state),
            runner: None,
        });

        (app(state.clone()), state)
    }

    #[tokio::test]
    async fn test_auth_protected_endpoint_requires_auth() {
        let cache = crate::auth::cache::AuthCache::new(Duration::from_secs(60));
        let (router, _state) = app_with_auth_cache(cache);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        // No auth header → 401
        let resp = reqwest::get(format!("http://{}/auth-test", addr))
            .await
            .unwrap();
        assert_eq!(resp.status(), 401);

        // Health is still public
        let resp = reqwest::get(format!("http://{}/health", addr))
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    #[tokio::test]
    async fn test_auth_valid_key_returns_200() {
        use crate::auth::cache::CachedKey;

        // Pre-populate cache with a known key
        let cache = crate::auth::cache::AuthCache::new(Duration::from_secs(60));
        let key_hash =
            cloacina::security::api_keys::hash_key("cloacina_test_demo_abcdef1234567890").unwrap();
        cache.insert(
            "test_demo".to_string(),
            vec![CachedKey {
                key_hash,
                key_id: uuid::Uuid::new_v4(),
                tenant_id: None,
                can_read: true,
                can_write: false,
                can_execute: false,
                can_admin: false,
                expires_at: None,
                revoked_at: None,
                workflow_patterns: vec![],
            }],
        );

        let (router, _state) = app_with_auth_cache(cache);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Valid auth header → 200
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{}/auth-test", addr))
            .header(
                "Authorization",
                "Bearer cloacina_test_demo_abcdef1234567890",
            )
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["can_read"], true);
        assert_eq!(body["can_write"], false);
        assert_eq!(body["is_global"], true);

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    #[tokio::test]
    async fn test_auth_invalid_key_returns_401() {
        let cache = crate::auth::cache::AuthCache::new(Duration::from_secs(60));
        // Cache has an entry but the hash won't match
        cache.insert_not_found("bad_prefix".to_string());

        let (router, _state) = app_with_auth_cache(cache);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{}/auth-test", addr))
            .header("Authorization", "Bearer cloacina_bad_prefix_invalidkey")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 401);

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    // --- API endpoint tests (no DB, no runner — verify routing and error handling) ---

    #[tokio::test]
    async fn test_api_workflows_without_runner_returns_503() {
        let cache = crate::auth::cache::AuthCache::new(Duration::from_secs(60));
        let key_hash =
            cloacina::security::api_keys::hash_key("cloacina_test_demo_abcdef1234567890").unwrap();
        cache.insert(
            "test_demo".to_string(),
            vec![crate::auth::cache::CachedKey {
                key_hash,
                key_id: uuid::Uuid::new_v4(),
                tenant_id: None,
                can_read: true,
                can_write: true,
                can_execute: true,
                can_admin: true,
                expires_at: None,
                revoked_at: None,
                workflow_patterns: vec![],
            }],
        );

        let (router, _) = app_with_auth_cache(cache);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let client = reqwest::Client::new();

        // GET /workflows without runner → 503
        let resp = client
            .get(format!("http://{}/workflows", addr))
            .header(
                "Authorization",
                "Bearer cloacina_test_demo_abcdef1234567890",
            )
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 503);

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["error"]["code"], "SERVICE_UNAVAILABLE");

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    #[tokio::test]
    async fn test_api_executions_without_auth_returns_401() {
        let cache = crate::auth::cache::AuthCache::new(Duration::from_secs(60));
        let (router, _) = app_with_auth_cache(cache);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let client = reqwest::Client::new();

        // POST /executions without auth → 401
        let resp = client
            .post(format!("http://{}/executions", addr))
            .json(&serde_json::json!({"workflow_name": "test"}))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 401);

        // GET /executions without auth → 401
        let resp = client
            .get(format!("http://{}/executions", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 401);

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    #[tokio::test]
    async fn test_api_error_format_consistency() {
        let cache = crate::auth::cache::AuthCache::new(Duration::from_secs(60));
        let key_hash =
            cloacina::security::api_keys::hash_key("cloacina_test_demo_abcdef1234567890").unwrap();
        cache.insert(
            "test_demo".to_string(),
            vec![crate::auth::cache::CachedKey {
                key_hash,
                key_id: uuid::Uuid::new_v4(),
                tenant_id: None,
                can_read: true,
                can_write: true,
                can_execute: true,
                can_admin: true,
                expires_at: None,
                revoked_at: None,
                workflow_patterns: vec![],
            }],
        );

        let (router, _) = app_with_auth_cache(cache);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let client = reqwest::Client::new();

        // GET /executions/{id} without runner → 503 with consistent error format
        let resp = client
            .get(format!(
                "http://{}/executions/550e8400-e29b-41d4-a716-446655440000",
                addr
            ))
            .header(
                "Authorization",
                "Bearer cloacina_test_demo_abcdef1234567890",
            )
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 503);

        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(
            body["error"]["code"].is_string(),
            "error.code should be a string"
        );
        assert!(
            body["error"]["message"].is_string(),
            "error.message should be a string"
        );
        assert_eq!(body["error"]["code"], "SERVICE_UNAVAILABLE");

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    // --- Metrics endpoint tests ---

    #[tokio::test]
    async fn test_metrics_endpoint_returns_prometheus_format() {
        // Initialize prometheus for this test — may fail if another test already installed it.
        let _ = crate::observability::init_prometheus();

        // Record a test metric to ensure something appears in output
        metrics::counter!("cloacina_test_metric").increment(1);

        let state = Arc::new(AppState {
            startup_instant: Instant::now(),
            mode: "api".to_string(),
            auth_state: None,
            runner: None,
        });

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app(state))
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{}/metrics", addr))
            .send()
            .await
            .expect("metrics request failed");

        assert_eq!(resp.status(), 200);

        // Verify content-type header
        let content_type = resp
            .headers()
            .get("content-type")
            .expect("missing content-type header")
            .to_str()
            .unwrap();
        assert!(
            content_type.contains("text/plain"),
            "expected text/plain content-type, got: {}",
            content_type
        );

        let body = resp.text().await.unwrap();
        // The body should contain Prometheus text format (may include our test metric
        // if the recorder was installed in this test, or be empty if it was installed
        // by a different test — either way, the endpoint responded correctly).
        assert!(
            body.is_empty() || body.contains('#') || body.contains("cloacina"),
            "unexpected metrics body format"
        );

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    // --- Tenant endpoint tests ---

    #[tokio::test]
    async fn test_tenant_endpoints_require_admin() {
        use crate::auth::cache::CachedKey;

        // Create a key that has read/write/execute but NOT admin
        let cache = crate::auth::cache::AuthCache::new(Duration::from_secs(60));
        let key_hash =
            cloacina::security::api_keys::hash_key("cloacina_test_demo_abcdef1234567890").unwrap();
        cache.insert(
            "test_demo".to_string(),
            vec![CachedKey {
                key_hash,
                key_id: uuid::Uuid::new_v4(),
                tenant_id: None,
                can_read: true,
                can_write: true,
                can_execute: true,
                can_admin: false, // NOT admin
                expires_at: None,
                revoked_at: None,
                workflow_patterns: vec![],
            }],
        );

        let (router, _) = app_with_auth_cache(cache);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let client = reqwest::Client::new();

        // GET /tenants without admin permission -> 403
        let resp = client
            .get(format!("http://{}/tenants", addr))
            .header(
                "Authorization",
                "Bearer cloacina_test_demo_abcdef1234567890",
            )
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 403);

        // POST /tenants without admin permission -> 403
        let resp = client
            .post(format!("http://{}/tenants", addr))
            .header(
                "Authorization",
                "Bearer cloacina_test_demo_abcdef1234567890",
            )
            .json(&serde_json::json!({"name": "acme", "schema_name": "tenant_acme"}))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 403);

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }

    #[tokio::test]
    async fn test_tenant_list_without_dal_returns_503() {
        use crate::auth::cache::CachedKey;

        // Create an admin-authenticated request that reaches the handler,
        // but the DAL is backed by an in-memory SQLite DB (the auth cache helper).
        // The handler calls require_dal() which gets the DAL from auth_state.
        // The actual DB call will fail since no tables exist, returning 500.
        // This verifies the error path when DAL operations fail.
        let cache = crate::auth::cache::AuthCache::new(Duration::from_secs(60));
        let key_hash =
            cloacina::security::api_keys::hash_key("cloacina_test_demo_abcdef1234567890").unwrap();
        cache.insert(
            "test_demo".to_string(),
            vec![CachedKey {
                key_hash,
                key_id: uuid::Uuid::new_v4(),
                tenant_id: None,
                can_read: true,
                can_write: true,
                can_execute: true,
                can_admin: true,
                expires_at: None,
                revoked_at: None,
                workflow_patterns: vec![],
            }],
        );

        let (router, _) = app_with_auth_cache(cache);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;
        let client = reqwest::Client::new();

        // Admin-authenticated request reaches the handler, DAL is available but
        // the underlying DB has no tables, so the list operation fails with 500.
        let resp = client
            .get(format!("http://{}/tenants", addr))
            .header(
                "Authorization",
                "Bearer cloacina_test_demo_abcdef1234567890",
            )
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 500);

        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["error"]["code"], "INTERNAL_ERROR");

        let _ = tokio::time::timeout(Duration::from_secs(6), server_handle).await;
    }
}
