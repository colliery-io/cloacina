---
title: "cloacina::dal::unified::api_keys"
description: "API key DAL — Postgres only."
editUrl: false
sidebar:
  order: 68
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

API key DAL — Postgres only.

The server uses API keys for authentication. Keys are stored as SHA-256
hashes — plaintext is never persisted. This module provides CRUD operations
for the `api_keys` table.

## Structs

### <span id="struct.ApiKeyInfo"></span><code style="font-size: 0.85em">struct ApiKeyInfo</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Information about an API key (never includes the hash).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `uuid::Uuid` |  |
| `name` | `String` |  |
| `permissions` | `String` |  |
| `created_at` | `chrono::DateTime<chrono::Utc>` |  |
| `revoked` | `bool` |  |
| `tenant_id` | `Option<String>` |  |
| `is_admin` | `bool` |  |
| `expires_at` | `Option<chrono::DateTime<chrono::Utc>>` | CLOACI-T-0792: minted-key expiry (`None` for manual keys = no expiry). |
| `issued_via` | `Option<String>` | CLOACI-T-0792: minted-key provenance (e.g. `oidc:<issuer>:<sub>` / `local:<account_id>`; `None` for manual keys). |

### <span id="struct.ApiKeyDAL"></span><code style="font-size: 0.85em">struct ApiKeyDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

DAL for API key operations. Postgres only.

#### Methods

##### <span id="ApiKeyDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="ApiKeyDAL.create_key"></span><code>create_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create_key(
    &self,
    key_hash: &str,
    name: &str,
    tenant_id: Option<&str>,
    is_admin: bool,
    role: &str,
) -> Result<ApiKeyInfo, ValidationError>
```

Create a new API key record.

<details>
<summary>Source</summary>

```rust
    pub async fn create_key(
        &self,
        key_hash: &str,
        name: &str,
        tenant_id: Option<&str>,
        is_admin: bool,
        role: &str,
    ) -> Result<ApiKeyInfo, ValidationError> {
        crud::create_key(self.dal, key_hash, name, tenant_id, is_admin, role).await
    }
```

</details>

##### <span id="ApiKeyDAL.mint_key"></span><code>mint_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn mint_key(
    &self,
    key_hash: &str,
    name: &str,
    tenant_id: Option<&str>,
    role: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
    issued_via: &str,
) -> Result<ApiKeyInfo, ValidationError>
```

CLOACI-T-0792: mint a short-TTL, provenance-tagged key (never god-mode).

<details>
<summary>Source</summary>

```rust
    pub async fn mint_key(
        &self,
        key_hash: &str,
        name: &str,
        tenant_id: Option<&str>,
        role: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        issued_via: &str,
    ) -> Result<ApiKeyInfo, ValidationError> {
        crud::mint_key(
            self.dal, key_hash, name, tenant_id, role, expires_at, issued_via,
        )
        .await
    }
```

</details>

##### <span id="ApiKeyDAL.validate_hash"></span><code>validate_​hash</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn validate_hash(
    &self,
    key_hash: &str,
) -> Result<Option<ApiKeyInfo>, ValidationError>
```

Validate a key hash — returns key info if found and not revoked.

<details>
<summary>Source</summary>

```rust
    pub async fn validate_hash(
        &self,
        key_hash: &str,
    ) -> Result<Option<ApiKeyInfo>, ValidationError> {
        crud::validate_hash(self.dal, key_hash).await
    }
```

</details>

##### <span id="ApiKeyDAL.has_any_keys"></span><code>has_​any_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn has_any_keys(&self) -> Result<bool, ValidationError>
```

Check if any non-revoked API keys exist.

<details>
<summary>Source</summary>

```rust
    pub async fn has_any_keys(&self) -> Result<bool, ValidationError> {
        crud::has_any_keys(self.dal).await
    }
```

</details>

##### <span id="ApiKeyDAL.list_keys"></span><code>list_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_keys(&self) -> Result<Vec<ApiKeyInfo>, ValidationError>
```

List all API keys (no hashes).

<details>
<summary>Source</summary>

```rust
    pub async fn list_keys(&self) -> Result<Vec<ApiKeyInfo>, ValidationError> {
        crud::list_keys(self.dal).await
    }
```

</details>

##### <span id="ApiKeyDAL.revoke_key"></span><code>revoke_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn revoke_key(
    &self,
    id: uuid::Uuid,
) -> Result<bool, ValidationError>
```

Soft-revoke a key. Returns true if found and revoked.

<details>
<summary>Source</summary>

```rust
    pub async fn revoke_key(&self, id: uuid::Uuid) -> Result<bool, ValidationError> {
        crud::revoke_key(self.dal, id).await
    }
```

</details>

##### <span id="ApiKeyDAL.list_keys_for_tenant"></span><code>list_​keys_​for_​tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_keys_for_tenant(
    &self,
    tenant_id: &str,
) -> Result<Vec<ApiKeyInfo>, ValidationError>
```

CLOACI-T-0784: list keys scoped to a single tenant (tenant-admin view).

<details>
<summary>Source</summary>

```rust
    pub async fn list_keys_for_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<ApiKeyInfo>, ValidationError> {
        crud::list_keys_for_tenant(self.dal, tenant_id).await
    }
```

</details>

##### <span id="ApiKeyDAL.get_key"></span><code>get_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_key(
    &self,
    id: uuid::Uuid,
) -> Result<Option<ApiKeyInfo>, ValidationError>
```

CLOACI-T-0784: fetch one key's info by id (for revoke ownership checks).

<details>
<summary>Source</summary>

```rust
    pub async fn get_key(&self, id: uuid::Uuid) -> Result<Option<ApiKeyInfo>, ValidationError> {
        crud::get_key(self.dal, id).await
    }
```

</details>

##### <span id="ApiKeyDAL.revoke_keys_for_tenant"></span><code>revoke_​keys_​for_​tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn revoke_keys_for_tenant(
    &self,
    tenant_id: &str,
) -> Result<usize, ValidationError>
```

CLOACI-T-0581: bulk-revoke every still-active key bound to the tenant. Returns the number of rows updated.

<details>
<summary>Source</summary>

```rust
    pub async fn revoke_keys_for_tenant(&self, tenant_id: &str) -> Result<usize, ValidationError> {
        crud::revoke_keys_for_tenant(self.dal, tenant_id).await
    }
```

</details>
