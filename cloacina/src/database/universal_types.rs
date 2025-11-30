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
//! This module provides wrapper types that work seamlessly with both PostgreSQL and SQLite
//! backends, handling the different type representations between the two databases.

use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};

#[cfg(feature = "sqlite")]
use diesel::serialize::IsNull;
use diesel::{AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use diesel::sql_types::Binary;
#[cfg(feature = "sqlite")]
use diesel::sqlite::Sqlite;

#[cfg(feature = "postgres")]
use diesel::pg::Pg;

#[cfg(feature = "sqlite")]
use diesel::sql_types::Text;

use chrono::{DateTime, Utc};
#[cfg(feature = "postgres")]
use chrono::{NaiveDateTime, TimeZone};

/// Universal UUID wrapper that works with both PostgreSQL and SQLite
#[derive(Debug, Clone, Copy, FromSqlRow, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlite", derive(AsExpression))]
#[cfg_attr(feature = "sqlite", diesel(sql_type = Binary))]
#[cfg_attr(feature = "postgres", derive(AsExpression))]
#[cfg_attr(feature = "postgres", diesel(sql_type = diesel::sql_types::Uuid))]
pub struct UniversalUuid(pub Uuid);

impl UniversalUuid {
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
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

// SQLite Binary storage implementation
#[cfg(feature = "sqlite")]
impl FromSql<Binary, Sqlite> for UniversalUuid {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let bytes = <Vec<u8> as FromSql<Binary, Sqlite>>::from_sql(bytes)?;
        if bytes.len() != 16 {
            return Err("Invalid UUID byte length".into());
        }
        let uuid = Uuid::from_slice(&bytes)?;
        Ok(UniversalUuid(uuid))
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<Binary, Sqlite> for UniversalUuid {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.0.as_bytes().to_vec());
        Ok(IsNull::No)
    }
}

// PostgreSQL implementations (uses native UUID type)
#[cfg(feature = "postgres")]
impl FromSql<diesel::sql_types::Uuid, Pg> for UniversalUuid {
    fn from_sql(
        bytes: <Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let uuid = <Uuid as FromSql<diesel::sql_types::Uuid, Pg>>::from_sql(bytes)?;
        Ok(UniversalUuid(uuid))
    }
}

#[cfg(feature = "postgres")]
impl ToSql<diesel::sql_types::Uuid, Pg> for UniversalUuid {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        <Uuid as ToSql<diesel::sql_types::Uuid, Pg>>::to_sql(&self.0, out)
    }
}

/// Universal timestamp wrapper that works with both PostgreSQL and SQLite
///
/// For PostgreSQL, maps to native Timestamp type.
/// For SQLite, stores as RFC3339 text string.
#[derive(Debug, Clone, Copy, FromSqlRow, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlite", derive(AsExpression))]
#[cfg_attr(feature = "sqlite", diesel(sql_type = Text))]
#[cfg_attr(feature = "postgres", derive(AsExpression))]
#[cfg_attr(feature = "postgres", diesel(sql_type = diesel::sql_types::Timestamp))]
pub struct UniversalTimestamp(pub DateTime<Utc>);

#[cfg(feature = "postgres")]
impl FromSql<diesel::sql_types::Timestamp, Pg> for UniversalTimestamp {
    fn from_sql(
        bytes: <Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let naive =
            <chrono::NaiveDateTime as FromSql<diesel::sql_types::Timestamp, Pg>>::from_sql(bytes)?;
        Ok(UniversalTimestamp(Utc.from_utc_datetime(&naive)))
    }
}

#[cfg(feature = "postgres")]
impl ToSql<diesel::sql_types::Timestamp, Pg> for UniversalTimestamp {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        // This is a workaround for the lifetime issue
        // We know the value is used immediately within this call
        let naive = self.0.naive_utc();
        let naive_ref: &NaiveDateTime =
            unsafe { std::mem::transmute(&naive as *const NaiveDateTime) };
        <NaiveDateTime as ToSql<diesel::sql_types::Timestamp, Pg>>::to_sql(naive_ref, out)
    }
}

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
}

