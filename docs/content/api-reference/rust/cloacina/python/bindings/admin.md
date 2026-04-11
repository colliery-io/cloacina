# cloacina::python::bindings::admin <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Python bindings for database administration functionality.

This module provides Python access to admin operations for managing
multi-tenant PostgreSQL deployments.

## Structs

### `cloacina::python::bindings::admin::TenantConfig`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.bindings.admin.TenantConfig](../../../../cloaca/python/bindings/admin.md#class-tenantconfig)

Python wrapper for TenantConfig

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `TenantConfig` |  |

#### Methods

##### `new`

```rust
fn new (schema_name : String , username : String , password : Option < String >) -> Self
```

> **Python API**: [cloaca.python.bindings.admin.TenantConfig.new](../../../../cloaca/python/bindings/admin.md#new)

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



##### `schema_name`

```rust
fn schema_name (& self) -> String
```

> **Python API**: [cloaca.python.bindings.admin.TenantConfig.schema_name](../../../../cloaca/python/bindings/admin.md#schema_name)

<details>
<summary>Source</summary>

```rust
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }
```

</details>



##### `username`

```rust
fn username (& self) -> String
```

> **Python API**: [cloaca.python.bindings.admin.TenantConfig.username](../../../../cloaca/python/bindings/admin.md#username)

<details>
<summary>Source</summary>

```rust
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }
```

</details>



##### `password`

```rust
fn password (& self) -> String
```

> **Python API**: [cloaca.python.bindings.admin.TenantConfig.password](../../../../cloaca/python/bindings/admin.md#password)

<details>
<summary>Source</summary>

```rust
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }
```

</details>



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.bindings.admin.TenantConfig.__repr__](../../../../cloaca/python/bindings/admin.md#__repr__)

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





### `cloacina::python::bindings::admin::TenantCredentials`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.bindings.admin.TenantCredentials](../../../../cloaca/python/bindings/admin.md#class-tenantcredentials)

Python wrapper for TenantCredentials

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `TenantCredentials` |  |

#### Methods

##### `username`

```rust
fn username (& self) -> String
```

> **Python API**: [cloaca.python.bindings.admin.TenantCredentials.username](../../../../cloaca/python/bindings/admin.md#username)

<details>
<summary>Source</summary>

```rust
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }
```

</details>



##### `password`

```rust
fn password (& self) -> String
```

> **Python API**: [cloaca.python.bindings.admin.TenantCredentials.password](../../../../cloaca/python/bindings/admin.md#password)

<details>
<summary>Source</summary>

```rust
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }
```

</details>



##### `schema_name`

```rust
fn schema_name (& self) -> String
```

> **Python API**: [cloaca.python.bindings.admin.TenantCredentials.schema_name](../../../../cloaca/python/bindings/admin.md#schema_name)

<details>
<summary>Source</summary>

```rust
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }
```

</details>



##### `connection_string`

```rust
fn connection_string (& self) -> String
```

> **Python API**: [cloaca.python.bindings.admin.TenantCredentials.connection_string](../../../../cloaca/python/bindings/admin.md#connection_string)

<details>
<summary>Source</summary>

```rust
    pub fn connection_string(&self) -> String {
        self.inner.connection_string.clone()
    }
```

</details>



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.bindings.admin.TenantCredentials.__repr__](../../../../cloaca/python/bindings/admin.md#__repr__)

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





### `cloacina::python::bindings::admin::DatabaseAdmin`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.bindings.admin.DatabaseAdmin](../../../../cloaca/python/bindings/admin.md#class-databaseadmin)

Python wrapper for DatabaseAdmin

Note: This class is only functional with PostgreSQL databases.
SQLite does not support database schemas or user management.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `DatabaseAdmin` |  |

#### Methods

##### `new`

```rust
fn new (database_url : String) -> PyResult < Self >
```

> **Python API**: [cloaca.python.bindings.admin.DatabaseAdmin.new](../../../../cloaca/python/bindings/admin.md#new)

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



##### `create_tenant`

```rust
fn create_tenant (& self , config : & PyTenantConfig) -> PyResult < PyTenantCredentials >
```

> **Python API**: [cloaca.python.bindings.admin.DatabaseAdmin.create_tenant](../../../../cloaca/python/bindings/admin.md#create_tenant)

<details>
<summary>Source</summary>

```rust
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

```rust
fn remove_tenant (& self , schema_name : String , username : String) -> PyResult < () >
```

> **Python API**: [cloaca.python.bindings.admin.DatabaseAdmin.remove_tenant](../../../../cloaca/python/bindings/admin.md#remove_tenant)

<details>
<summary>Source</summary>

```rust
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

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.bindings.admin.DatabaseAdmin.__repr__](../../../../cloaca/python/bindings/admin.md#__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        "DatabaseAdmin()".to_string()
    }
```

</details>





## Functions

### `cloacina::python::bindings::admin::is_postgres_url`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


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
