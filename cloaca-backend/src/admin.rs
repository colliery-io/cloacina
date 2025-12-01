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

//! Python bindings for database administration functionality.
//!
//! This module provides Python access to admin operations for managing
//! multi-tenant PostgreSQL deployments.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use cloacina::{AdminError, Database, DatabaseAdmin, TenantConfig, TenantCredentials};

/// Python wrapper for TenantConfig
#[pyclass(name = "TenantConfig")]
pub struct PyTenantConfig {
    pub inner: TenantConfig,
}

#[pymethods]
impl PyTenantConfig {
    #[new]
    pub fn new(schema_name: String, username: String, password: Option<String>) -> Self {
        Self {
            inner: TenantConfig {
                schema_name,
                username,
                password: password.unwrap_or_default(),
            },
        }
    }

    #[getter]
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }

    #[getter]
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }

    #[getter]
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "TenantConfig(schema_name='{}', username='{}', password='***')",
            self.inner.schema_name, self.inner.username
        )
    }
}

/// Python wrapper for TenantCredentials
#[pyclass(name = "TenantCredentials")]
pub struct PyTenantCredentials {
    pub inner: TenantCredentials,
}

#[pymethods]
impl PyTenantCredentials {
    #[getter]
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }

    #[getter]
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }

    #[getter]
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }

    #[getter]
    pub fn connection_string(&self) -> String {
        self.inner.connection_string.clone()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "TenantCredentials(username='{}', schema_name='{}', password='***', connection_string='***')",
            self.inner.username, self.inner.schema_name
        )
    }
}

/// Helper to check if a URL is a PostgreSQL connection string
fn is_postgres_url(url: &str) -> bool {
    url.starts_with("postgres://") || url.starts_with("postgresql://")
}

/// Python wrapper for DatabaseAdmin
///
/// Note: This class is only functional with PostgreSQL databases.
/// SQLite does not support database schemas or user management.
#[pyclass(name = "DatabaseAdmin")]
pub struct PyDatabaseAdmin {
    pub inner: DatabaseAdmin,
}

#[pymethods]
impl PyDatabaseAdmin {
    #[new]
    pub fn new(database_url: String) -> PyResult<Self> {
        // Runtime check for PostgreSQL
        if !is_postgres_url(&database_url) {
            return Err(PyRuntimeError::new_err(
                "DatabaseAdmin requires a PostgreSQL connection. \
                 SQLite does not support database schemas or user management. \
                 Use a PostgreSQL URL like 'postgres://user:pass@host/db'",
            ));
        }

        // Parse the database URL to extract components
        let url = url::Url::parse(&database_url)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid database URL: {}", e)))?;

        let database_name = url.path().trim_start_matches('/');
        if database_name.is_empty() {
            return Err(PyRuntimeError::new_err(
                "Database name is required in URL path",
            ));
        }

        // Build connection string with all components
        let username = url.username();
        let password = url.password().unwrap_or("");
        let host = url.host_str().unwrap_or("localhost");
        let port = url.port().unwrap_or(5432);

        let connection_string = if password.is_empty() {
            format!("{}://{}@{}:{}", url.scheme(), username, host, port)
        } else {
            format!(
                "{}://{}:{}@{}:{}",
                url.scheme(),
                username,
                password,
                host,
                port
            )
        };

        let database = Database::new(&connection_string, database_name, 10);
        let admin = DatabaseAdmin::new(database);
        Ok(Self { inner: admin })
    }

    pub fn create_tenant(&self, config: &PyTenantConfig) -> PyResult<PyTenantCredentials> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        let tenant_config = TenantConfig {
            schema_name: config.inner.schema_name.clone(),
            username: config.inner.username.clone(),
            password: config.inner.password.clone(),
        };

        let credentials = rt
            .block_on(async { self.inner.create_tenant(tenant_config).await })
            .map_err(|e: AdminError| {
                PyRuntimeError::new_err(format!("Failed to create tenant: {}", e))
            })?;

        Ok(PyTenantCredentials { inner: credentials })
    }

    pub fn remove_tenant(&self, schema_name: String, username: String) -> PyResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async { self.inner.remove_tenant(&schema_name, &username).await })
            .map_err(|e: AdminError| {
                PyRuntimeError::new_err(format!("Failed to remove tenant: {}", e))
            })?;

        Ok(())
    }

    pub fn __repr__(&self) -> String {
        "DatabaseAdmin()".to_string()
    }
}
