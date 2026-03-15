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

//! Trigger policies for continuous scheduling accumulators.
//!
//! A `TriggerPolicy` controls when an accumulator fires its downstream task.
//! Policies are inherently domain-dependent — "how many" and "how long" are
//! interpretations of boundary data.
//!
//! See CLOACI-S-0005 for the full specification.

use super::boundary::BufferedBoundary;
use std::time::{Duration, Instant};

/// Trait controlling when an accumulator should fire.
///
/// Implementations examine the current buffer of boundaries and return
/// `true` when the downstream task should execute.
pub trait TriggerPolicy: Send + Sync {
    /// Should the accumulator fire given the current buffer state?
    fn should_fire(&self, buffer: &[BufferedBoundary]) -> bool;
}

/// Fires on every boundary — as soon as the buffer is non-empty.
pub struct Immediate;

impl TriggerPolicy for Immediate {
    fn should_fire(&self, buffer: &[BufferedBoundary]) -> bool {
        !buffer.is_empty()
    }
}

/// Fires when wall clock time since last drain exceeds a configured duration.
///
/// The `last_drain_at` is updated externally by the accumulator when it drains.
/// If no drain has occurred yet, fires on the first boundary after the policy
/// is created (uses creation time as the initial reference).
pub struct WallClockWindow {
    /// Minimum wall clock duration between drains.
    pub duration: Duration,
    /// When the last drain occurred (or when the policy was created).
    last_drain_at: Instant,
}

impl WallClockWindow {
    /// Create a new WallClockWindow policy with the given duration.
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            last_drain_at: Instant::now(),
        }
    }

    /// Notify the policy that a drain occurred. Called by the accumulator.
    pub fn mark_drained(&mut self) {
        self.last_drain_at = Instant::now();
    }
}

impl TriggerPolicy for WallClockWindow {
    fn should_fire(&self, buffer: &[BufferedBoundary]) -> bool {
        if buffer.is_empty() {
            return false;
        }
        self.last_drain_at.elapsed() >= self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuous::boundary::{BoundaryKind, ComputationBoundary};
    use chrono::Utc;

    fn make_buffered() -> BufferedBoundary {
        BufferedBoundary::new(ComputationBoundary {
            kind: BoundaryKind::Cursor {
                value: "test".into(),
            },
            metadata: None,
            emitted_at: Utc::now(),
        })
    }

    #[test]
    fn test_immediate_fires_on_non_empty() {
        let policy = Immediate;
        assert!(policy.should_fire(&[make_buffered()]));
    }

    #[test]
    fn test_immediate_does_not_fire_on_empty() {
        let policy = Immediate;
        assert!(!policy.should_fire(&[]));
    }

    #[test]
    fn test_wall_clock_window_fires_after_duration() {
        let policy = WallClockWindow {
            duration: Duration::from_millis(0),
            last_drain_at: Instant::now() - Duration::from_secs(10),
        };
        assert!(policy.should_fire(&[make_buffered()]));
    }

    #[test]
    fn test_wall_clock_window_does_not_fire_early() {
        let policy = WallClockWindow::new(Duration::from_secs(3600)); // 1 hour
        assert!(!policy.should_fire(&[make_buffered()]));
    }

    #[test]
    fn test_wall_clock_window_does_not_fire_on_empty() {
        let policy = WallClockWindow {
            duration: Duration::from_millis(0),
            last_drain_at: Instant::now() - Duration::from_secs(10),
        };
        assert!(!policy.should_fire(&[]));
    }

    #[test]
    fn test_wall_clock_window_mark_drained() {
        let mut policy = WallClockWindow {
            duration: Duration::from_secs(1),
            last_drain_at: Instant::now() - Duration::from_secs(10),
        };
        // Before drain, would fire
        assert!(policy.should_fire(&[make_buffered()]));
        // After drain reset, should not fire
        policy.mark_drained();
        assert!(!policy.should_fire(&[make_buffered()]));
    }
}
