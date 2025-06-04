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
use serde_json::Value;
use std::collections::HashMap;

/// Python-friendly context wrapper for Cloacina Context
#[pyclass]
pub struct PyContext {
    data: HashMap<String, Value>,
}

#[pymethods]
impl PyContext {
    #[new]
    fn new(data: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut context_data = HashMap::new();

        if let Some(dict) = data {
            Python::with_gil(|py| {
                for item in dict.iter() {
                    let (key, value) = item;
                    let key_str: String = key.extract()?;
                    let json_value: Value = depythonize(&value)?;
                    context_data.insert(key_str, json_value);
                }
                Ok::<(), PyErr>(())
            })?;
        }

        Ok(PyContext { data: context_data })
    }

    fn get(&self, key: &str, default: Option<PyObject>) -> PyResult<PyObject> {
        Python::with_gil(|py| match self.data.get(key) {
            Some(value) => {
                let py_value = pythonize(py, value)?;
                Ok(py_value.into())
            }
            None => Ok(default.unwrap_or_else(|| py.None())),
        })
    }

    fn set(&mut self, key: &str, value: PyObject) -> PyResult<()> {
        Python::with_gil(|py| {
            let json_value: Value = depythonize(&value.as_ref(py))?;
            self.data.insert(key.to_string(), json_value);
            Ok(())
        })
    }

    fn update(&mut self, data: &Bound<'_, PyDict>) -> PyResult<()> {
        for item in data.iter() {
            let (key, value) = item;
            let key_str: String = key.extract()?;
            let json_value: Value = depythonize(&value)?;
            self.data.insert(key_str, json_value);
        }
        Ok(())
    }

    fn remove(&mut self, key: &str) -> PyResult<Option<PyObject>> {
        Python::with_gil(|py| match self.data.remove(key) {
            Some(value) => {
                let py_value = pythonize(py, &value)?;
                Ok(Some(py_value.into()))
            }
            None => Ok(None),
        })
    }

    fn exists(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    fn to_dict(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            for (key, value) in &self.data {
                let py_value = pythonize(py, value)?;
                dict.set_item(key, py_value)?;
            }
            Ok(dict.into())
        })
    }
}

impl PyContext {
    /// Convert to Cloacina's internal Context type
    pub fn to_cloacina_context(&self) -> cloacina::Context<Value> {
        cloacina::Context::new(self.data.clone())
    }

    /// Create from Cloacina's internal Context type
    pub fn from_cloacina_context(context: cloacina::Context<Value>) -> Self {
        PyContext {
            data: context.into_data(),
        }
    }
}
