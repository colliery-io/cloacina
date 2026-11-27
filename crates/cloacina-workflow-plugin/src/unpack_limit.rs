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

//! The limit of an unpacked package archive (CLOACINA-T-1048).
//!
//! A package archive (`.cloacina`) is unpacked by the server at upload, by
//! the compiler, by the loader on the server, the agent and the daemon, and
//! by `cloacinactl package validate`. Each of them refuses an archive that
//! unpacks to more than this limit: 500 MiB when not set.
//!
//! The setting is `--max-unpacked-bytes` / `CLOACINA_MAX_UNPACKED_BYTES`.
//! A binary with the flag calls [`set_max_unpacked_bytes`] at start. A
//! process that does not (an embedded engine, the Python bindings) reads the
//! environment variable at each unpack.
//!
//! Two more limits of the archive library stay as they are: 10,000 entries,
//! and 10 times the size of the archive.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use fidius_core::package::{PackageError, UnpackOptions};

/// The limit when the setting is not given: 500 MiB.
pub const DEFAULT_MAX_UNPACKED_BYTES: u64 = 500 * 1024 * 1024;

/// The environment variable of the setting.
pub const MAX_UNPACKED_BYTES_ENV: &str = "CLOACINA_MAX_UNPACKED_BYTES";

/// `0` means: not set in this process.
static MAX_UNPACKED_BYTES: AtomicU64 = AtomicU64::new(0);

/// Set the limit for this process. Call it at start. `0` is refused: no
/// package could be unpacked.
pub fn set_max_unpacked_bytes(bytes: u64) -> Result<(), String> {
    if bytes == 0 {
        return Err(format!(
            "--max-unpacked-bytes ({MAX_UNPACKED_BYTES_ENV}) must be greater than 0"
        ));
    }
    MAX_UNPACKED_BYTES.store(bytes, Ordering::Relaxed);
    Ok(())
}

/// The largest size, in bytes, that a package archive may unpack to.
pub fn max_unpacked_bytes() -> u64 {
    resolve(
        MAX_UNPACKED_BYTES.load(Ordering::Relaxed),
        std::env::var(MAX_UNPACKED_BYTES_ENV).ok().as_deref(),
    )
}

/// The value set in the process wins; then a whole number above 0 in the
/// environment variable; then the default.
fn resolve(set: u64, env: Option<&str>) -> u64 {
    if set > 0 {
        return set;
    }
    env.and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_MAX_UNPACKED_BYTES)
}

/// The value of [`MAX_UNPACKED_BYTES_ENV`] for a process that this one
/// starts (the server starts agents), so that it unpacks with the same
/// limit. `None` for the default: the environment of the child is then as it
/// was before the setting.
pub fn child_env_value() -> Option<String> {
    child_env_value_for(max_unpacked_bytes())
}

fn child_env_value_for(max_unpacked_bytes: u64) -> Option<String> {
    (max_unpacked_bytes != DEFAULT_MAX_UNPACKED_BYTES).then(|| max_unpacked_bytes.to_string())
}

/// The unpack limits for a size limit: the entry and ratio limits are the
/// defaults of the archive library.
pub fn unpack_options_for(max_unpacked_bytes: u64) -> UnpackOptions {
    UnpackOptions {
        max_decompressed: max_unpacked_bytes,
        ..UnpackOptions::default()
    }
}

/// The unpack limits of this process.
pub fn unpack_options() -> UnpackOptions {
    unpack_options_for(max_unpacked_bytes())
}

/// Unpack a package archive into `dest` with the limits of this process.
/// Every unpack of a package archive goes through this function.
pub fn unpack_package(archive: &Path, dest: &Path) -> Result<PathBuf, PackageError> {
    fidius_core::package::unpack_package_with_options(archive, dest, &unpack_options())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_limit_default_is_500_mib() {
        assert_eq!(DEFAULT_MAX_UNPACKED_BYTES, 524_288_000);
        assert_eq!(resolve(0, None), DEFAULT_MAX_UNPACKED_BYTES);
        // The default is the default of the archive library.
        assert_eq!(
            UnpackOptions::default().max_decompressed,
            DEFAULT_MAX_UNPACKED_BYTES
        );
    }

    #[test]
    fn unpack_limit_set_value_wins_over_the_environment() {
        assert_eq!(resolve(1_000, Some("2000")), 1_000);
        assert_eq!(resolve(0, Some("2000")), 2_000);
        assert_eq!(resolve(0, Some(" 2000 ")), 2_000);
    }

    #[test]
    fn unpack_limit_bad_environment_value_gives_the_default() {
        for bad in ["", "0", "-1", "1GiB", "abc"] {
            assert_eq!(resolve(0, Some(bad)), DEFAULT_MAX_UNPACKED_BYTES, "{bad}");
        }
    }

    #[test]
    fn unpack_limit_zero_is_refused() {
        // The process value is not changed by a refused call.
        let err = set_max_unpacked_bytes(0).unwrap_err();
        assert!(err.contains("--max-unpacked-bytes"), "{err}");
        assert!(err.contains("CLOACINA_MAX_UNPACKED_BYTES"), "{err}");
    }

    #[test]
    fn unpack_limit_is_passed_to_a_child_only_when_it_is_not_the_default() {
        assert_eq!(child_env_value_for(DEFAULT_MAX_UNPACKED_BYTES), None);
        assert_eq!(
            child_env_value_for(629_145_600),
            Some("629145600".to_string())
        );
    }

    #[test]
    fn unpack_limit_keeps_the_entry_and_ratio_limits() {
        let options = unpack_options_for(1_073_741_824);
        assert_eq!(options.max_decompressed, 1_073_741_824);
        assert_eq!(options.max_entries, 10_000);
        assert_eq!(options.max_ratio, 10);
    }

    /// An archive over the limit is refused, and the same archive unpacks
    /// under a larger limit.
    #[test]
    fn unpack_limit_refuses_an_archive_over_it_and_a_larger_limit_unpacks_it() {
        let src = tempfile::tempdir().unwrap();
        let pkg = src.path().join("demo");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("package.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\
             interface = \"cloacina-workflow-plugin\"\ninterface_version = 1\n\
             extension = \"cloacina\"\n\n[metadata]\nlanguage = \"python\"\n",
        )
        .unwrap();
        // Bytes that do not compress, so the ratio limit is not the one hit.
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let noise: Vec<u8> = (0..64 * 1024)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect();
        std::fs::write(pkg.join("data.bin"), &noise).unwrap();
        let archive = src.path().join("demo.cloacina");
        fidius_core::package::pack_package(&pkg, Some(&archive)).unwrap();

        let small = tempfile::tempdir().unwrap();
        let err = fidius_core::package::unpack_package_with_options(
            &archive,
            small.path(),
            &unpack_options_for(32 * 1024),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("exceeds limit of 32768 bytes"),
            "{err}"
        );

        let large = tempfile::tempdir().unwrap();
        let dir = fidius_core::package::unpack_package_with_options(
            &archive,
            large.path(),
            &unpack_options_for(1024 * 1024),
        )
        .unwrap();
        assert!(dir.join("data.bin").is_file());
    }
}
