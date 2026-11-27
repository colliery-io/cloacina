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

//! Priority of a workflow and of a task (CLOACINA-T-1045).
//!
//! A priority is an integer from 0 to 10. 0 is the highest priority, 10 the
//! lowest, 5 the default. Waiting work is dispatched in the order of
//! `(workflow priority, task priority, time waited)`: work of a more
//! important workflow goes first; inside one workflow priority, more
//! important tasks go first; inside that, the task that has waited longest.
//!
//! Waiting work ages: for each ageing interval a task has waited, both
//! numbers of its order improve by one, never below 0. The stored numbers do
//! not change.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// The highest priority.
pub const HIGHEST_PRIORITY: u8 = 0;
/// The lowest priority.
pub const LOWEST_PRIORITY: u8 = 10;
/// The priority of work that declares none.
pub const DEFAULT_PRIORITY: u8 = 5;
/// The ageing interval when none is set: two minutes.
pub const DEFAULT_PRIORITY_AGING_INTERVAL: Duration = Duration::from_secs(120);

/// Check a declared priority. The error is one plain sentence.
pub fn validate_priority(value: i64) -> Result<u8, String> {
    if (HIGHEST_PRIORITY as i64..=LOWEST_PRIORITY as i64).contains(&value) {
        Ok(value as u8)
    } else {
        Err(format!(
            "{value} is not a priority: a priority is an integer from {HIGHEST_PRIORITY} \
             (the highest) to {LOWEST_PRIORITY} (the lowest)"
        ))
    }
}

/// A stored priority as a number of the scale. A value outside the scale (a
/// row written by hand) is held to the nearest end.
pub fn clamp_priority(value: i32) -> u8 {
    value.clamp(HIGHEST_PRIORITY as i32, LOWEST_PRIORITY as i32) as u8
}

/// The number of ageing steps of a task that has waited `waited`. An interval
/// of zero switches ageing off.
pub fn aging_steps(waited: Duration, interval: Duration) -> u32 {
    if interval.is_zero() {
        return 0;
    }
    let steps = waited.as_millis() / interval.as_millis().max(1);
    u32::try_from(steps).unwrap_or(u32::MAX)
}

/// The priority used for ordering: the stored one, improved by one for each
/// ageing step, never below 0.
pub fn aged_priority(priority: u8, steps: u32) -> u8 {
    let steps = u8::try_from(steps).unwrap_or(u8::MAX);
    priority.saturating_sub(steps)
}

/// A priority that was asked for and the priority that was given.
///
/// `given` is `asked` held to the limit of the principal that asked: the
/// highest priority that principal may set. A principal that asks for more
/// than it may set is not refused: it gets its limit, and the answer says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PriorityGrant {
    /// The priority that was asked for.
    pub asked: u8,
    /// The priority the work runs at.
    pub given: u8,
}

impl PriorityGrant {
    /// `asked` with no limit: given as asked.
    pub fn unlimited(asked: u8) -> Self {
        Self {
            asked,
            given: asked,
        }
    }

    /// `asked` held to `limit`, the highest priority the principal may set.
    /// `None` is a principal that may set no priority: it gets the lowest.
    pub fn held_to(asked: u8, limit: Option<u8>) -> Self {
        let floor = limit.unwrap_or(LOWEST_PRIORITY).min(LOWEST_PRIORITY);
        Self {
            asked,
            given: asked.max(floor).min(LOWEST_PRIORITY),
        }
    }

    /// True when the given priority is lower than the one asked for.
    pub fn is_held(&self) -> bool {
        self.given != self.asked
    }

    /// The sentence a caller reads when the priority was lowered. `None`
    /// when it was given as asked.
    pub fn notice(&self) -> Option<String> {
        self.is_held().then(|| {
            format!(
                "priority {} was asked for and priority {} was given: {} is the highest \
                 priority this principal may set",
                self.asked, self.given, self.given
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_priority_is_an_integer_from_0_to_10() {
        assert_eq!(validate_priority(0), Ok(0));
        assert_eq!(validate_priority(5), Ok(5));
        assert_eq!(validate_priority(10), Ok(10));
        assert!(validate_priority(11).is_err());
        assert!(validate_priority(-1).is_err());
        assert_eq!(clamp_priority(-3), 0);
        assert_eq!(clamp_priority(99), 10);
        assert_eq!(clamp_priority(7), 7);
    }

    #[test]
    fn waiting_improves_the_priority_by_one_for_each_interval_and_stops_at_0() {
        let interval = DEFAULT_PRIORITY_AGING_INTERVAL;
        assert_eq!(aging_steps(Duration::from_secs(119), interval), 0);
        assert_eq!(aging_steps(Duration::from_secs(120), interval), 1);
        assert_eq!(aging_steps(Duration::from_secs(20 * 60), interval), 10);
        assert_eq!(aged_priority(10, 0), 10);
        assert_eq!(aged_priority(10, 5), 5);
        assert_eq!(aged_priority(10, 10), 0);
        assert_eq!(aged_priority(3, 10), 0);
        assert_eq!(aged_priority(3, u32::MAX), 0);
    }

    #[test]
    fn an_interval_of_zero_switches_ageing_off() {
        assert_eq!(aging_steps(Duration::from_secs(3600), Duration::ZERO), 0);
    }

    #[test]
    fn a_priority_above_the_limit_is_lowered_to_the_limit_and_says_so() {
        // A `write` principal may set 5 to 10.
        let grant = PriorityGrant::held_to(2, Some(5));
        assert_eq!((grant.asked, grant.given), (2, 5));
        assert!(grant.is_held());
        let notice = grant.notice().unwrap();
        assert!(notice.contains("priority 2 was asked"), "{notice}");
        assert!(notice.contains("priority 5 was given"), "{notice}");
    }

    #[test]
    fn a_priority_inside_the_limit_is_given_as_asked() {
        assert_eq!(PriorityGrant::held_to(7, Some(5)).given, 7);
        assert_eq!(PriorityGrant::held_to(5, Some(5)).given, 5);
        assert_eq!(PriorityGrant::held_to(0, Some(0)).given, 0);
        assert!(PriorityGrant::held_to(7, Some(5)).notice().is_none());
        assert_eq!(PriorityGrant::unlimited(0).given, 0);
    }

    #[test]
    fn a_principal_that_may_set_no_priority_gets_the_lowest() {
        let grant = PriorityGrant::held_to(5, None);
        assert_eq!(grant.given, LOWEST_PRIORITY);
        assert!(grant.is_held());
    }
}
