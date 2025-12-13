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
use diesel::deserialize::{FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::{Binary, Bool, Integer, Text, Timestamp};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Write;
use uuid::Uuid;

// ============================================================================
// Custom SQL Types for Diesel
// ============================================================================
// These custom SQL types allow a unified schema definition that works with
// both PostgreSQL and SQLite backends.

/// Custom SQL type for UUIDs that works across backends.
/// PostgreSQL: maps to native UUID type
/// SQLite: maps to BLOB (16-byte binary)
#[derive(Debug, Clone, Copy, diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
#[diesel(postgres_type(name = "uuid"))]
#[diesel(sqlite_type(name = "Binary"))]
pub struct DbUuid;

/// Custom SQL type for timestamps that works across backends.
/// PostgreSQL: maps to native TIMESTAMP type
/// SQLite: maps to TEXT (RFC3339 string format)
#[derive(Debug, Clone, Copy, diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
#[diesel(postgres_type(name = "timestamp"))]
#[diesel(sqlite_type(name = "Text"))]
pub struct DbTimestamp;

/// Custom SQL type for booleans that works across backends.
/// PostgreSQL: maps to native BOOL type
/// SQLite: maps to INTEGER (0/1)
#[derive(Debug, Clone, Copy, diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
#[diesel(postgres_type(name = "bool"))]
#[diesel(sqlite_type(name = "Integer"))]
pub struct DbBool;

/// Custom SQL type for binary data that works across backends.
/// PostgreSQL: maps to BYTEA
/// SQLite: maps to BLOB
#[derive(Debug, Clone, Copy, diesel::sql_types::SqlType, diesel::query_builder::QueryId)]
#[diesel(postgres_type(name = "bytea"))]
#[diesel(sqlite_type(name = "Binary"))]
pub struct DbBinary;

/// Universal UUID wrapper for cross-database compatibility
///
/// This is a domain type that wraps uuid::Uuid with Diesel support for
/// both PostgreSQL (native UUID) and SQLite (BLOB) backends.
#[derive(
    Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = DbUuid)]
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

// PostgreSQL FromSql/ToSql for UniversalUuid
#[cfg(feature = "postgres")]
impl FromSql<DbUuid, diesel::pg::Pg> for UniversalUuid {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let uuid =
            <uuid::Uuid as FromSql<diesel::sql_types::Uuid, diesel::pg::Pg>>::from_sql(bytes)?;
        Ok(UniversalUuid(uuid))
    }
}

#[cfg(feature = "postgres")]
impl ToSql<DbUuid, diesel::pg::Pg> for UniversalUuid {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        <uuid::Uuid as ToSql<diesel::sql_types::Uuid, diesel::pg::Pg>>::to_sql(&self.0, out)
    }
}

// SQLite FromSql/ToSql for UniversalUuid
#[cfg(feature = "sqlite")]
impl FromSql<DbUuid, diesel::sqlite::Sqlite> for UniversalUuid {
    fn from_sql(
        bytes: diesel::sqlite::SqliteValue<'_, '_, '_>,
    ) -> diesel::deserialize::Result<Self> {
        let blob = <Vec<u8> as FromSql<Binary, diesel::sqlite::Sqlite>>::from_sql(bytes)?;
        let uuid = Uuid::from_slice(&blob).map_err(|e| format!("Invalid UUID bytes: {}", e))?;
        Ok(UniversalUuid(uuid))
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<DbUuid, diesel::sqlite::Sqlite> for UniversalUuid {
    fn to_sql<'b>(
        &'b self,
        out: &mut Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(self.0.as_bytes().to_vec());
        Ok(diesel::serialize::IsNull::No)
    }
}

/// Universal timestamp wrapper for cross-database compatibility
///
/// This is a domain type that wraps DateTime<Utc> with Diesel support for
/// both PostgreSQL (native TIMESTAMP) and SQLite (TEXT as RFC3339) backends.
#[derive(
    Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = DbTimestamp)]
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
        DateTime::parse_from_rfc3339(s).map(|dt| UniversalTimestamp(dt.with_timezone(&Utc)))
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

// PostgreSQL FromSql/ToSql for UniversalTimestamp
#[cfg(feature = "postgres")]
impl FromSql<DbTimestamp, diesel::pg::Pg> for UniversalTimestamp {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let naive = <chrono::NaiveDateTime as FromSql<Timestamp, diesel::pg::Pg>>::from_sql(bytes)?;
        Ok(UniversalTimestamp::from_naive(naive))
    }
}

#[cfg(feature = "postgres")]
impl ToSql<DbTimestamp, diesel::pg::Pg> for UniversalTimestamp {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        // Write NaiveDateTime directly - Diesel's PG impl writes timestamp as i64 microseconds
        let naive = self.to_naive();
        let micros = naive.and_utc().timestamp_micros()
            - chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp_micros();
        out.write_all(&micros.to_be_bytes())?;
        Ok(diesel::serialize::IsNull::No)
    }
}

