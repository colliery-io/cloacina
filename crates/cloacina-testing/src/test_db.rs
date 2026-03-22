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

//! Test database helpers — real in-memory SQLite with migrations.
//!
//! These helpers create isolated, production-equivalent databases for testing.
//! No mocks — all queries run against real SQL with the real schema.
//!
//! # Example
//!
//! ```rust,ignore
//! use cloacina_testing::test_db::test_dal;
//!
//! #[tokio::test]
//! async fn test_cron_schedule_crud() {
//!     let dal = test_dal().await;
//!     // Use dal.cron_schedule().create(...) etc. with real SQL
//! }
//! ```

use cloacina::dal::DAL;
use cloacina::database::Database;

/// Create an in-memory SQLite `Database` with all migrations applied.
///
/// Each call returns a fresh, isolated database. PRAGMAs are set to match
/// production (foreign_keys=ON, WAL, busy_timeout=30s).
///
/// # Panics
///
/// Panics if database creation or migration fails — this is a test helper,
/// and a failure here means the test environment is broken.
pub async fn test_db() -> Database {
    // Use a unique file URI so each call gets an isolated in-memory database.
    // SQLite's `:memory:` shares the database within a process when using
    // the same connection string, but file URIs with mode=memory&cache=shared
    // are per-URI. Using a UUID ensures isolation.
    let id = uuid::Uuid::new_v4();
    let url = format!("file:testdb_{}?mode=memory&cache=shared", id);

    let db =
        Database::try_new_with_schema(&url, "", 1, None).expect("Failed to create test database");

    db.run_migrations()
        .await
        .expect("Failed to run migrations on test database");

    db
}

/// Create a `DAL` backed by an in-memory SQLite database with migrations applied.
///
/// Convenience wrapper around [`test_db()`] — returns a DAL ready for queries.
pub async fn test_dal() -> DAL {
    let db = test_db().await;
    DAL::new(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_creates_isolated_databases() {
        let db1 = test_db().await;
        let db2 = test_db().await;

        // Both should be functional SQLite databases
        assert!(format!("{:?}", db1.backend()).contains("Sqlite"));
        assert!(format!("{:?}", db2.backend()).contains("Sqlite"));
    }

    #[tokio::test]
    async fn test_dal_cron_schedule_roundtrip() {
        let dal = test_dal().await;

        // Create a cron schedule
        use cloacina::database::universal_types::{UniversalBool, UniversalTimestamp};
        use cloacina::models::cron_schedule::NewCronSchedule;

        let now = chrono::Utc::now();
        let new_schedule = NewCronSchedule {
            workflow_name: "test_workflow".to_string(),
            cron_expression: "*/5 * * * * *".to_string(),
            timezone: Some("UTC".to_string()),
            enabled: Some(UniversalBool::new(true)),
            catchup_policy: Some("skip".to_string()),
            start_date: None,
            end_date: None,
            next_run_at: UniversalTimestamp(now),
        };

        let schedule = dal
            .cron_schedule()
            .create(new_schedule)
            .await
            .expect("Failed to create cron schedule");

        assert_eq!(schedule.workflow_name, "test_workflow");
        assert_eq!(schedule.cron_expression, "*/5 * * * * *");
        assert!(schedule.enabled.0);

        // Read it back
        let schedules = dal
            .cron_schedule()
            .list(false, 100, 0)
            .await
            .expect("Failed to list schedules");

        assert_eq!(schedules.len(), 1);
        assert_eq!(schedules[0].workflow_name, "test_workflow");
    }

    #[tokio::test]
    async fn test_dal_isolation_between_tests() {
        // Each test_dal() call should be independent
        let dal1 = test_dal().await;
        let dal2 = test_dal().await;

        // Insert into dal1
        use cloacina::database::universal_types::{UniversalBool, UniversalTimestamp};
        use cloacina::models::cron_schedule::NewCronSchedule;

        let now = chrono::Utc::now();
        dal1.cron_schedule()
            .create(NewCronSchedule {
                workflow_name: "only_in_dal1".to_string(),
                cron_expression: "* * * * *".to_string(),
                timezone: Some("UTC".to_string()),
                enabled: Some(UniversalBool::new(true)),
                catchup_policy: Some("skip".to_string()),
                start_date: None,
                end_date: None,
                next_run_at: UniversalTimestamp(now),
            })
            .await
            .expect("insert failed");

        // dal2 should be empty
        let schedules = dal2
            .cron_schedule()
            .list(false, 100, 0)
            .await
            .expect("list failed");

        assert_eq!(schedules.len(), 0, "dal2 should not see dal1's data");
    }
}
