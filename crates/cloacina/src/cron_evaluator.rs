/*
 *  Copyright 2025 Colliery Software
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

//! Timezone-aware cron expression evaluator
//!
//! This module provides functionality for parsing and evaluating cron expressions
//! with proper timezone support. It uses the `croner` crate for cron parsing and
//! `chrono-tz` for timezone handling.
//!
//! # Examples
//!
//! ```rust
//! use cloacina::cron_evaluator::CronEvaluator;
//! use chrono::{DateTime, Utc};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create evaluator for daily 9 AM EST/EDT
//! let evaluator = CronEvaluator::new("0 9 * * *", "America/New_York")?;
//!
//! // Find next execution after current time
//! let now = Utc::now();
//! let next = evaluator.next_execution(now)?;
//!
//! println!("Next execution: {}", next);
//! # Ok(())
//! # }
//! ```

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use croner::Cron;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur during cron evaluation.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum CronError {
    /// Invalid cron expression format.
    #[error("Invalid cron expression: {0}")]
    InvalidExpression(String),

    /// Invalid timezone string.
    #[error("Invalid timezone: {0}")]
    InvalidTimezone(String),

    /// No next execution time found (e.g., end of time range).
    #[error("No next execution time found")]
    NoNextExecution,

    /// Error from the croner crate.
    #[error("Cron parsing error: {0}")]
    CronParsingError(String),
}

/// Timezone-aware cron expression evaluator.
///
/// This struct provides methods for evaluating cron expressions in specific timezones,
/// handling daylight saving time transitions automatically.
///
/// # Examples
///
/// ```rust
/// use cloacina::cron_evaluator::CronEvaluator;
/// use chrono::Utc;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Daily at 2 AM Eastern Time (handles EST/EDT automatically)
/// let evaluator = CronEvaluator::new("0 2 * * *", "America/New_York")?;
/// let next = evaluator.next_execution(Utc::now())?;
///
/// // Hourly during business hours in London
/// let evaluator = CronEvaluator::new("0 9-17 * * 1-5", "Europe/London")?;
/// let next = evaluator.next_execution(Utc::now())?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct CronEvaluator {
    /// Parsed cron expression
    cron: Cron,
    /// Timezone for interpreting the cron expression
    timezone: Tz,
    /// Original cron expression string for debugging
    expression: String,
    /// Original timezone string for debugging
    timezone_str: String,
}

impl CronEvaluator {
    /// Creates a new cron evaluator with the specified expression and timezone.
    ///
    /// # Arguments
    /// * `cron_expr` - Standard cron expression (5 fields: minute hour day month weekday)
    /// * `timezone_str` - IANA timezone name (e.g., "America/New_York", "Europe/London", "UTC")
    ///
    /// # Returns
    /// * `Result<Self, CronError>` - New evaluator instance or error
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::cron_evaluator::CronEvaluator;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Daily at 9 AM Eastern Time
    /// let evaluator = CronEvaluator::new("0 9 * * *", "America/New_York")?;
    ///
    /// // Every 15 minutes during UTC business hours
    /// let evaluator = CronEvaluator::new("*/15 9-17 * * 1-5", "UTC")?;
    ///
    /// // Monthly on the 1st at midnight Pacific Time
    /// let evaluator = CronEvaluator::new("0 0 1 * *", "America/Los_Angeles")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(cron_expr: &str, timezone_str: &str) -> Result<Self, CronError> {
        let cron = Cron::new(cron_expr)
            .with_seconds_optional() // Enable optional seconds support
            .parse()
            .map_err(|e| CronError::CronParsingError(e.to_string()))?;

        // Parse the timezone
        let timezone: Tz = timezone_str
            .parse()
            .map_err(|_| CronError::InvalidTimezone(timezone_str.to_string()))?;

        Ok(Self {
            cron,
            timezone,
            expression: cron_expr.to_string(),
            timezone_str: timezone_str.to_string(),
        })
    }

    /// Finds the next execution time after the given timestamp.
    ///
    /// This method converts the UTC timestamp to the evaluator's timezone,
    /// finds the next cron match in that timezone, then converts back to UTC.
    ///
    /// # Arguments
    /// * `after` - UTC timestamp to find the next execution after
    ///
    /// # Returns
    /// * `Result<DateTime<Utc>, CronError>` - Next execution time in UTC
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::cron_evaluator::CronEvaluator;
    /// use chrono::Utc;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let evaluator = CronEvaluator::new("0 14 * * *", "America/New_York")?;
    /// let now = Utc::now();
    /// let next = evaluator.next_execution(now)?;
    ///
    /// // Next execution will be at 2 PM Eastern Time, converted to UTC
    /// // During EST: 7 PM UTC, During EDT: 6 PM UTC
    /// # Ok(())
    /// # }
    /// ```
    pub fn next_execution(&self, after: DateTime<Utc>) -> Result<DateTime<Utc>, CronError> {
        // Convert UTC time to the target timezone
        let local_time = self.timezone.from_utc_datetime(&after.naive_utc());

        // Find the next execution in the local timezone
        let next_local = self
            .cron
            .find_next_occurrence(&local_time, false)
            .map_err(|e| CronError::CronParsingError(e.to_string()))?;

        // Convert back to UTC for storage and comparison
        Ok(next_local.with_timezone(&Utc))
    }

    /// Finds multiple next execution times after the given timestamp.
    ///
    /// This is useful for catchup policies that need to run multiple missed executions.
    ///
    /// # Arguments
    /// * `after` - UTC timestamp to find executions after
    /// * `limit` - Maximum number of executions to return
    ///
    /// # Returns
    /// * `Result<Vec<DateTime<Utc>>, CronError>` - List of next execution times in UTC
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::cron_evaluator::CronEvaluator;
    /// use chrono::Utc;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let evaluator = CronEvaluator::new("0 */6 * * *", "UTC").unwrap();
    /// let now = Utc::now();
    /// let next_executions = evaluator.next_executions(now, 5)?;
    ///
    /// // Returns next 5 executions every 6 hours
    /// # Ok(())
    /// # }
    /// ```
    pub fn next_executions(
        &self,
        after: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<DateTime<Utc>>, CronError> {
        let mut executions = Vec::with_capacity(limit);
        let mut current_time = after;

        for _ in 0..limit {
            match self.next_execution(current_time) {
                Ok(next_time) => {
                    executions.push(next_time);
                    current_time = next_time;
                }
                Err(CronError::NoNextExecution) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(executions)
    }

    /// Finds all execution times between two timestamps.
    ///
    /// This is useful for implementing catchup policies that need to execute
    /// all missed schedules within a time range.
    ///
    /// # Arguments
    /// * `start` - Start of the time range (inclusive)
    /// * `end` - End of the time range (exclusive)
    /// * `max_executions` - Maximum number of executions to prevent runaway
    ///
    /// # Returns
    /// * `Result<Vec<DateTime<Utc>>, CronError>` - List of execution times in the range
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::cron_evaluator::CronEvaluator;
    /// use chrono::{Duration, Utc};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let evaluator = CronEvaluator::new("0 * * * *", "UTC")?; // Hourly
    /// let start = Utc::now() - Duration::hours(6);
    /// let end = Utc::now();
    /// let missed = evaluator.executions_between(start, end, 10)?;
    ///
    /// // Returns up to 6 hourly executions from the past 6 hours
    /// # Ok(())
    /// # }
    /// ```
    pub fn executions_between(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        max_executions: usize,
    ) -> Result<Vec<DateTime<Utc>>, CronError> {
        let mut executions = Vec::new();
        let mut current_time = start;

        for _ in 0..max_executions {
            match self.next_execution(current_time) {
                Ok(next_time) => {
                    if next_time >= end {
                        break;
                    }
                    executions.push(next_time);
                    current_time = next_time;
                }
                Err(CronError::NoNextExecution) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(executions)
    }

    /// Returns the original cron expression string.
    pub fn expression(&self) -> &str {
        &self.expression
    }

    /// Returns the timezone string.
    pub fn timezone_str(&self) -> &str {
        &self.timezone_str
    }

    /// Returns the timezone object.
    pub fn timezone(&self) -> Tz {
        self.timezone
    }

    /// Validates a cron expression without creating an evaluator.
    ///
    /// # Arguments
    /// * `cron_expr` - Cron expression to validate
    ///
    /// # Returns
    /// * `Result<(), CronError>` - Success or validation error
    pub fn validate_expression(cron_expr: &str) -> Result<(), CronError> {
        Cron::new(cron_expr)
            .with_seconds_optional() // Enable optional seconds support
            .parse()
            .map_err(|e| CronError::CronParsingError(e.to_string()))?;
        Ok(())
    }

    /// Validates a timezone string.
    ///
    /// # Arguments
    /// * `timezone_str` - Timezone string to validate
    ///
    /// # Returns
    /// * `Result<(), CronError>` - Success or validation error
    pub fn validate_timezone(timezone_str: &str) -> Result<(), CronError> {
        timezone_str
            .parse::<Tz>()
            .map_err(|_| CronError::InvalidTimezone(timezone_str.to_string()))?;
        Ok(())
    }

    /// Validates both cron expression and timezone.
    ///
    /// # Arguments
    /// * `cron_expr` - Cron expression to validate
    /// * `timezone_str` - Timezone string to validate
    ///
    /// # Returns
    /// * `Result<(), CronError>` - Success or validation error
    pub fn validate(cron_expr: &str, timezone_str: &str) -> Result<(), CronError> {
        Self::validate_expression(cron_expr)?;
        Self::validate_timezone(timezone_str)?;
        Ok(())
    }
}

impl FromStr for CronEvaluator {
    type Err = CronError;

    /// Creates a CronEvaluator from a string in the format "expression@timezone"
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::cron_evaluator::CronEvaluator;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let evaluator: CronEvaluator = "0 9 * * *@America/New_York".parse()?;
    /// let evaluator: CronEvaluator = "*/15 * * * *@UTC".parse()?;
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('@').collect();
        if parts.len() != 2 {
            return Err(CronError::InvalidExpression(
                "Format should be 'expression@timezone'".to_string(),
            ));
        }

        Self::new(parts[0], parts[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone, Timelike};

    #[test]
    fn test_cron_evaluator_creation() {
        let evaluator = CronEvaluator::new("0 9 * * *", "America/New_York").unwrap();
        assert_eq!(evaluator.expression(), "0 9 * * *");
        assert_eq!(evaluator.timezone_str(), "America/New_York");
    }

    #[test]
    fn test_invalid_cron_expression() {
        let result = CronEvaluator::new("invalid", "UTC");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CronError::CronParsingError(_)
        ));
    }

    #[test]
    fn test_invalid_timezone() {
        let result = CronEvaluator::new("0 9 * * *", "Invalid/Timezone");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CronError::InvalidTimezone(_)));
    }

    #[test]
    fn test_next_execution_utc() {
        let evaluator = CronEvaluator::new("0 12 * * *", "UTC").unwrap(); // Daily at noon UTC
        let start = Utc.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap(); // 10 AM UTC
        let next = evaluator.next_execution(start).unwrap();

        // Should be noon on the same day
        assert_eq!(next.hour(), 12);
        assert_eq!(next.minute(), 0);
        assert_eq!(next.day(), 1);
    }

    #[test]
    fn test_next_execution_timezone() {
        let evaluator = CronEvaluator::new("0 9 * * *", "America/New_York").unwrap(); // 9 AM Eastern
        let start = Utc.with_ymd_and_hms(2025, 1, 1, 10, 0, 0).unwrap(); // 10 AM UTC (5 AM EST)

        let next = evaluator.next_execution(start).unwrap();

        // Should be 9 AM Eastern, which is 2 PM UTC in January (EST)
        assert_eq!(next.hour(), 14); // 2 PM UTC
        assert_eq!(next.minute(), 0);
    }

    #[test]
    fn test_next_executions() {
        let evaluator = CronEvaluator::new("0 */6 * * *", "UTC").unwrap(); // Every 6 hours
        let start = Utc.with_ymd_and_hms(2025, 1, 1, 1, 0, 0).unwrap();

        let executions = evaluator.next_executions(start, 3).unwrap();

        assert_eq!(executions.len(), 3);
        assert_eq!(executions[0].hour(), 6);
        assert_eq!(executions[1].hour(), 12);
        assert_eq!(executions[2].hour(), 18);
    }

    #[test]
    fn test_executions_between() {
        let evaluator = CronEvaluator::new("0 * * * *", "UTC").unwrap(); // Hourly
        let start = Utc.with_ymd_and_hms(2025, 1, 1, 10, 30, 0).unwrap(); // 10:30 AM UTC
        let end = Utc.with_ymd_and_hms(2025, 1, 1, 14, 0, 0).unwrap(); // 2:00 PM UTC

        let executions = evaluator.executions_between(start, end, 10).unwrap();

        // Should get executions at 11:00, 12:00, 13:00 (3 total)
        assert_eq!(executions.len(), 3);
        assert_eq!(executions[0].hour(), 11);
        assert_eq!(executions[1].hour(), 12);
        assert_eq!(executions[2].hour(), 13);
    }

    #[test]
    fn test_validation_functions() {
        assert!(CronEvaluator::validate_expression("0 9 * * *").is_ok());
        assert!(CronEvaluator::validate_expression("invalid").is_err());

        assert!(CronEvaluator::validate_timezone("UTC").is_ok());
        assert!(CronEvaluator::validate_timezone("Invalid/Zone").is_err());

        assert!(CronEvaluator::validate("0 9 * * *", "UTC").is_ok());
        assert!(CronEvaluator::validate("invalid", "UTC").is_err());
        assert!(CronEvaluator::validate("0 9 * * *", "Invalid/Zone").is_err());
    }

    #[test]
    fn test_from_str() {
        let evaluator: CronEvaluator = "0 9 * * *@America/New_York".parse().unwrap();
        assert_eq!(evaluator.expression(), "0 9 * * *");
        assert_eq!(evaluator.timezone_str(), "America/New_York");

        let result: Result<CronEvaluator, _> = "invalid format".parse();
        assert!(result.is_err());
    }
}
