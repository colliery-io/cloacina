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

//! The plugin interface version of a built Rust package (CLOACINA-T-1085).
//!
//! A Rust package is a library built against one version of the
//! `CloacinaPlugin` interface. fidius does not refuse a library of another
//! interface version at load: its load error names fidius ABI numbers or
//! interface hashes, and the workflow loaders do not check the hash. The
//! library's descriptor carries the number, so cloacina checks it here, and
//! the error names both versions.
//!
//! After a minor release that changes the interface, a package pinned to the
//! old minor rebuilds against the old crates and keeps the old interface. Its
//! team changes the pin and publishes again; the server lists such packages
//! ([`needs_new_build_reason`]).

use crate::fleet::PLUGIN_INTERFACE_VERSION;

/// The start of the error of [`check_plugin_interface`].
pub const INTERFACE_MISMATCH: &str = "plugin interface mismatch";

/// The error of a library built for `built_for`, opened by a build that
/// loads `loads`.
pub fn interface_mismatch(built_for: u32, loads: u32) -> String {
    format!(
        "{INTERFACE_MISMATCH}: the library was built for plugin interface {built_for}, \
         this build loads plugin interface {loads}"
    )
}

/// Refuse a library whose descriptor names another plugin interface version
/// than this build's [`PLUGIN_INTERFACE_VERSION`].
pub fn check_plugin_interface(library_interface: u32) -> Result<(), String> {
    if library_interface == PLUGIN_INTERFACE_VERSION {
        Ok(())
    } else {
        Err(interface_mismatch(
            library_interface,
            PLUGIN_INTERFACE_VERSION,
        ))
    }
}

/// The interface version a load error says the library was built for, when
/// it is the error of [`check_plugin_interface`]. `None` for any other
/// error: a library of another fidius ABI does not open, and its number is
/// not known.
pub fn built_for_of_error(message: &str) -> Option<u32> {
    let rest = message.split("built for plugin interface ").nth(1)?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Does this load failure mean the library was built for another interface
/// or another fidius ABI, so that a build from source against this
/// release's crates is what fixes it? The fidius texts are the ABI check
/// ("incompatible ABI version") and the hash check ("interface hash
/// mismatch"); the cloacina text is [`INTERFACE_MISMATCH`].
pub fn is_stale_artifact_error(message: &str) -> bool {
    message.contains("incompatible ABI version")
        || message.contains("interface hash mismatch")
        || message.contains(INTERFACE_MISMATCH)
}

/// The minor of this release (`0.13`), for the step a team takes.
pub fn release_minor() -> String {
    let release = cloacina_api_types::version::RELEASE;
    cloacina_api_types::version::Release::parse(release)
        .map(|r| r.minor_label())
        .unwrap_or_else(|_| release.to_string())
}

/// The text of a package that needs a new publish: what it was built for,
/// what this release loads, why its rebuild from kept source did not carry
/// it over, and the step.
pub fn needs_new_build_reason(built_for: Option<u32>, why: &str) -> String {
    let built = match built_for {
        Some(v) => format!("built for plugin interface {v}"),
        None => "built for another plugin interface or fidius ABI".to_string(),
    };
    format!(
        "{built}; this release loads plugin interface {PLUGIN_INTERFACE_VERSION}. The rebuild \
         from its kept source did not change that: {why}. Change the `cloacina` pin in the \
         package's Cargo.toml to \"{minor}\", update Cargo.lock, and publish the package again.",
        minor = release_minor()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mismatch_names_both_versions_and_reads_back() {
        let msg = interface_mismatch(7, 8);
        assert!(msg.contains("plugin interface 7"), "{msg}");
        assert!(msg.contains("plugin interface 8"), "{msg}");
        assert_eq!(built_for_of_error(&format!("load failed: {msg}")), Some(7));
        assert!(is_stale_artifact_error(&msg));
        assert!(check_plugin_interface(PLUGIN_INTERFACE_VERSION).is_ok());
        assert_eq!(
            check_plugin_interface(PLUGIN_INTERFACE_VERSION + 1).unwrap_err(),
            interface_mismatch(PLUGIN_INTERFACE_VERSION + 1, PLUGIN_INTERFACE_VERSION)
        );
    }

    #[test]
    fn the_stale_classifier_keeps_the_fidius_texts() {
        assert!(is_stale_artifact_error(
            "Failed to load library at /tmp/x.so: incompatible ABI version: got 200, expected 500"
        ));
        assert!(is_stale_artifact_error(
            "interface hash mismatch: got 0xdead, expected 0xbeef"
        ));
        assert!(!is_stale_artifact_error("No space left on device"));
        assert_eq!(
            built_for_of_error("incompatible ABI version: got 200, expected 500"),
            None
        );
    }

    #[test]
    fn the_reason_names_the_versions_and_the_step() {
        let reason = needs_new_build_reason(Some(7), "it resolved the old crates");
        assert!(reason.contains("built for plugin interface 7"), "{reason}");
        assert!(
            reason.contains(&format!(
                "loads plugin interface {PLUGIN_INTERFACE_VERSION}"
            )),
            "{reason}"
        );
        assert!(reason.contains(&release_minor()), "{reason}");
        assert!(reason.contains("publish the package again"), "{reason}");
    }
}
