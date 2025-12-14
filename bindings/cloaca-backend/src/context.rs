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

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::{depythonize, pythonize};
use serde_json;

/// PyContext - Python wrapper for Rust Context<serde_json::Value>
///
/// This class provides a Python interface to the Rust Context type, with methods
/// that exactly match the Rust interface for seamless integration.
///
/// # Examples
/// ```python
/// ctx = Context({"user_id": 123})
/// ctx.set("result", "processed")
/// value = ctx.get("user_id")
/// ctx.update({"more": "data"})
/// data_dict = ctx.to_dict()
/// ```
#[pyclass(name = "Context")]
#[derive(Debug)]
pub struct PyContext {
    pub(crate) inner: cloacina::Context<serde_json::Value>,
}

#[pymethods]
impl PyContext {
    /// Creates a new empty context
    ///
    /// # Arguments
    /// * `data` - Optional dictionary to initialize the context with
    ///
    /// # Returns
    /// A new PyContext instance
    #[new]
    #[pyo3(signature = (data = None))]
    pub fn new(data: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut context = cloacina::Context::new();

        if let Some(dict) = data {
            for (key, value) in dict.iter() {
                let key_str: String = key.extract()?;
                let json_value: serde_json::Value = depythonize(&value)?;
                context.insert(key_str, json_value).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Failed to insert key: {}",
                        e
                    ))
                })?;
            }
        }

        Ok(PyContext { inner: context })
    }

    /// Gets a value from the context
    ///
    /// # Arguments
    /// * `key` - The key to look up
    /// * `default` - Optional default value to return if key doesn't exist
    ///
    /// # Returns
    /// The value if it exists, default value if provided, None otherwise
    #[pyo3(signature = (key, default = None))]
    pub fn get(&self, key: &str, default: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        match self.inner.get(key) {
            Some(value) => Python::with_gil(|py| Ok(pythonize(py, value)?.into())),
            None => match default {
                Some(default_value) => Ok(default_value.clone().into()),
                None => Python::with_gil(|py| Ok(py.None())),
            },
        }
    }

    /// Sets a value in the context (insert or update)
    ///
    /// # Arguments
    /// * `key` - The key to set
    /// * `value` - The value to store
    ///
    /// # Note
    /// This method will insert if key doesn't exist, or update if it does.
    /// This matches Python dict behavior and is more convenient than separate insert/update.
    pub fn set(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;

        // Check if key exists and use appropriate method
        if self.inner.get(key).is_some() {
            self.inner.update(key, json_value)
        } else {
            self.inner.insert(key, json_value)
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to set key '{}': {}",
                key, e
            ))
        })
    }

    /// Updates an existing value in the context
    ///
    /// # Arguments
    /// * `key` - The key to update
    /// * `value` - The new value
    ///
    /// # Raises
    /// KeyError if the key doesn't exist
    pub fn update(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;
        self.inner.update(key, json_value).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!("Key not found: {}", e))
        })
    }

    /// Inserts a new value into the context
    ///
    /// # Arguments
    /// * `key` - The key to insert
    /// * `value` - The value to store
    ///
    /// # Raises
    /// ValueError if the key already exists
    pub fn insert(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;
        self.inner.insert(key, json_value).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Key already exists: {}", e))
        })
    }

    /// Removes and returns a value from the context
    ///
    /// # Arguments
    /// * `key` - The key to remove
    ///
    /// # Returns
    /// The removed value if it existed, None otherwise
    pub fn remove(&mut self, key: &str) -> PyResult<Option<PyObject>> {
        match self.inner.remove(key) {
            Some(value) => Python::with_gil(|py| Ok(Some(pythonize(py, &value)?.into()))),
            None => Ok(None),
        }
    }

    /// Returns the context as a Python dictionary
    ///
    /// # Returns
    /// A new Python dict containing all context data
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize(py, self.inner.data())?.into())
    }

    /// Updates the context with values from a Python dictionary
    ///
    /// # Arguments
    /// * `data` - Dictionary containing key-value pairs to merge
    pub fn update_from_dict(&mut self, data: &Bound<'_, PyDict>) -> PyResult<()> {
        for (key, value) in data.iter() {
            let key_str: String = key.extract()?;
            let json_value: serde_json::Value = depythonize(&value)?;

            // Use set behavior (insert or update)
            if self.inner.get(&key_str).is_some() {
                self.inner.update(key_str, json_value)
            } else {
                self.inner.insert(key_str, json_value)
            }
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to update from dict: {}",
                    e
                ))
            })?;
        }
        Ok(())
    }

    /// Serializes the context to a JSON string
    ///
    /// # Returns
    /// JSON string representation of the context
    pub fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to serialize to JSON: {}",
                e
            ))
        })
    }

    /// Creates a context from a JSON string
    ///
    /// # Arguments
    /// * `json_str` - JSON string to deserialize
    ///
    /// # Returns
    /// A new PyContext instance
    #[staticmethod]
    pub fn from_json(json_str: &str) -> PyResult<Self> {
        let context = cloacina::Context::from_json(json_str.to_string()).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to deserialize from JSON: {}",
                e
            ))
        })?;
        Ok(PyContext { inner: context })
    }

    /// Returns the number of key-value pairs in the context
    pub fn __len__(&self) -> usize {
        self.inner.data().len()
    }

    /// Checks if a key exists in the context
    pub fn __contains__(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }

    /// String representation of the context
    pub fn __repr__(&self) -> String {
        match self.inner.to_json() {
            Ok(json) => format!("Context({})", json),
            Err(_) => "Context(<serialization error>)".to_string(),
        }
    }

    /// Dictionary-style item access
    pub fn __getitem__(&self, key: &str) -> PyResult<PyObject> {
        let result = self.get(key, None)?;
        Python::with_gil(|py| {
            if result.is_none(py) {
                Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                    "Key not found: '{}'",
                    key
                )))
            } else {
                Ok(result)
            }
        })
    }

    /// Dictionary-style item assignment
    pub fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.set(key, value)
    }

    /// Dictionary-style item deletion
    pub fn __delitem__(&mut self, key: &str) -> PyResult<()> {
        match self.remove(key)? {
            Some(_) => Ok(()),
            None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Key not found: '{}'",
                key
            ))),
        }
    }
}

