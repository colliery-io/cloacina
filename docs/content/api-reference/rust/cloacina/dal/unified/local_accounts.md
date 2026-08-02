# cloacina::dal::unified::local_accounts <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Local accounts — self-managed username/password login (CLOACI-T-0795). **Postgres only.**

The minimal credential entity behind "IdP or self-manage": `username +
argon2id password hash + tenant + role + active/disabled status`. The
account record IS the identity→tenant/role mapping, so local login bypasses
the OIDC allowlist. Password plaintext is never stored — only the argon2id
PHC string. Local login (T-0796) verifies a password and resolves the
account to a `ResolvedPrincipal`; account management (T-0797) is tenant-admin.

## Structs

### `cloacina::dal::unified::local_accounts::LocalAccount`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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

##### `is_active` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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





### `cloacina::dal::unified::local_accounts::LocalAccountRow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


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



### `cloacina::dal::unified::local_accounts::NewLocalAccount`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


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



### `cloacina::dal::unified::local_accounts::LocalAccountDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


DAL for local accounts. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Enums

### `cloacina::dal::unified::local_accounts::LoginOutcome` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


A verified local-login result: the account + whether the password matched.

#### Variants

- **`Ok`** - Password verified and the account is active.
- **`Denied`** - Unknown user, wrong password, or disabled account. Callers MUST return
the same opaque error for all three (no user enumeration).



## Functions

### `cloacina::dal::unified::local_accounts::hash_password`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::local_accounts::verify_password`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::dal::unified::local_accounts::to_account`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


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
