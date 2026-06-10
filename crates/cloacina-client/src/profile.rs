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

//! `cloacinactl` profile interop — read `~/.cloacina/config.toml` and
//! resolve API-key schemes (`env:`, `file:`) exactly like the CLI does
//! (moved from `cloacinactl/src/shared/client_ctx.rs` in T-0646).

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::error::ClientError;
use crate::ClientBuilder;

#[derive(Debug, Default, Deserialize)]
struct CloacinactlConfig {
    #[serde(default)]
    default_profile: Option<String>,
    #[serde(default)]
    profiles: HashMap<String, Profile>,
}

#[derive(Debug, Deserialize)]
struct Profile {
    server: String,
    api_key: String,
}

pub(crate) fn builder_from_profile(
    home: Option<&Path>,
    profile: Option<&str>,
) -> Result<ClientBuilder, ClientError> {
    let home = match home {
        Some(h) => h.to_path_buf(),
        None => dirs::home_dir()
            .ok_or_else(|| ClientError::Config("cannot determine home directory".into()))?
            .join(".cloacina"),
    };
    let config_path = home.join("config.toml");
    let raw = std::fs::read_to_string(&config_path).map_err(|e| {
        ClientError::Config(format!("failed to read {}: {e}", config_path.display()))
    })?;
    let config: CloacinactlConfig = toml::from_str(&raw).map_err(|e| {
        ClientError::Config(format!("failed to parse {}: {e}", config_path.display()))
    })?;

    let name = profile
        .map(str::to_string)
        .or(config.default_profile.clone())
        .ok_or_else(|| {
            ClientError::Config(format!(
                "no profile named and no default_profile in {}",
                config_path.display()
            ))
        })?;
    let p = config.profiles.get(&name).ok_or_else(|| {
        ClientError::Config(format!(
            "profile '{name}' not found in {}",
            config_path.display()
        ))
    })?;

    Ok(ClientBuilder::new(&p.server).api_key(resolve_api_key_scheme(&p.api_key)?))
}

/// Resolve an api-key value that may carry a scheme prefix —
/// `env:VAR`, `file:/path`, or a literal key. `keyring:` is deferred
/// (CLOACI-I-0098 non-goals).
pub fn resolve_api_key_scheme(raw: &str) -> Result<String, ClientError> {
    if let Some(var) = raw.strip_prefix("env:") {
        std::env::var(var).map_err(|_| {
            ClientError::Config(format!(
                "api key references env var {var} but it is not set in the current environment"
            ))
        })
    } else if let Some(path) = raw.strip_prefix("file:") {
        read_key_file(Path::new(path))
    } else if raw.starts_with("keyring:") {
        Err(ClientError::Config(
            "keyring: scheme is deferred to v1.1 (CLOACI-I-0098 goals §non-goals). Use env: or \
             file: for now."
                .into(),
        ))
    } else {
        Ok(raw.to_string())
    }
}

fn read_key_file(path: &Path) -> Result<String, ClientError> {
    let contents = std::fs::read_to_string(path).map_err(|e| {
        ClientError::Config(format!(
            "failed to read API key file {}: {e}",
            path.display()
        ))
    })?;
    let line = contents
        .lines()
        .find(|l| !l.trim().is_empty())
        .ok_or_else(|| ClientError::Config(format!("API key file {} is empty", path.display())))?;
    Ok(line.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_scheme() {
        std::env::set_var("CLOACI_CLIENT_TEST_KEY", "secret-from-env");
        assert_eq!(
            resolve_api_key_scheme("env:CLOACI_CLIENT_TEST_KEY").unwrap(),
            "secret-from-env"
        );
        std::env::remove_var("CLOACI_CLIENT_TEST_KEY");
    }

    #[test]
    fn file_scheme() {
        let f = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(f.path(), "secret-from-file\n").unwrap();
        assert_eq!(
            resolve_api_key_scheme(&format!("file:{}", f.path().display())).unwrap(),
            "secret-from-file"
        );
    }

    #[test]
    fn keyring_scheme_deferred() {
        let err = resolve_api_key_scheme("keyring:foo").unwrap_err();
        assert!(err.to_string().contains("deferred"));
    }

    #[test]
    fn profile_resolution() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.toml"),
            r#"
default_profile = "dev"

[profiles.dev]
server = "http://localhost:8080"
api_key = "clk_test"
"#,
        )
        .unwrap();
        let builder = builder_from_profile(Some(dir.path()), None).unwrap();
        let client = builder.build().unwrap();
        assert_eq!(client.server(), "http://localhost:8080");
    }
}
