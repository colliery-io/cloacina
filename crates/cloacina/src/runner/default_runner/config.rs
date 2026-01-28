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
///
/// # Construction
///
/// Use [`DefaultRunnerConfig::builder()`] to create a configuration:
///
/// ```rust,ignore
/// let config = DefaultRunnerConfig::builder()
///     .max_concurrent_tasks(8)
///     .task_timeout(Duration::from_secs(600))
///     .build();
/// ```
///
/// Or use the default configuration:
///
/// ```rust,ignore
/// let config = DefaultRunnerConfig::default();
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DefaultRunnerConfig {
    max_concurrent_tasks: usize,
    scheduler_poll_interval: Duration,
    task_timeout: Duration,
    pipeline_timeout: Option<Duration>,
    db_pool_size: u32,
    enable_recovery: bool,
    enable_cron_scheduling: bool,
    cron_poll_interval: Duration,
    cron_max_catchup_executions: usize,
    cron_enable_recovery: bool,
    cron_recovery_interval: Duration,
    cron_lost_threshold_minutes: i32,
    cron_max_recovery_age: Duration,
    cron_max_recovery_attempts: usize,
    enable_trigger_scheduling: bool,
    trigger_base_poll_interval: Duration,
    trigger_poll_timeout: Duration,
    enable_registry_reconciler: bool,
    registry_reconcile_interval: Duration,
    registry_enable_startup_reconciliation: bool,
    registry_storage_path: Option<std::path::PathBuf>,
    registry_storage_backend: String,
    runner_id: Option<String>,
    runner_name: Option<String>,
    routing_config: Option<RoutingConfig>,
}

impl DefaultRunnerConfig {
    /// Creates a new configuration builder with default values.
    pub fn builder() -> DefaultRunnerConfigBuilder {
        DefaultRunnerConfigBuilder::default()
    }

    /// Maximum number of concurrent task executions allowed.
    pub fn max_concurrent_tasks(&self) -> usize {
        self.max_concurrent_tasks
    }

    /// How often the scheduler checks for ready tasks.
    pub fn scheduler_poll_interval(&self) -> Duration {
        self.scheduler_poll_interval
    }

    /// Maximum time allowed for a single task to execute.
    pub fn task_timeout(&self) -> Duration {
        self.task_timeout
    }

    /// Optional maximum time for an entire pipeline execution.
    pub fn pipeline_timeout(&self) -> Option<Duration> {
        self.pipeline_timeout
    }

    /// Number of database connections in the pool.
    pub fn db_pool_size(&self) -> u32 {
        self.db_pool_size
    }

    /// Whether automatic recovery is enabled.
    pub fn enable_recovery(&self) -> bool {
        self.enable_recovery
    }

    /// Whether cron scheduling is enabled.
    pub fn enable_cron_scheduling(&self) -> bool {
        self.enable_cron_scheduling
    }

    /// Poll interval for cron schedules.
    pub fn cron_poll_interval(&self) -> Duration {
        self.cron_poll_interval
    }

    /// Maximum catchup executions for missed cron runs.
    pub fn cron_max_catchup_executions(&self) -> usize {
        self.cron_max_catchup_executions
    }

    /// Whether cron recovery is enabled.
    pub fn cron_enable_recovery(&self) -> bool {
        self.cron_enable_recovery
    }

    /// How often to check for lost cron executions.
    pub fn cron_recovery_interval(&self) -> Duration {
        self.cron_recovery_interval
    }

    /// Minutes before an execution is considered lost.
    pub fn cron_lost_threshold_minutes(&self) -> i32 {
        self.cron_lost_threshold_minutes
    }

    /// Maximum age of executions to recover.
    pub fn cron_max_recovery_age(&self) -> Duration {
        self.cron_max_recovery_age
    }

    /// Maximum recovery attempts per execution.
    pub fn cron_max_recovery_attempts(&self) -> usize {
        self.cron_max_recovery_attempts
    }

    /// Whether trigger scheduling is enabled.
    pub fn enable_trigger_scheduling(&self) -> bool {
        self.enable_trigger_scheduling
    }

    /// Base poll interval for trigger readiness checks.
    pub fn trigger_base_poll_interval(&self) -> Duration {
        self.trigger_base_poll_interval
    }

    /// Timeout for trigger poll operations.
    pub fn trigger_poll_timeout(&self) -> Duration {
        self.trigger_poll_timeout
    }

    /// Whether the registry reconciler is enabled.
    pub fn enable_registry_reconciler(&self) -> bool {
        self.enable_registry_reconciler
    }

    /// How often to run registry reconciliation.
    pub fn registry_reconcile_interval(&self) -> Duration {
        self.registry_reconcile_interval
    }

