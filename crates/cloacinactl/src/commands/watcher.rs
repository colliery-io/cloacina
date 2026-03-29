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
