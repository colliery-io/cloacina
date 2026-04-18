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

//! Resolves `GlobalOpts` + `CloacinaConfig` into a concrete `ClientContext`
//! that client-side commands use to hit the server.

use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;

use crate::commands::config::CloacinaConfig;
use crate::{GlobalOpts, OutputFormat};

/// Resolved client context — everything a client command needs to talk to the
/// server.
#[derive(Debug, Clone)]
pub struct ClientContext {
    pub server: String,
    pub api_key: String,
    pub tenant: Option<String>,
    pub output: OutputFormat,
    pub no_color: bool,
}

impl ClientContext {
    /// Resolve against the precedence rule from ADR-0003 §3:
    /// explicit flag > named profile > default profile > error.
    pub fn resolve(opts: &GlobalOpts, config: &CloacinaConfig) -> Result<Self> {
        let profile_name = opts
            .profile
            .as_deref()
            .or(config.default_profile.as_deref());
        let profile = profile_name.and_then(|n| config.profiles.get(n));

        let server = opts
            .server
            .clone()
            .or_else(|| profile.map(|p| p.server.clone()))
            .ok_or_else(|| {
                anyhow!(
                    "no server configured. Pass --server <URL>, --profile <NAME>, or set \
                     default_profile in ~/.cloacina/config.toml."
                )
            })?;

        let api_key_raw = opts
            .api_key
            .clone()
            .or_else(|| profile.map(|p| p.api_key.clone()))
            .ok_or_else(|| {
                anyhow!(
                    "no API key configured. Pass --api-key, --profile, or set default_profile \
                     in ~/.cloacina/config.toml."
                )
            })?;

        let api_key = resolve_api_key_scheme(&api_key_raw)?;

        Ok(Self {
            server,
            api_key,
            tenant: opts.tenant.clone(),
            output: opts.effective_output(),
            no_color: opts.no_color,
        })
    }
}

/// Resolve an api-key value that may carry a scheme prefix.
pub fn resolve_api_key_scheme(raw: &str) -> Result<String> {
    if let Some(var) = raw.strip_prefix("env:") {
        std::env::var(var).with_context(|| {
            format!("api key references env var {var} but it is not set in the current environment")
        })
    } else if let Some(path) = raw.strip_prefix("file:") {
        read_key_file(Path::new(path))
    } else if raw.starts_with("keyring:") {
        bail!(
            "keyring: scheme is deferred to v1.1 (CLOACI-I-0098 goals §non-goals). Use env: or \
             file: for now."
        )
    } else {
        Ok(raw.to_string())
    }
}

fn read_key_file(path: &Path) -> Result<String> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read API key file {}", path.display()))?;
    // First non-empty line, trimmed.
    let line = contents
        .lines()
        .find(|l| !l.trim().is_empty())
        .ok_or_else(|| anyhow!("API key file {} is empty", path.display()))?;
    Ok(line.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::config::Profile;
    use tempfile::NamedTempFile;

    fn opts(overrides: impl FnOnce(&mut GlobalOpts)) -> GlobalOpts {
        let mut o = GlobalOpts {
            verbose: false,
            home: std::path::PathBuf::new(),
            profile: None,
            server: None,
            api_key: None,
            tenant: None,
            json: false,
            output: None,
            no_color: false,
        };
        overrides(&mut o);
        o
    }

    #[test]
    fn explicit_flag_wins() {
        let mut config = CloacinaConfig::default();
        config.default_profile = Some("p1".into());
        config.profiles.insert(
            "p1".into(),
            Profile {
                server: "http://from-profile".into(),
                api_key: "profile-key".into(),
            },
        );
        let o = opts(|o| {
            o.server = Some("http://from-flag".into());
            o.api_key = Some("flag-key".into());
        });
        let ctx = ClientContext::resolve(&o, &config).unwrap();
        assert_eq!(ctx.server, "http://from-flag");
        assert_eq!(ctx.api_key, "flag-key");
    }

    #[test]
    fn named_profile_wins_over_default() {
        let mut config = CloacinaConfig::default();
        config.default_profile = Some("a".into());
        config.profiles.insert(
            "a".into(),
            Profile {
                server: "http://a".into(),
                api_key: "k-a".into(),
            },
        );
        config.profiles.insert(
            "b".into(),
            Profile {
                server: "http://b".into(),
                api_key: "k-b".into(),
            },
        );
        let o = opts(|o| o.profile = Some("b".into()));
        let ctx = ClientContext::resolve(&o, &config).unwrap();
        assert_eq!(ctx.server, "http://b");
    }

    #[test]
    fn no_config_errors() {
        let config = CloacinaConfig::default();
        let o = opts(|_| {});
        assert!(ClientContext::resolve(&o, &config).is_err());
    }

    #[test]
    fn env_scheme() {
        std::env::set_var("CLOACI_TEST_KEY_12345", "secret-from-env");
        let v = resolve_api_key_scheme("env:CLOACI_TEST_KEY_12345").unwrap();
        assert_eq!(v, "secret-from-env");
        std::env::remove_var("CLOACI_TEST_KEY_12345");
    }

    #[test]
    fn file_scheme() {
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), "secret-from-file\n").unwrap();
        let v = resolve_api_key_scheme(&format!("file:{}", f.path().display())).unwrap();
        assert_eq!(v, "secret-from-file");
    }

    #[test]
    fn keyring_scheme_deferred() {
        let err = resolve_api_key_scheme("keyring:foo").unwrap_err();
        assert!(err.to_string().contains("deferred"));
    }
}