impl PyContext {
    /// Create a PyContext from a Rust Context (for internal use)
    pub(crate) fn from_rust_context(context: cloacina::Context<serde_json::Value>) -> Self {
        PyContext { inner: context }
    }

    /// Extract the inner Rust Context (for internal use)
    pub(crate) fn into_inner(self) -> cloacina::Context<serde_json::Value> {
        self.inner
    }

    /// Clone the inner Rust Context (for internal use)
    pub(crate) fn clone_inner(&self) -> cloacina::Context<serde_json::Value> {
        self.inner.clone_data()
    }

    /// Get a clone of the context data as a HashMap (for internal use)
    pub(crate) fn get_data_clone(&self) -> std::collections::HashMap<String, serde_json::Value> {
        self.inner.data().clone()
    }
}

/// Manual implementation of Clone since Context<T> doesn't implement Clone
/// We recreate the context from its data
impl Clone for PyContext {
    fn clone(&self) -> Self {
        // Get the data from the inner context
        let data = self.inner.data();

        // Create a new context and populate it
        let mut new_context = cloacina::Context::new();
        for (key, value) in data.iter() {
            // This should never fail since we're cloning existing valid data
            new_context.insert(key.clone(), value.clone()).unwrap();
        }

        PyContext { inner: new_context }
    }
}

/// PyDefaultRunnerConfig - Python wrapper for Rust DefaultRunnerConfig
///
/// This class provides a Python interface to the Rust DefaultRunnerConfig type,
/// with all the same fields and default values for configuring the DefaultRunner.
///
/// # Examples
/// ```python
/// # Use defaults
/// config = DefaultRunnerConfig()
///
/// # Custom configuration
/// config = DefaultRunnerConfig(
///     max_concurrent_tasks=8,
///     enable_cron_scheduling=False,
///     task_timeout_seconds=600
/// )
/// ```
#[pyclass(name = "DefaultRunnerConfig")]
#[derive(Debug, Clone)]
pub struct PyDefaultRunnerConfig {
    inner: cloacina::runner::DefaultRunnerConfig,
}

#[pymethods]
impl PyDefaultRunnerConfig {
    /// Creates a new DefaultRunnerConfig with customizable parameters
    ///
    /// All parameters are optional and will use sensible defaults if not provided.
    /// Durations are specified in convenient units (seconds, milliseconds) rather than Rust Duration objects.
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

        let mut config = cloacina::runner::DefaultRunnerConfig::default();

