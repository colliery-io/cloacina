# cloacina::security::audit <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Security audit logging for SIEM integration.

This module provides structured audit logging for all security-sensitive operations:
- Package loads (success/failure)
- Key operations (create, revoke, trust)
- Signature verification
All events use structured fields compatible with common SIEM systems.
Events are logged using the `tracing` crate at appropriate levels.

## Functions

### `cloacina::security::audit::log_signing_key_created`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_signing_key_created (org_id : UniversalUuid , key_id : UniversalUuid , key_fingerprint : & str , key_name : & str ,)
```

Log a signing key creation event.

<details>
<summary>Source</summary>

```rust
pub fn log_signing_key_created(
    org_id: UniversalUuid,
    key_id: UniversalUuid,
    key_fingerprint: &str,
    key_name: &str,
) {
    tracing::info!(
        event_type = events::KEY_SIGNING_CREATED,
        org_id = %org_id,
        key_id = %key_id,
        key_fingerprint = %key_fingerprint,
        key_name = %key_name,
        "Signing key created"
    );
}
```

</details>



### `cloacina::security::audit::log_signing_key_create_failed`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_signing_key_create_failed (org_id : UniversalUuid , key_name : & str , error : & str)
```

Log a signing key creation failure.

<details>
<summary>Source</summary>

```rust
pub fn log_signing_key_create_failed(org_id: UniversalUuid, key_name: &str, error: &str) {
    tracing::error!(
        event_type = events::KEY_SIGNING_CREATE_FAILED,
        org_id = %org_id,
        key_name = %key_name,
        error = %error,
        "Failed to create signing key"
    );
}
```

</details>



### `cloacina::security::audit::log_signing_key_revoked`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_signing_key_revoked (org_id : UniversalUuid , key_id : UniversalUuid , key_fingerprint : & str , key_name : Option < & str > ,)
```

Log a signing key revocation event.

<details>
<summary>Source</summary>

```rust
pub fn log_signing_key_revoked(
    org_id: UniversalUuid,
    key_id: UniversalUuid,
    key_fingerprint: &str,
    key_name: Option<&str>,
) {
    tracing::warn!(
        event_type = events::KEY_SIGNING_REVOKED,
        org_id = %org_id,
        key_id = %key_id,
        key_fingerprint = %key_fingerprint,
        key_name = key_name.unwrap_or("<unknown>"),
        "Signing key revoked"
    );
}
```

</details>



### `cloacina::security::audit::log_key_exported`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_key_exported (key_id : UniversalUuid , key_fingerprint : & str)
```

Log a public key export event.

<details>
<summary>Source</summary>

```rust
pub fn log_key_exported(key_id: UniversalUuid, key_fingerprint: &str) {
    tracing::info!(
        event_type = events::KEY_EXPORTED,
        key_id = %key_id,
        key_fingerprint = %key_fingerprint,
        "Public key exported"
    );
}
```

</details>



### `cloacina::security::audit::log_trusted_key_added`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_trusted_key_added (org_id : UniversalUuid , key_id : UniversalUuid , key_fingerprint : & str , key_name : Option < & str > ,)
```

Log a trusted key addition event.

<details>
<summary>Source</summary>

```rust
pub fn log_trusted_key_added(
    org_id: UniversalUuid,
    key_id: UniversalUuid,
    key_fingerprint: &str,
    key_name: Option<&str>,
) {
    tracing::warn!(
        event_type = events::KEY_TRUSTED_ADDED,
        org_id = %org_id,
        key_id = %key_id,
        key_fingerprint = %key_fingerprint,
        key_name = key_name.unwrap_or("<unnamed>"),
        "Trusted key added"
    );
}
```

</details>



### `cloacina::security::audit::log_trusted_key_revoked`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_trusted_key_revoked (key_id : UniversalUuid)
```

Log a trusted key revocation event.

<details>
<summary>Source</summary>

```rust
pub fn log_trusted_key_revoked(key_id: UniversalUuid) {
    tracing::warn!(
        event_type = events::KEY_TRUSTED_REVOKED,
        key_id = %key_id,
        "Trusted key revoked"
    );
}
```

</details>



### `cloacina::security::audit::log_trust_acl_granted`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_trust_acl_granted (parent_org : UniversalUuid , child_org : UniversalUuid)
```

Log a trust ACL grant event.

<details>
<summary>Source</summary>

```rust
pub fn log_trust_acl_granted(parent_org: UniversalUuid, child_org: UniversalUuid) {
    tracing::warn!(
        event_type = events::KEY_TRUST_ACL_GRANTED,
        parent_org_id = %parent_org,
        child_org_id = %child_org,
        "Trust ACL granted"
    );
}
```

</details>



### `cloacina::security::audit::log_trust_acl_revoked`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_trust_acl_revoked (parent_org : UniversalUuid , child_org : UniversalUuid)
```

