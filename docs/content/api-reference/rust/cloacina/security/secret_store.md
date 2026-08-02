# cloacina::security::secret_store <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Encrypted, tenant-scoped secrets store (CLOACI-I-0133 / T-0857).

A **Secret** is a named object of named fields (a `{field: value}` map),
encrypted at rest — the encrypted sibling of a parameter. This module is the
foundation task of I-0133: the store, the DAL, and metadata-only CRUD. It
deliberately does NOT implement the resolution side channel (the `ctx.secret()`
accessor, D-1), grants (D-3), instance-param `$secret` references (D-4), or the
fleet per-execution envelope wrap (D-2) — those are later tasks. [`resolve_secret`]
exists here as the INTERNAL decrypt primitive those tasks will call.

## Structs

### `cloacina::security::secret_store::SecretMetadata`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`

Metadata-only view of a secret. **Never** carries a plaintext or ciphertext value.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `name` | `String` |  |
| `field_names` | `Vec < String >` | The declared field names (plaintext metadata) — values are NOT included. |
| `created_at` | `UniversalTimestamp` |  |
| `updated_at` | `UniversalTimestamp` |  |

#### Methods

##### `from_row` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn from_row (row : & Secret) -> Result < Self , SecretError >
```

<details>
<summary>Source</summary>

```rust
    fn from_row(row: &Secret) -> Result<Self, SecretError> {
        let field_names: Vec<String> = serde_json::from_str(&row.field_names)
            .map_err(|e| SecretError::Serialization(e.to_string()))?;
        Ok(Self {
            id: row.id,
            org_id: row.org_id,
            name: row.name.clone(),
            field_names,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
```

</details>





### `cloacina::security::secret_store::SecretStore`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Encrypted, tenant-scoped secrets store (mirrors [`crate::security::DbKeyManager`]).

Holds no key material: the server KEK is passed into each method that needs it.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `DAL` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : DAL) -> Self
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



##### `serialize_fields` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn serialize_fields (fields : & BTreeMap < String , String >) -> Result < Vec < u8 > , SecretError >
```

Serialize the `{field: value}` map to canonical JSON bytes. A `BTreeMap` gives a deterministic field order in the plaintext.

<details>
<summary>Source</summary>

```rust
    fn serialize_fields(fields: &BTreeMap<String, String>) -> Result<Vec<u8>, SecretError> {
        serde_json::to_vec(fields).map_err(|e| SecretError::Serialization(e.to_string()))
    }
```

</details>



##### `serialize_field_names` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn serialize_field_names (fields : & BTreeMap < String , String >) -> Result < String , SecretError >
```

JSON array of just the field names (plaintext metadata).

<details>
<summary>Source</summary>

```rust
    fn serialize_field_names(fields: &BTreeMap<String, String>) -> Result<String, SecretError> {
        let names: Vec<&String> = fields.keys().collect();
        serde_json::to_string(&names).map_err(|e| SecretError::Serialization(e.to_string()))
    }
```

</details>



##### `encrypt_fields` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn encrypt_fields (fields : & BTreeMap < String , String > , dek : & [u8] ,) -> Result < Vec < u8 > , SecretError >
```

Encrypt a serialized field map under the tenant DEK.

<details>
<summary>Source</summary>

```rust
    fn encrypt_fields(
        fields: &BTreeMap<String, String>,
        dek: &[u8],
    ) -> Result<Vec<u8>, SecretError> {
        let plaintext = Self::serialize_fields(fields)?;
        encrypt_bytes(&plaintext, dek).map_err(|e| SecretError::Encryption(e.to_string()))
    }
```

</details>



##### `get_or_create_tenant_dek` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_or_create_tenant_dek (& self , org_id : UniversalUuid , kek : & [u8] ,) -> Result < Vec < u8 > , SecretError >
```

Get the tenant's data key (DEK), generating + wrapping one on first use.

Returns the UNWRAPPED 32-byte DEK. The DEK is stored wrapped under the
server `kek`; unwrapping happens only here, server-side. Internal — callers
in this module hold the plaintext DEK only transiently.

<details>
<summary>Source</summary>

```rust
    async fn get_or_create_tenant_dek(
        &self,
        org_id: UniversalUuid,
        kek: &[u8],
    ) -> Result<Vec<u8>, SecretError> {
        // Fast path: an existing wrapped DEK.
        if let Some(row) = self.get_tenant_data_key_row(org_id).await? {
            return decrypt_bytes(&row.wrapped_dek.into_inner(), kek)
                .map_err(|e| SecretError::Decryption(e.to_string()));
        }

        // Generate a fresh DEK and wrap it under the KEK.
        let mut dek = vec![0u8; DEK_SIZE];
        rand::thread_rng().fill_bytes(&mut dek);
        let wrapped =
            encrypt_bytes(&dek, kek).map_err(|e| SecretError::Encryption(e.to_string()))?;

        let new_row = NewTenantDataKey {
            id: UniversalUuid::new_v4(),
            org_id,
            wrapped_dek: UniversalBinary::new(wrapped),
            created_at: UniversalTimestamp::now(),
        };

        match self.insert_tenant_data_key(new_row).await {
            Ok(()) => Ok(dek),
            Err(SecretError::DuplicateName(_)) => {
                // Lost a race with a concurrent create: adopt the winner's DEK.
                let row = self
                    .get_tenant_data_key_row(org_id)
                    .await?
                    .ok_or_else(|| SecretError::Database("tenant DEK vanished".to_string()))?;
                decrypt_bytes(&row.wrapped_dek.into_inner(), kek)
                    .map_err(|e| SecretError::Decryption(e.to_string()))
            }
            Err(e) => Err(e),
        }
    }
```

</details>



##### `create_secret` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn create_secret (& self , org_id : UniversalUuid , name : & str , fields : & BTreeMap < String , String > , kek : & [u8] ,) -> Result < SecretMetadata , SecretError >
```

Create a new secret from a `{field: value}` map, encrypted under the tenant DEK.

Returns metadata only. Errors with [`SecretError::DuplicateName`] if a secret
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



##### `rotate_secret` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn rotate_secret (& self , org_id : UniversalUuid , name : & str , fields : & BTreeMap < String , String > , kek : & [u8] ,) -> Result < SecretMetadata , SecretError >
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



##### `list_secrets_metadata` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn list_secrets_metadata (& self , org_id : UniversalUuid ,) -> Result < Vec < SecretMetadata > , SecretError >
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



##### `get_secret_metadata` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_secret_metadata (& self , org_id : UniversalUuid , name : & str ,) -> Result < SecretMetadata , SecretError >
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



##### `resolve_secret` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn resolve_secret (& self , org_id : UniversalUuid , name : & str , kek : & [u8] ,) -> Result < BTreeMap < String , String > , SecretError >
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



##### `delete_secret` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn delete_secret (& self , org_id : UniversalUuid , name : & str ,) -> Result < () , SecretError >
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



##### `get_tenant_data_key_row` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_tenant_data_key_row (& self , org_id : UniversalUuid ,) -> Result < Option < TenantDataKey > , SecretError >
```

<details>
<summary>Source</summary>

```rust
    async fn get_tenant_data_key_row(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Option<TenantDataKey>, SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            tenant_data_keys::table
                .filter(tenant_data_keys::org_id.eq(org_id))
                .first(conn)
                .optional()
        })
        .map_err(|e| SecretError::Database(e.to_string()))
    }
```

</details>



##### `insert_tenant_data_key` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn insert_tenant_data_key (& self , new_row : NewTenantDataKey) -> Result < () , SecretError >
```

<details>
<summary>Source</summary>

```rust
    async fn insert_tenant_data_key(&self, new_row: NewTenantDataKey) -> Result<(), SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(tenant_data_keys::table)
                .values(&new_row)
                .execute(conn)
        })
        .map_err(|e| {
            if is_unique_violation(&e) {
                SecretError::DuplicateName("tenant_data_key".to_string())
            } else {
                SecretError::Database(e.to_string())
            }
        })?;

        Ok(())
    }
```

</details>



##### `insert_secret` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn insert_secret (& self , new_secret : NewSecret) -> Result < () , SecretError >
```

<details>
<summary>Source</summary>

```rust
    async fn insert_secret(&self, new_secret: NewSecret) -> Result<(), SecretError> {
        let name = new_secret.name.clone();

        crate::interact_on_backend!(self.dal, |conn| {
            diesel::insert_into(secrets::table)
                .values(&new_secret)
                .execute(conn)
        })
        .map_err(|e| {
            if is_unique_violation(&e) {
                SecretError::DuplicateName(name.clone())
            } else {
                SecretError::Database(e.to_string())
            }
        })?;

        Ok(())
    }
```

</details>



##### `update_secret` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn update_secret (& self , org_id : UniversalUuid , name : String , field_names : String , encrypted_fields : UniversalBinary , updated_at : UniversalTimestamp ,) -> Result < usize , SecretError >
```

<details>
<summary>Source</summary>

```rust
    async fn update_secret(
        &self,
        org_id: UniversalUuid,
        name: String,
        field_names: String,
        encrypted_fields: UniversalBinary,
        updated_at: UniversalTimestamp,
    ) -> Result<usize, SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                secrets::table
                    .filter(secrets::org_id.eq(org_id))
                    .filter(secrets::name.eq(name)),
            )
            .set((
                secrets::field_names.eq(field_names),
                secrets::encrypted_fields.eq(encrypted_fields),
                secrets::updated_at.eq(updated_at),
            ))
            .execute(conn)
        })
        .map_err(|e| SecretError::Database(e.to_string()))
    }
```

</details>



##### `get_secret_row` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_secret_row (& self , org_id : UniversalUuid , name : String ,) -> Result < Option < Secret > , SecretError >
```

<details>
<summary>Source</summary>

```rust
    async fn get_secret_row(
        &self,
        org_id: UniversalUuid,
        name: String,
    ) -> Result<Option<Secret>, SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            secrets::table
                .filter(secrets::org_id.eq(org_id))
                .filter(secrets::name.eq(name))
                .first(conn)
                .optional()
        })
        .map_err(|e| SecretError::Database(e.to_string()))
    }
