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

// PyContext has moved to cloacina::python::context.
// Re-export for internal crate compatibility.
pub use cloacina::python::context::PyContext;

use pyo3::prelude::*;

/// PyDefaultRunnerConfig - Python wrapper for Rust DefaultRunnerConfig
#[pyclass(name = "DefaultRunnerConfig")]
#[derive(Debug, Clone)]
pub struct PyDefaultRunnerConfig {
    inner: cloacina::runner::DefaultRunnerConfig,
}

#[pymethods]
impl PyDefaultRunnerConfig {
    /// Creates a new DefaultRunnerConfig with customizable parameters
    #[new]
    #[pyo3(signature = (
        max_concurrent_tasks = None,
        scheduler_poll_interval_ms = None,
        task_timeout_seconds = None,
        pipeline_timeout_seconds = None,
        db_pool_size = None,
        enable_recovery = None,
        enable_cron_scheduling = None,
        cron_poll_interval_seconds = None,
        cron_max_catchup_executions = None,
        cron_enable_recovery = None,
        cron_recovery_interval_seconds = None,
        cron_lost_threshold_minutes = None,
        cron_max_recovery_age_seconds = None,
        cron_max_recovery_attempts = None
    ))]
    pub fn new(
        max_concurrent_tasks: Option<usize>,
        scheduler_poll_interval_ms: Option<u64>,
        task_timeout_seconds: Option<u64>,
        pipeline_timeout_seconds: Option<u64>,
        db_pool_size: Option<u32>,
        enable_recovery: Option<bool>,
        enable_cron_scheduling: Option<bool>,
        cron_poll_interval_seconds: Option<u64>,
        cron_max_catchup_executions: Option<usize>,
        cron_enable_recovery: Option<bool>,
        cron_recovery_interval_seconds: Option<u64>,
        cron_lost_threshold_minutes: Option<i32>,
        cron_max_recovery_age_seconds: Option<u64>,
        cron_max_recovery_attempts: Option<usize>,
    ) -> Self {
        use std::time::Duration;

        let mut builder = cloacina::runner::DefaultRunnerConfig::builder();

        if let Some(val) = max_concurrent_tasks {
            builder = builder.max_concurrent_tasks(val);
        }
        if let Some(val) = scheduler_poll_interval_ms {
            builder = builder.scheduler_poll_interval(Duration::from_millis(val));
        }
        if let Some(val) = task_timeout_seconds {
            builder = builder.task_timeout(Duration::from_secs(val));
        }
        if let Some(val) = pipeline_timeout_seconds {
            builder = builder.pipeline_timeout(Some(Duration::from_secs(val)));
        }
        if let Some(val) = db_pool_size {
            builder = builder.db_pool_size(val);
        }
        if let Some(val) = enable_recovery {
            builder = builder.enable_recovery(val);
        }
        if let Some(val) = enable_cron_scheduling {
            builder = builder.enable_cron_scheduling(val);
        }
        if let Some(val) = cron_poll_interval_seconds {
            builder = builder.cron_poll_interval(Duration::from_secs(val));
        }
        if let Some(val) = cron_max_catchup_executions {
            builder = builder.cron_max_catchup_executions(val);
        }
        if let Some(val) = cron_enable_recovery {
            builder = builder.cron_enable_recovery(val);
        }
        if let Some(val) = cron_recovery_interval_seconds {
            builder = builder.cron_recovery_interval(Duration::from_secs(val));
        }
        if let Some(val) = cron_lost_threshold_minutes {
            builder = builder.cron_lost_threshold_minutes(val);
        }
        if let Some(val) = cron_max_recovery_age_seconds {
            builder = builder.cron_max_recovery_age(Duration::from_secs(val));
        }
        if let Some(val) = cron_max_recovery_attempts {
            builder = builder.cron_max_recovery_attempts(val);
        }

        PyDefaultRunnerConfig {
            inner: builder.build(),
        }
    }

    /// Creates a DefaultRunnerConfig with all default values
    #[staticmethod]
    pub fn default() -> Self {
        PyDefaultRunnerConfig {
            inner: cloacina::runner::DefaultRunnerConfig::default(),
        }
    }

    #[getter]
    pub fn max_concurrent_tasks(&self) -> usize {
        self.inner.max_concurrent_tasks()
    }

    #[getter]
    pub fn scheduler_poll_interval_ms(&self) -> u64 {
        self.inner.scheduler_poll_interval().as_millis() as u64
    }

    #[getter]
    pub fn task_timeout_seconds(&self) -> u64 {
        self.inner.task_timeout().as_secs()
    }

    #[getter]
    pub fn pipeline_timeout_seconds(&self) -> Option<u64> {
        self.inner.pipeline_timeout().map(|d| d.as_secs())
    }

    #[getter]
    pub fn db_pool_size(&self) -> u32 {
        self.inner.db_pool_size()
    }

    #[getter]
    pub fn enable_recovery(&self) -> bool {
        self.inner.enable_recovery()
    }

    #[getter]
    pub fn enable_cron_scheduling(&self) -> bool {
        self.inner.enable_cron_scheduling()
    }

    #[getter]
    pub fn cron_poll_interval_seconds(&self) -> u64 {
        self.inner.cron_poll_interval().as_secs()
    }

    #[getter]
    pub fn cron_max_catchup_executions(&self) -> usize {
        self.inner.cron_max_catchup_executions()
    }

    #[getter]
    pub fn cron_enable_recovery(&self) -> bool {
        self.inner.cron_enable_recovery()
    }

    #[getter]
    pub fn cron_recovery_interval_seconds(&self) -> u64 {
        self.inner.cron_recovery_interval().as_secs()
    }

    #[getter]
    pub fn cron_lost_threshold_minutes(&self) -> i32 {
        self.inner.cron_lost_threshold_minutes()
    }

    #[getter]
    pub fn cron_max_recovery_age_seconds(&self) -> u64 {
        self.inner.cron_max_recovery_age().as_secs()
    }

    #[getter]
    pub fn cron_max_recovery_attempts(&self) -> usize {
        self.inner.cron_max_recovery_attempts()
    }

    #[setter]
    pub fn set_max_concurrent_tasks(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.max_concurrent_tasks(value));
    }

    #[setter]
    pub fn set_scheduler_poll_interval_ms(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.scheduler_poll_interval(std::time::Duration::from_millis(value)));
    }

    #[setter]
    pub fn set_task_timeout_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.task_timeout(std::time::Duration::from_secs(value)));
    }

    #[setter]
    pub fn set_pipeline_timeout_seconds(&mut self, value: Option<u64>) {
        self.inner =
            self.rebuild(|b| b.pipeline_timeout(value.map(std::time::Duration::from_secs)));
    }

    #[setter]
    pub fn set_db_pool_size(&mut self, value: u32) {
        self.inner = self.rebuild(|b| b.db_pool_size(value));
    }

    #[setter]
    pub fn set_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_recovery(value));
    }

    #[setter]
    pub fn set_enable_cron_scheduling(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_cron_scheduling(value));
    }

    #[setter]
    pub fn set_cron_poll_interval_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.cron_poll_interval(std::time::Duration::from_secs(value)));
    }

    #[setter]
    pub fn set_cron_max_catchup_executions(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_catchup_executions(value));
    }

    #[setter]
    pub fn set_cron_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.cron_enable_recovery(value));
    }

    #[setter]
    pub fn set_cron_recovery_interval_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_recovery_interval(std::time::Duration::from_secs(value)));
    }

    #[setter]
    pub fn set_cron_lost_threshold_minutes(&mut self, value: i32) {
        self.inner = self.rebuild(|b| b.cron_lost_threshold_minutes(value));
    }

    #[setter]
    pub fn set_cron_max_recovery_age_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_max_recovery_age(std::time::Duration::from_secs(value)));
    }

    #[setter]
    pub fn set_cron_max_recovery_attempts(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_recovery_attempts(value));
    }

    /// Returns a dictionary representation of the configuration
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new(py);

        dict.set_item("max_concurrent_tasks", self.inner.max_concurrent_tasks())?;
        dict.set_item(
            "scheduler_poll_interval_ms",
            self.inner.scheduler_poll_interval().as_millis() as u64,
        )?;
        dict.set_item("task_timeout_seconds", self.inner.task_timeout().as_secs())?;
        dict.set_item(
            "pipeline_timeout_seconds",
            self.inner.pipeline_timeout().map(|d| d.as_secs()),
        )?;
        dict.set_item("db_pool_size", self.inner.db_pool_size())?;
        dict.set_item("enable_recovery", self.inner.enable_recovery())?;
        dict.set_item(
            "enable_cron_scheduling",
            self.inner.enable_cron_scheduling(),
        )?;
        dict.set_item(
            "cron_poll_interval_seconds",
            self.inner.cron_poll_interval().as_secs(),
        )?;
        dict.set_item(
            "cron_max_catchup_executions",
            self.inner.cron_max_catchup_executions(),
        )?;
        dict.set_item("cron_enable_recovery", self.inner.cron_enable_recovery())?;
        dict.set_item(
            "cron_recovery_interval_seconds",
            self.inner.cron_recovery_interval().as_secs(),
        )?;
        dict.set_item(
            "cron_lost_threshold_minutes",
            self.inner.cron_lost_threshold_minutes(),
        )?;
        dict.set_item(
            "cron_max_recovery_age_seconds",
            self.inner.cron_max_recovery_age().as_secs(),
        )?;
        dict.set_item(
            "cron_max_recovery_attempts",
            self.inner.cron_max_recovery_attempts(),
        )?;

        Ok(dict.into())
    }

    /// String representation of the configuration
    pub fn __repr__(&self) -> String {
        format!(
            "DefaultRunnerConfig(max_concurrent_tasks={}, enable_cron_scheduling={}, db_pool_size={})",
            self.inner.max_concurrent_tasks(),
            self.inner.enable_cron_scheduling(),
            self.inner.db_pool_size()
        )
    }
}

