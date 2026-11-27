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

//! UAT for the signature flags of cloacina-server (CLOACI-I-0103 / T-0567;
//! CLOACINA-A-0018).
//!
//! Exercises the binary as an operator would: spawn it with the flag
//! combinations and assert on stderr. `--require-signatures` starts with or
//! without `--verification-org-id`: with none, each tenant trusts only its
//! own keys.

use std::process::Command;
use std::time::Duration;

const SERVER_BIN: &str = env!("CARGO_BIN_EXE_cloacina-server");

/// The message of the start check this server had before CLOACINA-A-0018.
const OLD_REFUSAL: &str = "--require-signatures requires --verification-org-id";

/// `cloacina-server --require-signatures` without `--verification-org-id` is
/// a valid posture (CLOACINA-A-0018): the server does not refuse the flags.
/// We kill it shortly after spawn; it has no database to reach.
#[test]
fn require_signatures_without_org_id_is_not_refused() {
    let mut child = Command::new(SERVER_BIN)
        .args([
            "--require-signatures",
            "--database-url",
            "postgres://nobody:nobody@127.0.0.1:1/nonexistent",
            "--bind",
            "127.0.0.1:0",
        ])
        // Avoid env bleed-through in case the developer's shell has
        // CLOACINA_VERIFICATION_ORG_ID set.
        .env_remove("CLOACINA_VERIFICATION_ORG_ID")
        .env_remove("DATABASE_URL")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn cloacina-server");

    std::thread::sleep(Duration::from_millis(750));
    let _ = child.kill();
    let output = child.wait_with_output().expect("failed to wait for child");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains(OLD_REFUSAL) && !stderr.contains("CLOACINA_VERIFICATION_ORG_ID"),
        "the server refused --require-signatures with no org id; stderr:\n{}",
        stderr
    );
}

/// `cloacina-server --require-signatures` reads the org id from the env var
/// when the flag isn't given, and does not refuse the flags. We kill it
/// shortly after spawn.
#[test]
fn require_signatures_with_env_var_is_not_refused() {
    let env_uuid = uuid::Uuid::new_v4().to_string();

    let mut child = Command::new(SERVER_BIN)
        .args([
            "--require-signatures",
            "--database-url",
            "postgres://nobody:nobody@127.0.0.1:1/nonexistent",
            "--bind",
            "127.0.0.1:0",
        ])
        .env("CLOACINA_VERIFICATION_ORG_ID", &env_uuid)
        .env_remove("DATABASE_URL")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn cloacina-server");

    std::thread::sleep(Duration::from_millis(750));
    let _ = child.kill();
    let output = child.wait_with_output().expect("failed to wait for child");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains(OLD_REFUSAL),
        "the server refused --require-signatures with an org id in the env; stderr:\n{}",
        stderr
    );
}
