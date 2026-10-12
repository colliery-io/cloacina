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
    pub inner: cloacina::Context<serde_json::Value>,
}

#[pymethods]
impl PyContext {
    /// Creates a new empty context
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

    /// Read a resolved secret by name, returning a dict of its `{field: value}`
    /// pairs (CLOACI-I-0133 / T-0864 — the Python read accessor).
    ///
    /// Calls through to the SAME Rust [`cloacina::Context::secret`] accessor and
    /// the SAME `SecretResolver` the runtime attached to this context at
    /// execution time, so it is alias-aware through the instance's `{"$secret"}`
    /// map (T-0859): `name` may be either the declared local binding name or the
    /// concrete secret name.
    ///
    /// No-leak: the resolved value is returned to the caller ONLY. It is never
    /// written back into the context's serialized `data`, so it cannot reach
    /// `schedules.params`, the fires log, or execution history.
    ///
    /// Raises `RuntimeError` when no resolver is configured on this execution
    /// scope (`CLOACINA_SECRET_KEK` unset) or the backend fails, `KeyError` when
    /// the secret is not found, `PermissionError` when the name is not granted.
    pub fn secret(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        let fields = crate::gil::py_block_on(py, self.inner.secret(name))
            .map_err(secret_access_error_to_pyerr)?;
        let dict = PyDict::new(py);
        for (field, value) in fields {
            dict.set_item(field, value)?;
        }
        Ok(dict.into())
    }

    /// Read a single field of a named secret (convenience over [`Self::secret`]).
    ///
    /// Same resolver, alias-awareness, no-leak, and error mapping as
    /// [`Self::secret`]; additionally raises `KeyError` when the secret exists but
    /// has no field of that name.
    pub fn secret_field(&self, py: Python<'_>, name: &str, field: &str) -> PyResult<String> {
        crate::gil::py_block_on(py, self.inner.secret_field(name, field))
            .map_err(secret_access_error_to_pyerr)
    }
}

/// Map a Rust [`cloacina::SecretAccessError`] onto a clear Python exception.
///
/// `NotConfigured`/`Backend` → `RuntimeError` (environment/backend faults),
/// `NotFound`/`FieldNotFound` → `KeyError` (a missing name/field), `NotGranted`
/// → `PermissionError` (the grant gate denied it). The `Display` message is
/// non-plaintext by construction (it names the secret/field, never a value).
fn secret_access_error_to_pyerr(err: cloacina::SecretAccessError) -> PyErr {
    use cloacina::SecretAccessError as E;
    let msg = err.to_string();
    match err {
        E::NotConfigured | E::Backend(_) => pyo3::exceptions::PyRuntimeError::new_err(msg),
        E::NotFound(_) | E::FieldNotFound { .. } => pyo3::exceptions::PyKeyError::new_err(msg),
        E::NotGranted(_) => pyo3::exceptions::PyPermissionError::new_err(msg),
    }
}

impl PyContext {
    /// Create a PyContext from a Rust Context (for internal use)
    pub fn from_rust_context(context: cloacina::Context<serde_json::Value>) -> Self {
        PyContext { inner: context }
    }

    /// Extract the inner Rust Context (for internal use)
    pub fn into_inner(self) -> cloacina::Context<serde_json::Value> {
        self.inner
    }

    /// Clone the inner Rust Context (for internal use)
    pub fn clone_inner(&self) -> cloacina::Context<serde_json::Value> {
        self.inner.clone_data()
    }

    /// Get a clone of the context data as a HashMap (for internal use)
    pub fn get_data_clone(&self) -> std::collections::HashMap<String, serde_json::Value> {
        self.inner.data().clone()
    }
}

