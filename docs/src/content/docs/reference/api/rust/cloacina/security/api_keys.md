---
title: "cloacina::security::api_keys"
description: "API key generation and hashing utilities."
sidebar:
  order: 206
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


API key generation and hashing utilities.

## Functions

### <span id="fn.generate_api_key"></span>`fn generate_api_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn generate_api_key () -> (String , String)
```

Generates a new API key, returning `(plaintext, hash)`.

The plaintext has the form `clk_` followed by 32 random bytes encoded as
base64url (no padding). The hash is the lowercase hex SHA-256 digest of the
full plaintext string.

<details>
<summary>Source</summary>

```rust
pub fn generate_api_key() -> (String, String) {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);

    let plaintext = format!("clk_{}", URL_SAFE_NO_PAD.encode(bytes));
    let hash = hash_api_key(&plaintext);
    (plaintext, hash)
}
```

</details>



### <span id="fn.hash_api_key"></span>`fn hash_api_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn hash_api_key (key : & str) -> String
```

Returns the lowercase hex SHA-256 hash of an API key string.

<details>
<summary>Source</summary>

```rust
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

</details>



