---
title: "cloacina::dal::unified::local_accounts"
description: "Local accounts — self-managed username/password login (CLOACI-T-0795). **Postgres only.**"
sidebar:
  order: 91
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Local accounts — self-managed username/password login (CLOACI-T-0795). **Postgres only.**

The minimal credential entity behind "IdP or self-manage": `username +
argon2id password hash + tenant + role + active/disabled status`. The
account record IS the identity→tenant/role mapping, so local login bypasses
the OIDC allowlist. Password plaintext is never stored — only the argon2id
PHC string. Local login (T-0796) verifies a password and resolves the
account to a `ResolvedPrincipal`; account management (T-0797) is tenant-admin.

## Structs

### <span id="struct.LocalAccount"></span>`struct LocalAccount`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Public view of a local account (never includes the password hash).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `uuid :: Uuid` |  |
| `username` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `role` | `String` |  |
| `status` | `String` |  |

#### Methods

##### <span id="LocalAccount.is_active"></span>`is_active`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn is_active (& self) -> bool
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





### <span id="struct.LocalAccountRow"></span>`struct LocalAccountRow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `Queryable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` |  |
| `username` | `String` |  |
| `password_hash` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `role` | `String` |  |
| `status` | `String` |  |
| `created_at` | `chrono :: NaiveDateTime` |  |



### <span id="struct.NewLocalAccount"></span>`struct NewLocalAccount`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` |  |
| `username` | `String` |  |
| `password_hash` | `String` |  |
| `tenant_id` | `Option < String >` |  |
| `role` | `String` |  |
| `status` | `String` |  |



### <span id="struct.LocalAccountDAL"></span>`struct LocalAccountDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


DAL for local accounts. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Enums

### <span id="enum.LoginOutcome"></span>`enum LoginOutcome`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


A verified local-login result: the account + whether the password matched.

#### Variants

- **`Ok`** - Password verified and the account is active.
- **`Denied`** - Unknown user, wrong password, or disabled account. Callers MUST return
the same opaque error for all three (no user enumeration).



## Functions

### <span id="fn.hash_password"></span>`fn hash_password`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn hash_password (plaintext : & str) -> Result < String , ValidationError >
```

Hash a password with argon2id (random salt; returns a PHC string).

<details>
<summary>Source</summary>

```rust
pub fn hash_password(plaintext: &str) -> Result<String, ValidationError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plaintext.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ValidationError::ConnectionPool(format!("password hash: {e}")))
}
```

</details>



### <span id="fn.verify_password"></span>`fn verify_password`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn verify_password (plaintext : & str , phc : & str) -> bool
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



### <span id="fn.decoy_password_hash"></span>`fn decoy_password_hash`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn decoy_password_hash () -> & 'static str
```

A throwaway argon2id hash of a random secret, built once per process with exactly the parameters [`hash_password`] uses (CLOACI-T-0923).

Verifying an unknown username's password against this makes the "no such
user" path cost the same argon2 work as the "wrong password" path, closing
the timing side-channel that would otherwise enumerate accounts regardless
of how uniform the HTTP error body is. Nothing ever matches it: the input
is a fresh UUID that is discarded immediately.

<details>
<summary>Source</summary>

```rust
fn decoy_password_hash() -> &'static str {
    static DECOY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    DECOY.get_or_init(|| hash_password(&uuid::Uuid::new_v4().to_string()).unwrap_or_default())
}
```

</details>



### <span id="fn.to_account"></span>`fn to_account`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn to_account (row : & LocalAccountRow) -> LocalAccount
```

<details>
<summary>Source</summary>

```rust
fn to_account(row: &LocalAccountRow) -> LocalAccount {
    LocalAccount {
        id: row.id,
        username: row.username.clone(),
        tenant_id: row.tenant_id.clone(),
        role: row.role.clone(),
        status: row.status.clone(),
    }
}
```

</details>



