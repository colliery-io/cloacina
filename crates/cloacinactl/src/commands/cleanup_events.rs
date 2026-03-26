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

//! Implementation of the `admin cleanup-events` command.
//!
//! Cleans up old execution events from the database based on a retention policy.

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use cloacina::dal::DAL;
use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::Database;
use tracing::info;

/// Parse a duration string like "90d", "30d", "7d", "24h", "1h30m" into a chrono::Duration.
///
/// Supported units:
/// - `d` - days
/// - `h` - hours
/// - `m` - minutes
/// - `s` - seconds
///
/// Examples:
/// - "90d" -> 90 days
/// - "24h" -> 24 hours
/// - "7d12h" -> 7 days and 12 hours
fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return Err(anyhow!("Duration string cannot be empty"));
    }

    let mut total = Duration::zero();
    let mut current_num = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() {
            current_num.push(c);
        } else {
            if current_num.is_empty() {
                return Err(anyhow!(
                    "Invalid duration format: expected number before '{}'",
                    c
                ));
            }

            let num: i64 = current_num
                .parse()
                .with_context(|| format!("Invalid number in duration: {}", current_num))?;
            current_num.clear();

            let duration = match c {
                'd' => Duration::days(num),
                'h' => Duration::hours(num),
                'm' => Duration::minutes(num),
                's' => Duration::seconds(num),
                _ => return Err(anyhow!("Unknown duration unit: '{}'. Use d, h, m, or s", c)),
            };

            total = total + duration;
        }
    }

    // If there are remaining digits without a unit, that's an error
    if !current_num.is_empty() {
        return Err(anyhow!(
            "Duration '{}' is missing a unit. Use d (days), h (hours), m (minutes), or s (seconds)",
            s
        ));
    }

    if total == Duration::zero() {
        return Err(anyhow!("Duration must be greater than zero"));
    }

    Ok(total)
}

/// Run the cleanup-events command.
///
/// # Arguments
///
/// * `database_url` - The database connection URL
/// * `older_than` - Duration string (e.g., "90d", "30d")
/// * `dry_run` - If true, only report what would be deleted without actually deleting
pub async fn run(database_url: &str, older_than: &str, dry_run: bool) -> Result<()> {
    let duration = parse_duration(older_than)
        .with_context(|| format!("Invalid duration: '{}'", older_than))?;

    let cutoff = Utc::now() - duration;
    let cutoff_ts = UniversalTimestamp(cutoff);

    info!(
        "Cleaning up execution events older than {} (cutoff: {})",
        older_than, cutoff
    );

    // Connect to database
    // Use a default pool size of 4 for CLI operations
    let database = Database::try_new_with_schema(database_url, "", 4, None)
        .context("Failed to connect to database")?;

    let dal = DAL::new(database);

    if dry_run {
        let count = dal
            .execution_event()
            .count_older_than(cutoff_ts)
            .await
            .context("Failed to count events")?;

        if count == 0 {
            info!("No execution events found older than {}", cutoff);
        } else {
            info!(
                "[DRY RUN] Would delete {} execution event(s) older than {}",
                count, cutoff
            );
        }
    } else {
        let deleted = dal
            .execution_event()
            .delete_older_than(cutoff_ts)
            .await
            .context("Failed to delete events")?;

        if deleted == 0 {
            info!("No execution events found older than {}", cutoff);
        } else {
            info!(
                "Deleted {} execution event(s) older than {}",
                deleted, cutoff
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_days() {
        let d = parse_duration("90d").unwrap();
        assert_eq!(d, Duration::days(90));
    }

    #[test]
    fn test_parse_duration_hours() {
        let d = parse_duration("24h").unwrap();
        assert_eq!(d, Duration::hours(24));
    }

    #[test]
    fn test_parse_duration_minutes() {
        let d = parse_duration("30m").unwrap();
        assert_eq!(d, Duration::minutes(30));
    }

    #[test]
    fn test_parse_duration_seconds() {
        let d = parse_duration("60s").unwrap();
        assert_eq!(d, Duration::seconds(60));
    }

    #[test]
    fn test_parse_duration_combined() {
        let d = parse_duration("7d12h").unwrap();
        assert_eq!(d, Duration::days(7) + Duration::hours(12));
    }

    #[test]
    fn test_parse_duration_complex() {
        let d = parse_duration("1d2h30m45s").unwrap();
        assert_eq!(
            d,
            Duration::days(1) + Duration::hours(2) + Duration::minutes(30) + Duration::seconds(45)
        );
    }

    #[test]
    fn test_parse_duration_case_insensitive() {
        let d = parse_duration("30D").unwrap();
        assert_eq!(d, Duration::days(30));
    }

    #[test]
    fn test_parse_duration_empty() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn test_parse_duration_missing_unit() {
        assert!(parse_duration("90").is_err());
    }

    #[test]
    fn test_parse_duration_invalid_unit() {
        assert!(parse_duration("90x").is_err());
    }

    #[test]
    fn test_parse_duration_zero() {
        assert!(parse_duration("0d").is_err());
    }
}
