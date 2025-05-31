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

use cloacina::Context;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use serde_json::Value;

/// Convert a Python object to JSON Value for Context storage
pub fn python_to_json(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.is_none() {
        Ok(Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(Value::Number(i.into()))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(Value::Number(
            serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)),
        ))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(Value::String(s))
    } else if obj.is_instance_of::<pyo3::types::PyList>() {
        let list = obj.downcast::<pyo3::types::PyList>()?;
        let mut vec = Vec::new();
        for item in list {
            vec.push(python_to_json(&item)?);
        }
        Ok(Value::Array(vec))
    } else if obj.is_instance_of::<PyDict>() {
        let dict = obj.downcast::<PyDict>()?;
        let mut map = serde_json::Map::new();
        for (key, value) in dict {
            let key_str = key.extract::<String>()?;
            map.insert(key_str, python_to_json(&value)?);
        }
        Ok(Value::Object(map))
    } else {
        // Try to serialize using str() representation
        let s = obj.str()?.to_string();
        Ok(Value::String(s))
    }
}

/// Convert JSON Value back to Python object
pub fn json_to_python(value: &Value, py: Python<'_>) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.to_object(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Ok(n.to_string().to_object(py))
            }
        }
        Value::String(s) => Ok(s.to_object(py)),
        Value::Array(arr) => {
            let list = pyo3::types::PyList::empty_bound(py);
            for item in arr {
                list.append(json_to_python(item, py)?)?;
            }
            Ok(list.to_object(py))
        }
        Value::Object(map) => {
            let dict = PyDict::new_bound(py);
            for (key, value) in map {
                dict.set_item(key, json_to_python(value, py)?)?;
            }
            Ok(dict.to_object(py))
        }
    }
}

/// Create a new Rust Context from Python dict
pub fn context_from_python(py_dict: &Bound<'_, PyDict>) -> PyResult<Context<serde_json::Value>> {
    let mut context = Context::<serde_json::Value>::new();

    for (key, value) in py_dict {
        let key_str = key.extract::<String>()?;
        let json_value = python_to_json(&value)?;
        context
            .insert(key_str, json_value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    }

    Ok(context)
}

/// Convert Rust Context to Python dict
pub fn context_to_python(
    context: &Context<serde_json::Value>,
    py: Python<'_>,
) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);

    // Use the data() method to access the underlying HashMap
    for (key, value) in context.data() {
        let py_value = json_to_python(value, py)?;
        dict.set_item(key, py_value)?;
    }

    Ok(dict.to_object(py))
}
