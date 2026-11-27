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

//! Where tenant code runs: the single-trust setting (CLOACINA-T-1075).
//!
//! On a server, tenant code runs only on the tenant's agents and in the
//! tenant's compiler (CLOACINA-A-0019). By default the server loads each
//! package from the metadata document its compiler stored, opens no tenant
//! library and imports no tenant module, and sends every task to the agent
//! fleet.
//!
//! `--single-trust` (`CLOACINA_SINGLE_TRUST`) is for a server whose every
//! tenant is trusted as much as the server itself: one team, one machine.
//! It turns on both things that run tenant code in the server process: the
//! in-process load of packages and the in-process executor.

use cloacina::registry::PackageLoadMode;

/// The executor key of the in-process thread executor.
pub const IN_PROCESS_EXECUTOR: &str = "default";
/// The executor key of the agent fleet.
pub const FLEET_EXECUTOR: &str = "fleet";

/// How this server runs tenant code, from its settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionMode {
    /// The single-trust setting.
    pub single_trust: bool,
    /// The executor every task is sent to.
    pub default_executor: String,
    /// How the reconcilers load packages.
    pub package_load_mode: PackageLoadMode,
}

impl ExecutionMode {
    /// The mode of a server with `single_trust` and `--default-executor`
    /// (`None`: not set).
    ///
    /// - Single trust off: packages load from metadata only, and the
    ///   executor is `fleet`. An executor that is set to anything else is
    ///   refused, so the server does not start.
    /// - Single trust on: packages load in process, as before this setting;
    ///   the executor is `default` unless set.
    pub fn resolve(single_trust: bool, default_executor: Option<&str>) -> Result<Self, String> {
        let executor = default_executor
            .map(str::trim)
            .filter(|key| !key.is_empty());
        if single_trust {
            return Ok(Self {
                single_trust,
                default_executor: executor.unwrap_or(IN_PROCESS_EXECUTOR).to_string(),
                package_load_mode: PackageLoadMode::InProcess,
            });
        }
        match executor {
            None | Some(FLEET_EXECUTOR) => Ok(Self {
                single_trust,
                default_executor: FLEET_EXECUTOR.to_string(),
                package_load_mode: PackageLoadMode::MetadataOnly,
            }),
            Some(other) => Err(format!(
                "--default-executor '{other}' needs --single-trust (CLOACINA_SINGLE_TRUST=true). \
                 Without single trust this server runs no tenant code in its process: it loads \
                 packages from their metadata and sends every task to the agent fleet. Leave \
                 --default-executor unset or set it to 'fleet' and run agents for each tenant, \
                 or set --single-trust when every tenant is trusted as much as the server"
            )),
        }
    }

    /// The line the server logs at start about where tenant code runs.
    pub fn start_line(&self) -> String {
        if self.single_trust {
            format!(
                "single trust is on: tenant code runs in this server process (packages are \
                 loaded in process; executor '{}'). Every tenant can reach what this process \
                 holds: the database, the secrets key and the other tenants' data",
                self.default_executor
            )
        } else {
            "packages load from their metadata only; this server runs no tenant code, and \
             every task goes to the agent fleet"
                .to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Default settings: metadata only, and the fleet.
    #[test]
    fn with_no_setting_the_server_needs_a_fleet() {
        let mode = ExecutionMode::resolve(false, None).unwrap();
        assert_eq!(mode.default_executor, "fleet");
        assert_eq!(mode.package_load_mode, PackageLoadMode::MetadataOnly);
        assert!(!mode.single_trust);
        assert_eq!(ExecutionMode::resolve(false, Some("fleet")).unwrap(), mode);
        assert_eq!(ExecutionMode::resolve(false, Some("  ")).unwrap(), mode);
    }

    /// The in-process executor with no single trust stops the start, with a
    /// message that names the setting.
    #[test]
    fn the_in_process_executor_needs_single_trust() {
        let err = ExecutionMode::resolve(false, Some("default")).unwrap_err();
        assert!(err.contains("--single-trust"), "{err}");
        assert!(err.contains("CLOACINA_SINGLE_TRUST"), "{err}");
        assert!(err.contains("'fleet'"), "{err}");
        assert!(ExecutionMode::resolve(false, Some("nope")).is_err());
    }

    /// Single trust: today's behaviour, and the start line says so.
    #[test]
    fn single_trust_loads_in_process() {
        let mode = ExecutionMode::resolve(true, None).unwrap();
        assert_eq!(mode.default_executor, "default");
        assert_eq!(mode.package_load_mode, PackageLoadMode::InProcess);
        assert!(
            mode.start_line()
                .contains("tenant code runs in this server process")
        );

        let mode = ExecutionMode::resolve(true, Some("fleet")).unwrap();
        assert_eq!(mode.default_executor, "fleet");
        assert_eq!(mode.package_load_mode, PackageLoadMode::InProcess);
    }
}
