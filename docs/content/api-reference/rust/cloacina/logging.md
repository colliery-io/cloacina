# cloacina::logging <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Functions

### `cloacina::logging::init_logging`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn init_logging (level : Option < Level >)
```

Initializes the logging system with the specified log level.

If no level is provided, it will use the `RUST_LOG` environment variable
or default to "info" if the environment variable is not set.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `level` | `-` | Optional log level to use. If `None`, uses environment configuration. |


**Examples:**

```rust,ignore
use cloacina::init_logging;
use tracing::Level;

// Use environment variable or default to INFO
init_logging(None);

// Set specific level
init_logging(Some(Level::DEBUG));
```

<details>
<summary>Source</summary>

```rust
pub fn init_logging(level: Option<Level>) {
    let filter = match level {
        Some(level) => EnvFilter::new(level.as_str()),
        None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
```

</details>



### `cloacina::logging::init_test_logging`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn init_test_logging ()
```

Initializes the logging system for test environments.

This sets up a test-specific subscriber that:
- Captures logs for verification in tests
- Uses debug level by default
- Writes to test output that doesn't interfere with test results
- Can be called multiple times safely (subsequent calls are ignored)

**Examples:**

```rust,ignore
use cloacina::init_test_logging;

#[test]
fn test_with_logging() {
    init_test_logging();

    // Your test code here
    // Logs will be captured and can be verified if needed
}
```

<details>
<summary>Source</summary>

```rust
pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_test_writer()
        .try_init();
}
```

</details>



### `cloacina::logging::mask_db_url`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn mask_db_url (url : & str) -> String
```

Mask the password in a database URL for safe logging.

Replaces the password portion (between the last `:` before `@` and the `@`)
with `****`. If the URL does not contain credentials, returns it unchanged.

**Examples:**

```
use cloacina::logging::mask_db_url;
assert_eq!(
    mask_db_url("postgres://user:secret@localhost/db"),
    "postgres://user:****@localhost/db"
);
assert_eq!(
    mask_db_url("sqlite:///path/to/db"),
    "sqlite:///path/to/db"
);
```

<details>
<summary>Source</summary>

```rust
pub fn mask_db_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let prefix = &url[..colon_pos + 1];
            let suffix = &url[at_pos..];
            return format!("{}****{}", prefix, suffix);
        }
    }
    url.to_string()
}
```

</details>
