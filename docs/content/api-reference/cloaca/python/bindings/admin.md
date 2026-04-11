# cloaca.python.bindings.admin <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


Python bindings for database administration functionality.

This module provides Python access to admin operations for managing
multi-tenant PostgreSQL deployments.

## Classes

### `cloaca.python.bindings.admin.TenantConfig`

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantConfig](../../../rust/cloacina/python/bindings/admin.md#class-tenantconfig)

Python wrapper for TenantConfig

#### Methods

##### `new`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">new</span>(schema_name: str, username: str, password: Optional[str]) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantConfig::new](../../../rust/cloacina/python/bindings/admin.md#new)

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



##### `schema_name`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">schema_name</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantConfig::schema_name](../../../rust/cloacina/python/bindings/admin.md#schema_name)

<details>
<summary>Source</summary>

```python
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }
```

</details>



##### `username`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">username</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantConfig::username](../../../rust/cloacina/python/bindings/admin.md#username)

<details>
<summary>Source</summary>

```python
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }
```

</details>



##### `password`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">password</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantConfig::password](../../../rust/cloacina/python/bindings/admin.md#password)

<details>
<summary>Source</summary>

```python
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantConfig::__repr__](../../../rust/cloacina/python/bindings/admin.md#__repr__)

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





### `cloaca.python.bindings.admin.TenantCredentials`

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantCredentials](../../../rust/cloacina/python/bindings/admin.md#class-tenantcredentials)

Python wrapper for TenantCredentials

#### Methods

##### `username`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">username</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantCredentials::username](../../../rust/cloacina/python/bindings/admin.md#username)

<details>
<summary>Source</summary>

```python
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }
```

</details>



##### `password`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">password</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantCredentials::password](../../../rust/cloacina/python/bindings/admin.md#password)

<details>
<summary>Source</summary>

```python
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }
```

</details>



##### `schema_name`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">schema_name</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantCredentials::schema_name](../../../rust/cloacina/python/bindings/admin.md#schema_name)

<details>
<summary>Source</summary>

```python
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }
```

</details>



##### `connection_string`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">connection_string</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantCredentials::connection_string](../../../rust/cloacina/python/bindings/admin.md#connection_string)

<details>
<summary>Source</summary>

```python
    pub fn connection_string(&self) -> String {
        self.inner.connection_string.clone()
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyTenantCredentials::__repr__](../../../rust/cloacina/python/bindings/admin.md#__repr__)

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





### `cloaca.python.bindings.admin.DatabaseAdmin`

> **Rust Implementation**: [cloacina::python::bindings::admin::PyDatabaseAdmin](../../../rust/cloacina/python/bindings/admin.md#class-databaseadmin)

Python wrapper for DatabaseAdmin

Note: This class is only functional with PostgreSQL databases.
SQLite does not support database schemas or user management.

#### Methods

##### `new`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">new</span>(database_url: str) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyDatabaseAdmin::new](../../../rust/cloacina/python/bindings/admin.md#new)

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



##### `create_tenant`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">create_tenant</span>(config: PyTenantConfig) -> <span style="color: var(--md-default-fg-color--light);">PyTenantCredentials</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyDatabaseAdmin::create_tenant](../../../rust/cloacina/python/bindings/admin.md#create_tenant)

<details>
<summary>Source</summary>

```python
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
```

</details>



##### `remove_tenant`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">remove_tenant</span>(schema_name: str, username: str) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyDatabaseAdmin::remove_tenant](../../../rust/cloacina/python/bindings/admin.md#remove_tenant)

<details>
<summary>Source</summary>

```python
    pub fn remove_tenant(&self, schema_name: String, username: String) -> PyResult<()> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        rt.block_on(async { self.inner.remove_tenant(&schema_name, &username).await })
            .map_err(|e: AdminError| {
                PyRuntimeError::new_err(format!("Failed to remove tenant: {}", e))
            })?;

        Ok(())
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::bindings::admin::PyDatabaseAdmin::__repr__](../../../rust/cloacina/python/bindings/admin.md#__repr__)

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        "DatabaseAdmin()".to_string()
    }
```

</details>
