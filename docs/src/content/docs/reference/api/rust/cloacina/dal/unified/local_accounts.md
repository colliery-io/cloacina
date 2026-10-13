---
title: "cloacina::dal::unified::local_accounts"
description: "Local accounts — self-managed username/password login (CLOACI-T-0795)."
editUrl: false
sidebar:
  order: 75
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Local accounts — self-managed username/password login (CLOACI-T-0795).

**Postgres only.**

The minimal credential entity behind "IdP or self-manage": `username +
argon2id password hash + tenant + role + active/disabled status`. The
account record IS the identity→tenant/role mapping, so local login bypasses
the OIDC allowlist. Password plaintext is never stored — only the argon2id
PHC string. Local login (T-0796) verifies a password and resolves the
account to a `ResolvedPrincipal`; account management (T-0797) is tenant-admin.

## Structs

### <span id="struct.LocalAccount"></span><code style="font-size: 0.85em">struct LocalAccount</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Public view of a local account (never includes the password hash).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `uuid::Uuid` |  |
| `username` | `String` |  |
| `tenant_id` | `Option<String>` |  |
| `role` | `String` |  |
| `status` | `String` |  |

#### Methods

##### <span id="LocalAccount.is_active"></span><code>is_​active</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_active(&self) -> bool
```

True when the account may log in.

<details>
<summary>Source</summary>

```rust
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }
```

</details>

### <span id="struct.LocalAccountDAL"></span><code style="font-size: 0.85em">struct LocalAccountDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

DAL for local accounts. Postgres only.

#### Methods

##### <span id="LocalAccountDAL.new"></span><code>new</code>

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

##### <span id="LocalAccountDAL.create"></span><code>create</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create(
    &self,
    username: &str,
    password: &str,
    tenant_id: Option<&str>,
    role: &str,
) -> Result<LocalAccount, ValidationError>
```

Create a local account (tenant-admin op, T-0797). Hashes the password with argon2id. `role` is within the tenant; `status` starts `active`.

<details>
<summary>Source</summary>

```rust
    pub async fn create(
        &self,
        username: &str,
        password: &str,
        tenant_id: Option<&str>,
        role: &str,
    ) -> Result<LocalAccount, ValidationError> {
        let password_hash = hash_password(password)?;
        let row = NewLocalAccount {
            id: Uuid::new_v4(),
            username: username.to_string(),
            password_hash,
            tenant_id: tenant_id.map(|s| s.to_string()),
            role: role.to_string(),
            status: "active".to_string(),
        };
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let inserted: LocalAccountRow = conn
            .interact(move |conn| {
                diesel::insert_into(crate::database::schema::postgres::local_accounts::table)
                    .values(&row)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(to_account(&inserted))
    }
```

</details>