    /// Whether startup reconciliation is enabled.
    pub fn registry_enable_startup_reconciliation(&self) -> bool {
        self.registry_enable_startup_reconciliation
    }

    /// Path for registry storage (filesystem backend).
    pub fn registry_storage_path(&self) -> Option<&std::path::Path> {
        self.registry_storage_path.as_deref()
    }

    /// Registry storage backend type.
    pub fn registry_storage_backend(&self) -> &str {
        &self.registry_storage_backend
    }

    /// Optional runner identifier for logging.
    pub fn runner_id(&self) -> Option<&str> {
        self.runner_id.as_deref()
    }

    /// Optional runner name for logging.
    pub fn runner_name(&self) -> Option<&str> {
        self.runner_name.as_deref()
    }

    /// Routing configuration for task dispatch.
    pub fn routing_config(&self) -> Option<&RoutingConfig> {
        self.routing_config.as_ref()
    }
}

/// Builder for [`DefaultRunnerConfig`].
///
/// Use this builder to create a customized configuration:
///
/// ```rust,ignore
/// let config = DefaultRunnerConfig::builder()
///     .max_concurrent_tasks(8)
///     .enable_cron_scheduling(false)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct DefaultRunnerConfigBuilder {
    config: DefaultRunnerConfig,
}

impl Default for DefaultRunnerConfigBuilder {
    fn default() -> Self {
        Self {
            config: DefaultRunnerConfig {
                max_concurrent_tasks: 4,
                scheduler_poll_interval: Duration::from_millis(100),
                task_timeout: Duration::from_secs(300),
                pipeline_timeout: Some(Duration::from_secs(3600)),
                db_pool_size: 10,
                enable_recovery: true,
                enable_cron_scheduling: true,
                cron_poll_interval: Duration::from_secs(30),
                cron_max_catchup_executions: usize::MAX,
                cron_enable_recovery: true,
                cron_recovery_interval: Duration::from_secs(300),
                cron_lost_threshold_minutes: 10,
                cron_max_recovery_age: Duration::from_secs(86400),
                cron_max_recovery_attempts: 3,
                enable_trigger_scheduling: true,
                trigger_base_poll_interval: Duration::from_secs(1),
                trigger_poll_timeout: Duration::from_secs(30),
                enable_registry_reconciler: true,
                registry_reconcile_interval: Duration::from_secs(60),
                registry_enable_startup_reconciliation: true,
                registry_storage_path: None,
                registry_storage_backend: "filesystem".to_string(),
                runner_id: None,
                runner_name: None,
                routing_config: None,
            },
        }
    }
}

impl DefaultRunnerConfigBuilder {
    /// Sets the maximum number of concurrent task executions.
    pub fn max_concurrent_tasks(mut self, value: usize) -> Self {
        self.config.max_concurrent_tasks = value;
        self
    }

    /// Sets the scheduler poll interval.
    pub fn scheduler_poll_interval(mut self, value: Duration) -> Self {
        self.config.scheduler_poll_interval = value;
        self
    }

    /// Sets the task timeout.
    pub fn task_timeout(mut self, value: Duration) -> Self {
        self.config.task_timeout = value;
        self
    }

    /// Sets the pipeline timeout.
    pub fn pipeline_timeout(mut self, value: Option<Duration>) -> Self {
        self.config.pipeline_timeout = value;
        self
    }

    /// Sets the database pool size.
    pub fn db_pool_size(mut self, value: u32) -> Self {
        self.config.db_pool_size = value;
        self
    }

    /// Enables or disables automatic recovery.
    pub fn enable_recovery(mut self, value: bool) -> Self {
        self.config.enable_recovery = value;
        self
    }

    /// Enables or disables cron scheduling.
    pub fn enable_cron_scheduling(mut self, value: bool) -> Self {
        self.config.enable_cron_scheduling = value;
        self
    }

    /// Sets the cron poll interval.
    pub fn cron_poll_interval(mut self, value: Duration) -> Self {
        self.config.cron_poll_interval = value;
        self
    }

    /// Sets the maximum catchup executions for cron.
    pub fn cron_max_catchup_executions(mut self, value: usize) -> Self {
        self.config.cron_max_catchup_executions = value;
        self
    }

    /// Enables or disables cron recovery.
    pub fn cron_enable_recovery(mut self, value: bool) -> Self {
        self.config.cron_enable_recovery = value;
        self
    }

    /// Sets the cron recovery interval.
    pub fn cron_recovery_interval(mut self, value: Duration) -> Self {
        self.config.cron_recovery_interval = value;
        self
    }

