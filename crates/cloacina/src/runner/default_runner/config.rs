/*
 *  Copyright 2025 Colliery Software
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

//! Configuration types for the DefaultRunner.
//!
//! This module contains the configuration structs and builders for
//! configuring the DefaultRunner's behavior.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::dispatcher::{DefaultDispatcher, Dispatcher, RoutingConfig, TaskExecutor};
use crate::executor::pipeline_executor::PipelineError;
use crate::executor::types::ExecutorConfig;
use crate::executor::ThreadTaskExecutor;
use crate::Database;
use crate::TaskScheduler;

use super::{DefaultRunner, RuntimeHandles};

/// Configuration for the default runner
///
/// This struct defines the configuration parameters that control the behavior
/// of the DefaultRunner. It includes settings for concurrency, timeouts,
/// polling intervals, and database connection management.
#[derive(Debug, Clone)]
pub struct DefaultRunnerConfig {
    /// Maximum number of concurrent task executions allowed at any given time.
    /// This controls the parallelism of task processing.
    pub max_concurrent_tasks: usize,

    /// How often the scheduler should check for ready tasks and dependencies.
    /// Lower values increase responsiveness but may increase database load.
    pub scheduler_poll_interval: Duration,

    /// Maximum time allowed for a single task to execute before timing out.
    /// Tasks that exceed this duration will be marked as failed.
    pub task_timeout: Duration,

    /// Optional maximum time allowed for an entire pipeline execution.
    /// If set, the pipeline will be marked as failed if it exceeds this duration.
    pub pipeline_timeout: Option<Duration>,

    /// Number of database connections to maintain in the connection pool.
    /// This should be tuned based on expected concurrent load.
    pub db_pool_size: u32,

    /// Whether to enable automatic recovery of in-progress workflows on startup.
    /// When enabled, the executor will attempt to resume interrupted workflows.
    pub enable_recovery: bool,

    /// Whether to enable cron scheduling functionality
    pub enable_cron_scheduling: bool,

    /// How often to poll for due cron schedules (when cron enabled)
    pub cron_poll_interval: Duration,

    /// Maximum number of missed executions to run in catchup mode (usize::MAX = unlimited)
    pub cron_max_catchup_executions: usize,

    /// Whether to enable automatic recovery of lost cron executions
    pub cron_enable_recovery: bool,

    /// How often to check for lost cron executions
    pub cron_recovery_interval: Duration,

    /// Consider executions lost if claimed more than this many minutes ago
    pub cron_lost_threshold_minutes: i32,

    /// Maximum age of executions to recover (older ones are abandoned)
    pub cron_max_recovery_age: Duration,

    /// Maximum number of recovery attempts per execution
    pub cron_max_recovery_attempts: usize,

    /// Whether to enable trigger scheduling functionality
    pub enable_trigger_scheduling: bool,

    /// Base poll interval for checking trigger readiness
    pub trigger_base_poll_interval: Duration,

    /// Maximum time to wait for a trigger poll operation
    pub trigger_poll_timeout: Duration,

    /// Whether to enable the registry reconciler for packaged workflows
    pub enable_registry_reconciler: bool,

    /// How often to run registry reconciliation
    pub registry_reconcile_interval: Duration,

    /// Whether to perform startup reconciliation of packaged workflows
    pub registry_enable_startup_reconciliation: bool,

    /// Path for storing packaged workflow registry files (when using filesystem storage)
    pub registry_storage_path: Option<std::path::PathBuf>,

    /// Registry storage backend type ("filesystem", "sqlite", "postgres")
    pub registry_storage_backend: String,

    /// Optional runner identifier for logging context
    /// When set, all logs from this runner instance will include this context
    pub runner_id: Option<String>,

    /// Optional runner name for logging context
    pub runner_name: Option<String>,

    /// Routing configuration for task dispatch.
    ///
    /// Controls how tasks are routed to executor backends.
    /// If None, all tasks are routed to the "default" executor.
    pub routing_config: Option<RoutingConfig>,
}

impl Default for DefaultRunnerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            scheduler_poll_interval: Duration::from_millis(100), // 100ms for responsive scheduling
            task_timeout: Duration::from_secs(300),              // 5 minutes
            pipeline_timeout: Some(Duration::from_secs(3600)),   // 1 hour
            db_pool_size: 10, // Default pool size (works for both PostgreSQL and SQLite)
            enable_recovery: true,
            enable_cron_scheduling: true, // Opt-out
            cron_poll_interval: Duration::from_secs(30),
            cron_max_catchup_executions: usize::MAX, // No practical limit by default
            cron_enable_recovery: true,
            cron_recovery_interval: Duration::from_secs(300), // 5 minutes
            cron_lost_threshold_minutes: 10,
            cron_max_recovery_age: Duration::from_secs(86400), // 24 hours
            cron_max_recovery_attempts: 3,
            enable_trigger_scheduling: true, // Opt-out (enabled by default)
            trigger_base_poll_interval: Duration::from_secs(1),
            trigger_poll_timeout: Duration::from_secs(30),
            enable_registry_reconciler: true, // Opt-out
            registry_reconcile_interval: Duration::from_secs(60), // Every minute
            registry_enable_startup_reconciliation: true,
            registry_storage_path: None, // Use default temp directory
            registry_storage_backend: "filesystem".to_string(),
            runner_id: None,
            runner_name: None,
            routing_config: None,
        }
    }
}

/// Builder for creating a DefaultRunner with PostgreSQL schema-based multi-tenancy
///
/// This builder supports PostgreSQL schema-based multi-tenancy for complete tenant isolation.
/// Each schema provides complete data isolation with zero collision risk.
///
/// # Example
/// ```rust,ignore
/// // Single-tenant PostgreSQL (uses public schema)
/// let runner = DefaultRunnerBuilder::new()
///     .database_url("postgresql://user:pass@localhost/cloacina")
///     .build()
///     .await?;
///
/// // Multi-tenant PostgreSQL with schema isolation
/// let tenant_a = DefaultRunnerBuilder::new()
///     .database_url("postgresql://user:pass@localhost/cloacina")
///     .schema("tenant_a")
///     .build()
///     .await?;
///
/// let tenant_b = DefaultRunnerBuilder::new()
///     .database_url("postgresql://user:pass@localhost/cloacina")
///     .schema("tenant_b")
///     .build()
///     .await?;
/// ```
pub struct DefaultRunnerBuilder {
    pub(super) database_url: Option<String>,
    pub(super) schema: Option<String>,
    pub(super) config: DefaultRunnerConfig,
}

impl Default for DefaultRunnerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultRunnerBuilder {
    /// Creates a new builder with default configuration
    pub fn new() -> Self {
        Self {
            database_url: None,
            schema: None,
            config: DefaultRunnerConfig::default(),
        }
    }

    /// Sets the database URL
    pub fn database_url(mut self, url: &str) -> Self {
        self.database_url = Some(url.to_string());
        self
    }

    /// Sets the PostgreSQL schema for multi-tenant isolation
    ///
    /// # Arguments
    /// * `schema` - The schema name (must be alphanumeric with underscores only)
    pub fn schema(mut self, schema: &str) -> Self {
        self.schema = Some(schema.to_string());
        self
    }

    /// Sets the full configuration
    pub fn with_config(mut self, config: DefaultRunnerConfig) -> Self {
        self.config = config;
        self
    }

    /// Validates the schema name contains only alphanumeric characters and underscores
    pub(super) fn validate_schema_name(schema: &str) -> Result<(), PipelineError> {
        if !schema.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(PipelineError::Configuration {
                message: "Schema name must contain only alphanumeric characters and underscores"
                    .to_string(),
            });
        }
        Ok(())
    }

    /// Builds the DefaultRunner
    pub async fn build(self) -> Result<DefaultRunner, PipelineError> {
        let database_url = self
            .database_url
            .ok_or_else(|| PipelineError::Configuration {
                message: "Database URL is required".to_string(),
            })?;

        if let Some(ref schema) = self.schema {
            Self::validate_schema_name(schema)?;

            // Validate schema is only used with PostgreSQL
            if !database_url.starts_with("postgresql://")
                && !database_url.starts_with("postgres://")
            {
                return Err(PipelineError::Configuration {
                    message: "Schema isolation is only supported with PostgreSQL. \
                             For SQLite multi-tenancy, use separate database files instead."
                        .to_string(),
                });
            }
        }

        // Create the database with schema support
        let database = Database::new_with_schema(
            &database_url,
            "cloacina",
            self.config.db_pool_size,
            self.schema.as_deref(),
        );

        // Set up schema if specified (PostgreSQL only)
        #[cfg(feature = "postgres")]
        {
            if let Some(ref schema) = self.schema {
                database
                    .setup_schema(schema)
                    .await
                    .map_err(|e| PipelineError::Configuration {
                        message: format!("Failed to set up schema '{}': {}", schema, e),
                    })?;
            } else {
                // Run migrations in public schema
                database
                    .run_migrations()
                    .await
                    .map_err(|e| PipelineError::DatabaseConnection { message: e })?;
            }
        }

        #[cfg(not(feature = "postgres"))]
        {
            // SQLite: just run migrations (schemas not supported)
            database
                .run_migrations()
                .await
                .map_err(|e| PipelineError::DatabaseConnection { message: e })?;
        }

        // Create scheduler with global workflow registry (always dynamic)
        let scheduler = TaskScheduler::with_poll_interval(
            database.clone(),
            self.config.scheduler_poll_interval,
        )
        .await
        .map_err(|e| PipelineError::Executor(e.into()))?;

        // Create task executor
        let executor_config = ExecutorConfig {
            max_concurrent_tasks: self.config.max_concurrent_tasks,
            task_timeout: self.config.task_timeout,
        };

        let executor = ThreadTaskExecutor::with_global_registry(database.clone(), executor_config)
            .map_err(|e| PipelineError::Configuration {
                message: e.to_string(),
            })?;

        // Configure dispatcher for push-based task execution
        let dal = crate::dal::DAL::new(database.clone());
        let routing_config = self
            .config
            .routing_config
            .clone()
            .unwrap_or_else(RoutingConfig::default);
        let dispatcher = DefaultDispatcher::new(dal, routing_config);

        // Register the executor with the dispatcher
        dispatcher.register_executor("default", Arc::new(executor) as Arc<dyn TaskExecutor>);

        let scheduler = scheduler.with_dispatcher(Arc::new(dispatcher));

        let default_runner = DefaultRunner {
            database,
            config: self.config.clone(),
            scheduler: Arc::new(scheduler),
            runtime_handles: Arc::new(RwLock::new(RuntimeHandles {
                scheduler_handle: None,
                executor_handle: None,
                cron_scheduler_handle: None,
                cron_recovery_handle: None,
                registry_reconciler_handle: None,
                trigger_scheduler_handle: None,
                shutdown_sender: None,
            })),
            cron_scheduler: Arc::new(RwLock::new(None)), // Initially empty
            cron_recovery: Arc::new(RwLock::new(None)),  // Initially empty
            workflow_registry: Arc::new(RwLock::new(None)), // Initially empty
            registry_reconciler: Arc::new(RwLock::new(None)), // Initially empty
            trigger_scheduler: Arc::new(RwLock::new(None)), // Initially empty
        };

        // Start the background services immediately
        default_runner.start_background_services().await?;

        Ok(default_runner)
    }

    /// Sets custom routing configuration for task dispatch.
    ///
    /// Use this to route different tasks to different executor backends.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let runner = DefaultRunner::builder()
    ///     .database_url("sqlite://test.db")
    ///     .routing_config(
    ///         RoutingConfig::new("default")
    ///             .with_rule(RoutingRule::new("ml::*", "gpu"))
    ///     )
    ///     .build()
    ///     .await?;
    /// ```
    pub fn routing_config(mut self, config: RoutingConfig) -> Self {
        self.config.routing_config = Some(config);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_runner_config() {
        let config = DefaultRunnerConfig::default();

        // Test default values
        assert_eq!(config.max_concurrent_tasks, 4);
        assert_eq!(config.scheduler_poll_interval, Duration::from_millis(100));
        assert_eq!(config.task_timeout, Duration::from_secs(300));
        assert_eq!(config.pipeline_timeout, Some(Duration::from_secs(3600)));
        assert!(config.enable_recovery);
        assert!(config.enable_cron_scheduling);
        assert!(config.enable_registry_reconciler);
        assert_eq!(config.registry_storage_backend, "filesystem");
        assert!(config.registry_storage_path.is_none());
        assert!(config.runner_id.is_none());
        assert!(config.runner_name.is_none());
    }

    #[test]
    fn test_registry_storage_backend_configuration() {
        let mut config = DefaultRunnerConfig::default();

        // Test filesystem backend
        config.registry_storage_backend = "filesystem".to_string();
        assert_eq!(config.registry_storage_backend, "filesystem");

        // Test sqlite backend
        config.registry_storage_backend = "sqlite".to_string();
        assert_eq!(config.registry_storage_backend, "sqlite");

        // Test postgres backend
        config.registry_storage_backend = "postgres".to_string();
        assert_eq!(config.registry_storage_backend, "postgres");

        // Test custom path for filesystem
        let custom_path = std::path::PathBuf::from("/custom/registry/path");
        config.registry_storage_path = Some(custom_path.clone());
        assert_eq!(config.registry_storage_path, Some(custom_path));
    }

    #[test]
    fn test_runner_identification() {
        let mut config = DefaultRunnerConfig::default();

        config.runner_id = Some("test-runner-123".to_string());
        config.runner_name = Some("Test Runner".to_string());

        assert_eq!(config.runner_id, Some("test-runner-123".to_string()));
        assert_eq!(config.runner_name, Some("Test Runner".to_string()));
    }

    #[test]
    fn test_registry_configuration_options() {
        let mut config = DefaultRunnerConfig::default();

        // Test disabling registry reconciler
        config.enable_registry_reconciler = false;
        assert!(!config.enable_registry_reconciler);

        // Test custom reconcile interval
        config.registry_reconcile_interval = Duration::from_secs(30);
        assert_eq!(config.registry_reconcile_interval, Duration::from_secs(30));

        // Test disabling startup reconciliation
        config.registry_enable_startup_reconciliation = false;
        assert!(!config.registry_enable_startup_reconciliation);
    }

    #[test]
    fn test_cron_configuration() {
        let mut config = DefaultRunnerConfig::default();

        // Test cron settings
        config.cron_poll_interval = Duration::from_secs(60);
        config.cron_recovery_interval = Duration::from_secs(300);
        config.cron_lost_threshold_minutes = 15;
        config.cron_max_recovery_age = Duration::from_secs(86400);
        config.cron_max_recovery_attempts = 5;

        assert_eq!(config.cron_poll_interval, Duration::from_secs(60));
        assert_eq!(config.cron_recovery_interval, Duration::from_secs(300));
        assert_eq!(config.cron_lost_threshold_minutes, 15);
        assert_eq!(config.cron_max_recovery_age, Duration::from_secs(86400));
        assert_eq!(config.cron_max_recovery_attempts, 5);
    }

    #[test]
    fn test_db_pool_size_default() {
        let config = DefaultRunnerConfig::default();
        assert_eq!(config.db_pool_size, 10); // Default pool size for both backends
    }

    #[test]
    fn test_config_clone() {
        let config = DefaultRunnerConfig::default();
        let cloned = config.clone();

        assert_eq!(
            config.registry_storage_backend,
            cloned.registry_storage_backend
        );
        assert_eq!(config.max_concurrent_tasks, cloned.max_concurrent_tasks);
        assert_eq!(
            config.enable_registry_reconciler,
            cloned.enable_registry_reconciler
        );
    }

    #[test]
    fn test_config_debug() {
        let config = DefaultRunnerConfig::default();
        let debug_str = format!("{:?}", config);

        // Verify debug formatting includes key fields
        assert!(debug_str.contains("registry_storage_backend"));
        assert!(debug_str.contains("filesystem"));
        assert!(debug_str.contains("max_concurrent_tasks"));
    }
}
