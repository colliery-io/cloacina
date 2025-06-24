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

use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;

use super::types::{RunnerRecord, StoredRunnerConfig};

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[derive(Queryable, Insertable, AsChangeset, Selectable)]
#[diesel(table_name = local_runners)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct LocalRunner {
    pub id: String,
    pub config: String,
    pub is_paused: bool,
}

diesel::table! {
    local_runners (id) {
        id -> Text,
        config -> Text,
        is_paused -> Bool,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

pub struct RunnerDAL {
    pool: DbPool,
}

impl RunnerDAL {
    pub fn new(db_path: &str) -> Result<Self> {
        tracing::info!("Initializing RunnerDAL with database path: {}", db_path);
        let manager = ConnectionManager::<SqliteConnection>::new(db_path);
        let pool = r2d2::Pool::builder()
            .build(manager)
            .context("Failed to create connection pool")?;

        let repo = Self { pool };
        tracing::debug!("Connection pool created, initializing schema");
        repo.initialize_schema()?;
        tracing::debug!("RunnerDAL initialization complete");
        Ok(repo)
    }

    fn initialize_schema(&self) -> Result<()> {
        tracing::debug!("Initializing database schema for local_runners table");
        let mut conn = self.pool.get().context("Failed to get connection")?;

        diesel::sql_query(
            r#"
            CREATE TABLE IF NOT EXISTS local_runners (
                id TEXT PRIMARY KEY,
                config TEXT NOT NULL,
                is_paused BOOLEAN NOT NULL DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&mut conn)
        .context("Failed to create local_runners table")?;

        tracing::debug!("Database schema initialization complete");
        Ok(())
    }

    pub fn get_all_runners(&self) -> Result<Vec<RunnerRecord>> {
        use self::local_runners::dsl::*;

        tracing::debug!("Loading all runners from database");
        let mut conn = self.pool.get().context("Failed to get connection")?;

        let runners: Vec<LocalRunner> = local_runners
            .select(LocalRunner::as_select())
            .load(&mut conn)
            .context("Failed to load runners")?;

        tracing::debug!("Loaded {} runners from database", runners.len());

        let mut records = Vec::new();
        for runner in runners {
            let runner_config: StoredRunnerConfig = serde_json::from_str(&runner.config)
                .with_context(|| {
                    format!(
                        "Failed to deserialize runner config for runner {}",
                        runner.id
                    )
                })?;

            tracing::debug!(
                "Loaded runner: {} ({}), is_paused: {}",
                runner.id,
                runner_config.name,
                runner.is_paused
            );
            records.push(RunnerRecord {
                id: runner.id,
                config: runner_config,
                is_paused: runner.is_paused,
            });
        }

        tracing::debug!("Returning {} runner records", records.len());
        Ok(records)
    }

    pub fn save_runner(&self, record: &RunnerRecord) -> Result<()> {
        use self::local_runners::dsl::*;

        tracing::debug!(
            "Saving runner to database: {} ({})",
            record.id,
            record.config.name
        );
        let mut conn = self.pool.get().context("Failed to get connection")?;

        let config_json = serde_json::to_string(&record.config).with_context(|| {
            format!("Failed to serialize runner config for runner {}", record.id)
        })?;

        let local_runner = LocalRunner {
            id: record.id.clone(),
            config: config_json,
            is_paused: record.is_paused,
        };

        diesel::insert_into(local_runners)
            .values(&local_runner)
            .on_conflict(id)
            .do_update()
            .set(&local_runner)
            .execute(&mut conn)
            .with_context(|| format!("Failed to save runner {} to database", record.id))?;

        tracing::debug!("Runner {} saved successfully to database", record.id);
        Ok(())
    }

    pub fn delete_runner(&self, runner_id: &str) -> Result<()> {
        use self::local_runners::dsl::*;

        tracing::debug!("Deleting runner from database: {}", runner_id);
        let mut conn = self.pool.get().context("Failed to get connection")?;

        let rows_affected = diesel::delete(local_runners.filter(id.eq(runner_id)))
            .execute(&mut conn)
            .with_context(|| format!("Failed to delete runner {} from database", runner_id))?;

        if rows_affected > 0 {
            tracing::debug!("Runner {} deleted successfully from database", runner_id);
        } else {
            tracing::warn!(
                "No rows affected when deleting runner {} - may not have existed",
                runner_id
            );
        }
        Ok(())
    }

    pub fn update_runner_status(&self, runner_id: &str, paused: bool) -> Result<()> {
        use self::local_runners::dsl::*;

        tracing::debug!("Updating runner {} status: is_paused={}", runner_id, paused);
        let mut conn = self.pool.get().context("Failed to get connection")?;

        let rows_affected = diesel::update(local_runners.filter(id.eq(runner_id)))
            .set(is_paused.eq(paused))
            .execute(&mut conn)
            .with_context(|| format!("Failed to update runner {} status", runner_id))?;

        if rows_affected > 0 {
            tracing::debug!("Runner {} status updated successfully", runner_id);
        } else {
            tracing::warn!(
                "No rows affected when updating runner {} status - may not exist",
                runner_id
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::runners::types::{RunnerRecord, StoredRunnerConfig};
    use tempfile::NamedTempFile;

    fn create_test_dal() -> (RunnerDAL, tempfile::TempDir) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let dal = RunnerDAL::new(db_path.to_str().unwrap()).unwrap();
        (dal, temp_dir)
    }

    fn create_test_record() -> RunnerRecord {
        let config = StoredRunnerConfig {
            name: "Test Runner".to_string(),
            db_path: "./test.db".to_string(),
            max_concurrent_tasks: 4,
            enable_cron_scheduling: true,
            enable_registry_reconciler: false,
        };

        RunnerRecord {
            id: "test-id-123".to_string(),
            config,
            is_paused: false,
        }
    }

    #[test]
    fn test_dal_initialization() {
        let (_dal, _temp_dir) = create_test_dal();
        // Should create successfully without panicking
        assert!(true);
    }

    #[test]
    fn test_get_all_runners_empty() {
        let (dal, _temp_dir) = create_test_dal();
        let result = dal.get_all_runners();

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_save_and_retrieve_runner() {
        let (dal, _temp_dir) = create_test_dal();
        let record = create_test_record();

        // Save runner
        let save_result = dal.save_runner(&record);
        assert!(save_result.is_ok());

        // Retrieve runners
        let runners = dal.get_all_runners().unwrap();
        assert_eq!(runners.len(), 1);

        let retrieved = &runners[0];
        assert_eq!(retrieved.id, "test-id-123");
        assert_eq!(retrieved.config.name, "Test Runner");
        assert_eq!(retrieved.config.max_concurrent_tasks, 4);
        assert!(retrieved.config.enable_cron_scheduling);
        assert!(!retrieved.config.enable_registry_reconciler);
        assert!(!retrieved.is_paused);
    }

    #[test]
    fn test_update_runner_status() {
        let (dal, _temp_dir) = create_test_dal();
        let record = create_test_record();

        // Save runner
        dal.save_runner(&record).unwrap();

        // Update status to paused
        let update_result = dal.update_runner_status("test-id-123", true);
        assert!(update_result.is_ok());

        // Verify status was updated
        let runners = dal.get_all_runners().unwrap();
        assert_eq!(runners.len(), 1);
        assert!(runners[0].is_paused);
    }

    #[test]
    fn test_delete_runner() {
        let (dal, _temp_dir) = create_test_dal();
        let record = create_test_record();

        // Save runner
        dal.save_runner(&record).unwrap();

        // Verify it exists
        let runners = dal.get_all_runners().unwrap();
        assert_eq!(runners.len(), 1);

        // Delete runner
        let delete_result = dal.delete_runner("test-id-123");
        assert!(delete_result.is_ok());

        // Verify it's gone
        let runners = dal.get_all_runners().unwrap();
        assert_eq!(runners.len(), 0);
    }

    #[test]
    fn test_delete_nonexistent_runner() {
        let (dal, _temp_dir) = create_test_dal();

        // Try to delete non-existent runner
        let delete_result = dal.delete_runner("nonexistent");
        assert!(delete_result.is_ok()); // Should succeed even if not found
    }
}
