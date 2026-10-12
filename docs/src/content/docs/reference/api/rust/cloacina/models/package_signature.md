---
title: "cloacina::models::package_signature"
description: "Domain models for package signatures."
sidebar:
  order: 155
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Domain models for package signatures.

Package signatures provide cryptographic proof that a workflow package
was signed by a specific key. Signatures are Ed25519 signatures over
the SHA256 hash of the package binary.

## Structs

### <span id="struct.PackageSignature"></span>`struct PackageSignature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Domain model for a package signature.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `package_hash` | `String` | SHA256 hex hash of the package binary |
| `key_fingerprint` | `String` | SHA256 hex fingerprint of the signing key |
| `signature` | `Vec < u8 >` | Ed25519 signature (64 bytes) |
| `signed_at` | `UniversalTimestamp` |  |
| `org_id` | `Option < UniversalUuid >` | Organization id this signature is scoped to. NULL on rows that
pre-date the column (CLOACI-I-0103). Server-side verification rejects
NULL when `--require-signatures` is on. |



### <span id="struct.NewPackageSignature"></span>`struct NewPackageSignature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating a new package signature.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_hash` | `String` |  |
| `key_fingerprint` | `String` |  |
| `signature` | `Vec < u8 >` |  |
| `org_id` | `Option < UniversalUuid >` |  |

#### Methods

##### <span id="NewPackageSignature.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (package_hash : String , key_fingerprint : String , signature : Vec < u8 >) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(package_hash: String, key_fingerprint: String, signature: Vec<u8>) -> Self {
        Self {
            package_hash,
            key_fingerprint,
            signature,
            org_id: None,
        }
    }
```

</details>



##### <span id="NewPackageSignature.with_org_id"></span>`with_org_id`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn with_org_id (mut self , org_id : UniversalUuid) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn with_org_id(mut self, org_id: UniversalUuid) -> Self {
        self.org_id = Some(org_id);
        self
    }
```

</details>





### <span id="struct.SignatureVerification"></span>`struct SignatureVerification`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Result of signature verification.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `is_valid` | `bool` | Whether the signature is valid |
| `signer_fingerprint` | `String` | The fingerprint of the key that signed the package |
| `signed_at` | `UniversalTimestamp` | When the package was signed |
| `signer_name` | `Option < String >` | Name of the signing key (if known) |