    /// Sets the cron lost threshold in minutes.
    pub fn cron_lost_threshold_minutes(mut self, value: i32) -> Self {
        self.config.cron_lost_threshold_minutes = value;
        self
    }

    /// Sets the maximum cron recovery age.
    pub fn cron_max_recovery_age(mut self, value: Duration) -> Self {
        self.config.cron_max_recovery_age = value;
        self
    }

    /// Sets the maximum cron recovery attempts.
    pub fn cron_max_recovery_attempts(mut self, value: usize) -> Self {
        self.config.cron_max_recovery_attempts = value;
        self
    }

    /// Enables or disables trigger scheduling.
    pub fn enable_trigger_scheduling(mut self, value: bool) -> Self {
        self.config.enable_trigger_scheduling = value;
        self
    }

    /// Sets the trigger base poll interval.
    pub fn trigger_base_poll_interval(mut self, value: Duration) -> Self {
        self.config.trigger_base_poll_interval = value;
        self
    }

    /// Sets the trigger poll timeout.
    pub fn trigger_poll_timeout(mut self, value: Duration) -> Self {
        self.config.trigger_poll_timeout = value;
        self
    }

    /// Enables or disables the registry reconciler.
    pub fn enable_registry_reconciler(mut self, value: bool) -> Self {
        self.config.enable_registry_reconciler = value;
        self
    }

    /// Sets the registry reconcile interval.
    pub fn registry_reconcile_interval(mut self, value: Duration) -> Self {
        self.config.registry_reconcile_interval = value;
        self
    }

    /// Enables or disables startup reconciliation.
    pub fn registry_enable_startup_reconciliation(mut self, value: bool) -> Self {
        self.config.registry_enable_startup_reconciliation = value;
        self
    }

    /// Sets the registry storage path.
    pub fn registry_storage_path(mut self, value: Option<std::path::PathBuf>) -> Self {
        self.config.registry_storage_path = value;
        self
    }

    /// Sets the registry storage backend.
    pub fn registry_storage_backend(mut self, value: impl Into<String>) -> Self {
        self.config.registry_storage_backend = value.into();
        self
    }

    /// Sets the runner identifier.
    pub fn runner_id(mut self, value: Option<String>) -> Self {
        self.config.runner_id = value;
        self
    }

    /// Sets the runner name.
    pub fn runner_name(mut self, value: Option<String>) -> Self {
        self.config.runner_name = value;
        self
    }

    /// Sets the routing configuration.
    pub fn routing_config(mut self, value: Option<RoutingConfig>) -> Self {
        self.config.routing_config = value;
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> DefaultRunnerConfig {
        self.config
    }
}

impl Default for DefaultRunnerConfig {
    fn default() -> Self {
        DefaultRunnerConfigBuilder::default().build()
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
            self.config.db_pool_size(),
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
            self.config.scheduler_poll_interval(),
        )
        .await
        .map_err(|e| PipelineError::Executor(e.into()))?;

        // Create task executor
        let executor_config = ExecutorConfig {
            max_concurrent_tasks: self.config.max_concurrent_tasks(),
            task_timeout: self.config.task_timeout(),
        };

        let executor = ThreadTaskExecutor::with_global_registry(database.clone(), executor_config)
            .map_err(|e| PipelineError::Configuration {
                message: e.to_string(),
            })?;

        // Configure dispatcher for push-based task execution
        let dal = crate::dal::DAL::new(database.clone());
        let routing_config = self
            .config
            .routing_config()
            .cloned()
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

        // Test default values via getter methods
        assert_eq!(config.max_concurrent_tasks(), 4);
        assert_eq!(config.scheduler_poll_interval(), Duration::from_millis(100));
        assert_eq!(config.task_timeout(), Duration::from_secs(300));
        assert_eq!(config.pipeline_timeout(), Some(Duration::from_secs(3600)));
        assert!(config.enable_recovery());
        assert!(config.enable_cron_scheduling());
        assert!(config.enable_registry_reconciler());
        assert_eq!(config.registry_storage_backend(), "filesystem");
        assert!(config.registry_storage_path().is_none());
        assert!(config.runner_id().is_none());
        assert!(config.runner_name().is_none());
    }

    #[test]
    fn test_registry_storage_backend_configuration() {
        // Test filesystem backend (default)
        let config = DefaultRunnerConfig::default();
        assert_eq!(config.registry_storage_backend(), "filesystem");

        // Test sqlite backend via builder
        let config = DefaultRunnerConfig::builder()
            .registry_storage_backend("sqlite")
            .build();
        assert_eq!(config.registry_storage_backend(), "sqlite");

        // Test postgres backend via builder
        let config = DefaultRunnerConfig::builder()
            .registry_storage_backend("postgres")
            .build();
        assert_eq!(config.registry_storage_backend(), "postgres");

        // Test custom path for filesystem via builder
        let custom_path = std::path::PathBuf::from("/custom/registry/path");
        let config = DefaultRunnerConfig::builder()
            .registry_storage_path(Some(custom_path.clone()))
            .build();
        assert_eq!(config.registry_storage_path(), Some(custom_path.as_path()));
    }

