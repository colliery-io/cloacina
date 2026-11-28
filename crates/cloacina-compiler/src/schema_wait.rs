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

//! The compiler's schema check at start (CLOACINA-T-1087).
//!
//! The compiler runs no migration: only the server migrates. At start the
//! compiler reads its schema with no write:
//!
//! - the schema has each migration of this binary: start;
//! - the schema lacks some (the server has not migrated it yet, or is older):
//!   wait, and log every 30 seconds with both releases, up to 10 minutes;
//!   then exit with an error. A compiler started before the server, or
//!   upgraded before it, so changes no schema;
//! - the schema is too new for this binary (more than one minor ahead): exit
//!   at once, with both releases.
//!
//! A schema that cannot be read (no database yet, or no grant) counts as
//! behind: the compiler waits for it.

use std::future::Future;
use std::time::{Duration, Instant};

use cloacina::Database;
use cloacina::database::schema_release::{
    CompilerSchema, RELEASE, SchemaStatus, compiler_check, schema_status,
};
use tracing::{info, warn};

/// How long and how often the compiler waits for its schema.
#[derive(Debug, Clone)]
pub struct WaitSettings {
    /// The longest wait for the server's migration. Default 10 minutes.
    pub limit: Duration,
    /// How often the schema is read. Default 2 seconds.
    pub poll: Duration,
    /// How often the wait is logged. Default 30 seconds.
    pub log_every: Duration,
}

impl Default for WaitSettings {
    fn default() -> Self {
        Self {
            limit: Duration::from_secs(600),
            poll: Duration::from_secs(2),
            log_every: Duration::from_secs(30),
        }
    }
}

/// Waits until the schema of `database` has each migration of this binary.
/// `Err` names both releases: a schema too new, or still behind at the end of
/// the wait.
pub async fn wait_for_schema(database: &Database, settings: &WaitSettings) -> Result<(), String> {
    wait_with(RELEASE, settings, || schema_status(database)).await
}

/// [`wait_for_schema`] for release `own`, with `read` in place of the
/// database. The tests give both.
pub async fn wait_with<F, Fut>(
    own: &str,
    settings: &WaitSettings,
    mut read: F,
) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<SchemaStatus, String>>,
{
    let started = Instant::now();
    let mut last_log: Option<Instant> = None;
    loop {
        let reason = match read().await {
            Ok(status) => match compiler_check(own, status.written_by.as_deref(), status.pending) {
                CompilerSchema::Ready => {
                    info!(
                        binary_release = %own,
                        schema_release = status.written_by.as_deref().unwrap_or("none"),
                        "schema check passed; the compiler runs no migration"
                    );
                    return Ok(());
                }
                CompilerSchema::TooNew(message) => {
                    return Err(format!("the compiler does not start: {message}"));
                }
                CompilerSchema::Behind { message, .. } => message,
            },
            Err(e) => format!(
                "the schema cannot be read ({e}); this compiler is {own}. The compiler runs no \
                 migration: start or upgrade the server, which migrates the schema"
            ),
        };
        let waited = started.elapsed();
        if waited >= settings.limit {
            return Err(format!(
                "the compiler does not start: {reason}. Waited {} s",
                waited.as_secs()
            ));
        }
        if last_log.is_none_or(|at| at.elapsed() >= settings.log_every) {
            warn!(
                waited_s = waited.as_secs(),
                limit_s = settings.limit.as_secs(),
                "waiting for the server to migrate the schema: {reason}"
            );
            last_log = Some(Instant::now());
        }
        tokio::time::sleep(settings.poll).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn quick() -> WaitSettings {
        WaitSettings {
            limit: Duration::from_millis(300),
            poll: Duration::from_millis(10),
            log_every: Duration::from_millis(100),
        }
    }

    fn status(written_by: Option<&str>, pending: usize) -> Result<SchemaStatus, String> {
        Ok(SchemaStatus {
            written_by: written_by.map(str::to_string),
            pending,
        })
    }

    /// Started first on an empty database, the compiler waits; it starts
    /// once the server has migrated the schema.
    #[tokio::test]
    async fn the_compiler_waits_for_the_server_then_starts() {
        let reads = AtomicUsize::new(0);
        let result = wait_with("0.13.0", &quick(), || {
            let n = reads.fetch_add(1, Ordering::SeqCst);
            async move {
                match n {
                    0 => Err("relation does not exist".to_string()),
                    1 | 2 => status(None, 73),
                    _ => status(Some("0.13.0"), 0),
                }
            }
        })
        .await;
        assert_eq!(result, Ok(()));
        assert_eq!(reads.load(Ordering::SeqCst), 4);
    }

    /// A schema that stays behind stops the compiler at the end of the wait,
    /// with both releases.
    #[tokio::test]
    async fn a_schema_still_behind_at_the_limit_stops_the_compiler() {
        let started = Instant::now();
        let error = wait_with("0.14.0", &quick(), || async { status(Some("0.13.2"), 2) })
            .await
            .unwrap_err();
        assert!(started.elapsed() >= Duration::from_millis(300));
        assert!(
            error.contains("0.14.0") && error.contains("0.13.2"),
            "{error}"
        );
    }

    /// A schema too new stops the compiler at once, with both releases.
    #[tokio::test]
    async fn a_schema_too_new_stops_the_compiler_at_once() {
        let reads = AtomicUsize::new(0);
        let error = wait_with("0.13.0", &quick(), || {
            reads.fetch_add(1, Ordering::SeqCst);
            async { status(Some("0.15.0"), 0) }
        })
        .await
        .unwrap_err();
        assert_eq!(reads.load(Ordering::SeqCst), 1, "no wait");
        assert!(
            error.contains("0.15.0") && error.contains("0.13.0"),
            "{error}"
        );
    }

    /// The previous minor's compiler on a schema one minor ahead starts.
    #[tokio::test]
    async fn a_schema_one_minor_ahead_starts() {
        assert_eq!(
            wait_with("0.13.0", &quick(), || async { status(Some("0.14.1"), 0) }).await,
            Ok(())
        );
    }
}