/// Manual implementation of Clone since Context<T> doesn't implement Clone.
///
/// Delegates to `Context::clone_data`, which clones the data map AND carries the
/// (Arc) secret-resolver handle. This preservation is load-bearing (CLOACI-T-0864):
/// the executor attaches the resolver to the Rust `Context` before running a
/// Python body, and the body is handed `py_context.clone()` (see
/// `PythonTaskWrapper::execute`). A rebuild via `Context::new()` here would drop
/// the resolver, so `context.secret(...)` inside the task body would fail with
/// `NotConfigured` even though the runtime configured one.
impl Clone for PyContext {
    fn clone(&self) -> Self {
        PyContext {
            inner: self.inner.clone_data(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::PyDict;

    #[test]
    fn test_new_empty() {
        pyo3::prepare_freethreaded_python();
        let ctx = PyContext::new(None).unwrap();
        assert_eq!(ctx.__len__(), 0);
    }

    #[test]
    fn test_new_from_dict() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let dict = PyDict::new(py);
            dict.set_item("key", "value").unwrap();
            let ctx = PyContext::new(Some(&dict.as_borrowed())).unwrap();
            assert_eq!(ctx.__len__(), 1);
            assert!(ctx.__contains__("key"));
        });
    }

    #[test]
    fn test_set_and_get() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut ctx = PyContext::new(None).unwrap();
            let val = 42i64.into_pyobject(py).unwrap();
            ctx.set("answer", &val.as_borrowed()).unwrap();

