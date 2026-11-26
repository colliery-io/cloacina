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

//! PostgreSQL checks of CLOACINA-T-1042: migrations under the advisory lock,
//! migration 060 on a schema that has doubles, and concurrent registrations
//! of one cron schedule. Each test works in a schema of its own and drops it.

#![cfg(feature = "postgres")]

use cloacina::dal::DAL;
use cloacina::database::Database;
use cloacina::database::migration_lock::run_pending_postgres_migrations_locked;
use cloacina::database::universal_types::UniversalTimestamp;
use cloacina::models::schedule::NewSchedule;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use serial_test::serial;

const DATABASE_URL: &str = "postgres://cloacina:cloacina@localhost:15432/cloacina";

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

fn count(conn: &mut PgConnection, sql: &str) -> i64 {
    diesel::sql_query(sql)
        .get_result::<Count>(conn)
        .expect("count")
        .count
}

/// A connection whose search path is a new, empty schema.
fn connect_to(schema: &str) -> PgConnection {
    let mut conn = PgConnection::establish(DATABASE_URL).expect("connect");
    conn.batch_execute(&format!(
        "CREATE SCHEMA IF NOT EXISTS {schema}; SET search_path TO {schema}, public;"
    ))
    .expect("schema");
    conn
}

fn new_schema_name(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4().simple())
}

fn drop_schema(schema: &str) {
    let mut conn = PgConnection::establish(DATABASE_URL).expect("connect");
    conn.batch_execute(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE;"))
        .expect("drop schema");
}

/// Two processes that start together on a schema with every migration
/// pending: both calls succeed, one runs the migrations and the other finds
/// none.
#[test]
#[serial]
fn two_concurrent_migration_runs_both_succeed() {
    let schema = new_schema_name("mig_lock");
    // The schema exists before the two runs start.
    drop(connect_to(&schema));

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let run = || {
        let schema = schema.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            let mut conn = connect_to(&schema);
            barrier.wait();
            run_pending_postgres_migrations_locked(&mut conn)
        })
    };
    let (first, second) = (run(), run());
    let first = first.join().expect("thread");
    let second = second.join().expect("thread");

    let mut conn = connect_to(&schema);
    let applied = count(
        &mut conn,
        "SELECT COUNT(*) AS count FROM __diesel_schema_migrations",
    );
    let index = count(
        &mut conn,
        &format!(
            "SELECT COUNT(*) AS count FROM pg_indexes \
             WHERE schemaname = '{schema}' AND indexname = 'idx_schedules_cron_identity'"
        ),
    );
    drop(conn);
    drop_schema(&schema);

    let mut ran = [first.expect("first run"), second.expect("second run")];
    ran.sort();
    assert_eq!(ran[0], 0, "the run that waited finds nothing pending");
    assert_eq!(
        ran[1] as i64, applied,
        "the other run applied each migration"
    );
    assert_eq!(index, 1, "migration 060 made its index");
}

