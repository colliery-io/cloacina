# cloacina::database::universal_types <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Universal type wrappers for cross-database compatibility

This module provides wrapper types that work as domain types, convertible
to/from backend-specific database types. These types are used at the API
boundary and in business logic, while backend-specific models handle
the actual database storage.

## Structs

### `cloacina::database::universal_types::DbUuid`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Copy`, `diesel :: sql_types :: SqlType`, `diesel :: query_builder :: QueryId`

Custom SQL type for UUIDs that works across backends. PostgreSQL: maps to native UUID type SQLite: maps to BLOB (16-byte binary)



### `cloacina::database::universal_types::DbTimestamp`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Copy`, `diesel :: sql_types :: SqlType`, `diesel :: query_builder :: QueryId`

Custom SQL type for timestamps that works across backends. PostgreSQL: maps to native TIMESTAMP type SQLite: maps to TEXT (RFC3339 string format)



### `cloacina::database::universal_types::DbBool`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Copy`, `diesel :: sql_types :: SqlType`, `diesel :: query_builder :: QueryId`

Custom SQL type for booleans that works across backends. PostgreSQL: maps to native BOOL type SQLite: maps to INTEGER (0/1)



### `cloacina::database::universal_types::DbBinary`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Copy`, `diesel :: sql_types :: SqlType`, `diesel :: query_builder :: QueryId`

Custom SQL type for binary data that works across backends. PostgreSQL: maps to BYTEA SQLite: maps to BLOB



### `cloacina::database::universal_types::UniversalUuid`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Copy`, `Hash`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`, `AsExpression`, `FromSqlRow`, ``

Universal UUID wrapper for cross-database compatibility

This is a domain type that wraps uuid::Uuid with Diesel support for
both PostgreSQL (native UUID) and SQLite (BLOB) backends.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `Uuid` |  |

#### Methods

##### `new_v4` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new_v4 () -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }
```

</details>



##### `as_uuid` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn as_uuid (& self) -> Uuid
```

<details>
<summary>Source</summary>

```rust
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
```

</details>



##### `as_bytes` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn as_bytes (& self) -> & [u8 ; 16]
```

Convert to bytes for SQLite BLOB storage

<details>
<summary>Source</summary>

```rust
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
```

</details>



##### `from_bytes` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_bytes (bytes : & [u8]) -> Result < Self , uuid :: Error >
```

Create from bytes (SQLite BLOB)

<details>
<summary>Source</summary>

```rust
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, uuid::Error> {
        Uuid::from_slice(bytes).map(UniversalUuid)
    }
```

</details>





### `cloacina::database::universal_types::UniversalTimestamp`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Copy`, `Hash`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`, `AsExpression`, `FromSqlRow`, ``

Universal timestamp wrapper for cross-database compatibility

This is a domain type that wraps DateTime<Utc> with Diesel support for
both PostgreSQL (native TIMESTAMP) and SQLite (TEXT as RFC3339) backends.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `DateTime < Utc >` |  |

#### Methods

##### `now` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn now () -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn now() -> Self {
        Self(Utc::now())
    }
```

</details>



##### `as_datetime` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn as_datetime (& self) -> & DateTime < Utc >
```

<details>
<summary>Source</summary>

```rust
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }
```

</details>



##### `into_inner` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn into_inner (self) -> DateTime < Utc >
```

<details>
<summary>Source</summary>

```rust
    pub fn into_inner(self) -> DateTime<Utc> {
        self.0
    }
```

</details>



##### `to_rfc3339` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn to_rfc3339 (& self) -> String
```

Convert to RFC3339 string for SQLite TEXT storage

<details>
<summary>Source</summary>

```rust
    pub fn to_rfc3339(&self) -> String {
        self.0.to_rfc3339()
    }
```

</details>



##### `from_rfc3339` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_rfc3339 (s : & str) -> Result < Self , chrono :: ParseError >
```

Create from RFC3339 string (SQLite TEXT)

<details>
<summary>Source</summary>

```rust
    pub fn from_rfc3339(s: &str) -> Result<Self, chrono::ParseError> {
        DateTime::parse_from_rfc3339(s).map(|dt| UniversalTimestamp(dt.with_timezone(&Utc)))
    }
```

</details>



##### `to_naive` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn to_naive (& self) -> chrono :: NaiveDateTime
```

Convert to NaiveDateTime for PostgreSQL TIMESTAMP storage

<details>
<summary>Source</summary>

```rust
    pub fn to_naive(&self) -> chrono::NaiveDateTime {
        self.0.naive_utc()
    }
```

</details>



##### `from_naive` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_naive (naive : chrono :: NaiveDateTime) -> Self
```

Create from NaiveDateTime (PostgreSQL TIMESTAMP)

<details>
<summary>Source</summary>

```rust
    pub fn from_naive(naive: chrono::NaiveDateTime) -> Self {
        use chrono::TimeZone;
        UniversalTimestamp(Utc.from_utc_datetime(&naive))
    }
```

</details>





### `cloacina::database::universal_types::UniversalBool`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Copy`, `Hash`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`, `AsExpression`, `FromSqlRow`, ``

Universal boolean wrapper for cross-database compatibility

This is a domain type that wraps bool with Diesel support for
both PostgreSQL (native BOOL) and SQLite (INTEGER 0/1) backends.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `bool` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (value : bool) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(value: bool) -> Self {
        Self(value)
    }
```

</details>



##### `is_true` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_true (& self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn is_true(&self) -> bool {
        self.0
    }
```

</details>



##### `is_false` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_false (& self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn is_false(&self) -> bool {
        !self.0
    }
```

</details>



##### `to_i32` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn to_i32 (& self) -> i32
```

Convert to i32 for SQLite INTEGER storage

<details>
<summary>Source</summary>

```rust
    pub fn to_i32(&self) -> i32 {
        if self.0 {
            1
        } else {
            0
        }
    }
```

</details>



##### `from_i32` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_i32 (value : i32) -> Self
```

Create from i32 (SQLite INTEGER)

<details>
<summary>Source</summary>

```rust
    pub fn from_i32(value: i32) -> Self {
        Self(value != 0)
    }
```

</details>





### `cloacina::database::universal_types::UniversalBinary`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Hash`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`, `AsExpression`, `FromSqlRow`

Universal binary wrapper for cross-database compatibility

This is a domain type that wraps Vec<u8> with Diesel support for
both PostgreSQL (BYTEA) and SQLite (BLOB) backends.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `Vec < u8 >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (data : Vec < u8 >) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }
```

</details>



##### `as_slice` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn as_slice (& self) -> & [u8]
```

<details>
<summary>Source</summary>

```rust
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
```

</details>



##### `into_inner` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn into_inner (self) -> Vec < u8 >
```

<details>
<summary>Source</summary>

```rust
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
```

</details>





## Functions

### `cloacina::database::universal_types::current_timestamp`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn current_timestamp () -> UniversalTimestamp
```

Helper function for current timestamp

<details>
<summary>Source</summary>

```rust
pub fn current_timestamp() -> UniversalTimestamp {
    UniversalTimestamp::now()
}
```

</details>
