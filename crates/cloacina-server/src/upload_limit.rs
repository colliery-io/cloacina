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

//! The size limit of a package upload (CLOACINA-T-1048).
//!
//! A server setting: `--max-package-bytes` / `CLOACINA_MAX_PACKAGE_BYTES`,
//! 100 MiB when not set. A Python package carries its dependencies in
//! `vendor/`, one folder per platform, so a package with a data stack can be
//! larger than the default.

use std::sync::atomic::{AtomicU64, Ordering};

/// The limit when the setting is not given: 100 MiB.
pub const DEFAULT_MAX_PACKAGE_BYTES: u64 = 100 * 1024 * 1024;

/// Room for the multipart framing around the file (boundaries and part
/// headers), so that a package of exactly the limit is accepted.
pub const MULTIPART_OVERHEAD_BYTES: u64 = 64 * 1024;

static MAX_PACKAGE_BYTES: AtomicU64 = AtomicU64::new(DEFAULT_MAX_PACKAGE_BYTES);

/// Set the limit for this process. Call it at start, before the router is
/// built. `0` is refused: no package could be uploaded.
pub fn set_max_package_bytes(bytes: u64) -> Result<(), String> {
    if bytes == 0 {
        return Err(
            "--max-package-bytes (CLOACINA_MAX_PACKAGE_BYTES) must be greater than 0".to_string(),
        );
    }
    MAX_PACKAGE_BYTES.store(bytes, Ordering::Relaxed);
    Ok(())
}

/// The largest package archive, in bytes, that an upload accepts.
pub fn max_package_bytes() -> u64 {
    MAX_PACKAGE_BYTES.load(Ordering::Relaxed)
}

/// The limit of a request body: the package limit and the multipart framing.
pub fn max_request_body_bytes() -> usize {
    max_package_bytes()
        .saturating_add(MULTIPART_OVERHEAD_BYTES)
        .min(usize::MAX as u64) as usize
}

fn mib(bytes: u64) -> String {
    format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
}

/// The error text for an upload over the limit. `size` is the size of the
/// upload in bytes when it is known.
pub fn too_large_message(size: Option<u64>, limit: u64) -> String {
    let what = match size {
        Some(size) => format!("the upload is {size} bytes ({})", mib(size)),
        None => "the upload is larger than the limit".to_string(),
    };
    format!(
        "package too large: {what}, and the limit of this server is {limit} bytes ({}). \
         The server operator can raise it with --max-package-bytes or \
         CLOACINA_MAX_PACKAGE_BYTES. A Python package can vendor for fewer platforms \
         (`cloacinactl package build --arch`).",
        mib(limit)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_100_mib() {
        assert_eq!(DEFAULT_MAX_PACKAGE_BYTES, 104_857_600);
    }

    #[test]
    fn message_names_the_size_and_the_limit() {
        let text = too_large_message(Some(170_000_000), DEFAULT_MAX_PACKAGE_BYTES);
        assert!(text.contains("170000000 bytes"), "{text}");
        assert!(text.contains("162.1 MiB"), "{text}");
        assert!(text.contains("104857600 bytes"), "{text}");
        assert!(text.contains("100.0 MiB"), "{text}");
        assert!(text.contains("--max-package-bytes"), "{text}");
        assert!(text.contains("CLOACINA_MAX_PACKAGE_BYTES"), "{text}");
    }

    #[test]
    fn message_without_a_known_size_names_the_limit() {
        let text = too_large_message(None, 1024);
        assert!(text.contains("larger than the limit"), "{text}");
        assert!(text.contains("1024 bytes"), "{text}");
    }

    #[test]
    fn zero_is_refused() {
        assert!(set_max_package_bytes(0).is_err());
    }
}
