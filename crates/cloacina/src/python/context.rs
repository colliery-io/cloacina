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

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pythonize::{depythonize, pythonize};
use serde_json;

/// PyContext - Python wrapper for Rust Context<serde_json::Value>
#[pyclass(name = "Context")]
#[derive(Debug)]
pub struct PyContext {
    pub inner: crate::Context<serde_json::Value>,
}

#[pymethods]
impl PyContext {
    /// Creates a new empty context
    #[new]
    #[pyo3(signature = (data = None))]
    pub fn new(data: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut context = crate::Context::new();

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
    pub fn set(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;

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
    pub fn update(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;
        self.inner.update(key, json_value).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!("Key not found: {}", e))
        })
    }

    /// Inserts a new value into the context
    pub fn insert(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;
        self.inner.insert(key, json_value).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Key already exists: {}", e))
        })
    }

    /// Removes and returns a value from the context
    pub fn remove(&mut self, key: &str) -> PyResult<Option<PyObject>> {
        match self.inner.remove(key) {
            Some(value) => Python::with_gil(|py| Ok(Some(pythonize(py, &value)?.into()))),
            None => Ok(None),
        }
    }

    /// Returns the context as a Python dictionary
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize(py, self.inner.data())?.into())
    }

    /// Updates the context with values from a Python dictionary
    pub fn update_from_dict(&mut self, data: &Bound<'_, PyDict>) -> PyResult<()> {
        for (key, value) in data.iter() {
            let key_str: String = key.extract()?;
            let json_value: serde_json::Value = depythonize(&value)?;

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
    pub fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to serialize to JSON: {}",
                e
            ))
        })
    }

    /// Creates a context from a JSON string
    #[staticmethod]
    pub fn from_json(json_str: &str) -> PyResult<Self> {
        let context = crate::Context::from_json(json_str.to_string()).map_err(|e| {
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
    pub fn from_rust_context(context: crate::Context<serde_json::Value>) -> Self {
        PyContext { inner: context }
    }

    /// Extract the inner Rust Context (for internal use)
    pub fn into_inner(self) -> crate::Context<serde_json::Value> {
        self.inner
    }

    /// Clone the inner Rust Context (for internal use)
    pub fn clone_inner(&self) -> crate::Context<serde_json::Value> {
        self.inner.clone_data()
    }

    /// Get a clone of the context data as a HashMap (for internal use)
    pub fn get_data_clone(&self) -> std::collections::HashMap<String, serde_json::Value> {
        self.inner.data().clone()
    }
}

/// Manual implementation of Clone since Context<T> doesn't implement Clone
impl Clone for PyContext {
    fn clone(&self) -> Self {
        let data = self.inner.data();
        let mut new_context = crate::Context::new();
        for (key, value) in data.iter() {
            new_context.insert(key.clone(), value.clone()).unwrap();
        }
        PyContext { inner: new_context }
    }
}
