# cloacina::database::connection::schema_validation <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


PostgreSQL identifier validation to prevent SQL injection.

This module provides validation functions for PostgreSQL identifiers
(schema names, usernames, etc.) to ensure they cannot be used for SQL injection.

## Enums

### `cloacina::database::connection::schema_validation::SchemaError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during schema name validation.

These errors are returned when a schema name fails validation checks
designed to prevent SQL injection attacks.

#### Variants

- **`InvalidLength`** - Schema name is empty or exceeds the maximum length.
- **`InvalidStart`** - Schema name does not start with a letter or underscore.
- **`InvalidCharacters`** - Schema name contains characters other than alphanumeric or underscore.
- **`ReservedName`** - Schema name is a reserved PostgreSQL name.



### `cloacina::database::connection::schema_validation::UsernameError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during username validation.

These errors are returned when a username fails validation checks
designed to prevent SQL injection attacks.

#### Variants

- **`InvalidLength`** - Username is empty or exceeds the maximum length.
- **`InvalidStart`** - Username does not start with a letter or underscore.
- **`InvalidCharacters`** - Username contains characters other than alphanumeric or underscore.
- **`ReservedName`** - Username is a reserved PostgreSQL role name.



## Functions

### `cloacina::database::connection::schema_validation::validate_schema_name`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_schema_name (name : & str) -> Result < & str , SchemaError >
```

Validates a PostgreSQL schema name to prevent SQL injection.

This function enforces PostgreSQL identifier naming rules:
- Length must be between 1 and 63 characters
- Must start with a letter (a-z, A-Z) or underscore
- Subsequent characters must be alphanumeric or underscore
- Cannot be a reserved PostgreSQL schema name

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `-` | The schema name to validate |


**Returns:**

* `Ok(&str)` - The validated schema name (zero-copy) * `Err(SchemaError)` - Description of the validation failure

**Examples:**

```
use cloacina::database::connection::validate_schema_name;

assert!(validate_schema_name("my_schema").is_ok());
assert!(validate_schema_name("tenant_123").is_ok());
assert!(validate_schema_name("public").is_err()); // Reserved
assert!(validate_schema_name("123abc").is_err()); // Starts with number
assert!(validate_schema_name("my-schema").is_err()); // Contains hyphen
```

<details>
<summary>Source</summary>

```rust
pub fn validate_schema_name(name: &str) -> Result<&str, SchemaError> {
    // Check length
    if name.is_empty() || name.len() > MAX_SCHEMA_NAME_LENGTH {
        return Err(SchemaError::InvalidLength {
            name: name.to_string(),
            max: MAX_SCHEMA_NAME_LENGTH,
        });
    }

    // Must start with letter or underscore
    let first_char = name.chars().next().unwrap(); // Safe: we checked non-empty above
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(SchemaError::InvalidStart(name.to_string()));
    }

    // Only allow alphanumeric and underscore
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(SchemaError::InvalidCharacters(name.to_string()));
    }

    // Reject reserved names (case-insensitive)
    let lower_name = name.to_lowercase();
    if RESERVED_SCHEMA_NAMES.contains(&lower_name.as_str()) {
        return Err(SchemaError::ReservedName(name.to_string()));
    }

    Ok(name)
}
```

</details>



### `cloacina::database::connection::schema_validation::validate_username`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_username (name : & str) -> Result < & str , UsernameError >
```

Validates a PostgreSQL username to prevent SQL injection.

This function enforces PostgreSQL identifier naming rules:
- Length must be between 1 and 63 characters
- Must start with a letter (a-z, A-Z) or underscore
- Subsequent characters must be alphanumeric or underscore
- Cannot be a reserved PostgreSQL role name

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `-` | The username to validate |


**Returns:**

* `Ok(&str)` - The validated username (zero-copy) * `Err(UsernameError)` - Description of the validation failure

**Examples:**

```
use cloacina::database::connection::validate_username;

assert!(validate_username("tenant_user").is_ok());
assert!(validate_username("acme_admin").is_ok());
assert!(validate_username("postgres").is_err()); // Reserved
assert!(validate_username("123user").is_err()); // Starts with number
assert!(validate_username("user; DROP TABLE users; --").is_err()); // SQL injection
```

<details>
<summary>Source</summary>

```rust
pub fn validate_username(name: &str) -> Result<&str, UsernameError> {
    // Check length (same as schema names - PostgreSQL NAMEDATALEN)
    if name.is_empty() || name.len() > MAX_SCHEMA_NAME_LENGTH {
        return Err(UsernameError::InvalidLength {
            name: name.to_string(),
            max: MAX_SCHEMA_NAME_LENGTH,
        });
    }

    // Must start with letter or underscore
    let first_char = name.chars().next().unwrap(); // Safe: we checked non-empty above
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return Err(UsernameError::InvalidStart(name.to_string()));
    }

    // Only allow alphanumeric and underscore
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(UsernameError::InvalidCharacters(name.to_string()));
    }

    // Reject reserved names (case-insensitive)
    let lower_name = name.to_lowercase();
    if RESERVED_USERNAMES.contains(&lower_name.as_str()) {
        return Err(UsernameError::ReservedName(name.to_string()));
    }

    Ok(name)
}
```

</details>



### `cloacina::database::connection::schema_validation::escape_password`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn escape_password (password : & str) -> String
```

Escapes a password string for safe use in PostgreSQL SQL statements.

This function escapes single quotes by doubling them, which is the
standard PostgreSQL escaping mechanism for string literals.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `password` | `-` | The password to escape |


**Returns:**

A new String with all single quotes escaped

**Examples:**

```
use cloacina::database::connection::escape_password;

assert_eq!(escape_password("simple"), "simple");
assert_eq!(escape_password("pass'word"), "pass''word");
assert_eq!(escape_password("it's a test"), "it''s a test");
```

<details>
<summary>Source</summary>

```rust
pub fn escape_password(password: &str) -> String {
    password.replace('\'', "''")
}
```

</details>
