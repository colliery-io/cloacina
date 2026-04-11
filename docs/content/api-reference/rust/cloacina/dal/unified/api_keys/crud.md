# cloacina::dal::unified::api_keys::crud <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Postgres CRUD operations for api_keys table.

## Structs

### `cloacina::dal::unified::api_keys::crud::ApiKeyRow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


**Derives:** `Queryable`, `Debug`

Diesel model for reading api_keys rows.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` |  |
| `key_hash` | `String` |  |
| `name` | `String` |  |
| `permissions` | `String` |  |
| `created_at` | `chrono :: NaiveDateTime` |  |
| `revoked_at` | `Option < chrono :: NaiveDateTime >` |  |
| `tenant_id` | `Option < String >` |  |
| `is_admin` | `bool` |  |



### `cloacina::dal::unified::api_keys::crud::NewApiKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


**Derives:** `Insertable`

Diesel model for inserting api_keys rows.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` |  |
| `key_hash` | `String` |  |
| `name` | `String` |  |
| `permissions` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `is_admin` | `bool` |  |



## Functions

### `cloacina::dal::unified::api_keys::crud::to_info`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn to_info (row : ApiKeyRow) -> ApiKeyInfo
```

<details>
<summary>Source</summary>

```rust
fn to_info(row: ApiKeyRow) -> ApiKeyInfo {
    ApiKeyInfo {
        id: row.id,
        name: row.name,
        permissions: row.permissions,
        created_at: row.created_at.and_utc(),
        revoked: row.revoked_at.is_some(),
        tenant_id: row.tenant_id,
        is_admin: row.is_admin,
    }
}
```

</details>



### `cloacina::dal::unified::api_keys::crud::create_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn create_key (dal : & DAL , key_hash : & str , name : & str , tenant_id : Option < & str > , is_admin : bool , role : & str ,) -> Result < ApiKeyInfo , ValidationError >
```

<details>
<summary>Source</summary>

```rust
pub async fn create_key(
    dal: &DAL,
    key_hash: &str,
    name: &str,
    tenant_id: Option<&str>,
    is_admin: bool,
    role: &str,
) -> Result<ApiKeyInfo, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let id = Uuid::new_v4();
    let new_key = NewApiKey {
        id,
        key_hash: key_hash.to_string(),
        name: name.to_string(),
        permissions: role.to_string(),
        tenant_id: tenant_id.map(|s| s.to_string()),
        is_admin,
    };

    let row: ApiKeyRow = conn
        .interact(move |conn| {
            diesel::insert_into(api_keys::table)
                .values(&new_key)
                .get_result(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(to_info(row))
}
```

</details>



### `cloacina::dal::unified::api_keys::crud::validate_hash`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn validate_hash (dal : & DAL , key_hash : & str ,) -> Result < Option < ApiKeyInfo > , ValidationError >
```

<details>
<summary>Source</summary>

```rust
pub async fn validate_hash(
    dal: &DAL,
    key_hash: &str,
) -> Result<Option<ApiKeyInfo>, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let hash = key_hash.to_string();
    let result: Option<ApiKeyRow> = conn
        .interact(move |conn| {
            api_keys::table
                .filter(api_keys::key_hash.eq(&hash))
                .filter(api_keys::revoked_at.is_null())
                .first(conn)
                .optional()
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(result.map(to_info))
}
```

</details>



### `cloacina::dal::unified::api_keys::crud::has_any_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn has_any_keys (dal : & DAL) -> Result < bool , ValidationError >
```

<details>
<summary>Source</summary>

```rust
pub async fn has_any_keys(dal: &DAL) -> Result<bool, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let result: Option<ApiKeyRow> = conn
        .interact(move |conn| {
            api_keys::table
                .filter(api_keys::revoked_at.is_null())
                .first(conn)
                .optional()
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(result.is_some())
}
```

</details>



### `cloacina::dal::unified::api_keys::crud::list_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn list_keys (dal : & DAL) -> Result < Vec < ApiKeyInfo > , ValidationError >
```

<details>
<summary>Source</summary>

```rust
pub async fn list_keys(dal: &DAL) -> Result<Vec<ApiKeyInfo>, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let results: Vec<ApiKeyRow> = conn
        .interact(move |conn| {
            api_keys::table
                .order(api_keys::created_at.desc())
                .load(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(results.into_iter().map(to_info).collect())
}
```

</details>



### `cloacina::dal::unified::api_keys::crud::revoke_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn revoke_key (dal : & DAL , id : Uuid) -> Result < bool , ValidationError >
```

<details>
<summary>Source</summary>

```rust
pub async fn revoke_key(dal: &DAL, id: Uuid) -> Result<bool, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let now = Utc::now().naive_utc();
    let rows: usize = conn
        .interact(move |conn| {
            diesel::update(
                api_keys::table
                    .find(id)
                    .filter(api_keys::revoked_at.is_null()),
            )
            .set(api_keys::revoked_at.eq(Some(now)))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(rows > 0)
}
```

</details>
