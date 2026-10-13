---
title: "cloacina::database::universal_types"
description: "Universal type wrappers for cross-database compatibility"
editUrl: false
sidebar:
  order: 102
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Universal type wrappers for cross-database compatibility

This module provides wrapper types that work as domain types, convertible
to/from backend-specific database types. These types are used at the API
boundary and in business logic, while backend-specific models handle
the actual database storage.

**Architecture**

When both postgres and sqlite features are enabled:
- Domain code uses UniversalUuid, UniversalTimestamp, UniversalBool
- PostgreSQL DAL converts to/from uuid::Uuid, NaiveDateTime, bool
- SQLite DAL converts to/from Vec<u8>, String, i32

This avoids conflicting Diesel trait implementations by keeping
Diesel-specific code isolated in backend-specific model modules.

## Structs

### <span id="struct.DbUuid"></span><code style="font-size: 0.85em">struct DbUuid</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Copy`, `diesel::sql_types::SqlType`, `diesel::query_builder::QueryId`

Custom SQL type for UUIDs that works across backends. PostgreSQL: maps to native UUID type SQLite: maps to BLOB (16-byte binary)

### <span id="struct.DbTimestamp"></span><code style="font-size: 0.85em">struct DbTimestamp</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Copy`, `diesel::sql_types::SqlType`, `diesel::query_builder::QueryId`

Custom SQL type for timestamps that works across backends. PostgreSQL: maps to native TIMESTAMP type SQLite: maps to TEXT (RFC3339 string format)

### <span id="struct.DbBool"></span><code style="font-size: 0.85em">struct DbBool</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Copy`, `diesel::sql_types::SqlType`, `diesel::query_builder::QueryId`

Custom SQL type for booleans that works across backends. PostgreSQL: maps to native BOOL type SQLite: maps to INTEGER (0/1)

### <span id="struct.DbBinary"></span><code style="font-size: 0.85em">struct DbBinary</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Copy`, `diesel::sql_types::SqlType`, `diesel::query_builder::QueryId`

Custom SQL type for binary data that works across backends. PostgreSQL: maps to BYTEA SQLite: maps to BLOB

### <span id="struct.UniversalUuid"></span><code style="font-size: 0.85em">struct UniversalUuid</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Copy`, `Hash`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`, `AsExpression`, `FromSqlRow`

Universal UUID wrapper for cross-database compatibility

This is a domain type that wraps uuid::Uuid with Diesel support for
both PostgreSQL (native UUID) and SQLite (BLOB) backends.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `Uuid` |  |

#### Methods

##### <span id="UniversalUuid.new_v4"></span><code>new_​v4</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new_v4() -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }
```

</details>

##### <span id="UniversalUuid.as_uuid"></span><code>as_​uuid</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn as_uuid(&self) -> Uuid
```

<details>
<summary>Source</summary>

```rust
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
```

</details>

##### <span id="UniversalUuid.as_bytes"></span><code>as_​bytes</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn as_bytes(&self) -> &[u8; 16]
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

##### <span id="UniversalUuid.from_bytes"></span><code>from_​bytes</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_bytes(bytes: &[u8]) -> Result<Self, uuid::Error>
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

### <span id="struct.UniversalTimestamp"></span><code style="font-size: 0.85em">struct UniversalTimestamp</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Copy`, `Hash`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`, `AsExpression`, `FromSqlRow`

Universal timestamp wrapper for cross-database compatibility

This is a domain type that wraps DateTime<Utc> with Diesel support for
both PostgreSQL (native TIMESTAMP) and SQLite (TEXT as RFC3339) backends.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `DateTime<Utc>` |  |

#### Methods

##### <span id="UniversalTimestamp.now"></span><code>now</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn now() -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn now() -> Self {
        Self(Utc::now())
    }
```

</details>

##### <span id="UniversalTimestamp.as_datetime"></span><code>as_​datetime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn as_datetime(&self) -> &DateTime<Utc>
```

<details>
<summary>Source</summary>

```rust
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }
```

</details>

##### <span id="UniversalTimestamp.into_inner"></span><code>into_​inner</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn into_inner(self) -> DateTime<Utc>
```

<details>
<summary>Source</summary>

```rust
    pub fn into_inner(self) -> DateTime<Utc> {
        self.0
    }
```

</details>

##### <span id="UniversalTimestamp.to_rfc3339"></span><code>to_​rfc3339</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn to_rfc3339(&self) -> String
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

##### <span id="UniversalTimestamp.from_rfc3339"></span><code>from_​rfc3339</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_rfc3339(s: &str) -> Result<Self, chrono::ParseError>
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

##### <span id="UniversalTimestamp.to_naive"></span><code>to_​naive</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn to_naive(&self) -> chrono::NaiveDateTime
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

##### <span id="UniversalTimestamp.from_naive"></span><code>from_​naive</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_naive(naive: chrono::NaiveDateTime) -> Self
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

### <span id="struct.UniversalBool"></span><code style="font-size: 0.85em">struct UniversalBool</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Copy`, `Hash`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`, `AsExpression`, `FromSqlRow`

Universal boolean wrapper for cross-database compatibility

This is a domain type that wraps bool with Diesel support for
both PostgreSQL (native BOOL) and SQLite (INTEGER 0/1) backends.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `bool` |  |

#### Methods

##### <span id="UniversalBool.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(value: bool) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(value: bool) -> Self {
        Self(value)
    }
```

</details>

##### <span id="UniversalBool.is_true"></span><code>is_​true</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_true(&self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn is_true(&self) -> bool {
        self.0
    }
```

</details>

##### <span id="UniversalBool.is_false"></span><code>is_​false</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_false(&self) -> bool
```

<details>
<summary>Source</summary>

```rust
    pub fn is_false(&self) -> bool {
        !self.0
    }
```

</details>

##### <span id="UniversalBool.to_i32"></span><code>to_​i32</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn to_i32(&self) -> i32
```

Convert to i32 for SQLite INTEGER storage

<details>
<summary>Source</summary>

```rust
    pub fn to_i32(&self) -> i32 {
        if self.0 { 1 } else { 0 }
    }
```

</details>

##### <span id="UniversalBool.from_i32"></span><code>from_​i32</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_i32(value: i32) -> Self
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

### <span id="struct.UniversalBinary"></span><code style="font-size: 0.85em">struct UniversalBinary</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Hash`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`, `AsExpression`, `FromSqlRow`

Universal binary wrapper for cross-database compatibility

This is a domain type that wraps Vec<u8> with Diesel support for
both PostgreSQL (BYTEA) and SQLite (BLOB) backends.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `0` | `Vec<u8>` |  |

#### Methods

##### <span id="UniversalBinary.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(data: Vec<u8>) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }
```

</details>

##### <span id="UniversalBinary.as_slice"></span><code>as_​slice</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn as_slice(&self) -> &[u8]
```

<details>
<summary>Source</summary>

```rust
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
```

</details>

##### <span id="UniversalBinary.into_inner"></span><code>into_​inner</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn into_inner(self) -> Vec<u8>
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

### <span id="fn.current_timestamp"></span><code style="font-size: 0.85em">fn current_​timestamp</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn current_timestamp() -> UniversalTimestamp
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
