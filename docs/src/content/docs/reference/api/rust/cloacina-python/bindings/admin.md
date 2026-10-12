---
title: "cloacina-python::bindings::admin"
description: "Python bindings for database administration functionality."
sidebar:
  order: 21
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Python bindings for database administration functionality.

This module provides Python access to admin operations for managing
multi-tenant PostgreSQL deployments.

## Structs

### <span id="struct.PyTenantConfig"></span>`class TenantConfig`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.bindings.admin.TenantConfig](../../../../cloacina-python/bindings/admin/#class.TenantConfig)

Python wrapper for TenantConfig

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `TenantConfig` |  |

#### Methods

##### <span id="PyTenantConfig.new"></span>`new`

```rust
fn new (schema_name : String , username : String , password : Option < String >) -> Self
```

> **Python API**: [cloacina-python.bindings.admin.TenantConfig.new](../../../../cloacina-python/bindings/admin/#TenantConfig.new)

<details>
<summary>Source</summary>

```rust
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



##### <span id="PyTenantConfig.schema_name"></span>`schema_name`

```rust
fn schema_name (& self) -> String
```

> **Python API**: [cloacina-python.bindings.admin.TenantConfig.schema_name](../../../../cloacina-python/bindings/admin/#TenantConfig.schema_name)

<details>
<summary>Source</summary>

```rust
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }
```

</details>



##### <span id="PyTenantConfig.username"></span>`username`

```rust
fn username (& self) -> String
```

> **Python API**: [cloacina-python.bindings.admin.TenantConfig.username](../../../../cloacina-python/bindings/admin/#TenantConfig.username)

<details>
<summary>Source</summary>

```rust
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }
```

</details>



##### <span id="PyTenantConfig.password"></span>`password`

```rust
fn password (& self) -> String
```

> **Python API**: [cloacina-python.bindings.admin.TenantConfig.password](../../../../cloacina-python/bindings/admin/#TenantConfig.password)

<details>
<summary>Source</summary>

```rust
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }
```

</details>



##### <span id="PyTenantConfig.__repr__"></span>`__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloacina-python.bindings.admin.TenantConfig.__repr__](../../../../cloacina-python/bindings/admin/#TenantConfig.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "TenantConfig(schema_name='{}', username='{}', password='***')",
            self.inner.schema_name, self.inner.username
        )
    }
```

</details>





### <span id="struct.PyTenantCredentials"></span>`class TenantCredentials`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.bindings.admin.TenantCredentials](../../../../cloacina-python/bindings/admin/#class.TenantCredentials)

Python wrapper for TenantCredentials

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `TenantCredentials` |  |

#### Methods

##### <span id="PyTenantCredentials.username"></span>`username`

```rust
fn username (& self) -> String
```

> **Python API**: [cloacina-python.bindings.admin.TenantCredentials.username](../../../../cloacina-python/bindings/admin/#TenantCredentials.username)

<details>
<summary>Source</summary>

```rust
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }
```

</details>



##### <span id="PyTenantCredentials.password"></span>`password`

```rust
fn password (& self) -> String
```

> **Python API**: [cloacina-python.bindings.admin.TenantCredentials.password](../../../../cloacina-python/bindings/admin/#TenantCredentials.password)

<details>
<summary>Source</summary>

```rust
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }
```

</details>



##### <span id="PyTenantCredentials.schema_name"></span>`schema_name`

```rust
fn schema_name (& self) -> String
```

> **Python API**: [cloacina-python.bindings.admin.TenantCredentials.schema_name](../../../../cloacina-python/bindings/admin/#TenantCredentials.schema_name)

<details>
<summary>Source</summary>

```rust
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }
```

</details>



##### <span id="PyTenantCredentials.connection_string"></span>`connection_string`

```rust
fn connection_string (& self) -> String
```

> **Python API**: [cloacina-python.bindings.admin.TenantCredentials.connection_string](../../../../cloacina-python/bindings/admin/#TenantCredentials.connection_string)

<details>
<summary>Source</summary>

```rust
    pub fn connection_string(&self) -> String {
        self.inner.connection_string.clone()
    }
```

</details>



##### <span id="PyTenantCredentials.__repr__"></span>`__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloacina-python.bindings.admin.TenantCredentials.__repr__](../../../../cloacina-python/bindings/admin/#TenantCredentials.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "TenantCredentials(username='{}', schema_name='{}', password='***', connection_string='***')",
            self.inner.username, self.inner.schema_name
        )
    }
```

</details>





### <span id="struct.PyDatabaseAdmin"></span>`class DatabaseAdmin`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.bindings.admin.DatabaseAdmin](../../../../cloacina-python/bindings/admin/#class.DatabaseAdmin)

Python wrapper for DatabaseAdmin

Note: This class is only functional with PostgreSQL databases.
SQLite does not support database schemas or user management.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `DatabaseAdmin` |  |

#### Methods

##### <span id="PyDatabaseAdmin.new"></span>`new`

```rust
fn new (database_url : String) -> PyResult < Self >
```

> **Python API**: [cloacina-python.bindings.admin.DatabaseAdmin.new](../../../../cloacina-python/bindings/admin/#DatabaseAdmin.new)

<details>
<summary>Source</summary>

```rust
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



##### <span id="PyDatabaseAdmin.create_tenant"></span>`create_tenant`

```rust
fn create_tenant (& self , py : Python < '_ > , config : & PyTenantConfig ,) -> PyResult < PyTenantCredentials >
```

> **Python API**: [cloacina-python.bindings.admin.DatabaseAdmin.create_tenant](../../../../cloacina-python/bindings/admin/#DatabaseAdmin.create_tenant)

<details>
<summary>Source</summary>

```rust
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



##### <span id="PyDatabaseAdmin.remove_tenant"></span>`remove_tenant`

```rust
fn remove_tenant (& self , py : Python < '_ > , schema_name : String , username : String ,) -> PyResult < () >
```

> **Python API**: [cloacina-python.bindings.admin.DatabaseAdmin.remove_tenant](../../../../cloacina-python/bindings/admin/#DatabaseAdmin.remove_tenant)

<details>
<summary>Source</summary>

```rust
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



##### <span id="PyDatabaseAdmin.__repr__"></span>`__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloacina-python.bindings.admin.DatabaseAdmin.__repr__](../../../../cloacina-python/bindings/admin/#DatabaseAdmin.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        "DatabaseAdmin()".to_string()
    }
```

</details>





## Functions

### <span id="fn.is_postgres_url"></span>`fn is_postgres_url`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn is_postgres_url (url : & str) -> bool
```

Helper to check if a URL is a PostgreSQL connection string

<details>
<summary>Source</summary>

```rust
fn is_postgres_url(url: &str) -> bool {
    url.starts_with("postgres://") || url.starts_with("postgresql://")
}
```

</details>



