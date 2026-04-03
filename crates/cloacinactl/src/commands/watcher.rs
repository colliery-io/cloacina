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

//! Filesystem watcher for `.cloacina` package directories.
//!
//! Uses the `notify` crate to watch directories for file changes and
//! sends reconciliation signals when `.cloacina` files are added,
//! modified, or removed.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Signal sent when the watcher detects a relevant filesystem change.
#[derive(Debug)]
pub struct ReconcileSignal;

/// Watches directories for `.cloacina` file changes and signals the daemon
/// to trigger reconciliation.
pub struct PackageWatcher {
    _watcher: RecommendedWatcher,
}

impl PackageWatcher {
    /// Create a new watcher monitoring the given directories.
    ///
    /// Returns the watcher and a channel receiver that emits `ReconcileSignal`
    /// whenever a `.cloacina` file is created, modified, or removed.
    ///
    /// Events are debounced: rapid changes within `debounce` duration are
    /// collapsed into a single signal.
    pub fn new(
        watch_dirs: &[PathBuf],
        debounce: Duration,
    ) -> Result<(Self, mpsc::Receiver<ReconcileSignal>), notify::Error> {
        let (signal_tx, signal_rx) = mpsc::channel(16);

        // Create a debounced event handler
        let debounce_tx = signal_tx.clone();
        let debounce_dur = debounce;

        // Track last signal time for manual debouncing
        let last_signal = std::sync::Arc::new(std::sync::Mutex::new(std::time::Instant::now()));

        let handler_last_signal = last_signal.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    // Only care about .cloacina files
                    let has_cloacina = event
                        .paths
                        .iter()
                        .any(|p| p.extension().and_then(|e| e.to_str()) == Some("cloacina"));

                    if !has_cloacina {
                        return;
                    }

                    // Only care about create/modify/remove events
                    let relevant = matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    );

                    if !relevant {
                        return;
                    }

                    // Manual debounce: skip if we signaled recently
                    let mut last = handler_last_signal.lock().unwrap();
                    if last.elapsed() < debounce_dur {
                        debug!(
                            "Debouncing filesystem event ({}ms since last signal)",
                            last.elapsed().as_millis()
                        );
                        return;
                    }
                    *last = std::time::Instant::now();
                    drop(last);

                    debug!(
                        "Filesystem change detected: {:?} on {:?}",
                        event.kind, event.paths
                    );