// SQLite Text storage implementation
#[cfg(feature = "sqlite")]
impl FromSql<Text, Sqlite> for UniversalTimestamp {
    fn from_sql(
        value: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let text = <String as FromSql<Text, Sqlite>>::from_sql(value)?;
        let datetime = DateTime::parse_from_rfc3339(&text)
            .map_err(|e| format!("Invalid timestamp format: {}", e))?
            .with_timezone(&Utc);
        Ok(UniversalTimestamp(datetime))
    }
}

#[cfg(feature = "sqlite")]
impl ToSql<Text, Sqlite> for UniversalTimestamp {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let text = self.0.to_rfc3339();
        out.set_value(text);
        Ok(IsNull::No)
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

// Helper function for current timestamp

pub fn current_timestamp() -> UniversalTimestamp {
    UniversalTimestamp::now()
}

/// Universal boolean wrapper that works with both PostgreSQL and SQLite
///
/// For PostgreSQL, maps to native Bool type.
/// For SQLite, stores as Integer (0/1).
#[derive(Debug, Clone, Copy, FromSqlRow, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "postgres", derive(AsExpression))]
#[cfg_attr(feature = "postgres", diesel(sql_type = diesel::sql_types::Bool))]
#[cfg_attr(feature = "sqlite", derive(AsExpression))]
#[cfg_attr(feature = "sqlite", diesel(sql_type = diesel::sql_types::Integer))]
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

// PostgreSQL implementations
#[cfg(feature = "postgres")]
impl ToSql<diesel::sql_types::Bool, diesel::pg::Pg> for UniversalBool {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        <bool as ToSql<diesel::sql_types::Bool, diesel::pg::Pg>>::to_sql(&self.0, out)
    }
}

#[cfg(feature = "postgres")]
impl FromSql<diesel::sql_types::Bool, diesel::pg::Pg> for UniversalBool {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        Ok(Self(<bool as FromSql<
            diesel::sql_types::Bool,
            diesel::pg::Pg,
        >>::from_sql(bytes)?))
    }
}

// SQLite implementations - store as integer (0/1)
#[cfg(feature = "sqlite")]
impl ToSql<diesel::sql_types::Integer, diesel::sqlite::Sqlite> for UniversalBool {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        let int_value = if self.0 { 1i32 } else { 0i32 };
        out.set_value(int_value);
        Ok(IsNull::No)
    }
}

#[cfg(feature = "sqlite")]
impl FromSql<diesel::sql_types::Integer, diesel::sqlite::Sqlite> for UniversalBool {
    fn from_sql(
        value: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        let int_value =
            <i32 as FromSql<diesel::sql_types::Integer, diesel::sqlite::Sqlite>>::from_sql(value)?;
        Ok(Self(int_value != 0))
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
    fn test_universal_uuid_display() {
        let uuid = UniversalUuid::new_v4();
        let display = format!("{}", uuid);
        assert_eq!(display, uuid.to_string());
    }

    #[test]
    fn test_universal_uuid_as_uuid() {
        let uuid = UniversalUuid::new_v4();
        let inner = uuid.as_uuid();
        assert_eq!(inner, uuid.0);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_universal_timestamp_sqlite() {
        let now = Utc::now();
        let ts = UniversalTimestamp::from(now);
        let back: DateTime<Utc> = ts.into();

        // Should be equal to the second
        assert_eq!(now.timestamp(), back.timestamp());
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn test_universal_timestamp_postgres() {
        // For PostgreSQL, UniversalTimestamp is just a type alias
        let now: UniversalTimestamp = crate::UniversalTimestamp(Utc::now());
        // Should compile and work normally
        assert!(now.0.timestamp() > 0);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_universal_timestamp_as_datetime() {
        let now = Utc::now();
        let ts = UniversalTimestamp::from(now);
        assert_eq!(ts.as_datetime(), &now);
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        // Should be a recent timestamp
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