    #[test]
    fn test_runner_identification() {
        let config = DefaultRunnerConfig::builder()
            .runner_id(Some("test-runner-123".to_string()))
            .runner_name(Some("Test Runner".to_string()))
            .build();

        assert_eq!(config.runner_id(), Some("test-runner-123"));
        assert_eq!(config.runner_name(), Some("Test Runner"));
    }

    #[test]
    fn test_registry_configuration_options() {
        // Test disabling registry reconciler via builder
        let config = DefaultRunnerConfig::builder()
            .enable_registry_reconciler(false)
            .build();
        assert!(!config.enable_registry_reconciler());

        // Test custom reconcile interval via builder
        let config = DefaultRunnerConfig::builder()
            .registry_reconcile_interval(Duration::from_secs(30))
            .build();
        assert_eq!(
            config.registry_reconcile_interval(),
            Duration::from_secs(30)
        );

        // Test disabling startup reconciliation via builder
        let config = DefaultRunnerConfig::builder()
            .registry_enable_startup_reconciliation(false)
            .build();
        assert!(!config.registry_enable_startup_reconciliation());
    }

    #[test]
    fn test_cron_configuration() {
        // Test cron settings via builder
        let config = DefaultRunnerConfig::builder()
            .cron_poll_interval(Duration::from_secs(60))
            .cron_recovery_interval(Duration::from_secs(300))
            .cron_lost_threshold_minutes(15)
            .cron_max_recovery_age(Duration::from_secs(86400))
            .cron_max_recovery_attempts(5)
            .build();

        assert_eq!(config.cron_poll_interval(), Duration::from_secs(60));
        assert_eq!(config.cron_recovery_interval(), Duration::from_secs(300));
        assert_eq!(config.cron_lost_threshold_minutes(), 15);
        assert_eq!(config.cron_max_recovery_age(), Duration::from_secs(86400));
        assert_eq!(config.cron_max_recovery_attempts(), 5);
    }

    #[test]
    fn test_db_pool_size_default() {
        let config = DefaultRunnerConfig::default();
        assert_eq!(config.db_pool_size(), 10); // Default pool size for both backends
    }

    #[test]
    fn test_config_clone() {
        let config = DefaultRunnerConfig::default();
        let cloned = config.clone();

        assert_eq!(
            config.registry_storage_backend(),
            cloned.registry_storage_backend()
        );
        assert_eq!(config.max_concurrent_tasks(), cloned.max_concurrent_tasks());
        assert_eq!(
            config.enable_registry_reconciler(),
            cloned.enable_registry_reconciler()
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

    #[test]
    fn test_builder_all_fields() {
        // Test that builder can set all fields
        let config = DefaultRunnerConfig::builder()
            .max_concurrent_tasks(8)
            .scheduler_poll_interval(Duration::from_millis(200))
            .task_timeout(Duration::from_secs(600))
            .pipeline_timeout(Some(Duration::from_secs(7200)))
            .db_pool_size(20)
            .enable_recovery(false)
            .enable_cron_scheduling(false)
            .cron_poll_interval(Duration::from_secs(60))
            .cron_max_catchup_executions(10)
            .cron_enable_recovery(false)
            .enable_trigger_scheduling(false)
            .trigger_base_poll_interval(Duration::from_secs(5))
            .trigger_poll_timeout(Duration::from_secs(60))
            .build();

        assert_eq!(config.max_concurrent_tasks(), 8);
        assert_eq!(config.scheduler_poll_interval(), Duration::from_millis(200));
        assert_eq!(config.task_timeout(), Duration::from_secs(600));
        assert_eq!(config.pipeline_timeout(), Some(Duration::from_secs(7200)));
        assert_eq!(config.db_pool_size(), 20);
        assert!(!config.enable_recovery());
        assert!(!config.enable_cron_scheduling());
        assert_eq!(config.cron_poll_interval(), Duration::from_secs(60));
        assert_eq!(config.cron_max_catchup_executions(), 10);
        assert!(!config.cron_enable_recovery());
        assert!(!config.enable_trigger_scheduling());
        assert_eq!(config.trigger_base_poll_interval(), Duration::from_secs(5));
        assert_eq!(config.trigger_poll_timeout(), Duration::from_secs(60));
    }
}
