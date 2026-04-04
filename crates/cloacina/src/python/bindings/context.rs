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
pub use crate::python::context::PyContext;

use pyo3::prelude::*;

/// PyDefaultRunnerConfig - Python wrapper for Rust DefaultRunnerConfig
#[pyclass(name = "DefaultRunnerConfig")]
#[derive(Debug, Clone)]
pub struct PyDefaultRunnerConfig {
    inner: crate::runner::DefaultRunnerConfig,
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
    #[allow(clippy::too_many_arguments)]
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

        let mut builder = crate::runner::DefaultRunnerConfig::builder();

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
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        PyDefaultRunnerConfig {
            inner: crate::runner::DefaultRunnerConfig::default(),
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
    pub(crate) fn to_rust_config(&self) -> crate::runner::DefaultRunnerConfig {
        self.inner.clone()
    }

    fn rebuild(
        &self,
        apply: impl FnOnce(
            crate::runner::DefaultRunnerConfigBuilder,
        ) -> crate::runner::DefaultRunnerConfigBuilder,
    ) -> crate::runner::DefaultRunnerConfig {
        let c = &self.inner;
        let builder = crate::runner::DefaultRunnerConfig::builder()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_construction() {
        pyo3::prepare_freethreaded_python();
        let config = PyDefaultRunnerConfig::default();
        // Should have reasonable defaults
        assert!(config.max_concurrent_tasks() > 0);
        assert!(config.task_timeout_seconds() > 0);
        assert!(config.db_pool_size() > 0);
    }

    #[test]
    fn test_new_with_defaults() {
        pyo3::prepare_freethreaded_python();
        let config = PyDefaultRunnerConfig::new(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
        );
        // Should match ::default()
        let default = PyDefaultRunnerConfig::default();
        assert_eq!(
            config.max_concurrent_tasks(),
            default.max_concurrent_tasks()
        );
        assert_eq!(
            config.task_timeout_seconds(),
            default.task_timeout_seconds()
        );
    }

    #[test]
    fn test_new_with_custom_params() {
        pyo3::prepare_freethreaded_python();
        let config = PyDefaultRunnerConfig::new(
            Some(16),
            Some(500),
            Some(120),
            Some(3600),
            Some(10),
            Some(true),
            Some(true),
            Some(30),
            Some(5),
            Some(true),
            Some(60),
            Some(15),
            Some(7200),
            Some(3),
        );
        assert_eq!(config.max_concurrent_tasks(), 16);
        assert_eq!(config.scheduler_poll_interval_ms(), 500);
        assert_eq!(config.task_timeout_seconds(), 120);
        assert_eq!(config.pipeline_timeout_seconds(), Some(3600));
        assert_eq!(config.db_pool_size(), 10);
        assert!(config.enable_recovery());
        assert!(config.enable_cron_scheduling());
        assert_eq!(config.cron_poll_interval_seconds(), 30);
        assert_eq!(config.cron_max_catchup_executions(), 5);
        assert!(config.cron_enable_recovery());
        assert_eq!(config.cron_recovery_interval_seconds(), 60);
        assert_eq!(config.cron_lost_threshold_minutes(), 15);
        assert_eq!(config.cron_max_recovery_age_seconds(), 7200);
        assert_eq!(config.cron_max_recovery_attempts(), 3);
    }

    #[test]
    fn test_repr() {
        pyo3::prepare_freethreaded_python();
        let config = PyDefaultRunnerConfig::default();
        let repr = config.__repr__();
        assert!(repr.starts_with("DefaultRunnerConfig("));
        assert!(repr.contains("max_concurrent_tasks="));
        assert!(repr.contains("db_pool_size="));
    }

    #[test]
    fn test_setters() {
        pyo3::prepare_freethreaded_python();
        let mut config = PyDefaultRunnerConfig::default();

        config.set_max_concurrent_tasks(32);
        assert_eq!(config.max_concurrent_tasks(), 32);

        config.set_task_timeout_seconds(999);
        assert_eq!(config.task_timeout_seconds(), 999);

        config.set_db_pool_size(20);
        assert_eq!(config.db_pool_size(), 20);

        config.set_enable_cron_scheduling(true);
        assert!(config.enable_cron_scheduling());
    }

    #[test]
    fn test_to_dict() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let config = PyDefaultRunnerConfig::default();
            let dict_obj = config.to_dict(py).unwrap();
            let dict = dict_obj.downcast_bound::<pyo3::types::PyDict>(py).unwrap();
            assert!(dict.contains("max_concurrent_tasks").unwrap());
            assert!(dict.contains("db_pool_size").unwrap());
            assert!(dict.contains("enable_cron_scheduling").unwrap());
        });
    }
}