##### <span id="LocalAccountDAL.authenticate"></span><code>authenticate</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn authenticate(
    &self,
    username: &str,
    password: &str,
    tenant_id: Option<&str>,
) -> Result<LoginOutcome, ValidationError>
```

Authenticate a login. Verifies the password against the account in `tenant_id` (constant work whether or not the user exists, to limit enumeration). Returns [`LoginOutcome::Denied`](../../../../../rust/cloacina/dal/unified/local_accounts/#enum.LoginOutcome) for unknown/wrong/disabled.

<details>
<summary>Source</summary>

```rust
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
        tenant_id: Option<&str>,
    ) -> Result<LoginOutcome, ValidationError> {
        use crate::database::schema::postgres::local_accounts;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let uname = username.to_string();
        let tenant = tenant_id.map(|s| s.to_string());
        let row: Option<LocalAccountRow> = conn
            .interact(move |conn| {
                let mut q = local_accounts::table
                    .filter(local_accounts::username.eq(uname))
                    .into_boxed();
                q = match tenant {
                    Some(t) => q.filter(local_accounts::tenant_id.eq(Some(t))),
                    None => q.filter(local_accounts::tenant_id.is_null()),
                };
                q.first(conn).optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        match row {
            Some(r) if verify_password(password, &r.password_hash) && r.status == "active" => {
                Ok(LoginOutcome::Ok(to_account(&r)))
            }
            Some(_) => Ok(LoginOutcome::Denied),
            None => {
                // CLOACI-T-0923: an unknown username used to return without
                // hashing anything, so "no such user" answered in microseconds
                // while "wrong password" paid the full argon2id cost — a clean
                // timing oracle for user enumeration that no amount of
                // uniform-error-body care could close. Burn an equivalent
                // verification against a fixed decoy hash so both paths cost
                // the same argon2 work.
                let _ = verify_password(password, decoy_password_hash());
                Ok(LoginOutcome::Denied)
            }
        }
    }
```

</details>

##### <span id="LocalAccountDAL.get_by_id"></span><code>get_​by_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_by_id(
    &self,
    id: Uuid,
) -> Result<Option<LocalAccount>, ValidationError>
```

Fetch an account by id (any status). Used by `/auth/refresh` (T-0794) to re-check the account is still `active` before re-minting a key.

<details>
<summary>Source</summary>

```rust
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<LocalAccount>, ValidationError> {
        use crate::database::schema::postgres::local_accounts;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let row: Option<LocalAccountRow> = conn
            .interact(move |conn| local_accounts::table.find(id).first(conn).optional())
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row.as_ref().map(to_account))
    }
```

</details>

##### <span id="LocalAccountDAL.list_for_tenant"></span><code>list_​for_​tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_for_tenant(
    &self,
    tenant_id: Option<&str>,
) -> Result<Vec<LocalAccount>, ValidationError>
```

List accounts in a tenant (no hashes). Tenant-admin view (T-0797).

<details>
<summary>Source</summary>

```rust
    pub async fn list_for_tenant(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<Vec<LocalAccount>, ValidationError> {
        use crate::database::schema::postgres::local_accounts;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant_id.map(|s| s.to_string());
        let rows: Vec<LocalAccountRow> = conn
            .interact(move |conn| {
                let q = local_accounts::table.order(local_accounts::created_at.desc());
                match tenant {
                    Some(t) => q.filter(local_accounts::tenant_id.eq(Some(t))).load(conn),
                    None => q.filter(local_accounts::tenant_id.is_null()).load(conn),
                }
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows.iter().map(to_account).collect())
    }
```

</details>

##### <span id="LocalAccountDAL.set_status"></span><code>set_​status</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn set_status(
    &self,
    id: Uuid,
    tenant: &str,
    status: &str,
) -> Result<bool, ValidationError>
```

Disable an account (it can no longer log in / refresh). Returns true if a row changed.

<details>
<summary>Source</summary>

```rust
    pub async fn set_status(
        &self,
        id: Uuid,
        tenant: &str,
        status: &str,
    ) -> Result<bool, ValidationError> {
        use crate::database::schema::postgres::local_accounts;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let status = status.to_string();
        let tenant = tenant.to_string();
        // Scoped to (id AND tenant) so a tenant-admin can only touch its own
        // tenant's accounts; a cross-tenant id matches no row (-> false / 404).
        let n: usize = conn
            .interact(move |conn| {
                diesel::update(
                    local_accounts::table
                        .filter(local_accounts::id.eq(id))
                        .filter(local_accounts::tenant_id.eq(Some(tenant))),
                )
                .set(local_accounts::status.eq(status))
                .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n > 0)
    }
```

</details>

##### <span id="LocalAccountDAL.set_password"></span><code>set_​password</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn set_password(
    &self,
    id: Uuid,
    tenant: &str,
    new_password: &str,
) -> Result<bool, ValidationError>
```

Reset an account's password (admin-reset-only, OQ-12). Hashes the new password with argon2id.

<details>
<summary>Source</summary>

```rust
    pub async fn set_password(
        &self,
        id: Uuid,
        tenant: &str,
        new_password: &str,
    ) -> Result<bool, ValidationError> {
        use crate::database::schema::postgres::local_accounts;
        let hash = hash_password(new_password)?;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant.to_string();
        let n: usize = conn
            .interact(move |conn| {
                diesel::update(
                    local_accounts::table
                        .filter(local_accounts::id.eq(id))
                        .filter(local_accounts::tenant_id.eq(Some(tenant))),
                )
                .set(local_accounts::password_hash.eq(hash))
                .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n > 0)
    }
```

</details>

## Enums

### <span id="enum.LoginOutcome"></span><code style="font-size: 0.85em">enum LoginOutcome</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

A verified local-login result: the account + whether the password matched.

#### Variants

- **`Ok`** - Password verified and the account is active.
- **`Denied`** - Unknown user, wrong password, or disabled account. Callers MUST return
  the same opaque error for all three (no user enumeration).

## Functions

### <span id="fn.hash_password"></span><code style="font-size: 0.85em">fn hash_​password</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn hash_password(plaintext: &str) -> Result<String, ValidationError>
```

Hash a password with argon2id (random salt; returns a PHC string).

<details>
<summary>Source</summary>

```rust
pub fn hash_password(plaintext: &str) -> Result<String, ValidationError> {
    // argon2 0.6 draws the random salt itself.
    Argon2::default()
        .hash_password(plaintext.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| ValidationError::ConnectionPool(format!("password hash: {e}")))
}
```

</details>

### <span id="fn.verify_password"></span><code style="font-size: 0.85em">fn verify_​password</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn verify_password(plaintext: &str, phc: &str) -> bool
```

Verify a password against a stored argon2id PHC string. Returns `false` on any parse/verify failure (never leaks the reason).

<details>
<summary>Source</summary>

```rust
pub fn verify_password(plaintext: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => Argon2::default()
            .verify_password(plaintext.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}
```

</details>
