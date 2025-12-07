/*
 *  Copyright 2025 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! PostgreSQL schema validation to prevent SQL injection.

use thiserror::Error;

// =============================================================================
// Schema Validation
// =============================================================================

/// Maximum length for PostgreSQL schema names (NAMEDATALEN - 1).
const MAX_SCHEMA_NAME_LENGTH: usize = 63;

/// Reserved PostgreSQL schema names that cannot be used.
const RESERVED_SCHEMA_NAMES: &[&str] = &["public", "pg_catalog", "information_schema", "pg_temp"];

/// Errors that can occur during schema name validation.
///
/// These errors are returned when a schema name fails validation checks
/// designed to prevent SQL injection attacks.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// Schema name is empty or exceeds the maximum length.
    #[error("Schema name length invalid: '{name}' (must be 1-{max} characters)")]
    InvalidLength { name: String, max: usize },

    /// Schema name does not start with a letter or underscore.
    #[error("Schema name must start with a letter or underscore: '{0}'")]
    InvalidStart(String),

    /// Schema name contains characters other than alphanumeric or underscore.
    #[error(
        "Schema name contains invalid characters (only alphanumeric and underscore allowed): '{0}'"
    )]
    InvalidCharacters(String),

    /// Schema name is a reserved PostgreSQL name.
    #[error("Schema name is reserved: '{0}'")]
    ReservedName(String),
}

/// Validates a PostgreSQL schema name to prevent SQL injection.
///
/// This function enforces PostgreSQL identifier naming rules:
/// - Length must be between 1 and 63 characters
/// - Must start with a letter (a-z, A-Z) or underscore
/// - Subsequent characters must be alphanumeric or underscore
/// - Cannot be a reserved PostgreSQL schema name
///
/// # Arguments
/// * `name` - The schema name to validate
///
/// # Returns
/// * `Ok(&str)` - The validated schema name (zero-copy)
/// * `Err(SchemaError)` - Description of the validation failure
///
/// # Example
/// ```
/// use cloacina::database::connection::validate_schema_name;
///
/// assert!(validate_schema_name("my_schema").is_ok());
/// assert!(validate_schema_name("tenant_123").is_ok());
/// assert!(validate_schema_name("public").is_err()); // Reserved
/// assert!(validate_schema_name("123abc").is_err()); // Starts with number
/// assert!(validate_schema_name("my-schema").is_err()); // Contains hyphen
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_schema_names() {
        // Simple valid names
        assert!(validate_schema_name("my_schema").is_ok());
        assert!(validate_schema_name("tenant_123").is_ok());
        assert!(validate_schema_name("MySchema").is_ok());

        // Starting with underscore
        assert!(validate_schema_name("_private").is_ok());
        assert!(validate_schema_name("_123").is_ok());

        // Single character
        assert!(validate_schema_name("a").is_ok());
        assert!(validate_schema_name("_").is_ok());

        // Maximum length (63 characters)
        let max_name = "a".repeat(63);
        assert!(validate_schema_name(&max_name).is_ok());
    }

    #[test]
    fn test_sql_injection_attempts_rejected() {
        // Command injection with semicolon
        assert!(matches!(
            validate_schema_name("test; DROP TABLE users; --"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Quote injection
        assert!(matches!(
            validate_schema_name("test' OR '1'='1"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Comment injection
        assert!(matches!(
            validate_schema_name("test/*comment*/"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Double dash comment
        assert!(matches!(
            validate_schema_name("test--comment"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Parentheses
        assert!(matches!(
            validate_schema_name("test()"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Equals sign
        assert!(matches!(
            validate_schema_name("test=1"),
            Err(SchemaError::InvalidCharacters(_))
        ));
    }

    #[test]
    fn test_invalid_length() {
        // Empty string
        assert!(matches!(
            validate_schema_name(""),
            Err(SchemaError::InvalidLength { .. })
        ));

        // Too long (64 characters)
        let too_long = "a".repeat(64);
        assert!(matches!(
            validate_schema_name(&too_long),
            Err(SchemaError::InvalidLength { .. })
        ));

        // Way too long
        let way_too_long = "a".repeat(1000);
        assert!(matches!(
            validate_schema_name(&way_too_long),
            Err(SchemaError::InvalidLength { .. })
        ));
    }

    #[test]
    fn test_invalid_start_character() {
        // Starting with number
        assert!(matches!(
            validate_schema_name("123abc"),
            Err(SchemaError::InvalidStart(_))
        ));

        // Starting with hyphen
        assert!(matches!(
            validate_schema_name("-schema"),
            Err(SchemaError::InvalidStart(_))
        ));

        // Starting with dot
        assert!(matches!(
            validate_schema_name(".schema"),
            Err(SchemaError::InvalidStart(_))
        ));

        // Starting with space
        assert!(matches!(
            validate_schema_name(" schema"),
            Err(SchemaError::InvalidStart(_))
        ));
    }

    #[test]
    fn test_invalid_characters() {
        // Hyphen
        assert!(matches!(
            validate_schema_name("my-schema"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Dot
        assert!(matches!(
            validate_schema_name("my.schema"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Space
        assert!(matches!(
            validate_schema_name("my schema"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Special characters
        assert!(matches!(
            validate_schema_name("schema@test"),
            Err(SchemaError::InvalidCharacters(_))
        ));
        assert!(matches!(
            validate_schema_name("schema#1"),
            Err(SchemaError::InvalidCharacters(_))
        ));
        assert!(matches!(
            validate_schema_name("schema$"),
            Err(SchemaError::InvalidCharacters(_))
        ));
    }

    #[test]
    fn test_reserved_names() {
        // Reserved names (case-insensitive)
        assert!(matches!(
            validate_schema_name("public"),
            Err(SchemaError::ReservedName(_))
        ));
        assert!(matches!(
            validate_schema_name("PUBLIC"),
            Err(SchemaError::ReservedName(_))
        ));
        assert!(matches!(
            validate_schema_name("Public"),
            Err(SchemaError::ReservedName(_))
        ));

        assert!(matches!(
            validate_schema_name("pg_catalog"),
            Err(SchemaError::ReservedName(_))
        ));
        assert!(matches!(
            validate_schema_name("PG_CATALOG"),
            Err(SchemaError::ReservedName(_))
        ));

        assert!(matches!(
            validate_schema_name("information_schema"),
            Err(SchemaError::ReservedName(_))
        ));
        assert!(matches!(
            validate_schema_name("INFORMATION_SCHEMA"),
            Err(SchemaError::ReservedName(_))
        ));

        assert!(matches!(
            validate_schema_name("pg_temp"),
            Err(SchemaError::ReservedName(_))
        ));
    }

    #[test]
    fn test_schema_error_display() {
        // Verify error messages are informative
        let err = validate_schema_name("").unwrap_err();
        assert!(err.to_string().contains("length"));

        let err = validate_schema_name("123abc").unwrap_err();
        assert!(err.to_string().contains("start"));

        let err = validate_schema_name("my-schema").unwrap_err();
        assert!(err.to_string().contains("invalid characters"));

        let err = validate_schema_name("public").unwrap_err();
        assert!(err.to_string().contains("reserved"));
    }

    #[test]
    fn test_unicode_characters_rejected() {
        // Unicode Greek letter alpha
        assert!(matches!(
            validate_schema_name("schema_\u{03B1}"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Unicode snowman emoji
        assert!(matches!(
            validate_schema_name("schema_\u{2603}"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Non-ASCII e with acute accent (cafe with accented e)
        assert!(matches!(
            validate_schema_name("caf\u{00E9}"),
            Err(SchemaError::InvalidCharacters(_))
        ));

        // Chinese character
        assert!(matches!(
            validate_schema_name("schema_\u{4E2D}"),
            Err(SchemaError::InvalidCharacters(_))
        ));
    }
}
