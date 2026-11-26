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

//! Wait for the database at start (CLOACINA-T-1035).
//!
//! On a fresh install the server can start before Postgres accepts
//! connections. The server tries a connection, waits, and tries again with a
//! growing delay, for a bounded time. It runs before the migrations and before
//! the HTTP listener binds, so `/ready` does not answer during the wait.

use std::fmt::Display;
use std::future::Future;
use std::time::Duration;

use tracing::{info, warn};

/// Delays between tries: `initial`, then twice the last, never above `max`.
#[derive(Debug, Clone, Copy)]
pub struct Backoff {
    pub initial: Duration,
    pub max: Duration,
    /// The longest one try may take before it counts as failed.
    pub attempt_timeout: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            initial: Duration::from_millis(500),
            max: Duration::from_secs(5),
            attempt_timeout: Duration::from_secs(10),
        }
    }
}

/// The database did not accept a connection within the limit.
#[derive(Debug)]
pub struct DatabaseWaitTimeout {
    pub waited_s: u64,
    pub attempts: u32,
    pub last_error: String,
}

impl Display for DatabaseWaitTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the database did not accept a connection within {} s ({} tries); last error: {}. \
             Check DATABASE_URL and that Postgres is up, or raise --database-wait-timeout-s \
             (CLOACINA_DATABASE_WAIT_TIMEOUT_S)",
            self.waited_s, self.attempts, self.last_error
        )
    }
}

impl std::error::Error for DatabaseWaitTimeout {}

/// Call `connect` until it succeeds or `limit` has passed.
///
/// A `limit` of zero means one try and no wait. Returns the number of tries
/// on success.
pub async fn wait_until_connected<F, Fut, E>(
    limit: Duration,
    backoff: Backoff,
    mut connect: F,
) -> Result<u32, DatabaseWaitTimeout>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<(), E>>,
    E: Display,
{
    let started = tokio::time::Instant::now();
    let mut delay = backoff.initial;
    let mut attempts: u32 = 0;
    loop {
        attempts += 1;
        let last_error = match tokio::time::timeout(backoff.attempt_timeout, connect()).await {
            Ok(Ok(())) => {
                if attempts > 1 {
                    info!(
                        attempts,
                        waited_ms = started.elapsed().as_millis() as u64,
                        "The database accepts connections"
                    );
                }
                return Ok(attempts);
            }
            Ok(Err(e)) => e.to_string(),
            Err(_) => format!(
                "no answer within {} s",
                backoff.attempt_timeout.as_secs_f32()
            ),
        };
        let waited = started.elapsed();
        if waited >= limit {
            return Err(DatabaseWaitTimeout {
                waited_s: waited.as_secs(),
                attempts,
                last_error,
            });
        }
        let sleep_for = delay.min(limit - waited);
        warn!(
            attempt = attempts,
            retry_in_ms = sleep_for.as_millis() as u64,
            limit_s = limit.as_secs(),
            error = %last_error,
            "Waiting for the database to accept connections"
        );
        tokio::time::sleep(sleep_for).await;
        delay = (delay * 2).min(backoff.max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn fast() -> Backoff {
        Backoff {
            initial: Duration::from_millis(1),
            max: Duration::from_millis(4),
            attempt_timeout: Duration::from_millis(200),
        }
    }

    /// A connector that fails `failures` times and then succeeds.
    fn flaky(
        failures: u32,
    ) -> (
        Arc<AtomicU32>,
        impl FnMut() -> std::future::Ready<Result<(), String>>,
    ) {
        let calls = Arc::new(AtomicU32::new(0));
        let seen = calls.clone();
        let connect = move || {
            let n = seen.fetch_add(1, Ordering::SeqCst);
            std::future::ready(if n < failures {
                Err("connection refused".to_string())
            } else {
                Ok(())
            })
        };
        (calls, connect)
    }

    #[tokio::test]
    async fn a_database_that_is_up_needs_one_try() {
        let (calls, connect) = flaky(0);
        let tries = wait_until_connected(Duration::from_secs(5), fast(), connect)
            .await
            .unwrap();
        assert_eq!(tries, 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn it_tries_again_until_the_database_answers() {
        let (calls, connect) = flaky(3);
        let tries = wait_until_connected(Duration::from_secs(5), fast(), connect)
            .await
            .unwrap();
        assert_eq!(tries, 4);
        assert_eq!(calls.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn it_gives_up_after_the_limit_with_a_clear_message() {
        let (calls, connect) = flaky(u32::MAX);
        let err = wait_until_connected(Duration::from_millis(30), fast(), connect)
            .await
            .unwrap_err();
        assert!(err.attempts > 1, "it tried more than once: {err:?}");
        assert_eq!(calls.load(Ordering::SeqCst), err.attempts);
        assert_eq!(err.last_error, "connection refused");
        let message = err.to_string();
        assert!(message.contains("did not accept a connection"), "{message}");
        assert!(message.contains("connection refused"), "{message}");
        assert!(
            message.contains("CLOACINA_DATABASE_WAIT_TIMEOUT_S"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn a_limit_of_zero_is_one_try_and_no_wait() {
        let (calls, connect) = flaky(u32::MAX);
        let err = wait_until_connected(Duration::ZERO, fast(), connect)
            .await
            .unwrap_err();
        assert_eq!(err.attempts, 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_try_that_does_not_answer_counts_as_failed() {
        let calls = Arc::new(AtomicU32::new(0));
        let seen = calls.clone();
        let connect = move || {
            let n = seen.fetch_add(1, Ordering::SeqCst);
            async move {
                if n == 0 {
                    std::future::pending::<()>().await;
                }
                Ok::<(), String>(())
            }
        };
        let backoff = Backoff {
            attempt_timeout: Duration::from_millis(20),
            ..fast()
        };
        let tries = wait_until_connected(Duration::from_secs(5), backoff, connect)
            .await
            .unwrap();
        assert_eq!(tries, 2);
    }

    #[test]
    fn the_delay_doubles_up_to_the_cap() {
        let b = Backoff::default();
        let mut delay = b.initial;
        let mut seen = vec![delay];
        for _ in 0..5 {
            delay = (delay * 2).min(b.max);
            seen.push(delay);
        }
        assert_eq!(
            seen.iter().map(|d| d.as_millis()).collect::<Vec<_>>(),
            vec![500, 1000, 2000, 4000, 5000, 5000]
        );
    }
}