```

</details>



##### `list_secret_rows` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn list_secret_rows (& self , org_id : UniversalUuid) -> Result < Vec < Secret > , SecretError >
```

<details>
<summary>Source</summary>

```rust
    async fn list_secret_rows(&self, org_id: UniversalUuid) -> Result<Vec<Secret>, SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            secrets::table.filter(secrets::org_id.eq(org_id)).load(conn)
        })
        .map_err(|e| SecretError::Database(e.to_string()))
    }
```

</details>



##### `delete_secret_row` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn delete_secret_row (& self , org_id : UniversalUuid , name : String ,) -> Result < usize , SecretError >
```

<details>
<summary>Source</summary>

```rust
    async fn delete_secret_row(
        &self,
        org_id: UniversalUuid,
        name: String,
    ) -> Result<usize, SecretError> {
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                secrets::table
                    .filter(secrets::org_id.eq(org_id))
                    .filter(secrets::name.eq(name)),
            )
            .execute(conn)
        })
        .map_err(|e| SecretError::Database(e.to_string()))
    }
```

</details>





## Enums

### `cloacina::security::secret_store::SecretError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur in the secrets store.

#### Variants

- **`NotFound`**
- **`DuplicateName`**
- **`Encryption`**
- **`Decryption`**
- **`Serialization`**
- **`Database`**



## Functions

### `cloacina::security::secret_store::is_unique_violation`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn is_unique_violation (e : & diesel :: result :: Error) -> bool
```

Classify an insert error as a uniqueness violation vs. a generic DB error.

<details>
<summary>Source</summary>

```rust
fn is_unique_violation(e: &diesel::result::Error) -> bool {
    let s = e.to_string();
    s.contains("duplicate") || s.contains("UNIQUE")
}
```

</details>
