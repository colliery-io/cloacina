---
title: "cloacina::security::secret_store"
description: "Encrypted, tenant-scoped secrets store (CLOACI-I-0133 / T-0857)."
editUrl: false
sidebar:
  order: 195
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Encrypted, tenant-scoped secrets store (CLOACI-I-0133 / T-0857).

A **Secret** is a named object of named fields (a `{field: value}` map),
encrypted at rest — the encrypted sibling of a parameter. This module is the
foundation task of I-0133: the store, the DAL, and metadata-only CRUD. It
deliberately does NOT implement the resolution side channel (the `ctx.secret()`
accessor, D-1), grants (D-3), instance-param `$secret` references (D-4), or the
fleet per-execution envelope wrap (D-2) — those are later tasks. [`resolve_secret`](../../../../rust/cloacina/security/secret_store/#SecretStore.resolve_secret)
exists here as the INTERNAL decrypt primitive those tasks will call.

**Envelope encryption (D-7)**

Each tenant has a per-tenant **data key (DEK)** — 32 random bytes generated on
first use — wrapped by a server **KEK** and stored in `tenant_data_keys`. A
secret's field map is serialized to JSON and encrypted under the tenant DEK
(AES-256-GCM, `nonce || ciphertext || tag`). The KEK is supplied by the caller
on every method as `kek: &[u8]` (32 bytes), mirroring the `master_key`
parameter of [`crate::security::DbKeyManager`](../../../../rust/cloacina/security/db_key_manager/#struct.DbKeyManager). Unwrapping the DEK happens only
server-side; the plaintext DEK and plaintext field values live only transiently
in memory.

**Metadata vs. plaintext (REQ-002 / NFR-001)**

[`SecretMetadata`](../../../../rust/cloacina/security/secret_store/#struct.SecretMetadata) (returned by list/get) carries names, field names, and
timestamps — NEVER a plaintext or ciphertext value. Plaintext field values are
returned ONLY by [`resolve_secret`](../../../../rust/cloacina/security/secret_store/#SecretStore.resolve_secret), the internal resolve path.

## Structs

### <span id="struct.SecretMetadata"></span><code style="font-size: 0.85em">struct SecretMetadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`

Metadata-only view of a secret. **Never** carries a plaintext or ciphertext value.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `name` | `String` |  |
| `field_names` | `Vec<String>` | The declared field names (plaintext metadata) — values are NOT included. |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |

### <span id="struct.SecretStore"></span><code style="font-size: 0.85em">struct SecretStore</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Encrypted, tenant-scoped secrets store (mirrors [`crate::security::DbKeyManager`](../../../../rust/cloacina/security/db_key_manager/#struct.DbKeyManager)).

Holds no key material: the server KEK is passed into each method that needs it.

#### Methods

##### <span id="SecretStore.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: DAL) -> Self
```

Creates a new secrets store over the given DAL.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="SecretStore.create_secret"></span><code>create_​secret</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create_secret(
    &self,
    org_id: UniversalUuid,
    name: &str,
    fields: &BTreeMap<String, String>,
    kek: &[u8],
) -> Result<SecretMetadata, SecretError>
```

Create a new secret from a `{field: value}` map, encrypted under the tenant DEK.

Returns metadata only. Errors with [`SecretError::DuplicateName`](../../../../rust/cloacina/security/secret_store/#enum.SecretError) if a secret
of that name already exists for the tenant.

<details>
<summary>Source</summary>

```rust
    pub async fn create_secret(
        &self,
        org_id: UniversalUuid,
        name: &str,
        fields: &BTreeMap<String, String>,
        kek: &[u8],
    ) -> Result<SecretMetadata, SecretError> {
        let dek = self.get_or_create_tenant_dek(org_id, kek).await?;
        let encrypted_fields = Self::encrypt_fields(fields, &dek)?;
        let field_names = Self::serialize_field_names(fields)?;
        let now = UniversalTimestamp::now();

        let new_secret = NewSecret {
            id: UniversalUuid::new_v4(),
            org_id,
            name: name.to_string(),
            field_names,
            encrypted_fields: UniversalBinary::new(encrypted_fields),
            created_at: now,
            updated_at: now,
        };

        self.insert_secret(new_secret).await?;

        // Re-read so the returned metadata reflects exactly what was stored.
        self.get_secret_metadata(org_id, name).await
    }
```

</details>

##### <span id="SecretStore.rotate_secret"></span><code>rotate_​secret</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn rotate_secret(
    &self,
    org_id: UniversalUuid,
    name: &str,
    fields: &BTreeMap<String, String>,
    kek: &[u8],
) -> Result<SecretMetadata, SecretError>
```

Rotate a secret's values in place (D-8 OQ-5: in-place, no versioning).

Replaces the encrypted field map with a fresh one and bumps `updated_at`.
The next resolve sees the new value. Returns metadata only.

<details>
<summary>Source</summary>

```rust
    pub async fn rotate_secret(
        &self,
        org_id: UniversalUuid,
        name: &str,
        fields: &BTreeMap<String, String>,
        kek: &[u8],
    ) -> Result<SecretMetadata, SecretError> {
        let dek = self.get_or_create_tenant_dek(org_id, kek).await?;
        let encrypted_fields = Self::encrypt_fields(fields, &dek)?;
        let field_names = Self::serialize_field_names(fields)?;
        let now = UniversalTimestamp::now();

        let updated = self
            .update_secret(
                org_id,
                name.to_string(),
                field_names,
                UniversalBinary::new(encrypted_fields),
                now,
            )
            .await?;

        if updated == 0 {
            return Err(SecretError::NotFound(name.to_string()));
        }

        self.get_secret_metadata(org_id, name).await
    }
```

</details>

##### <span id="SecretStore.list_secrets_metadata"></span><code>list_​secrets_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_secrets_metadata(
    &self,
    org_id: UniversalUuid,
) -> Result<Vec<SecretMetadata>, SecretError>
```

List metadata for all of a tenant's secrets. **No plaintext, no ciphertext.**

<details>
<summary>Source</summary>

```rust
    pub async fn list_secrets_metadata(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<SecretMetadata>, SecretError> {
        let rows = self.list_secret_rows(org_id).await?;
        rows.iter().map(SecretMetadata::from_row).collect()
    }
```

</details>

##### <span id="SecretStore.get_secret_metadata"></span><code>get_​secret_​metadata</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_secret_metadata(
    &self,
    org_id: UniversalUuid,
    name: &str,
) -> Result<SecretMetadata, SecretError>
```

Get metadata for one secret by name. **No plaintext, no ciphertext.**

<details>
<summary>Source</summary>

```rust
    pub async fn get_secret_metadata(
        &self,
        org_id: UniversalUuid,
        name: &str,
    ) -> Result<SecretMetadata, SecretError> {
        let row = self
            .get_secret_row(org_id, name.to_string())
            .await?
            .ok_or_else(|| SecretError::NotFound(name.to_string()))?;
        SecretMetadata::from_row(&row)
    }
```

</details>

##### <span id="SecretStore.resolve_secret"></span><code>resolve_​secret</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn resolve_secret(
    &self,
    org_id: UniversalUuid,
    name: &str,
    kek: &[u8],
) -> Result<BTreeMap<String, String>, SecretError>
```

INTERNAL: resolve a secret to its plaintext `{field: value}` map.

This is the only method that returns plaintext values. Later tasks
(the resolution side channel D-1, grants D-3, fleet envelope wrap D-2)
call this; it must never feed `Context`, `schedules.params`, logs, audit,
or the fires log (NFR-001).

<details>
<summary>Source</summary>

```rust
    pub async fn resolve_secret(
        &self,
        org_id: UniversalUuid,
        name: &str,
        kek: &[u8],
    ) -> Result<BTreeMap<String, String>, SecretError> {
        let row = self
            .get_secret_row(org_id, name.to_string())
            .await?
            .ok_or_else(|| SecretError::NotFound(name.to_string()))?;

        let dek = self.get_or_create_tenant_dek(org_id, kek).await?;
        let plaintext = decrypt_bytes(&row.encrypted_fields.into_inner(), &dek)
            .map_err(|e| SecretError::Decryption(e.to_string()))?;
        serde_json::from_slice(&plaintext).map_err(|e| SecretError::Serialization(e.to_string()))
    }
```

</details>

##### <span id="SecretStore.delete_secret"></span><code>delete_​secret</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn delete_secret(
    &self,
    org_id: UniversalUuid,
    name: &str,
) -> Result<(), SecretError>
```

Delete a secret by name.

<details>
<summary>Source</summary>

```rust
    pub async fn delete_secret(
        &self,
        org_id: UniversalUuid,
        name: &str,
    ) -> Result<(), SecretError> {
        let deleted = self.delete_secret_row(org_id, name.to_string()).await?;
        if deleted == 0 {
            return Err(SecretError::NotFound(name.to_string()));
        }
        Ok(())
    }
```

</details>

## Enums

### <span id="enum.SecretError"></span><code style="font-size: 0.85em">enum SecretError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur in the secrets store.

#### Variants

- **`NotFound`**
- **`DuplicateName`**
- **`Encryption`**
- **`Decryption`**
- **`Serialization`**
- **`Database`**