            let result = ctx.get("answer", None).unwrap();
            let extracted: i64 = result.extract(py).unwrap();
            assert_eq!(extracted, 42);
        });
    }

    #[test]
    fn test_insert_new_key() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut ctx = PyContext::new(None).unwrap();
            let val = "hello".into_pyobject(py).unwrap();
            ctx.insert("greeting", &val.as_borrowed()).unwrap();
            assert!(ctx.__contains__("greeting"));
        });
    }

    #[test]
    fn test_insert_duplicate_errors() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut ctx = PyContext::new(None).unwrap();
            let val = "hello".into_pyobject(py).unwrap();
            ctx.insert("key", &val.as_borrowed()).unwrap();
            let val2 = "world".into_pyobject(py).unwrap();
            assert!(ctx.insert("key", &val2.as_borrowed()).is_err());
        });
    }

    #[test]
    fn test_update_existing_key() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut ctx = PyContext::new(None).unwrap();
            let val = "hello".into_pyobject(py).unwrap();
            ctx.insert("key", &val.as_borrowed()).unwrap();
            let val2 = "world".into_pyobject(py).unwrap();
            ctx.update("key", &val2.as_borrowed()).unwrap();

            let result = ctx.get("key", None).unwrap();
            let extracted: String = result.extract(py).unwrap();
            assert_eq!(extracted, "world");
        });
    }

    #[test]
    fn test_update_missing_key_errors() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut ctx = PyContext::new(None).unwrap();
            let val = "hello".into_pyobject(py).unwrap();
            assert!(ctx.update("missing", &val.as_borrowed()).is_err());
        });
    }

    #[test]
    fn test_remove_existing() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut ctx = PyContext::new(None).unwrap();
            let val = 99i64.into_pyobject(py).unwrap();
            ctx.insert("num", &val.as_borrowed()).unwrap();

            let removed = ctx.remove("num").unwrap();
            assert!(removed.is_some());
            assert_eq!(ctx.__len__(), 0);
        });
    }

    #[test]
    fn test_remove_missing_returns_none() {
        pyo3::prepare_freethreaded_python();
        let mut ctx = PyContext::new(None).unwrap();
        let removed = ctx.remove("nope").unwrap();
        assert!(removed.is_none());
    }

    #[test]
    fn test_len_and_contains() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut ctx = PyContext::new(None).unwrap();
            assert_eq!(ctx.__len__(), 0);
            assert!(!ctx.__contains__("a"));

            let val = 1i64.into_pyobject(py).unwrap();
            ctx.insert("a", &val.as_borrowed()).unwrap();
            assert_eq!(ctx.__len__(), 1);
            assert!(ctx.__contains__("a"));
        });
    }

    #[test]
    fn test_to_json_and_from_json() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut ctx = PyContext::new(None).unwrap();
            let val = "bar".into_pyobject(py).unwrap();
            ctx.insert("foo", &val.as_borrowed()).unwrap();

            let json = ctx.to_json().unwrap();
            let ctx2 = PyContext::from_json(&json).unwrap();
            assert_eq!(ctx2.__len__(), 1);
            assert!(ctx2.__contains__("foo"));

            let result = ctx2.get("foo", None).unwrap();
            let extracted: String = result.extract(py).unwrap();
            assert_eq!(extracted, "bar");
        });
    }

    #[test]
    fn test_to_dict() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut ctx = PyContext::new(None).unwrap();
            let val = 42i64.into_pyobject(py).unwrap();
            ctx.insert("x", &val.as_borrowed()).unwrap();

            let dict_obj = ctx.to_dict(py).unwrap();
            let dict = dict_obj.downcast_bound::<PyDict>(py).unwrap();
            let x_val: i64 = dict.get_item("x").unwrap().unwrap().extract().unwrap();
            assert_eq!(x_val, 42);
        });
    }

    #[test]
    fn test_repr() {
        pyo3::prepare_freethreaded_python();
        let ctx = PyContext::new(None).unwrap();
        let repr = ctx.__repr__();
        assert!(repr.starts_with("Context("));
    }

    #[test]
    fn test_from_rust_context_and_clone_inner() {
        pyo3::prepare_freethreaded_python();
        let mut rust_ctx = cloacina::Context::new();
        rust_ctx
            .insert("k".to_string(), serde_json::json!("v"))
            .unwrap();
        let py_ctx = PyContext::from_rust_context(rust_ctx);
        assert!(py_ctx.__contains__("k"));

        let cloned = py_ctx.clone_inner();
        assert_eq!(cloned.get("k"), Some(&serde_json::json!("v")));
    }

    #[test]
    fn test_clone_preserves_data() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let mut ctx = PyContext::new(None).unwrap();
            let val = "hello".into_pyobject(py).unwrap();
            ctx.insert("key", &val.as_borrowed()).unwrap();

            let cloned = ctx.clone();
            assert_eq!(cloned.__len__(), 1);
            assert!(cloned.__contains__("key"));
        });
    }

    // ── Secret read accessor (CLOACI-I-0133 / T-0864) ───────────────────────
    //
    // These drive the Python `secret`/`secret_field` bindings through the SAME
    // Rust `Context::secret` accessor a task body would, using an in-memory test
    // `SecretResolver` (no DB/KEK), and assert read parity, the no-leak
    // guarantee, alias-awareness, and the Python exception mapping.

    use async_trait::async_trait;
    use cloacina::{SecretResolver, SecretResolverError};
    use std::collections::{BTreeMap, HashMap};
    use std::sync::Arc;

    /// In-memory resolver: returns preloaded fields, `NotFound` otherwise.
    struct MapResolver {
        secrets: HashMap<String, BTreeMap<String, String>>,
    }

    #[async_trait]
    impl SecretResolver for MapResolver {
        async fn resolve(
            &self,
            name: &str,
        ) -> Result<BTreeMap<String, String>, SecretResolverError> {
            self.secrets
                .get(name)
                .cloned()
                .ok_or_else(|| SecretResolverError::NotFound(name.to_string()))
        }
    }

    fn ctx_with_secret(name: &str, fields: &[(&str, &str)]) -> PyContext {
        let mut field_map = BTreeMap::new();
        for (k, v) in fields {
            field_map.insert(k.to_string(), v.to_string());
        }
        let mut secrets = HashMap::new();
        secrets.insert(name.to_string(), field_map);
        let resolver: Arc<dyn SecretResolver> = Arc::new(MapResolver { secrets });
        let rust_ctx = cloacina::Context::<serde_json::Value>::new().with_secret_resolver(resolver);
        PyContext::from_rust_context(rust_ctx)
    }

    #[test]
    fn test_secret_returns_resolved_fields() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let ctx = ctx_with_secret(
                "db_prod",
                &[("password", "hunter2"), ("host", "db.internal")],
            );

            let result = ctx.secret(py, "db_prod").unwrap();
            let dict = result.downcast_bound::<PyDict>(py).unwrap();
            let pw: String = dict
                .get_item("password")
                .unwrap()
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(pw, "hunter2");
            let host: String = dict.get_item("host").unwrap().unwrap().extract().unwrap();
            assert_eq!(host, "db.internal");
        });
    }

    #[test]
    fn test_secret_field_returns_single_value() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let ctx = ctx_with_secret("db_prod", &[("password", "hunter2")]);
            let pw = ctx.secret_field(py, "db_prod", "password").unwrap();
            assert_eq!(pw, "hunter2");
        });
    }

    #[test]
    fn test_secret_is_alias_aware_through_secret_refs_map() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            // Build a context whose data carries the T-0859 `{"$secret"}` alias
            // map (name→name, no values) mapping local binding `creds` → `db_prod`.
            let mut field_map = BTreeMap::new();
            field_map.insert("token".to_string(), "abc123".to_string());
            let mut secrets = HashMap::new();
            secrets.insert("db_prod".to_string(), field_map);
            let resolver: Arc<dyn SecretResolver> = Arc::new(MapResolver { secrets });

            let mut rust_ctx =
                cloacina::Context::<serde_json::Value>::new().with_secret_resolver(resolver);
            // The alias map key is the crate-internal SECRET_REFS_KEY constant.
            rust_ctx
                .insert(
                    "__cloacina_secret_refs__".to_string(),
                    serde_json::json!({"creds": "db_prod"}),
                )
                .unwrap();
            let ctx = PyContext::from_rust_context(rust_ctx);

            // Read by the LOCAL binding name; alias map routes it to `db_prod`.
            let token = ctx.secret_field(py, "creds", "token").unwrap();
            assert_eq!(token, "abc123");
        });
    }

    #[test]
    fn test_secret_value_not_leaked_into_serialized_data() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let ctx = ctx_with_secret(
                "db_prod",
                &[("password", "hunter2"), ("host", "db.internal")],
            );

            // Read the secret, then confirm no plaintext entered the context.
            let _ = ctx.secret(py, "db_prod").unwrap();
            let _ = ctx.secret_field(py, "db_prod", "password").unwrap();

            let json = ctx.to_json().unwrap();
            assert!(!json.contains("hunter2"), "secret leaked into json: {json}");
            assert!(
                !json.contains("db.internal"),
                "secret leaked into json: {json}"
            );
            // Nothing was stashed back into the context data at all.
            assert_eq!(ctx.__len__(), 0);
        });
    }

    #[test]
    fn test_secret_not_configured_raises_runtime_error() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            // A context with NO resolver attached.
            let ctx = PyContext::new(None).unwrap();
            let err = ctx.secret(py, "db_prod").unwrap_err();
            assert!(err.is_instance_of::<pyo3::exceptions::PyRuntimeError>(py));
        });
    }

    #[test]
    fn test_secret_not_found_raises_key_error() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let ctx = ctx_with_secret("db_prod", &[("password", "hunter2")]);
            let err = ctx.secret(py, "does_not_exist").unwrap_err();
            assert!(err.is_instance_of::<pyo3::exceptions::PyKeyError>(py));
        });
    }

    #[test]
    fn test_secret_field_missing_field_raises_key_error() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let ctx = ctx_with_secret("db_prod", &[("password", "hunter2")]);
            let err = ctx
                .secret_field(py, "db_prod", "no_such_field")
                .unwrap_err();
            assert!(err.is_instance_of::<pyo3::exceptions::PyKeyError>(py));
        });
    }

    /// The Python task body is handed `py_context.clone()` (see
    /// `PythonTaskWrapper::execute`), so `clone()` MUST carry the resolver — a
    /// rebuild would make `context.secret(...)` fail with NotConfigured in real
    /// execution even though the runtime configured a resolver. This exercises
    /// exactly that path.
    #[test]
    fn test_clone_preserves_secret_resolver() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let ctx = ctx_with_secret("db_prod", &[("password", "hunter2")]);
            let cloned = ctx.clone();
            assert!(
                cloned.inner.has_secret_resolver(),
                "clone dropped the secret resolver handle"
            );
            let pw = cloned.secret_field(py, "db_prod", "password").unwrap();
            assert_eq!(pw, "hunter2");
        });
    }
}
