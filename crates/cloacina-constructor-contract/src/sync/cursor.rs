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

//! Cursor-value ordering shared by every **client-side** incremental filter
//!. SQL-side predicates cast in the query instead
//! (, postgres); this helper is for guests that compare cursor
//! strings in code (rest, mssql, snowflake).

use std::cmp::Ordering;

/// Compare two cursor values **numeric-aware**: when both sides parse as
/// numbers they compare numerically — a plain string compare mis-orders numeric
/// cursors once digit counts differ (`"9" > "12"` lexicographically), which
/// re-delivers or skips rows. Anything else compares as strings, which is
/// correct for ISO-8601 timestamps and opaque tokens. Mixed numeric/non-numeric
/// pairs fall back to the string compare (no ordering is right there; the
/// fallback is at least total and stable).
pub fn cursor_cmp(a: &str, b: &str) -> Ordering {
    if let (Ok(x), Ok(y)) = (a.parse::<i128>(), b.parse::<i128>()) {
        return x.cmp(&y);
    }
    if let (Ok(x), Ok(y)) = (a.parse::<f64>(), b.parse::<f64>()) {
        return x.partial_cmp(&y).unwrap_or(Ordering::Equal);
    }
    a.cmp(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_order_by_value_not_lexicographically() {
        assert_eq!(cursor_cmp("9", "12"), Ordering::Less);
        assert_eq!(cursor_cmp("2", "12"), Ordering::Less); // the T-0182 re-delivery class
        assert_eq!(cursor_cmp("100", "99"), Ordering::Greater);
        assert_eq!(cursor_cmp("-3", "2"), Ordering::Less);
        assert_eq!(cursor_cmp("007", "8"), Ordering::Less);
    }

    #[test]
    fn floats_order_numerically() {
        assert_eq!(cursor_cmp("2.5", "10.25"), Ordering::Less);
        assert_eq!(cursor_cmp("1e3", "999"), Ordering::Greater);
    }

    #[test]
    fn equal_values() {
        assert_eq!(cursor_cmp("7", "7"), Ordering::Equal);
        assert_eq!(cursor_cmp("abc", "abc"), Ordering::Equal);
    }

    #[test]
    fn iso_timestamps_order_as_strings() {
        assert_eq!(
            cursor_cmp("2026-01-02T00:00:00Z", "2026-01-10T00:00:00Z"),
            Ordering::Less
        );
        assert_eq!(
            cursor_cmp("2026-02-01T00:00:00Z", "2026-01-31T23:59:59Z"),
            Ordering::Greater
        );
    }

    #[test]
    fn mixed_or_unparseable_falls_back_to_string_compare() {
        assert_eq!(cursor_cmp("abc", "12"), "abc".cmp("12"));
        assert_eq!(cursor_cmp("12", "abc"), "12".cmp("abc"));
        assert_eq!(cursor_cmp("id-10", "id-9"), "id-10".cmp("id-9"));
    }
}
