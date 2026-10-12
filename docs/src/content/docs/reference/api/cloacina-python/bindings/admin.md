---
title: "cloacina-python.bindings.admin"
description: "Python bindings for database administration functionality."
sidebar:
  order: 1
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


Python bindings for database administration functionality.

This module provides Python access to admin operations for managing
multi-tenant PostgreSQL deployments.

## Classes

### <span id="class.TenantConfig"></span>`class TenantConfig`

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantConfig](../../../rust/cloacina-python/bindings/admin/#struct.PyTenantConfig)

Python wrapper for TenantConfig

#### Methods

##### <span id="TenantConfig.new"></span>`new`

```python
def new(schema_name: str, username: str, password: Option < String >) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantConfig::new](../../../rust/cloacina-python/bindings/admin/#PyTenantConfig.new)

<details>
<summary>Source</summary>

```python
    pub fn new(schema_name: String, username: String, password: Option<String>) -> Self {
        Self {
            inner: TenantConfig {
                schema_name,
                username,
                password: password.unwrap_or_default(),
            },
        }
    }
```

</details>



##### <span id="TenantConfig.schema_name"></span>`schema_name`

```python
def schema_name() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantConfig::schema_name](../../../rust/cloacina-python/bindings/admin/#PyTenantConfig.schema_name)

<details>
<summary>Source</summary>

```python
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }
```

</details>



##### <span id="TenantConfig.username"></span>`username`

```python
def username() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantConfig::username](../../../rust/cloacina-python/bindings/admin/#PyTenantConfig.username)

<details>
<summary>Source</summary>

```python
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }
```

</details>



##### <span id="TenantConfig.password"></span>`password`

```python
def password() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantConfig::password](../../../rust/cloacina-python/bindings/admin/#PyTenantConfig.password)

<details>
<summary>Source</summary>

```python
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }
```

</details>



##### <span id="TenantConfig.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantConfig::__repr__](../../../rust/cloacina-python/bindings/admin/#PyTenantConfig.__repr__)

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        format!(
            "TenantConfig(schema_name='{}', username='{}', password='***')",
            self.inner.schema_name, self.inner.username
        )
    }
```

</details>





### <span id="class.TenantCredentials"></span>`class TenantCredentials`

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantCredentials](../../../rust/cloacina-python/bindings/admin/#struct.PyTenantCredentials)

Python wrapper for TenantCredentials

#### Methods

##### <span id="TenantCredentials.username"></span>`username`

```python
def username() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantCredentials::username](../../../rust/cloacina-python/bindings/admin/#PyTenantCredentials.username)

<details>
<summary>Source</summary>

```python
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }
```

</details>



##### <span id="TenantCredentials.password"></span>`password`

```python
def password() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantCredentials::password](../../../rust/cloacina-python/bindings/admin/#PyTenantCredentials.password)

<details>
<summary>Source</summary>

```python
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }
```

</details>



##### <span id="TenantCredentials.schema_name"></span>`schema_name`

```python
def schema_name() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantCredentials::schema_name](../../../rust/cloacina-python/bindings/admin/#PyTenantCredentials.schema_name)

<details>
<summary>Source</summary>

```python
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }
```

</details>



##### <span id="TenantCredentials.connection_string"></span>`connection_string`

```python
def connection_string() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantCredentials::connection_string](../../../rust/cloacina-python/bindings/admin/#PyTenantCredentials.connection_string)

<details>
<summary>Source</summary>

```python
    pub fn connection_string(&self) -> String {
        self.inner.connection_string.clone()
    }
```

</details>



##### <span id="TenantCredentials.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyTenantCredentials::__repr__](../../../rust/cloacina-python/bindings/admin/#PyTenantCredentials.__repr__)

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        format!(
            "TenantCredentials(username='{}', schema_name='{}', password='***', connection_string='***')",
            self.inner.username, self.inner.schema_name
        )
    }
```

</details>





### <span id="class.DatabaseAdmin"></span>`class DatabaseAdmin`

> **Rust Implementation**: [cloacina-python::bindings::admin::PyDatabaseAdmin](../../../rust/cloacina-python/bindings/admin/#struct.PyDatabaseAdmin)

Python wrapper for DatabaseAdmin

Note: This class is only functional with PostgreSQL databases.
SQLite does not support database schemas or user management.

#### Methods

##### <span id="DatabaseAdmin.new"></span>`new`

```python
def new(database_url: str) -> PyResult < Self >
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyDatabaseAdmin::new](../../../rust/cloacina-python/bindings/admin/#PyDatabaseAdmin.new)

<details>
<summary>Source</summary>

```python
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
```

</details>



##### <span id="DatabaseAdmin.create_tenant"></span>`create_tenant`

```python
def create_tenant(config:  PyTenantConfig) -> PyResult < PyTenantCredentials >
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyDatabaseAdmin::create_tenant](../../../rust/cloacina-python/bindings/admin/#PyDatabaseAdmin.create_tenant)

<details>
<summary>Source</summary>

```python
    pub fn create_tenant(
        &self,
        py: Python<'_>,
        config: &PyTenantConfig,
    ) -> PyResult<PyTenantCredentials> {
        let tenant_config = TenantConfig {
            schema_name: config.inner.schema_name.clone(),
            username: config.inner.username.clone(),
            password: config.inner.password.clone(),
        };

        // CLOACI-I-0136: drive the async admin call through the one GIL-safe
        // bridge (releases the GIL before blocking) — previously this held the
        // GIL across the await on a fresh runtime.
        let credentials =
            crate::gil::py_block_on(py, async { self.inner.create_tenant(tenant_config).await })
                .map_err(|e: AdminError| {
                    PyRuntimeError::new_err(format!("Failed to create tenant: {}", e))
                })?;

        Ok(PyTenantCredentials { inner: credentials })
    }
```

</details>



##### <span id="DatabaseAdmin.remove_tenant"></span>`remove_tenant`

```python
def remove_tenant(schema_name: str, username: str) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyDatabaseAdmin::remove_tenant](../../../rust/cloacina-python/bindings/admin/#PyDatabaseAdmin.remove_tenant)

<details>
<summary>Source</summary>

```python
    pub fn remove_tenant(
        &self,
        py: Python<'_>,
        schema_name: String,
        username: String,
    ) -> PyResult<()> {
        // CLOACI-I-0136: GIL-safe bridge (see create_tenant).
        crate::gil::py_block_on(py, async {
            self.inner.remove_tenant(&schema_name, &username).await
        })
        .map_err(|e: AdminError| {
            PyRuntimeError::new_err(format!("Failed to remove tenant: {}", e))
        })?;

        Ok(())
    }
```

</details>



##### <span id="DatabaseAdmin.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::admin::PyDatabaseAdmin::__repr__](../../../rust/cloacina-python/bindings/admin/#PyDatabaseAdmin.__repr__)

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        "DatabaseAdmin()".to_string()
    }
```

</details>





