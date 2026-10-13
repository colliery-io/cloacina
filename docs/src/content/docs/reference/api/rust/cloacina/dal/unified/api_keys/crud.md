---
title: "cloacina::dal::unified::api_keys::crud"
description: "Postgres CRUD operations for api_keys table."
editUrl: false
sidebar:
  order: 69
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Postgres CRUD operations for api_keys table.

## Functions

### <span id="fn.create_key"></span><code style="font-size: 0.85em">fn create_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn create_key(
    dal: &DAL,
    key_hash: &str,
    name: &str,
    tenant_id: Option<&str>,
    is_admin: bool,
    role: &str,
) -> Result<ApiKeyInfo, ValidationError>
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

### <span id="fn.mint_key"></span><code style="font-size: 0.85em">fn mint_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn mint_key(
    dal: &DAL,
    key_hash: &str,
    name: &str,
    tenant_id: Option<&str>,
    role: &str,
    expires_at: chrono::DateTime<Utc>,
    issued_via: &str,
) -> Result<ApiKeyInfo, ValidationError>
```

CLOACI-T-0792: mint a short-TTL, provenance-tagged key (OIDC / local login). Like `create_key` but sets `expires_at` + `issued_via`; `is_admin` is always false — minted keys are never god-mode. The minted row is an ordinary `api_keys` row that flows through `validate_hash` + the bearer path unchanged (and is rejected once `expires_at` passes).

<details>
<summary>Source</summary>

```rust
pub async fn mint_key(
    dal: &DAL,
    key_hash: &str,
    name: &str,
    tenant_id: Option<&str>,
    role: &str,
    expires_at: chrono::DateTime<Utc>,
    issued_via: &str,
) -> Result<ApiKeyInfo, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let new_key = MintApiKey {
        id: Uuid::new_v4(),
        key_hash: key_hash.to_string(),
        name: name.to_string(),
        permissions: role.to_string(),
        tenant_id: tenant_id.map(|s| s.to_string()),
        is_admin: false,
        expires_at: Some(expires_at.naive_utc()),
        issued_via: Some(issued_via.to_string()),
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

### <span id="fn.validate_hash"></span><code style="font-size: 0.85em">fn validate_​hash</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn validate_hash(
    dal: &DAL,
    key_hash: &str,
) -> Result<Option<ApiKeyInfo>, ValidationError>
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
    // CLOACI-T-0792: reject expired minted keys (NULL expiry = never expires).
    let now = Utc::now().naive_utc();
    let result: Option<ApiKeyRow> = conn
        .interact(move |conn| {
            api_keys::table
                .filter(api_keys::key_hash.eq(&hash))
                .filter(api_keys::revoked_at.is_null())
                .filter(
                    api_keys::expires_at
                        .is_null()
                        .or(api_keys::expires_at.gt(now)),
                )
                .first(conn)
                .optional()
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(result.map(to_info))
}
```

</details>

### <span id="fn.has_any_keys"></span><code style="font-size: 0.85em">fn has_​any_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn has_any_keys(dal: &DAL) -> Result<bool, ValidationError>
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

### <span id="fn.list_keys"></span><code style="font-size: 0.85em">fn list_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn list_keys(
    dal: &DAL,
) -> Result<Vec<ApiKeyInfo>, ValidationError>
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

### <span id="fn.list_keys_for_tenant"></span><code style="font-size: 0.85em">fn list_​keys_​for_​tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn list_keys_for_tenant(
    dal: &DAL,
    tenant_id: &str,
) -> Result<Vec<ApiKeyInfo>, ValidationError>
```

CLOACI-T-0784: list keys scoped to a single tenant (tenant-admin view). Includes revoked rows (like `list_keys`); the caller renders the `revoked` flag.

<details>
<summary>Source</summary>

```rust
pub async fn list_keys_for_tenant(
    dal: &DAL,
    tenant_id: &str,
) -> Result<Vec<ApiKeyInfo>, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let tenant_owned = tenant_id.to_string();
    let results: Vec<ApiKeyRow> = conn
        .interact(move |conn| {
            api_keys::table
                .filter(api_keys::tenant_id.eq(Some(tenant_owned)))
                .order(api_keys::created_at.desc())
                .load(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(results.into_iter().map(to_info).collect())
}
```

</details>

### <span id="fn.get_key"></span><code style="font-size: 0.85em">fn get_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn get_key(
    dal: &DAL,
    id: Uuid,
) -> Result<Option<ApiKeyInfo>, ValidationError>
```

CLOACI-T-0784: fetch a single key's info by id (used by the tenant-scoped revoke path to verify the target key belongs to the caller's tenant before revoking). Returns `None` if no such key exists.

<details>
<summary>Source</summary>

```rust
pub async fn get_key(dal: &DAL, id: Uuid) -> Result<Option<ApiKeyInfo>, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let result: Option<ApiKeyRow> = conn
        .interact(move |conn| api_keys::table.find(id).first(conn).optional())
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(result.map(to_info))
}
```

</details>

### <span id="fn.revoke_keys_for_tenant"></span><code style="font-size: 0.85em">fn revoke_​keys_​for_​tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn revoke_keys_for_tenant(
    dal: &DAL,
    tenant_id: &str,
) -> Result<usize, ValidationError>
```

CLOACI-T-0581: bulk-revoke every still-active key bound to `tenant_id`. Returns the number of rows updated. Used by tenant teardown to close out the auth surface before the schema is dropped.

<details>
<summary>Source</summary>

```rust
pub async fn revoke_keys_for_tenant(dal: &DAL, tenant_id: &str) -> Result<usize, ValidationError> {
    let conn = dal
        .database
        .get_postgres_connection()
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

    let now = Utc::now().naive_utc();
    let tenant_owned = tenant_id.to_string();
    let rows: usize = conn
        .interact(move |conn| {
            diesel::update(
                api_keys::table
                    .filter(api_keys::tenant_id.eq(Some(tenant_owned)))
                    .filter(api_keys::revoked_at.is_null()),
            )
            .set(api_keys::revoked_at.eq(Some(now)))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

    Ok(rows)
}
```

</details>

### <span id="fn.revoke_key"></span><code style="font-size: 0.85em">fn revoke_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn revoke_key(
    dal: &DAL,
    id: Uuid,
) -> Result<bool, ValidationError>
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
