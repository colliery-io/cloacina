/*
 *  Copyright 2026 Colliery Software
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

//! Variable registry for external connections and configuration.
//!
//! Provides a type-agnostic variable system using the `CLOACINA_VAR_{NAME}`
//! environment variable convention. Similar to Airflow's connection/variable
//! system — external connections, secrets, and config values are referenced
//! by name and resolved from env vars at runtime.
//!
//! # Convention
//!
//! ```text
//! CLOACINA_VAR_KAFKA_BROKER=localhost:9092
//! CLOACINA_VAR_ANALYTICS_DB=postgres://user:pass@host/db
//! CLOACINA_VAR_API_KEY=abc123
//! CLOACINA_VAR_MODEL_THRESHOLD=0.85
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use cloacina::var;
//!
//! let broker = var("KAFKA_BROKER")?;
//! let threshold: f64 = var_or("MODEL_THRESHOLD", "0.5").parse().unwrap();
//! ```
//!
//! # Template Resolution
//!
//! Package metadata can reference variables with `{{ VAR_NAME }}` syntax:
//!
//! ```toml
//! [[metadata.accumulators]]
//! broker = "{{ KAFKA_BROKER }}"
//! ```
//!
//! Use [`resolve_template`] to expand these references.

use std::fmt;

const PREFIX: &str = "CLOACINA_VAR_";

/// Error returned when a required variable is not found.
#[derive(Debug, Clone)]
pub struct VarNotFound {
    /// The variable name (without the `CLOACINA_VAR_` prefix).
    pub name: String,
}

impl fmt::Display for VarNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "required variable '{}' not set (expected env var {}{})",
            self.name, PREFIX, self.name
        )
    }
}

impl std::error::Error for VarNotFound {}

/// Resolve a variable by name from `CLOACINA_VAR_{NAME}`.
///
/// # Arguments
/// * `name` - Variable name without the prefix (e.g., `"KAFKA_BROKER"`)
///
/// # Returns
/// The variable value, or `VarNotFound` if not set.
///
/// # Example
/// ```rust,ignore
/// // With CLOACINA_VAR_KAFKA_BROKER=localhost:9092
/// let broker = cloacina::var("KAFKA_BROKER")?;
/// assert_eq!(broker, "localhost:9092");
/// ```
pub fn var(name: &str) -> Result<String, VarNotFound> {
    let env_key = format!("{}{}", PREFIX, name);
    std::env::var(&env_key).map_err(|_| VarNotFound {
        name: name.to_string(),
    })
}

/// Resolve a variable by name, returning a default if not set.
///
/// # Arguments
/// * `name` - Variable name without the prefix (e.g., `"MODEL_THRESHOLD"`)
/// * `default` - Default value if the variable is not set
///
/// # Example
/// ```rust,ignore
/// let threshold = cloacina::var_or("MODEL_THRESHOLD", "0.5");
/// ```
pub fn var_or(name: &str, default: &str) -> String {
    var(name).unwrap_or_else(|_| default.to_string())
}

/// Resolve template references in a string, replacing `{{ VAR_NAME }}`
/// with the corresponding `CLOACINA_VAR_{VAR_NAME}` value.
///
/// Unresolved references (missing env vars) are returned as errors
/// with the list of missing variable names.
///
/// # Example
/// ```rust,ignore
/// // With CLOACINA_VAR_KAFKA_BROKER=localhost:9092
/// let resolved = cloacina::var::resolve_template("broker={{ KAFKA_BROKER }}")?;
/// assert_eq!(resolved, "broker=localhost:9092");
/// ```
pub fn resolve_template(input: &str) -> Result<String, Vec<VarNotFound>> {
    let mut result = String::with_capacity(input.len());
    let mut missing = Vec::new();
    let mut rest = input;

    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];

        if let Some(end) = after_open.find("}}") {
            let var_name = after_open[..end].trim();
            match var(var_name) {
                Ok(value) => result.push_str(&value),
                Err(e) => {
                    // Keep the original placeholder so caller can see what failed
                    result.push_str(&rest[start..start + 2 + end + 2]);
                    missing.push(e);
                }
            }
            rest = &after_open[end + 2..];
        } else {
            // Unclosed {{ — copy literally
            result.push_str(&rest[start..]);
            rest = "";
        }
    }
    result.push_str(rest);

    if missing.is_empty() {
        Ok(result)
    } else {
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_found() {
        std::env::set_var("CLOACINA_VAR_TEST_VAR_1", "hello");
        assert_eq!(var("TEST_VAR_1").unwrap(), "hello");
        std::env::remove_var("CLOACINA_VAR_TEST_VAR_1");
    }

    #[test]
    fn test_var_not_found() {
        let err = var("NONEXISTENT_VAR_12345").unwrap_err();
        assert_eq!(err.name, "NONEXISTENT_VAR_12345");
        assert!(err
            .to_string()
            .contains("CLOACINA_VAR_NONEXISTENT_VAR_12345"));
    }

    #[test]
    fn test_var_or_found() {
        std::env::set_var("CLOACINA_VAR_TEST_VAR_2", "value");
        assert_eq!(var_or("TEST_VAR_2", "default"), "value");
        std::env::remove_var("CLOACINA_VAR_TEST_VAR_2");
    }

    #[test]
    fn test_var_or_default() {
        assert_eq!(var_or("MISSING_VAR_67890", "fallback"), "fallback");
    }

    #[test]
    fn test_resolve_template_simple() {
        std::env::set_var("CLOACINA_VAR_TMPL_HOST", "localhost");
        std::env::set_var("CLOACINA_VAR_TMPL_PORT", "9092");

        let result = resolve_template("{{ TMPL_HOST }}:{{ TMPL_PORT }}").unwrap();
        assert_eq!(result, "localhost:9092");

        std::env::remove_var("CLOACINA_VAR_TMPL_HOST");
        std::env::remove_var("CLOACINA_VAR_TMPL_PORT");
    }

    #[test]
    fn test_resolve_template_no_placeholders() {
        assert_eq!(resolve_template("plain text").unwrap(), "plain text");
    }

    #[test]
    fn test_resolve_template_missing_var() {
        let err = resolve_template("broker={{ MISSING_TMPL_VAR }}").unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].name, "MISSING_TMPL_VAR");
    }

    #[test]
    fn test_resolve_template_mixed() {
        std::env::set_var("CLOACINA_VAR_TMPL_FOUND", "yes");

        let err = resolve_template("a={{ TMPL_FOUND }},b={{ TMPL_MISSING }}").unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].name, "TMPL_MISSING");

        std::env::remove_var("CLOACINA_VAR_TMPL_FOUND");
    }

    #[test]
    fn test_resolve_template_whitespace_trimmed() {
        std::env::set_var("CLOACINA_VAR_TMPL_WS", "trimmed");
        let result = resolve_template("{{  TMPL_WS  }}").unwrap();
        assert_eq!(result, "trimmed");
        std::env::remove_var("CLOACINA_VAR_TMPL_WS");
    }
}