Log a trust ACL revocation event.

<details>
<summary>Source</summary>

```rust
pub fn log_trust_acl_revoked(parent_org: UniversalUuid, child_org: UniversalUuid) {
    tracing::warn!(
        event_type = events::KEY_TRUST_ACL_REVOKED,
        parent_org_id = %parent_org,
        child_org_id = %child_org,
        "Trust ACL revoked"
    );
}
```

</details>



### `cloacina::security::audit::log_package_signed`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_package_signed (package_path : & str , package_hash : & str , key_fingerprint : & str)
```

Log a package signing event.

<details>
<summary>Source</summary>

```rust
pub fn log_package_signed(package_path: &str, package_hash: &str, key_fingerprint: &str) {
    tracing::info!(
        event_type = events::PACKAGE_SIGNED,
        package_path = %package_path,
        package_hash = %package_hash,
        key_fingerprint = %key_fingerprint,
        "Package signed"
    );
}
```

</details>



### `cloacina::security::audit::log_package_sign_failed`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_package_sign_failed (package_path : & str , error : & str)
```

Log a package signing failure.

<details>
<summary>Source</summary>

```rust
pub fn log_package_sign_failed(package_path: &str, error: &str) {
    tracing::error!(
        event_type = events::PACKAGE_SIGN_FAILURE,
        package_path = %package_path,
        error = %error,
        "Package signing failed"
    );
}
```

</details>



### `cloacina::security::audit::log_package_load_success`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_package_load_success (org_id : UniversalUuid , package_path : & str , package_hash : & str , signer_fingerprint : Option < & str > , signature_verified : bool ,)
```

Log a package load success event.

<details>
<summary>Source</summary>

```rust
pub fn log_package_load_success(
    org_id: UniversalUuid,
    package_path: &str,
    package_hash: &str,
    signer_fingerprint: Option<&str>,
    signature_verified: bool,
) {
    tracing::info!(
        event_type = events::PACKAGE_LOAD_SUCCESS,
        org_id = %org_id,
        package_path = %package_path,
        package_hash = %package_hash,
        signer_fingerprint = signer_fingerprint.unwrap_or("<none>"),
        signature_verified = signature_verified,
        "Package loaded successfully"
    );
}
```

</details>



### `cloacina::security::audit::log_package_load_failure`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_package_load_failure (org_id : UniversalUuid , package_path : & str , error : & str , failure_reason : & str ,)
```

Log a package load failure event.

<details>
<summary>Source</summary>

```rust
pub fn log_package_load_failure(
    org_id: UniversalUuid,
    package_path: &str,
    error: &str,
    failure_reason: &str,
) {
    tracing::warn!(
        event_type = events::PACKAGE_LOAD_FAILURE,
        org_id = %org_id,
        package_path = %package_path,
        error = %error,
        failure_reason = %failure_reason,
        "Package load failed"
    );
}
```

</details>



### `cloacina::security::audit::log_verification_success`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_verification_success (org_id : UniversalUuid , package_hash : & str , signer_fingerprint : & str , signer_name : Option < & str > ,)
```

Log a verification success event.

<details>
<summary>Source</summary>

```rust
pub fn log_verification_success(
    org_id: UniversalUuid,
    package_hash: &str,
    signer_fingerprint: &str,
    signer_name: Option<&str>,
) {
    tracing::info!(
        event_type = events::VERIFICATION_SUCCESS,
        org_id = %org_id,
        package_hash = %package_hash,
        signer_fingerprint = %signer_fingerprint,
        signer_name = signer_name.unwrap_or("<unknown>"),
        "Package signature verified successfully"
    );
}
```

</details>



### `cloacina::security::audit::log_verification_failure`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn log_verification_failure (org_id : UniversalUuid , package_hash : & str , failure_reason : & str , signer_fingerprint : Option < & str > ,)
```

Log a verification failure event.

<details>
<summary>Source</summary>

```rust
pub fn log_verification_failure(
    org_id: UniversalUuid,
    package_hash: &str,
    failure_reason: &str,
    signer_fingerprint: Option<&str>,
) {
    tracing::warn!(
        event_type = events::VERIFICATION_FAILURE,
        org_id = %org_id,
        package_hash = %package_hash,
        failure_reason = %failure_reason,
        signer_fingerprint = signer_fingerprint.unwrap_or("<unknown>"),
        "Package signature verification failed"
    );
}
```

</details>
