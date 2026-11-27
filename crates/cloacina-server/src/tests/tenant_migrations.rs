/*
 *  Copyright 2026 Colliery Software
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

//! Server start migrates each tenant schema (CLOACINA-T-1056). The tests
//! need Postgres, like the other server tests.
//!
//! A test tenant is a schema made with the older migrations only: every
//! migration before [`FIRST_NEW_MIGRATION`]. That is a tenant that was
//! created by an older server version.
//!
//! The tests give the start step the names of their own tenants
//! (`TenantSchemas::Named`), not every schema: the test database is shared.

use super::*;

use crate::startup_migrations::{StartMigrationError, TenantSchemas, migrate_at_start};
use cloacina::database::{DatabaseAdmin, POSTGRES_MIGRATIONS};
use diesel::connection::SimpleConnection;
use diesel::migration::{Migration, MigrationSource};
use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

/// The first migration that the old tenants of these tests do not have. It
/// makes the table `audit_events`.
const FIRST_NEW_MIGRATION: &str = "056_create_audit_events";

/// A table that the old tenants do not have.
const NEW_TABLE: &str = "audit_events";

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct SchemaOfTable {
    #[diesel(sql_type = diesel::sql_types::Text)]
    nspname: String,
}

fn connect() -> PgConnection {
    PgConnection::establish(TEST_DB_URL).expect("connect")
}

/// Every migration of this server version, oldest first.
fn all_migrations() -> Vec<Box<dyn Migration<Pg>>> {
    let mut migrations =
        MigrationSource::<Pg>::migrations(&POSTGRES_MIGRATIONS).expect("migrations");
    migrations.sort_by(|a, b| a.name().version().cmp(&b.name().version()));
    migrations
}

/// Makes a tenant schema as an older server version made it: the migrations
/// before [`FIRST_NEW_MIGRATION`] and no other. Returns its name and how many
/// migrations it has.
fn make_old_tenant(label: &str) -> (String, i64) {
    let tenant = format!("t1056_{}_{}", label, uuid::Uuid::new_v4().simple());
    let mut conn = connect();
    conn.batch_execute(&format!(
        "CREATE SCHEMA {tenant}; SET search_path TO {tenant}, public;"
    ))
    .expect("schema");
    // Makes the tenant's own `__diesel_schema_migrations`.
    conn.applied_migrations().expect("migrations table");

    let migrations = all_migrations();
    let old: Vec<_> = migrations
        .iter()
        .take_while(|m| m.name().to_string() != FIRST_NEW_MIGRATION)
        .collect();
    assert!(
        old.len() < migrations.len(),
        "{FIRST_NEW_MIGRATION} is a migration of this version"
    );
    for migration in &old {
        conn.run_migration(migration.as_ref())
            .expect("older migration");
    }
    (tenant, old.len() as i64)
}

fn drop_tenants(tenants: &[&str]) {
    let mut conn = connect();
    for tenant in tenants {
        // The build role that the migration run made (CLOACINA-T-1063)
        // goes too; its grants went with the schema.
        conn.batch_execute(&format!(
            "DROP SCHEMA IF EXISTS {tenant} CASCADE; DROP ROLE IF EXISTS \"{}\";",
            cloacina::database::build_role_name(tenant)
        ))
        .expect("drop schema");
    }
}

/// How many migrations the schema has.
fn applied(schema: &str) -> i64 {
    diesel::sql_query(format!(
        "SELECT COUNT(*) AS count FROM {schema}.__diesel_schema_migrations"
    ))
    .get_result::<Count>(&mut connect())
    .expect("count")
    .count
}

fn has_table(schema: &str, table: &str) -> bool {
    diesel::sql_query(
        "SELECT COUNT(*) AS count FROM pg_tables WHERE schemaname = $1 AND tablename = $2",
    )
    .bind::<diesel::sql_types::Text, _>(schema)
    .bind::<diesel::sql_types::Text, _>(table)
    .get_result::<Count>(&mut connect())
    .expect("count")
    .count
        == 1
}

/// The schema whose table a query on `table` reads, for a connection of the
/// tenant as the server opens one (`search_path` = `<tenant>, public`).
/// `None`: no such table, the query is an error.
async fn schema_a_tenant_query_lands_in(tenant: &str, table: &'static str) -> Option<String> {
    let database =
        cloacina::Database::try_new_with_schema(TEST_DB_URL, "cloacina", 1, Some(tenant))
            .expect("tenant database");
    let landed = {
        let conn = database
            .get_postgres_connection()
            .await
            .expect("tenant connection");
        conn.interact(move |conn| {
            diesel::sql_query(
                "SELECT n.nspname::text AS nspname \
                 FROM pg_catalog.pg_class c \
                 JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
                 WHERE c.oid = to_regclass($1)",
            )
            .bind::<diesel::sql_types::Text, _>(table)
            .get_result::<SchemaOfTable>(conn)
            .optional()
        })
        .await
        .expect("interact")
        .expect("query")
        .map(|row| row.nspname)
    };
    database.close();
    landed
}

fn admin_database() -> cloacina::Database {
    cloacina::Database::new(TEST_DB_URL, "cloacina", 2)
}

/// The defect: a tenant made by an older version, then a start of this
/// version. The start brings the tenant schema to this version.
///
/// Before the start, the tenant has no `audit_events`, and a query of the
/// tenant on that name reads the table of `public` (finding 24 of the
/// security sweep). After it, the query reads the tenant's own table.
#[tokio::test]
#[serial]
async fn server_start_migrates_a_tenant_schema_made_at_an_older_level() {
    let database = admin_database();
    // `public` is at this version, as after any start.
    database.run_migrations().await.expect("public migrations");

    let (tenant, old_level) = make_old_tenant("old");
    let total = all_migrations().len() as i64;
    let lands_before = schema_a_tenant_query_lands_in(&tenant, NEW_TABLE).await;

    let result = migrate_at_start(
        &database,
        TenantSchemas::Named(std::slice::from_ref(&tenant)),
    )
    .await;

    let level_after = applied(&tenant);
    let lands_after = schema_a_tenant_query_lands_in(&tenant, NEW_TABLE).await;
    database.close();
    drop_tenants(&[&tenant]);

    assert!(old_level < total, "the tenant starts behind this version");
    assert_eq!(
        lands_before.as_deref(),
        Some("public"),
        "before the migration, the tenant's query on {NEW_TABLE} reads the table of public"
    );
    let migrated = result.expect("start migrations");
    assert_eq!(
        level_after, total,
        "the tenant schema has every migration after the start (it had {old_level})"
    );
    assert_eq!(migrated.tenants_changed, vec![tenant.clone()]);
    assert_eq!(
        lands_after.as_deref(),
        Some(tenant.as_str()),
        "after the migration, the tenant's query on {NEW_TABLE} reads the tenant's own table"
    );
}

/// One tenant fails at its last migration. The error names it, its schema is
/// as before (the migrations that ran before the failure are rolled back),
/// and the other tenant, tried after it, is migrated.
#[tokio::test]
#[serial]
async fn a_tenant_that_fails_is_named_and_the_other_tenant_is_migrated() {
    let database = admin_database();
    database.run_migrations().await.expect("public migrations");

    let (broken, old_level) = make_old_tenant("broken");
    let (sound, _) = make_old_tenant("sound");
    let total = all_migrations().len() as i64;
    // Migration 060 makes an index with this name: it now fails in `broken`,
    // after 056 and 057 ran.
    connect()
        .batch_execute(&format!(
            "CREATE TABLE {broken}.idx_schedules_cron_identity (x INT);"
        ))
        .expect("blocker");

    let result = migrate_at_start(
        &database,
        TenantSchemas::Named(&[broken.clone(), sound.clone()]),
    )
    .await;

    let broken_level = applied(&broken);
    let broken_has_new_table = has_table(&broken, NEW_TABLE);
    let sound_level = applied(&sound);
    database.close();
    drop_tenants(&[&broken, &sound]);

    let error = result.expect_err("the start fails");
    let message = error.to_string();
    let StartMigrationError::Tenants { tenants, failures } = error else {
        panic!("not a tenant failure: {message}");
    };
    assert_eq!(tenants, 2);
    assert_eq!(failures.len(), 1, "{message}");
    assert_eq!(failures[0].tenant, broken);
    assert!(
        failures[0].error.contains("idx_schedules_cron_identity"),
        "the cause is in the error: {message}"
    );
    assert!(message.contains(&broken), "the error names the tenant");
    assert!(
        !message.contains(&sound),
        "the error names no tenant that migrated: {message}"
    );
    assert_eq!(broken_level, old_level, "the failed tenant is as before");
    assert!(
        !broken_has_new_table,
        "the migrations before the failed one are rolled back"
    );
    assert_eq!(sound_level, total, "the other tenant is migrated");
}

/// Two servers start together. Both starts succeed, and the tenant's
/// migrations ran once: one start ran them, the other found none pending.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn two_starts_together_migrate_a_tenant_once() {
    let first = admin_database();
    let second = admin_database();
    first.run_migrations().await.expect("public migrations");

    let (tenant, _) = make_old_tenant("pair");
    let total = all_migrations().len() as i64;
    let names = vec![tenant.clone()];

    let (a, b) = tokio::join!(
        migrate_at_start(&first, TenantSchemas::Named(&names)),
        migrate_at_start(&second, TenantSchemas::Named(&names)),
    );

    let level_after = applied(&tenant);
    first.close();
    second.close();
    drop_tenants(&[&tenant]);

    let mut changed = [
        a.expect("first start").tenants_changed.len(),
        b.expect("second start").tenants_changed.len(),
    ];
    changed.sort();
    assert_eq!(changed, [0, 1], "one start migrated the tenant");
    assert_eq!(level_after, total);
}

/// The list that the server migrates at start has each schema that holds
/// migrations, and no schema without a migrations table.
#[tokio::test]
#[serial]
async fn the_start_list_has_the_schemas_that_hold_migrations() {
    let database = admin_database();
    let (tenant, _) = make_old_tenant("listed");
    let other = format!("t1056_other_{}", uuid::Uuid::new_v4().simple());
    connect()
        .batch_execute(&format!(
            "CREATE SCHEMA {other}; CREATE TABLE {other}.notes (x INT);"
        ))
        .expect("other schema");

    let listed = DatabaseAdmin::new(database.clone())
        .list_migrated_tenant_schemas()
        .await;
    database.close();
    drop_tenants(&[&tenant, &other]);

    let listed = listed.expect("list");
    assert!(listed.contains(&tenant), "a tenant schema is in the list");
    assert!(
        !listed.contains(&other),
        "a schema with no migrations table is not a tenant"
    );
    assert!(!listed.iter().any(|schema| schema == "public"));
}
