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

//! Work distribution abstraction for efficient task notification.
//!
//! This module provides the `WorkDistributor` trait that abstracts how workers
//! wait for new work to become available. Different backends can implement
//! different notification mechanisms:
//!
//! - PostgreSQL: Uses LISTEN/NOTIFY for instant notifications with poll fallback
//! - SQLite: Uses periodic polling since SQLite lacks notification support
//!
//! # Example
//!
//! ```rust,ignore
//! use cloacina::dispatcher::{WorkDistributor, SqliteDistributor};
//!
//! let distributor = SqliteDistributor::new();
//!
//! loop {
//!     // Wait for work signal
//!     distributor.wait_for_work().await;
//!
//!     // Try to claim and execute tasks
//!     let tasks = dal.task_execution().claim_ready_task(10).await?;
//!     for task in tasks {
//!         executor.execute(task).await?;
//!     }
//! }
//! ```

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tracing::{debug, error, info, warn};

/// Trait for abstracting work notification mechanisms.
///
/// Implementations provide a way to efficiently wait for work to become available,
/// avoiding busy polling while still responding quickly to new tasks.
#[async_trait]
pub trait WorkDistributor: Send + Sync {
    /// Wait until work might be available, or timeout.
    ///
    /// This method should block until:
    /// - A notification arrives indicating new work
    /// - A timeout period elapses (for fallback polling)
    ///
    /// The caller should attempt to claim work after this returns,
    /// handling the case where no work is actually available.
    async fn wait_for_work(&self);

    /// Signals that the distributor should stop waiting and shutdown.
    ///
    /// After calling this, `wait_for_work` should return promptly.
    fn shutdown(&self);
}

/// PostgreSQL work distributor using LISTEN/NOTIFY.
///
/// Uses PostgreSQL's LISTEN/NOTIFY mechanism for instant work notifications.
/// Falls back to periodic polling every 30 seconds in case notifications are missed.
///
/// The NOTIFY is triggered by a database trigger on the `task_outbox` table:
/// ```sql
/// CREATE TRIGGER task_outbox_notify
///     AFTER INSERT ON task_outbox
///     FOR EACH ROW EXECUTE FUNCTION notify_task_ready();
/// ```
#[cfg(feature = "postgres")]
pub struct PostgresDistributor {
    /// The database URL for creating a dedicated listen connection
    database_url: String,
    /// Notification channel for waking waiters
    notify: Arc<Notify>,
    /// Shutdown signal
    shutdown: Arc<std::sync::atomic::AtomicBool>,
    /// Background listener task handle
    listener_handle: Option<tokio::task::JoinHandle<()>>,
}

#[cfg(feature = "postgres")]
impl PostgresDistributor {
    /// Fallback poll interval when no notifications received
    const POLL_FALLBACK: Duration = Duration::from_secs(30);

    /// Creates a new PostgreSQL work distributor.
    ///
    /// Spawns a background task that listens for `task_ready` notifications
    /// and wakes any waiters.
    ///
    /// # Arguments
    ///
    /// * `database_url` - PostgreSQL connection URL for the listen connection
    ///
    /// # Returns
    ///
    /// A new `PostgresDistributor` that is ready to receive notifications.
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let notify = Arc::new(Notify::new());
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Spawn background listener
        let listener_handle =
            Self::spawn_listener(database_url.to_string(), notify.clone(), shutdown.clone())
                .await?;

        Ok(Self {
            database_url: database_url.to_string(),
            notify,
            shutdown,
            listener_handle: Some(listener_handle),
        })
    }

    /// Spawns the background listener task.
    async fn spawn_listener(
        database_url: String,
        notify: Arc<Notify>,
        shutdown: Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
        use futures::StreamExt;
        use tokio::sync::mpsc;

        // Parse URL and connect using tokio-postgres
        let (client, mut connection) =
            tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await?;

        // Create channel to receive notifications from connection
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Spawn connection driver that forwards messages to channel
        let conn_shutdown = shutdown.clone();
        tokio::spawn(async move {
            // Use poll_fn to poll the connection for messages
            let stream = futures::stream::poll_fn(move |cx| connection.poll_message(cx));

            futures::pin_mut!(stream);

            while !conn_shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                match stream.next().await {
                    Some(Ok(msg)) => {
                        if tx.send(msg).is_err() {
                            // Receiver dropped
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        error!("PostgreSQL listener connection error: {}", e);
                        break;
                    }
                    None => {
                        // Connection closed
                        break;
                    }
                }
            }
        });

        // Start listening
        client.execute("LISTEN task_ready", &[]).await?;
        info!("PostgreSQL LISTEN/NOTIFY listener started on channel 'task_ready'");

        // Spawn notification handler that receives from channel
        let handle = tokio::spawn(async move {
            // Keep client alive for the duration
            let _client = client;

            loop {
                if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                    debug!("PostgreSQL listener shutting down");
                    break;
                }

                // Wait for notification with timeout
                match tokio::time::timeout(Self::POLL_FALLBACK, rx.recv()).await {
                    Ok(Some(tokio_postgres::AsyncMessage::Notification(notification))) => {
                        debug!(
                            "Received NOTIFY on channel '{}': {}",
                            notification.channel(),
                            notification.payload()
                        );
                        notify.notify_waiters();
                    }
                    Ok(Some(_)) => {
                        // Other message types (e.g., Notice) - ignore
                    }
                    Ok(None) => {
                        // Channel closed - connection dropped
                        warn!("PostgreSQL listener channel closed");
                        break;
                    }
                    Err(_) => {
                        // Timeout - trigger fallback poll
                        debug!("LISTEN timeout, triggering fallback poll");
                        notify.notify_waiters();
                    }
                }
            }
        });

        Ok(handle)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl WorkDistributor for PostgresDistributor {
    async fn wait_for_work(&self) {
        // Wait for notification or timeout
        tokio::select! {
            _ = self.notify.notified() => {
                debug!("Woke from NOTIFY signal");
            }
            _ = tokio::time::sleep(Self::POLL_FALLBACK) => {
                debug!("Woke from fallback poll timeout");
            }
        }
    }

    fn shutdown(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.notify.notify_waiters();
    }
}

