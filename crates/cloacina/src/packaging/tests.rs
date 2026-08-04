/*
 *  Copyright 2025-2026 Colliery Software
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

//! Unit tests for packaging functionality

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::*;

    #[test]
    fn test_compile_options_builder_pattern() {
        let options = CompileOptions {
            target: Some("aarch64-apple-darwin".to_string()),
            profile: "release".to_string(),
            cargo_flags: vec!["--features".to_string(), "postgres".to_string()],
            jobs: Some(8),
        };

        assert_eq!(options.target.as_ref().unwrap(), "aarch64-apple-darwin");
        assert_eq!(options.profile, "release");
        assert_eq!(options.cargo_flags.len(), 2);
        assert_eq!(options.jobs.unwrap(), 8);
    }

    /// CLOACI-T-0918: `parse_duration_str` moved here from the deleted
    /// test-only `manifest_schema` module (it was its one production-used
    /// piece — trigger poll intervals).
    #[test]
    fn test_parse_duration_str_units() {
        use std::time::Duration;
        assert_eq!(
            parse_duration_str("100ms").unwrap(),
            Duration::from_millis(100)
        );
        assert_eq!(parse_duration_str("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration_str("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration_str("2h").unwrap(), Duration::from_secs(7200));
        assert!(parse_duration_str("").is_err());
        assert!(parse_duration_str("30").is_err());
        assert!(parse_duration_str("abc").is_err());
        assert!(parse_duration_str("5d").is_err());
    }

    #[test]
    fn test_constants() {
        assert_eq!(types::MANIFEST_FILENAME, "manifest.json");
        assert!(!types::CLOACINA_VERSION.is_empty());

        // Verify version follows semver format
        let version_parts: Vec<&str> = types::CLOACINA_VERSION.split('.').collect();
        assert!(
            version_parts.len() >= 2,
            "Version should have at least major.minor"
        );

        // Each part should be numeric
        for part in version_parts.iter().take(2) {
            assert!(
                part.parse::<u32>().is_ok(),
                "Version parts should be numeric: {}",
                part
            );
        }
    }
}