                    if let Err(e) = debounce_tx.try_send(ReconcileSignal) {
                        debug!(
                            "Failed to send reconcile signal (channel full or closed): {}",
                            e
                        );
                    }
                }
                Err(e) => {
                    error!("Filesystem watcher error: {}", e);
                }
            }
        })?;

        // Watch all directories
        for dir in watch_dirs {
            if dir.exists() {
                watcher.watch(dir, RecursiveMode::NonRecursive)?;
                info!("Watching directory: {}", dir.display());
            } else {
                warn!(
                    "Watch directory does not exist (will not be watched): {}",
                    dir.display()
                );
            }
        }

        Ok((Self { _watcher: watcher }, signal_rx))
    }

    /// Add a new directory to the watcher.
    pub fn watch_dir(&mut self, dir: &Path) -> Result<(), notify::Error> {
        self._watcher.watch(dir, RecursiveMode::NonRecursive)?;
        info!("Added watch directory: {}", dir.display());
        Ok(())
    }

    /// Remove a directory from the watcher.
    pub fn unwatch_dir(&mut self, dir: &Path) -> Result<(), notify::Error> {
        self._watcher.unwatch(dir)?;
        info!("Removed watch directory: {}", dir.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn watcher_creates_on_valid_directory() {
        let dir = TempDir::new().unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        let result = PackageWatcher::new(&dirs, Duration::from_millis(100));
        assert!(result.is_ok());
    }

    /// kqueue (macOS) needs time to register the watch before events fire.
    async fn settle() {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    #[tokio::test]
    async fn watcher_signals_on_cloacina_file_create() {
        let dir = TempDir::new().unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        let (_watcher, mut rx) = PackageWatcher::new(&dirs, Duration::from_millis(50)).unwrap();

        settle().await;

        // Create a .cloacina file
        let file_path = dir.path().join("test-package.cloacina");
        std::fs::write(&file_path, b"fake package data").unwrap();

        let signal = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(
            signal.is_ok(),
            "Expected reconcile signal after .cloacina file creation"
        );
        assert!(signal.unwrap().is_some());
    }

    #[tokio::test]
    async fn watcher_ignores_non_cloacina_files() {
        let dir = TempDir::new().unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        let (_watcher, mut rx) = PackageWatcher::new(&dirs, Duration::from_millis(50)).unwrap();

        settle().await;

        // Create non-.cloacina files
        std::fs::write(dir.path().join("readme.txt"), b"hello").unwrap();
        std::fs::write(dir.path().join("data.json"), b"{}").unwrap();

        // Should NOT receive a signal
        let signal = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await;
        assert!(signal.is_err(), "Should not signal for non-.cloacina files");
    }

    #[tokio::test]
    async fn watcher_signals_on_cloacina_file_modify() {
        let dir = TempDir::new().unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        let (_watcher, mut rx) = PackageWatcher::new(&dirs, Duration::from_millis(50)).unwrap();

        settle().await;

        // Create the file (watcher is already active — kqueue sees the create)
        let file_path = dir.path().join("test.cloacina");
        std::fs::write(&file_path, b"initial").unwrap();

        // Drain the create signal
        let _ = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;

        // Now modify the file
        settle().await;
        std::fs::write(&file_path, b"modified content").unwrap();

        let signal = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(
            signal.is_ok(),
            "Expected signal after .cloacina file modification"
        );
    }

    #[tokio::test]
    async fn watcher_signals_on_cloacina_file_remove() {
        let dir = TempDir::new().unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        let (_watcher, mut rx) = PackageWatcher::new(&dirs, Duration::from_millis(50)).unwrap();

        settle().await;

        // Create the file while watcher is active
        let file_path = dir.path().join("remove-me.cloacina");
        std::fs::write(&file_path, b"data").unwrap();

        // Drain the create signal
        let _ = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;

        // Now remove the file
        settle().await;
        std::fs::remove_file(&file_path).unwrap();

        let signal = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(
            signal.is_ok(),
            "Expected signal after .cloacina file removal"
        );
    }

    #[tokio::test]
    async fn watcher_debounces_rapid_changes() {
        let dir = TempDir::new().unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        // Long debounce window
        let (_watcher, mut rx) = PackageWatcher::new(&dirs, Duration::from_millis(500)).unwrap();

        settle().await;

        // Rapid-fire create multiple .cloacina files
        for i in 0..5 {
            std::fs::write(
                dir.path().join(format!("pkg-{}.cloacina", i)),
                format!("data-{}", i),
            )
            .unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Should get at most 1 signal due to debouncing
        let first = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(first.is_ok(), "Should get at least one signal");

        // Second signal should NOT arrive within the debounce window
        let second = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await;
        assert!(second.is_err(), "Debounce should collapse rapid changes");
    }

    #[tokio::test]
    async fn watcher_watch_dir_adds_directory() {
        let dir1 = TempDir::new().unwrap();
        let dir2 = TempDir::new().unwrap();
        let dirs = vec![dir1.path().to_path_buf()];
        let (mut watcher, mut rx) = PackageWatcher::new(&dirs, Duration::from_millis(50)).unwrap();

        // Add second directory
        watcher.watch_dir(dir2.path()).unwrap();

        settle().await;

        // Create a .cloacina file in the new directory
        std::fs::write(dir2.path().join("new.cloacina"), b"data").unwrap();

        let signal = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(
            signal.is_ok(),
            "Should signal for files in newly watched directory"
        );
    }

    #[tokio::test]
    async fn watcher_unwatch_dir_removes_directory() {
        let dir1 = TempDir::new().unwrap();
        let dir2 = TempDir::new().unwrap();
        let dirs = vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()];
        let (mut watcher, mut rx) = PackageWatcher::new(&dirs, Duration::from_millis(50)).unwrap();

        settle().await;

        // Unwatch dir2
        watcher.unwatch_dir(dir2.path()).unwrap();

        // Create a .cloacina file in the unwatched directory
        std::fs::write(dir2.path().join("ignored.cloacina"), b"data").unwrap();

        // Should NOT receive a signal from unwatched dir
        let signal = tokio::time::timeout(Duration::from_secs(1), rx.recv()).await;
        assert!(
            signal.is_err(),
            "Should not signal for files in unwatched directory"
        );
    }

    #[tokio::test]
    async fn watcher_skips_nonexistent_directories() {
        let existing = TempDir::new().unwrap();
        let dirs = vec![
            existing.path().to_path_buf(),
            PathBuf::from("/nonexistent/dir/that/does/not/exist"),
        ];
        // Should succeed — nonexistent dirs are warned but not fatal
        let result = PackageWatcher::new(&dirs, Duration::from_millis(100));
        assert!(result.is_ok());
    }
}
