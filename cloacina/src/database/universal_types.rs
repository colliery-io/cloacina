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

//! Universal type wrappers for cross-database compatibility
//!
//! This module provides wrapper types that work as domain types, convertible
//! to/from backend-specific database types. These types are used at the API
//! boundary and in business logic, while backend-specific models handle
//! the actual database storage.
//!
//! # Architecture
//!
//! When both postgres and sqlite features are enabled:
//! - Domain code uses UniversalUuid, UniversalTimestamp, UniversalBool
//! - PostgreSQL DAL converts to/from uuid::Uuid, NaiveDateTime, bool
//! - SQLite DAL converts to/from Vec<u8>, String, i32
//!
//! This avoids conflicting Diesel trait implementations by keeping
//! Diesel-specific code isolated in backend-specific model modules.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Universal UUID wrapper for cross-database compatibility
///
/// This is a domain type that wraps uuid::Uuid. It does not have Diesel
/// derives - instead, backend-specific models use native types and convert
/// to/from this type at the DAL boundary.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniversalUuid(pub Uuid);

impl UniversalUuid {
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Convert to bytes for SQLite BLOB storage
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }

    /// Create from bytes (SQLite BLOB)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, uuid::Error> {
        Uuid::from_slice(bytes).map(UniversalUuid)
    }
}

impl fmt::Display for UniversalUuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for UniversalUuid {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<UniversalUuid> for Uuid {
    fn from(wrapper: UniversalUuid) -> Self {
        wrapper.0
    }
}

impl From<&UniversalUuid> for Uuid {
    fn from(wrapper: &UniversalUuid) -> Self {
        wrapper.0
    }
}

/// Universal timestamp wrapper for cross-database compatibility
///
/// This is a domain type that wraps DateTime<Utc>. Backend-specific models
/// handle conversion to/from database-native types.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniversalTimestamp(pub DateTime<Utc>);

impl UniversalTimestamp {
    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    pub fn into_inner(self) -> DateTime<Utc> {
        self.0
    }

    /// Convert to RFC3339 string for SQLite TEXT storage
    pub fn to_rfc3339(&self) -> String {
        self.0.to_rfc3339()
    }

    /// Create from RFC3339 string (SQLite TEXT)
    pub fn from_rfc3339(s: &str) -> Result<Self, chrono::ParseError> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| UniversalTimestamp(dt.with_timezone(&Utc)))
    }

    /// Convert to NaiveDateTime for PostgreSQL TIMESTAMP storage
    pub fn to_naive(&self) -> chrono::NaiveDateTime {
        self.0.naive_utc()
    }

    /// Create from NaiveDateTime (PostgreSQL TIMESTAMP)
    pub fn from_naive(naive: chrono::NaiveDateTime) -> Self {
        use chrono::TimeZone;
        UniversalTimestamp(Utc.from_utc_datetime(&naive))
    }
}

impl fmt::Display for UniversalTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_rfc3339())
    }
}

impl From<DateTime<Utc>> for UniversalTimestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<UniversalTimestamp> for DateTime<Utc> {
    fn from(wrapper: UniversalTimestamp) -> Self {
        wrapper.0
    }
}

impl From<chrono::NaiveDateTime> for UniversalTimestamp {
    fn from(naive: chrono::NaiveDateTime) -> Self {
        Self::from_naive(naive)
    }
}

/// Helper function for current timestamp
pub fn current_timestamp() -> UniversalTimestamp {
    UniversalTimestamp::now()
}

/// Universal boolean wrapper for cross-database compatibility
///
/// This is a domain type that wraps bool. Backend-specific models
/// handle conversion to/from database-native types.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniversalBool(pub bool);

impl UniversalBool {
    pub fn new(value: bool) -> Self {
        Self(value)
    }

    pub fn is_true(&self) -> bool {
        self.0
    }

    pub fn is_false(&self) -> bool {
        !self.0
    }

    /// Convert to i32 for SQLite INTEGER storage
    pub fn to_i32(&self) -> i32 {
        if self.0 { 1 } else { 0 }
    }

    /// Create from i32 (SQLite INTEGER)
    pub fn from_i32(value: i32) -> Self {
        Self(value != 0)
    }
}

impl From<bool> for UniversalBool {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

impl From<UniversalBool> for bool {
    fn from(wrapper: UniversalBool) -> Self {
        wrapper.0
    }
}

impl fmt::Display for UniversalBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universal_uuid_creation() {
        let uuid = UniversalUuid::new_v4();
        assert!(!uuid.to_string().is_empty());

        // Test conversion from/to standard UUID
        let std_uuid = Uuid::new_v4();
        let universal = UniversalUuid::from(std_uuid);
        let back: Uuid = universal.into();
        assert_eq!(std_uuid, back);
    }

    #[test]
    fn test_universal_uuid_bytes() {
        let uuid = UniversalUuid::new_v4();
        let bytes = uuid.as_bytes();
        let reconstructed = UniversalUuid::from_bytes(bytes).unwrap();
        assert_eq!(uuid, reconstructed);
    }

    #[test]
    fn test_universal_uuid_display() {
        let uuid = UniversalUuid::new_v4();
        let display = format!("{}", uuid);
        assert_eq!(display, uuid.to_string());
    }

    #[test]
    fn test_universal_timestamp_now() {
        let ts = UniversalTimestamp::now();
        assert!(ts.0.timestamp() > 0);
    }

    #[test]
    fn test_universal_timestamp_rfc3339() {
        let now = Utc::now();
        let ts = UniversalTimestamp::from(now);
        let s = ts.to_rfc3339();
        let back = UniversalTimestamp::from_rfc3339(&s).unwrap();
        // Compare to the second (rfc3339 may lose sub-second precision depending on format)
        assert_eq!(ts.0.timestamp(), back.0.timestamp());
    }

    #[test]
    fn test_universal_timestamp_naive() {
        let now = Utc::now();
        let ts = UniversalTimestamp::from(now);
        let naive = ts.to_naive();
        let back = UniversalTimestamp::from_naive(naive);
        // NaiveDateTime preserves precision
        assert_eq!(ts.0.timestamp(), back.0.timestamp());
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts.0.timestamp() > 0);
    }

    #[test]
    fn test_universal_bool_creation() {
        let bool_true = UniversalBool::new(true);
        let bool_false = UniversalBool::new(false);

        assert!(bool_true.is_true());
        assert!(!bool_true.is_false());
        assert!(bool_false.is_false());
        assert!(!bool_false.is_true());
    }

    #[test]
    fn test_universal_bool_i32() {
        let bool_true = UniversalBool::new(true);
        let bool_false = UniversalBool::new(false);

        assert_eq!(bool_true.to_i32(), 1);
        assert_eq!(bool_false.to_i32(), 0);

        assert!(UniversalBool::from_i32(1).is_true());
        assert!(UniversalBool::from_i32(0).is_false());
        assert!(UniversalBool::from_i32(42).is_true()); // Any non-zero is true
    }

    #[test]
    fn test_universal_bool_conversion() {
        let rust_bool = true;
        let universal = UniversalBool::from(rust_bool);
        let back: bool = universal.into();
        assert_eq!(rust_bool, back);

        let rust_bool = false;
        let universal = UniversalBool::from(rust_bool);
        let back: bool = universal.into();
        assert_eq!(rust_bool, back);
    }

    #[test]
    fn test_universal_bool_display() {
        let bool_true = UniversalBool::new(true);
        let bool_false = UniversalBool::new(false);

        assert_eq!(format!("{}", bool_true), "true");
        assert_eq!(format!("{}", bool_false), "false");
    }
}
