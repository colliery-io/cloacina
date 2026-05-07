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

//! UAT for cloacina-server CLI argument validation (CLOACI-I-0103 / T-0567).
//!
//! Exercises the binary as an operator would: spawn it with various flag
//! combinations and assert on exit code + stderr. The unit tests in
//! `lib.rs::tests::validate_security_args_*` cover the validation function
//! in isolation; this file confirms the validation actually fires through
//! the full clap → run() path.

use std::process::Command;
use std::time::Duration;

const SERVER_BIN: &str = env!("CARGO_BIN_EXE_cloacina-server");

/// `cloacina-server --require-signatures` without `--verification-org-id` must
/// refuse to start with a clear, operator-actionable error message that names
/// both the missing flag and the env var alternative.
#[test]
fn require_signatures_without_org_id_fails_fast() {
    let output = Command::new(SERVER_BIN)
        .args([
            "--require-signatures",
            // A plausible but unreachable URL — validation fires before any
            // DB connection attempt, so the value doesn't actually matter.
            "--database-url",
            "postgres://nobody:nobody@127.0.0.1:1/nonexistent",
        ])
        // Avoid env bleed-through in case the developer's shell has
        // CLOACINA_VERIFICATION_ORG_ID set.
        .env_remove("CLOACINA_VERIFICATION_ORG_ID")
        .output()
        .expect("failed to spawn cloacina-server");

    assert!(
        !output.status.success(),
        "expected non-zero exit when --require-signatures is set without org id; \
         got status: {:?}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("verification-org-id"),
        "stderr must name the missing flag for the operator; got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("CLOACINA_VERIFICATION_ORG_ID"),
        "stderr must name the env var alternative; got:\n{}",
        stderr
    );
}

/// `cloacina-server --require-signatures` reads the org id from the env var
/// when the flag isn't given. Validation should pass and the server should
/// proceed past the security gate — we kill it shortly after spawn and check
/// that the validation error is *not* in stderr.
#[test]
fn require_signatures_with_env_var_passes_validation() {
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

    // Validation is microseconds. 750ms is generous and lets the binary
    // get well past validation before we kill it.
    std::thread::sleep(Duration::from_millis(750));
    let _ = child.kill();
    let output = child.wait_with_output().expect("failed to wait for child");
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Negative assertion: the validation error message must NOT appear.
    // If validation had failed, anyhow::bail! prints the message to stderr
    // before exit (Rust flushes stderr on Err return from main).
    assert!(
        !stderr.contains("--verification-org-id <UUID>"),
        "validation incorrectly fired despite CLOACINA_VERIFICATION_ORG_ID being set; \
         stderr:\n{}",
        stderr
    );
}