// SQLite FromSql/ToSql for UniversalTimestamp
#[cfg(feature = "sqlite")]
impl FromSql<DbTimestamp, diesel::sqlite::Sqlite> for UniversalTimestamp {
    fn from_sql(
        bytes: diesel::sqlite::SqliteValue<'_, '_, '_>,
    ) -> diesel::deserialize::Result<Self> {
        let text = <String as FromSql<Text, diesel::sqlite::Sqlite>>::from_sql(bytes)?;
        UniversalTimestamp::from_rfc3339(&text)
            .map_err(|e| format!("Invalid timestamp: {}", e).into())
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<DbTimestamp, diesel::sqlite::Sqlite> for UniversalTimestamp {
    fn to_sql<'b>(
        &'b self,
        out: &mut Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(self.to_rfc3339());
        Ok(diesel::serialize::IsNull::No)
    }
}

/// Helper function for current timestamp
pub fn current_timestamp() -> UniversalTimestamp {
    UniversalTimestamp::now()
}

/// Universal boolean wrapper for cross-database compatibility
///
/// This is a domain type that wraps bool with Diesel support for
/// both PostgreSQL (native BOOL) and SQLite (INTEGER 0/1) backends.
#[derive(
    Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize, AsExpression, FromSqlRow,
)]
#[diesel(sql_type = DbBool)]
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
        if self.0 {
            1
        } else {
            0
        }
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

// PostgreSQL FromSql/ToSql for UniversalBool
#[cfg(feature = "postgres")]
impl FromSql<DbBool, diesel::pg::Pg> for UniversalBool {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let value = <bool as FromSql<Bool, diesel::pg::Pg>>::from_sql(bytes)?;
        Ok(UniversalBool(value))
    }
}

#[cfg(feature = "postgres")]
impl ToSql<DbBool, diesel::pg::Pg> for UniversalBool {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        <bool as ToSql<Bool, diesel::pg::Pg>>::to_sql(&self.0, out)
    }
}

// SQLite FromSql/ToSql for UniversalBool
#[cfg(feature = "sqlite")]
impl FromSql<DbBool, diesel::sqlite::Sqlite> for UniversalBool {
    fn from_sql(
        bytes: diesel::sqlite::SqliteValue<'_, '_, '_>,
    ) -> diesel::deserialize::Result<Self> {
        let value = <i32 as FromSql<Integer, diesel::sqlite::Sqlite>>::from_sql(bytes)?;
        Ok(UniversalBool(value != 0))
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<DbBool, diesel::sqlite::Sqlite> for UniversalBool {
    fn to_sql<'b>(
        &'b self,
        out: &mut Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        let value: i32 = if self.0 { 1 } else { 0 };
        out.set_value(value);
        Ok(diesel::serialize::IsNull::No)
    }
}

/// Universal binary wrapper for cross-database compatibility
///
/// This is a domain type that wraps Vec<u8> with Diesel support for
/// both PostgreSQL (BYTEA) and SQLite (BLOB) backends.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = DbBinary)]
pub struct UniversalBinary(pub Vec<u8>);

impl UniversalBinary {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl From<Vec<u8>> for UniversalBinary {
    fn from(data: Vec<u8>) -> Self {
        Self(data)
    }
}

impl From<UniversalBinary> for Vec<u8> {
    fn from(wrapper: UniversalBinary) -> Self {
        wrapper.0
    }
}

impl From<&[u8]> for UniversalBinary {
    fn from(data: &[u8]) -> Self {
        Self(data.to_vec())
    }
}

// PostgreSQL FromSql/ToSql for UniversalBinary
#[cfg(feature = "postgres")]
impl FromSql<DbBinary, diesel::pg::Pg> for UniversalBinary {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        let data =
            <Vec<u8> as FromSql<diesel::sql_types::Binary, diesel::pg::Pg>>::from_sql(bytes)?;
        Ok(UniversalBinary(data))
    }
}

#[cfg(feature = "postgres")]
impl ToSql<DbBinary, diesel::pg::Pg> for UniversalBinary {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        out.write_all(&self.0)?;
        Ok(diesel::serialize::IsNull::No)
    }
}

// SQLite FromSql/ToSql for UniversalBinary
#[cfg(feature = "sqlite")]
impl FromSql<DbBinary, diesel::sqlite::Sqlite> for UniversalBinary {
    fn from_sql(
        bytes: diesel::sqlite::SqliteValue<'_, '_, '_>,
    ) -> diesel::deserialize::Result<Self> {
        let data = <Vec<u8> as FromSql<Binary, diesel::sqlite::Sqlite>>::from_sql(bytes)?;
        Ok(UniversalBinary(data))
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<DbBinary, diesel::sqlite::Sqlite> for UniversalBinary {
    fn to_sql<'b>(
        &'b self,
        out: &mut Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(self.0.clone());
        Ok(diesel::serialize::IsNull::No)
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
