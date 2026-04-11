# cloacina::security::api_keys <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


API key generation and hashing utilities.

## Functions

### `cloacina::security::api_keys::generate_api_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::security::api_keys::hash_api_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
