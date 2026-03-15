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

/// Fires when ANY sub-policy returns true (OR combinator).
///
/// ```rust,ignore
/// AnyPolicy(vec![
///     Box::new(WallClockWindow::new(Duration::from_secs(300))),
///     Box::new(BoundaryCount::new(20)),
/// ])
/// // fires after 5 minutes OR 20 boundaries, whichever comes first
/// ```
pub struct AnyPolicy(pub Vec<Box<dyn TriggerPolicy>>);

impl TriggerPolicy for AnyPolicy {
    fn should_fire(&self, buffer: &[BufferedBoundary]) -> bool {
        self.0.iter().any(|p| p.should_fire(buffer))
    }
}

/// Fires when ALL sub-policies return true (AND combinator).
///
/// ```rust,ignore
/// AllPolicy(vec![
///     Box::new(BoundaryCount::new(1000)),
///     Box::new(WallClockWindow::new(Duration::from_secs(60))),
/// ])
/// // fires after at least 1000 boundaries AND at least 1 minute
/// ```
pub struct AllPolicy(pub Vec<Box<dyn TriggerPolicy>>);

impl TriggerPolicy for AllPolicy {
    fn should_fire(&self, buffer: &[BufferedBoundary]) -> bool {
        !self.0.is_empty() && self.0.iter().all(|p| p.should_fire(buffer))
    }
}

/// Fires when N boundaries are buffered.
pub struct BoundaryCount {
    /// Minimum number of boundaries to trigger.
    pub count: usize,
}

impl BoundaryCount {
    /// Create a new BoundaryCount policy.
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

impl TriggerPolicy for BoundaryCount {
    fn should_fire(&self, buffer: &[BufferedBoundary]) -> bool {
        buffer.len() >= self.count
    }
}

/// Fires when no new boundary has been received for `duration` (debounce).
///
/// "Silence means the burst is over." Checks the newest `received_at` in the
/// buffer against wall clock time. Only fires if there are buffered boundaries
/// AND the newest one arrived more than `duration` ago.
pub struct WallClockDebounce {
    /// Silence duration before triggering.
    pub duration: Duration,
}

impl WallClockDebounce {
    /// Create a new WallClockDebounce policy.
    pub fn new(duration: Duration) -> Self {
        Self { duration }
    }
}

impl TriggerPolicy for WallClockDebounce {
    fn should_fire(&self, buffer: &[BufferedBoundary]) -> bool {
        if buffer.is_empty() {
            return false;
        }
        // Find the newest received_at
        let newest_received = buffer.iter().map(|b| b.received_at).max().unwrap();
        let elapsed = chrono::Utc::now().signed_duration_since(newest_received);
        elapsed >= chrono::Duration::from_std(self.duration).unwrap_or(chrono::Duration::weeks(52))
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

    // --- BoundaryCount tests ---

    #[test]
    fn test_boundary_count_fires_at_threshold() {
        let policy = BoundaryCount::new(3);
        let buf = vec![make_buffered(), make_buffered(), make_buffered()];
        assert!(policy.should_fire(&buf));
    }

    #[test]
    fn test_boundary_count_does_not_fire_below() {
        let policy = BoundaryCount::new(3);
        let buf = vec![make_buffered(), make_buffered()];
        assert!(!policy.should_fire(&buf));
    }

    #[test]
    fn test_boundary_count_fires_above() {
        let policy = BoundaryCount::new(2);
        let buf = vec![make_buffered(), make_buffered(), make_buffered()];
        assert!(policy.should_fire(&buf));
    }

    // --- WallClockDebounce tests ---

    #[test]
    fn test_debounce_fires_after_silence() {
        let policy = WallClockDebounce::new(Duration::from_millis(0));
        // Boundary received "now" with received_at = now - make_buffered uses Utc::now()
        // With 0ms debounce, any boundary should trigger immediately
        let buf = vec![make_buffered()];
        // Sleep-free: 0ms debounce fires immediately
        assert!(policy.should_fire(&buf));
    }

    #[test]
    fn test_debounce_does_not_fire_during_burst() {
        let policy = WallClockDebounce::new(Duration::from_secs(3600)); // 1 hour
        let buf = vec![make_buffered()]; // just received
        assert!(!policy.should_fire(&buf));
    }

    #[test]
    fn test_debounce_empty_buffer() {
        let policy = WallClockDebounce::new(Duration::from_millis(0));
        assert!(!policy.should_fire(&[]));
    }

    // --- AnyPolicy tests ---

    #[test]
    fn test_any_fires_when_one_matches() {
        let policy = AnyPolicy(vec![
            Box::new(BoundaryCount::new(100)), // won't fire (only 1 boundary)
            Box::new(Immediate),               // fires immediately
        ]);
        assert!(policy.should_fire(&[make_buffered()]));
    }

    #[test]
    fn test_any_does_not_fire_when_none_match() {
        let policy = AnyPolicy(vec![
            Box::new(BoundaryCount::new(100)),
            Box::new(BoundaryCount::new(50)),
        ]);
        assert!(!policy.should_fire(&[make_buffered()]));
    }

    // --- AllPolicy tests ---

    #[test]
    fn test_all_fires_when_all_match() {
        let policy = AllPolicy(vec![Box::new(Immediate), Box::new(BoundaryCount::new(1))]);
        assert!(policy.should_fire(&[make_buffered()]));
    }

    #[test]
    fn test_all_does_not_fire_when_one_fails() {
        let policy = AllPolicy(vec![
            Box::new(Immediate),               // fires
            Box::new(BoundaryCount::new(100)), // won't fire
        ]);
        assert!(!policy.should_fire(&[make_buffered()]));
    }

    #[test]
    fn test_all_empty_policies_does_not_fire() {
        let policy = AllPolicy(vec![]);
        assert!(!policy.should_fire(&[make_buffered()]));
    }

    // --- Nesting tests ---

    #[test]
    fn test_nested_any_all() {
        // "at least 1 boundary AND (immediate OR count >= 100)"
        let policy = AllPolicy(vec![
            Box::new(BoundaryCount::new(1)),
            Box::new(AnyPolicy(vec![
                Box::new(Immediate),
                Box::new(BoundaryCount::new(100)),
            ])),
        ]);
        assert!(policy.should_fire(&[make_buffered()]));
    }
}