#[cfg(feature = "postgres")]
impl Drop for PostgresDistributor {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(handle) = self.listener_handle.take() {
            handle.abort();
        }
    }
}

/// SQLite work distributor using periodic polling.
///
/// Since SQLite lacks notification capabilities, this implementation
/// uses simple periodic polling at a configurable interval.
#[cfg(feature = "sqlite")]
pub struct SqliteDistributor {
    /// Poll interval
    poll_interval: Duration,
    /// Shutdown signal
    shutdown: Arc<std::sync::atomic::AtomicBool>,
    /// Notify for shutdown signal
    notify: Arc<Notify>,
}

#[cfg(feature = "sqlite")]
impl SqliteDistributor {
    /// Default poll interval for SQLite
    const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(500);

    /// Creates a new SQLite work distributor with default poll interval (500ms).
    pub fn new() -> Self {
        Self::with_poll_interval(Self::DEFAULT_POLL_INTERVAL)
    }

    /// Creates a new SQLite work distributor with custom poll interval.
    ///
    /// # Arguments
    ///
    /// * `poll_interval` - How often to wake up and check for work
    pub fn with_poll_interval(poll_interval: Duration) -> Self {
        Self {
            poll_interval,
            shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }
}

#[cfg(feature = "sqlite")]
impl Default for SqliteDistributor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl WorkDistributor for SqliteDistributor {
    async fn wait_for_work(&self) {
        if self.shutdown.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }

        tokio::select! {
            _ = tokio::time::sleep(self.poll_interval) => {
                debug!("SQLite poll interval elapsed");
            }
            _ = self.notify.notified() => {
                debug!("SQLite distributor shutdown signal received");
            }
        }
    }

    fn shutdown(&self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.notify.notify_waiters();
    }
}

/// Creates the appropriate work distributor based on database backend.
///
/// # Arguments
///
/// * `database` - The database instance to detect backend from
///
/// # Returns
///
/// A boxed `WorkDistributor` appropriate for the database backend.
pub async fn create_work_distributor(
    database: &crate::Database,
) -> Result<Box<dyn WorkDistributor>, Box<dyn std::error::Error + Send + Sync>> {
    match database.backend() {
        #[cfg(feature = "postgres")]
        crate::database::BackendType::Postgres => {
            // Extract the database URL - this is tricky since we don't store it
            // For now, return an error suggesting the caller should use PostgresDistributor::new directly
            Err("PostgreSQL distributor requires database URL. Use PostgresDistributor::new() directly.".into())
        }
        #[cfg(feature = "sqlite")]
        crate::database::BackendType::Sqlite => Ok(Box::new(SqliteDistributor::new())),
        #[allow(unreachable_patterns)]
        _ => Err("Unsupported database backend".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_sqlite_distributor_poll_interval() {
        let distributor = SqliteDistributor::with_poll_interval(Duration::from_millis(50));

        let start = std::time::Instant::now();
        distributor.wait_for_work().await;
        let elapsed = start.elapsed();

        // Should have waited approximately 50ms
        assert!(elapsed >= Duration::from_millis(40));
        assert!(elapsed < Duration::from_millis(100));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_sqlite_distributor_shutdown() {
        let distributor = SqliteDistributor::with_poll_interval(Duration::from_secs(60));

        let start = std::time::Instant::now();

        // Signal shutdown from another task
        let shutdown_distributor = distributor.shutdown.clone();
        let shutdown_notify = distributor.notify.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            shutdown_distributor.store(true, std::sync::atomic::Ordering::SeqCst);
            shutdown_notify.notify_waiters();
        });

        distributor.wait_for_work().await;
        let elapsed = start.elapsed();

        // Should have woken up quickly due to shutdown, not waited 60s
        assert!(elapsed < Duration::from_secs(1));
    }
}
