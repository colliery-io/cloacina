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
use serde_json;
use pythonize::{pythonize, depythonize};

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
    inner: cloacina::Context<serde_json::Value>,
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
                context.insert(key_str, json_value)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        format!("Failed to insert key: {}", e)
                    ))?;
            }
        }
        
        Ok(PyContext { inner: context })
    }

    /// Gets a value from the context
    ///
    /// # Arguments
    /// * `key` - The key to look up
    ///
    /// # Returns
    /// The value if it exists, None otherwise
    pub fn get(&self, key: &str) -> PyResult<Option<PyObject>> {
        match self.inner.get(key) {
            Some(value) => {
                Python::with_gil(|py| {
                    Ok(Some(pythonize(py, value)?.into()))
                })
            }
            None => Ok(None)
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
        }.map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Failed to set key '{}': {}", key, e)
        ))
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
        self.inner.update(key, json_value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyKeyError, _>(
                format!("Key not found: {}", e)
            ))
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
        self.inner.insert(key, json_value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Key already exists: {}", e)
            ))
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
            Some(value) => {
                Python::with_gil(|py| {
                    Ok(Some(pythonize(py, &value)?.into()))
                })
            }
            None => Ok(None)
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
            }.map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Failed to update from dict: {}", e)
            ))?;
        }
        Ok(())
    }

    /// Serializes the context to a JSON string
    ///
    /// # Returns
    /// JSON string representation of the context
    pub fn to_json(&self) -> PyResult<String> {
        self.inner.to_json()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Failed to serialize to JSON: {}", e)
            ))
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
        let context = cloacina::Context::from_json(json_str.to_string())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Failed to deserialize from JSON: {}", e)
            ))?;
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
        match self.get(key)? {
            Some(value) => Ok(value),
            None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(
                format!("Key not found: '{}'", key)
            ))
        }
    }

    /// Dictionary-style item assignment
    pub fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.set(key, value)
    }

    /// Dictionary-style item deletion
    pub fn __delitem__(&mut self, key: &str) -> PyResult<()> {
        match self.remove(key)? {
            Some(_) => Ok(()),
            None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(
                format!("Key not found: '{}'", key)
            ))
        }
    }
}