impl PyDefaultRunnerConfig {
    /// Get the inner Rust config (for internal use)
    pub(crate) fn to_rust_config(&self) -> cloacina::runner::DefaultRunnerConfig {
        self.inner.clone()
    }

    fn rebuild(
        &self,
        apply: impl FnOnce(
            cloacina::runner::DefaultRunnerConfigBuilder,
        ) -> cloacina::runner::DefaultRunnerConfigBuilder,
    ) -> cloacina::runner::DefaultRunnerConfig {
        let c = &self.inner;
        let builder = cloacina::runner::DefaultRunnerConfig::builder()
            .max_concurrent_tasks(c.max_concurrent_tasks())
            .scheduler_poll_interval(c.scheduler_poll_interval())
            .task_timeout(c.task_timeout())
            .pipeline_timeout(c.pipeline_timeout())
            .db_pool_size(c.db_pool_size())
            .enable_recovery(c.enable_recovery())
            .enable_cron_scheduling(c.enable_cron_scheduling())
            .cron_poll_interval(c.cron_poll_interval())
            .cron_max_catchup_executions(c.cron_max_catchup_executions())
            .cron_enable_recovery(c.cron_enable_recovery())
            .cron_recovery_interval(c.cron_recovery_interval())
            .cron_lost_threshold_minutes(c.cron_lost_threshold_minutes())
            .cron_max_recovery_age(c.cron_max_recovery_age())
            .cron_max_recovery_attempts(c.cron_max_recovery_attempts());
        apply(builder).build()
    }
}