        // Apply any provided overrides
        if let Some(val) = max_concurrent_tasks {
            config.max_concurrent_tasks = val;
        }
        if let Some(val) = scheduler_poll_interval_ms {
            config.scheduler_poll_interval = Duration::from_millis(val);
        }
        if let Some(val) = task_timeout_seconds {
            config.task_timeout = Duration::from_secs(val);
        }
        if let Some(val) = pipeline_timeout_seconds {
            config.pipeline_timeout = Some(Duration::from_secs(val));
        }
        if let Some(val) = db_pool_size {
            config.db_pool_size = val;
        }
        if let Some(val) = enable_recovery {
            config.enable_recovery = val;
        }
        if let Some(val) = enable_cron_scheduling {
            config.enable_cron_scheduling = val;
        }
        if let Some(val) = cron_poll_interval_seconds {
            config.cron_poll_interval = Duration::from_secs(val);
        }
        if let Some(val) = cron_max_catchup_executions {
            config.cron_max_catchup_executions = val;
        }
        if let Some(val) = cron_enable_recovery {
            config.cron_enable_recovery = val;
        }
        if let Some(val) = cron_recovery_interval_seconds {
            config.cron_recovery_interval = Duration::from_secs(val);
        }
        if let Some(val) = cron_lost_threshold_minutes {
            config.cron_lost_threshold_minutes = val;
        }
        if let Some(val) = cron_max_recovery_age_seconds {
            config.cron_max_recovery_age = Duration::from_secs(val);
        }
        if let Some(val) = cron_max_recovery_attempts {
            config.cron_max_recovery_attempts = val;
        }