/// Migration 060 on a schema that has doubles: the row that ran last stays,
/// the schedule execution rows of the others move to it with one row per
/// fire time, and an instance and another workflow are not touched.
#[test]
#[serial]
fn migration_060_merges_double_cron_schedules() {
    let schema = new_schema_name("mig_060");
    let mut conn = connect_to(&schema);
    run_pending_postgres_migrations_locked(&mut conn).expect("migrations");

    // The state before the migration: no index, and doubles.
    conn.batch_execute(
        "DROP INDEX idx_schedules_cron_identity;
         INSERT INTO schedules (id, schedule_type, workflow_name, cron_expression, timezone, instance_name, last_run_at, created_at) VALUES
           ('00000000-0000-0000-0000-00000000000a', 'cron', 'wf', '0 * * * *', 'UTC', NULL, '2026-01-01 01:00:00', '2026-01-01 00:00:00'),
           ('00000000-0000-0000-0000-00000000000b', 'cron', 'wf', '0 * * * *', NULL,  NULL, '2026-01-01 03:00:00', '2026-01-01 00:00:01'),
           ('00000000-0000-0000-0000-00000000000c', 'cron', 'wf', '0 * * * *', 'UTC', NULL, NULL,                  '2026-01-01 00:00:02'),
           ('00000000-0000-0000-0000-00000000000d', 'cron', 'other', '0 * * * *', 'UTC', NULL, NULL,               '2026-01-01 00:00:03'),
           ('00000000-0000-0000-0000-00000000000e', 'cron', 'wf', '0 * * * *', 'UTC', 'prod', NULL,                '2026-01-01 00:00:04');
         INSERT INTO schedule_executions (id, schedule_id, scheduled_time) VALUES
           ('10000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-00000000000a', '2026-01-01 01:00:00'),
           ('10000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-00000000000a', '2026-01-01 02:00:00'),
           ('10000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-00000000000b', '2026-01-01 02:00:00'),
           ('10000000-0000-0000-0000-000000000004', '00000000-0000-0000-0000-00000000000b', '2026-01-01 03:00:00'),
           ('10000000-0000-0000-0000-000000000005', '00000000-0000-0000-0000-00000000000c', '2026-01-01 04:00:00'),
           ('10000000-0000-0000-0000-000000000006', '00000000-0000-0000-0000-00000000000d', '2026-01-01 02:00:00'),
           ('10000000-0000-0000-0000-000000000007', '00000000-0000-0000-0000-00000000000e', '2026-01-01 02:00:00');",
    )
    .expect("doubles");

    let up = include_str!(
        "../../../src/database/migrations/postgres/060_unique_cron_schedule_identity/up.sql"
    );
    let migrated = conn.batch_execute(up);

    let kept = count(
        &mut conn,
        "SELECT COUNT(*) AS count FROM schedules \
         WHERE workflow_name = 'wf' AND instance_name IS NULL \
           AND id = '00000000-0000-0000-0000-00000000000b'",
    );
    let schedules = count(&mut conn, "SELECT COUNT(*) AS count FROM schedules");
    let on_kept = count(
        &mut conn,
        "SELECT COUNT(*) AS count FROM schedule_executions \
         WHERE schedule_id = '00000000-0000-0000-0000-00000000000b'",
    );
    let kept_row_of_the_double_fire = count(
        &mut conn,
        "SELECT COUNT(*) AS count FROM schedule_executions \
         WHERE id = '10000000-0000-0000-0000-000000000003'",
    );
    let executions = count(
        &mut conn,
        "SELECT COUNT(*) AS count FROM schedule_executions",
    );
    let second_insert = conn.batch_execute(
        "INSERT INTO schedules (id, schedule_type, workflow_name, cron_expression, timezone) \
         VALUES (gen_random_uuid(), 'cron', 'wf', '0 * * * *', 'UTC');",
    );
    drop(conn);
    drop_schema(&schema);

    migrated.expect("migration 060");
    assert_eq!(kept, 1, "the row with the latest last_run_at stays");
    assert_eq!(schedules, 3, "kept row, other workflow, instance");
    assert_eq!(on_kept, 4, "fire times 1 to 4, one row each");
    assert_eq!(kept_row_of_the_double_fire, 1);
    assert_eq!(executions, 6, "one row of the double fire is removed");
    assert!(second_insert.is_err(), "the index refuses a double");
}

/// Registrations of one cron schedule at the same moment, as two replicas
/// that load one package: one row, and each caller gets it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn concurrent_upsert_cron_leaves_one_row() {
    let schema = new_schema_name("upsert_cron");
    let database = Database::new_with_schema(DATABASE_URL, "cloacina", 4, Some(&schema));
    database.setup_schema(&schema).await.expect("schema");
    let dal = DAL::new(database.clone());

    let mut failures = Vec::new();
    for round in 0..10 {
        let workflow = format!("wf_{round}");
        let mut tasks = Vec::new();
        for _ in 0..4 {
            let dal = dal.clone();
            let workflow = workflow.clone();
            tasks.push(tokio::spawn(async move {
                dal.schedule()
                    .upsert_cron(NewSchedule::cron(
                        &workflow,
                        "0 * * * *",
                        UniversalTimestamp::now(),
                    ))
                    .await
                    .map(|schedule| schedule.id)
            }));
        }
        let mut ids = Vec::new();
        for task in tasks {
            match task.await.expect("task") {
                Ok(id) => ids.push(id),
                Err(e) => failures.push(format!("{workflow}: {e}")),
            }
        }
        let rows = dal
            .schedule()
            .find_by_workflow(&workflow)
            .await
            .expect("rows");
        if rows.len() != 1 || ids.iter().any(|id| *id != rows[0].id) {
            failures.push(format!("{workflow}: {} row(s)", rows.len()));
        }
    }
    database.close();
    drop_schema(&schema);

    assert!(failures.is_empty(), "{failures:?}");
}
