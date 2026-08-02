# cloacina::security::db_key_manager <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Database-backed key manager implementation.

This module provides a [`KeyManager`] implementation that stores keys
in the database using Diesel. Private keys are encrypted at rest using
AES-256-GCM.

## Structs

### `cloacina::security::db_key_manager::DbKeyManager`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Database-backed implementation of the [`KeyManager`] trait.

This implementation:
- Stores signing keys with AES-256-GCM encrypted private keys
- Supports trust relationships between organizations via ACLs
- Does NOT cache any data to ensure immediate effect of revocations

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `DAL` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : DAL) -> Self
```

Creates a new database-backed key manager.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: DAL) -> Self {
        Self { dal }
    }
```

</details>



##### `encode_public_key_pem` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn encode_public_key_pem (public_key : & [u8]) -> String
```

Encodes a raw Ed25519 public key to PEM format.

<details>
<summary>Source</summary>

```rust
    fn encode_public_key_pem(public_key: &[u8]) -> String {
        // Create DER-encoded SubjectPublicKeyInfo
        let mut der = Vec::with_capacity(ED25519_DER_PREFIX.len() + public_key.len());
        der.extend_from_slice(&ED25519_DER_PREFIX);
        der.extend_from_slice(public_key);

        // Encode as PEM
        let pem = pem::Pem::new(ED25519_PEM_TAG, der);
        pem::encode(&pem)
    }
```

</details>



##### `decode_public_key_pem` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn decode_public_key_pem (pem_str : & str) -> Result < Vec < u8 > , KeyError >
```

Decodes a PEM-encoded Ed25519 public key to raw bytes.

<details>
<summary>Source</summary>

```rust
    fn decode_public_key_pem(pem_str: &str) -> Result<Vec<u8>, KeyError> {
        let pem = pem::parse(pem_str).map_err(|e| KeyError::InvalidPem(e.to_string()))?;

        if pem.tag() != ED25519_PEM_TAG {
            return Err(KeyError::InvalidPem(format!(
                "Expected tag '{}', got '{}'",
                ED25519_PEM_TAG,
                pem.tag()
            )));
        }

        let der = pem.contents();

        // Verify the DER prefix
        if der.len() != ED25519_DER_PREFIX.len() + 32 {
            return Err(KeyError::InvalidPem(format!(
                "Invalid DER length: expected {}, got {}",
                ED25519_DER_PREFIX.len() + 32,
                der.len()
            )));
        }

        if der[..ED25519_DER_PREFIX.len()] != ED25519_DER_PREFIX {
            return Err(KeyError::InvalidPem(
                "Invalid DER prefix for Ed25519 key".to_string(),
            ));
        }

        // Extract the 32-byte public key
        Ok(der[ED25519_DER_PREFIX.len()..].to_vec())
    }
```

</details>



##### `to_signing_key_info` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn to_signing_key_info (key : UnifiedSigningKey) -> SigningKeyInfo
```

Convert database model to SigningKeyInfo.

<details>
<summary>Source</summary>

```rust
    fn to_signing_key_info(key: UnifiedSigningKey) -> SigningKeyInfo {
        SigningKeyInfo {
            id: key.id,
            org_id: key.org_id,
            key_name: key.key_name,
            fingerprint: key.key_fingerprint,
            public_key: key.public_key.into_inner(),
            created_at: key.created_at,
            revoked_at: key.revoked_at,
        }
    }
```

</details>



##### `to_trusted_key_info` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn to_trusted_key_info (key : UnifiedTrustedKey) -> TrustedKeyInfo
```

Convert database model to TrustedKeyInfo.

<details>
<summary>Source</summary>

```rust
    fn to_trusted_key_info(key: UnifiedTrustedKey) -> TrustedKeyInfo {
        TrustedKeyInfo {
            id: key.id,
            org_id: key.org_id,
            fingerprint: key.key_fingerprint,
            public_key: key.public_key.into_inner(),
            key_name: key.key_name,
            trusted_at: key.trusted_at,
            revoked_at: key.revoked_at,
        }
    }
```

</details>



##### `list_direct_trusted_keys` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn list_direct_trusted_keys (& self , org_id : UniversalUuid ,) -> Result < Vec < TrustedKeyInfo > , KeyError >
```

<details>
<summary>Source</summary>

```rust
    async fn list_direct_trusted_keys(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<TrustedKeyInfo>, KeyError> {
        let keys: Vec<UnifiedTrustedKey> = crate::interact_on_backend!(self.dal, |conn| {
            trusted_keys::table
                .filter(trusted_keys::org_id.eq(org_id))
                .filter(trusted_keys::revoked_at.is_null())
                .load(conn)
        })
        .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(keys.into_iter().map(Self::to_trusted_key_info).collect())
    }
```

</details>



##### `get_trusted_child_orgs` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_trusted_child_orgs (& self , org_id : UniversalUuid ,) -> Result < Vec < UniversalUuid > , KeyError >
```

<details>
<summary>Source</summary>

```rust
    async fn get_trusted_child_orgs(
        &self,
        org_id: UniversalUuid,
    ) -> Result<Vec<UniversalUuid>, KeyError> {
        let acls: Vec<UnifiedKeyTrustAcl> = crate::interact_on_backend!(self.dal, |conn| {
            key_trust_acls::table
                .filter(key_trust_acls::parent_org_id.eq(org_id))
                .filter(key_trust_acls::revoked_at.is_null())
                .load(conn)
        })
        .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(acls.into_iter().map(|acl| acl.child_org_id).collect())
    }
```

</details>



##### `find_direct_trusted_key` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn find_direct_trusted_key (& self , org_id : UniversalUuid , fingerprint : & str ,) -> Result < Option < TrustedKeyInfo > , KeyError >
```

<details>
<summary>Source</summary>

```rust
    async fn find_direct_trusted_key(
        &self,
        org_id: UniversalUuid,
        fingerprint: &str,
    ) -> Result<Option<TrustedKeyInfo>, KeyError> {
        let fingerprint = fingerprint.to_string();

        let key: Option<UnifiedTrustedKey> = crate::interact_on_backend!(self.dal, |conn| {
            trusted_keys::table
                .filter(trusted_keys::org_id.eq(org_id))
                .filter(trusted_keys::key_fingerprint.eq(&fingerprint))
                .filter(trusted_keys::revoked_at.is_null())
                .first(conn)
                .optional()
        })
        .map_err(|e| KeyError::Database(e.to_string()))?;

        Ok(key.map(Self::to_trusted_key_info))
    }
```

</details>