        PyDefaultRunnerConfig { inner: config }
    }

    /// Creates a DefaultRunnerConfig with all default values
    ///
    /// # Returns
    /// A new PyDefaultRunnerConfig instance with all default values
    #[staticmethod]
    pub fn default() -> Self {
        PyDefaultRunnerConfig {
            inner: cloacina::runner::DefaultRunnerConfig::default(),
        }
    }

    // Getters for all fields

    #[getter]
    pub fn max_concurrent_tasks(&self) -> usize {
        self.inner.max_concurrent_tasks
    }

    #[getter]
    pub fn scheduler_poll_interval_ms(&self) -> u64 {
        self.inner.scheduler_poll_interval.as_millis() as u64
    }

    #[getter]
    pub fn task_timeout_seconds(&self) -> u64 {
        self.inner.task_timeout.as_secs()
    }

    #[getter]
    pub fn pipeline_timeout_seconds(&self) -> Option<u64> {
        self.inner.pipeline_timeout.map(|d| d.as_secs())
    }

    #[getter]
    pub fn db_pool_size(&self) -> u32 {
        self.inner.db_pool_size
    }

    #[getter]
    pub fn enable_recovery(&self) -> bool {
        self.inner.enable_recovery
    }

    #[getter]
    pub fn enable_cron_scheduling(&self) -> bool {
        self.inner.enable_cron_scheduling
    }

    #[getter]
    pub fn cron_poll_interval_seconds(&self) -> u64 {
        self.inner.cron_poll_interval.as_secs()
    }

    #[getter]
    pub fn cron_max_catchup_executions(&self) -> usize {
        self.inner.cron_max_catchup_executions
    }

    #[getter]
    pub fn cron_enable_recovery(&self) -> bool {
        self.inner.cron_enable_recovery
    }

    #[getter]
    pub fn cron_recovery_interval_seconds(&self) -> u64 {
        self.inner.cron_recovery_interval.as_secs()
    }

    #[getter]
    pub fn cron_lost_threshold_minutes(&self) -> i32 {
        self.inner.cron_lost_threshold_minutes
    }

    #[getter]
    pub fn cron_max_recovery_age_seconds(&self) -> u64 {
        self.inner.cron_max_recovery_age.as_secs()
    }

    #[getter]
    pub fn cron_max_recovery_attempts(&self) -> usize {
        self.inner.cron_max_recovery_attempts
    }

    // Setters for all fields

    #[setter]
    pub fn set_max_concurrent_tasks(&mut self, value: usize) {
        self.inner.max_concurrent_tasks = value;
    }

    #[setter]
    pub fn set_scheduler_poll_interval_ms(&mut self, value: u64) {
        self.inner.scheduler_poll_interval = std::time::Duration::from_millis(value);
    }

    #[setter]
    pub fn set_task_timeout_seconds(&mut self, value: u64) {
        self.inner.task_timeout = std::time::Duration::from_secs(value);
    }

    #[setter]
    pub fn set_pipeline_timeout_seconds(&mut self, value: Option<u64>) {
        self.inner.pipeline_timeout = value.map(std::time::Duration::from_secs);
    }

    #[setter]
    pub fn set_db_pool_size(&mut self, value: u32) {
        self.inner.db_pool_size = value;
    }

    #[setter]
    pub fn set_enable_recovery(&mut self, value: bool) {
        self.inner.enable_recovery = value;
    }

    #[setter]
    pub fn set_enable_cron_scheduling(&mut self, value: bool) {
        self.inner.enable_cron_scheduling = value;
    }

    #[setter]
    pub fn set_cron_poll_interval_seconds(&mut self, value: u64) {
        self.inner.cron_poll_interval = std::time::Duration::from_secs(value);
    }

    #[setter]
    pub fn set_cron_max_catchup_executions(&mut self, value: usize) {
        self.inner.cron_max_catchup_executions = value;
    }

    #[setter]
    pub fn set_cron_enable_recovery(&mut self, value: bool) {
        self.inner.cron_enable_recovery = value;
    }

    #[setter]
    pub fn set_cron_recovery_interval_seconds(&mut self, value: u64) {
        self.inner.cron_recovery_interval = std::time::Duration::from_secs(value);
    }

    #[setter]
    pub fn set_cron_lost_threshold_minutes(&mut self, value: i32) {
        self.inner.cron_lost_threshold_minutes = value;
    }

    #[setter]
    pub fn set_cron_max_recovery_age_seconds(&mut self, value: u64) {
        self.inner.cron_max_recovery_age = std::time::Duration::from_secs(value);
    }

    #[setter]
    pub fn set_cron_max_recovery_attempts(&mut self, value: usize) {
        self.inner.cron_max_recovery_attempts = value;
    }

    /// Returns a dictionary representation of the configuration
    ///
    /// # Returns
    /// A Python dict containing all configuration values with friendly names
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new(py);

        dict.set_item("max_concurrent_tasks", self.inner.max_concurrent_tasks)?;
        dict.set_item(
            "scheduler_poll_interval_ms",
            self.inner.scheduler_poll_interval.as_millis(),
        )?;
        dict.set_item("task_timeout_seconds", self.inner.task_timeout.as_secs())?;
        dict.set_item(
            "pipeline_timeout_seconds",
            self.inner.pipeline_timeout.map(|d| d.as_secs()),
        )?;
        dict.set_item("db_pool_size", self.inner.db_pool_size)?;
        dict.set_item("enable_recovery", self.inner.enable_recovery)?;
        dict.set_item("enable_cron_scheduling", self.inner.enable_cron_scheduling)?;
        dict.set_item(
            "cron_poll_interval_seconds",
            self.inner.cron_poll_interval.as_secs(),
        )?;
        dict.set_item(
            "cron_max_catchup_executions",
            self.inner.cron_max_catchup_executions,
        )?;
        dict.set_item("cron_enable_recovery", self.inner.cron_enable_recovery)?;
        dict.set_item(
            "cron_recovery_interval_seconds",
            self.inner.cron_recovery_interval.as_secs(),
        )?;
        dict.set_item(
            "cron_lost_threshold_minutes",
            self.inner.cron_lost_threshold_minutes,
        )?;
        dict.set_item(
            "cron_max_recovery_age_seconds",
            self.inner.cron_max_recovery_age.as_secs(),
        )?;
        dict.set_item(
            "cron_max_recovery_attempts",
            self.inner.cron_max_recovery_attempts,
        )?;

        Ok(dict.into())
    }

    /// String representation of the configuration
    pub fn __repr__(&self) -> String {
        format!(
            "DefaultRunnerConfig(max_concurrent_tasks={}, enable_cron_scheduling={}, db_pool_size={})",
            self.inner.max_concurrent_tasks,
            self.inner.enable_cron_scheduling,
            self.inner.db_pool_size
        )
    }
}

impl PyDefaultRunnerConfig {
    /// Get the inner Rust config (for internal use)
    pub(crate) fn to_rust_config(&self) -> cloacina::runner::DefaultRunnerConfig {
        self.inner.clone()
    }
}
