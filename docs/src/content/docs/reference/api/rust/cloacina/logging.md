---
title: "cloacina::logging"
description: "API reference of the Rust module cloacina::logging."
editUrl: false
sidebar:
  order: 132
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Logging Configuration**

This module provides structured logging setup for Cloacina using the `tracing` ecosystem.
It supports both production and test environments with configurable log levels.

**Dependencies**

- `tracing`: Core logging framework
- `tracing-subscriber`: Subscriber implementation and formatting

**Features**

- **Structured Logging**: Uses `tracing` for structured, contextual logging
- **Environment Configuration**: Respects `RUST_LOG` environment variable
- **Test Support**: Special test logging that doesn't interfere with test output
- **Flexible Levels**: Support for all standard log levels (error, warn, info, debug, trace)
- **Thread Safety**: All logging operations are thread-safe
- **Zero Cost**: When logging is disabled, the compiler eliminates the logging code

**Usage**

**Production**

```rust
use cloacina::init_logging;
use tracing::Level;

// Initialize with default level (respects RUST_LOG env var)
init_logging(None);

// Or specify a level explicitly
init_logging(Some(Level::DEBUG));

// Log messages with context
tracing::info!(target: "my_module", "Processing request", request_id = "123");
```

**Testing**

In your test functions, call `init_test_logging()` at the start:

```rust
use cloacina::init_test_logging;

#[test]
fn my_test() {
    init_test_logging();
    // Your test code with logging
    tracing::debug!("Test debug message");
}
```

**Log Levels**

- `ERROR`: Critical errors that may cause system failure
- `WARN`: Warning conditions that don't stop execution
- `INFO`: General information about system operation
- `DEBUG`: Detailed diagnostic information
- `TRACE`: Very detailed diagnostic information

**Environment Variables**

The `RUST_LOG` environment variable supports various formats:

- `RUST_LOG=debug` - Enable debug logging for all modules
- `RUST_LOG=myapp=trace,other_crate=warn` - Fine-grained control per module
- `RUST_LOG=info,myapp::module=debug` - Mix of global and module-specific levels

**Performance Considerations**

- Logging is disabled at compile time for levels below the configured threshold
- Structured fields are only evaluated if the log level is enabled
- Test logging uses a special writer that minimizes impact on test performance

**Integration**

This logging system integrates with:

- Standard Rust logging macros (`tracing::info!`, `tracing::error!`, etc.)
- Structured field support for JSON logging
- Span-based context tracking
- Test frameworks for log verification

**Error Handling**

- Initialization errors are handled gracefully with fallback to default settings
- Invalid log levels in environment variables default to "info"
- Test logging initialization is idempotent and safe to call multiple times

## Functions

### <span id="fn.init_logging"></span><code style="font-size: 0.85em">fn init_​logging</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn init_logging(level: Option<Level>)
```

Initializes the logging system with the specified log level.

If no level is provided, it will use the `RUST_LOG` environment variable
or default to "info" if the environment variable is not set.

**Environment Variables**

The `RUST_LOG` environment variable can be used to control logging:
- `RUST_LOG=debug` - Enable debug logging
- `RUST_LOG=myapp=trace,other_crate=warn` - Fine-grained control

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `level` | `Option<Level>` | Optional log level to use. If `None`, uses environment configuration. |

**Examples:**

```rust
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

### <span id="fn.mask_db_url"></span><code style="font-size: 0.85em">fn mask_​db_​url</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn mask_db_url(url: &str) -> String
```

Mask the password in a database URL for safe logging.

Replaces the password portion (between the last `:` before `@` and the `@`)
with `****`. If the URL does not contain credentials, returns it unchanged.

**Examples:**

```rust
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
