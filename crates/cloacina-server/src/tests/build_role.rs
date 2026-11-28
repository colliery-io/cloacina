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

//! The build role of a tenant (CLOACINA-T-1063), on Postgres. Each test
//! connects as the build role, with the password the server issues, and
//! shows what it reaches:
//!
//! - every query of a compiler build, of Python vendoring and of the
//!   per-target fill succeeds;
//! - another tenant's schema, and every table of `public`, are refused;
//! - a table that the tenant schema lacks is refused, not read from `public`;
//! - a table made by a later migration is reachable after it;
//! - tenant delete drops the role.
//!
//! The database is shared: each test makes tenants with names of its own
//! and drops their schemas and roles at the end. A password is never printed.

use super::*;

use cloacina::dal::UnifiedRegistryStorage;
use cloacina::database::{DatabaseAdmin, POSTGRES_MIGRATIONS, TenantConfig, build_role_name};
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use diesel::connection::SimpleConnection;
use diesel::migration::{Migration, MigrationSource};
use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use diesel_migrations::MigrationHarness;

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}

fn admin_connection() -> PgConnection {
    PgConnection::establish(TEST_DB_URL).expect("connect")
}

fn admin_database() -> cloacina::Database {
    cloacina::Database::new(TEST_DB_URL, "cloacina", 2)
}

fn unique(label: &str) -> String {
    format!(
        "t1063_{label}_{}",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    )
}

/// Makes a tenant the way the server does (`create_tenant`).
async fn make_tenant(admin: &DatabaseAdmin, label: &str) -> String {
    let name = unique(label);
    admin
        .create_tenant(TenantConfig {
            schema_name: name.clone(),
            username: name.clone(),
            password: String::new(),
        })
        .await
        .expect("create tenant");
    name
}

/// The URL of a tenant's build role, with a password the server just set.
async fn build_url(admin: &DatabaseAdmin, schema: &str) -> String {
    let credential = admin
        .issue_build_password(schema)
        .await
        .expect("issue build password");
    cloacina::database::connection::inject_credentials(
        TEST_DB_URL,
        &credential.role,
        &credential.password,
    )
    .expect("build role url")
}

/// A connection of the build role, with the `search_path` a tenant compiler
/// has (`<tenant>, public`).
fn build_connection(url: &str, schema: &str) -> PgConnection {
    let mut conn = PgConnection::establish(url)
        .unwrap_or_else(|_| panic!("the build role of {schema} logs in"));
    conn.batch_execute(&format!("SET search_path TO {schema}, public"))
        .expect("search path");
    conn
}

/// `Ok` when Postgres runs `sql`, else the error text (no password in it).
fn try_sql(conn: &mut PgConnection, sql: &str) -> Result<(), String> {
    conn.batch_execute(sql).map_err(|e| e.to_string())
}

fn refused(conn: &mut PgConnection, sql: &str) {
    let err = try_sql(conn, sql).expect_err(sql);
    assert!(
        err.contains("permission denied"),
        "{sql}: refused for another reason: {err}"
    );
}

fn role_count(role: &str) -> i64 {
    diesel::sql_query("SELECT COUNT(*) AS n FROM pg_roles WHERE rolname = $1")
        .bind::<diesel::sql_types::Text, _>(role)
        .get_result::<Count>(&mut admin_connection())
        .expect("count")
        .n
}

async fn drop_tenants(admin: &DatabaseAdmin, tenants: &[&str]) {
    for tenant in tenants {
        let _ = admin.remove_tenant(tenant, tenant).await;
        // A schema made by hand has no tenant role; drop what is left.
        let _ = admin_connection().batch_execute(&format!(
            "DROP SCHEMA IF EXISTS {tenant} CASCADE; DROP ROLE IF EXISTS \"{}\";",
            build_role_name(tenant)
        ));
    }
}

/// Every call that a tenant compiler makes to the database, as the build
/// role: claim, read the source, heartbeat, store providers (primary and per
/// target), store and clear vendored Python sets, success, failure, the
/// sweeper, the queue count, and the per-target scan and fill.
#[tokio::test]
#[serial]
async fn the_build_role_runs_every_query_of_a_compiler() {
    let admin_db = admin_database();
    let admin = DatabaseAdmin::new(admin_db.clone());
    let tenant = make_tenant(&admin, "build").await;

    // The upload, as the server does it (admin role, tenant schema).
    let package = std::fs::read(fixture_path("python-workflow.cloacina")).expect("fixture");
    let tenant_admin_db =
        cloacina::Database::try_new_with_schema(TEST_DB_URL, "cloacina", 2, Some(&tenant))
            .expect("tenant database");
    let mut uploader = WorkflowRegistryImpl::new(
        UnifiedRegistryStorage::new(tenant_admin_db.clone()),
        tenant_admin_db.clone(),
    )
    .expect("registry");
    let package_id = uploader
        .register_workflow_package(package)
        .await
        .expect("upload");

    // The compiler, as the build role.
    let url = build_url(&admin, &tenant).await;
    let build_db = cloacina::Database::try_new_with_schema(&url, "cloacina", 2, Some(&tenant))
        .expect("build database");
    let registry = WorkflowRegistryImpl::new(
        UnifiedRegistryStorage::new(build_db.clone()),
        build_db.clone(),
    )
    .expect("registry");
    let dal = cloacina::dal::DAL::new(build_db.clone());

    let outcome = async {
        let stats = registry
            .build_queue_stats()
            .await
            .map_err(|e| format!("queue stats: {e}"))?;
        if stats.pending != 1 {
            return Err(format!("queue: {} pending", stats.pending));
        }
        let claim = registry
            .claim_next_build()
            .await
            .map_err(|e| format!("claim: {e}"))?
            .ok_or("nothing claimed")?;
        let (meta, _source) = registry
            .get_source_for_build(claim.id)
            .await
            .map_err(|e| format!("source: {e}"))?
            .ok_or("no source")?;
        registry
            .heartbeat_build(claim.id)
            .await
            .map_err(|e| format!("heartbeat: {e}"))?;
        let provider = || {
            vec![(
                "probe".to_string(),
                "0.1.0".to_string(),
                "wasm".to_string(),
                b"provider".to_vec(),
            )]
        };
        registry
            .store_package_providers(&meta.package_name, &meta.version, None, provider())
            .await
            .map_err(|e| format!("providers: {e}"))?;
        registry
            .store_package_providers(
                &meta.package_name,
                &meta.version,
                Some("x86_64-unknown-linux-gnu"),
                provider(),
            )
            .await
            .map_err(|e| format!("providers per target: {e}"))?;
        registry
            .store_python_vendor(
                claim.id,
                &meta.package_name,
                &meta.version,
                vec![("x86_64-linux".to_string(), b"vendored".to_vec())],
                Some(vec!["x86_64-linux".to_string()]),
            )
            .await
            .map_err(|e| format!("store vendored set: {e}"))?;
        registry
            .clear_python_vendor(&meta.package_name, &meta.version)
            .await
            .map_err(|e| format!("clear vendored sets: {e}"))?;
        registry
            .mark_build_success_with_docs(
                claim.id,
                Vec::new(),
                std::collections::HashMap::new(),
                Vec::new(),
                Vec::new(),
            )
            .await
            .map_err(|e| format!("success: {e}"))?;
        registry
            .sweep_stale_builds(std::time::Duration::from_secs(600))
            .await
            .map_err(|e| format!("sweep: {e}"))?;
        // The per-target compiler.
        let target = "aarch64-unknown-linux-gnu";
        dal.workflow_packages()
            .find_packages_missing_target_artifact(target, None, None)
            .await
            .map_err(|e| format!("missing artifacts: {e}"))?;
        dal.workflow_packages()
            .find_packages_missing_target_provider(target, None, None)
            .await
            .map_err(|e| format!("missing providers: {e}"))?;
        dal.workflow_packages()
            .upsert_artifact(
                &meta.package_name,
                &meta.version,
                None,
                target,
                "digest",
                b"library".to_vec(),
            )
            .await
            .map_err(|e| format!("store artifact: {e}"))?;
        // A failed build: the server puts the row back in the queue (as a
        // rebuild request does), the compiler claims and fails it.
        uploader
            .request_recompile(claim.id)
            .await
            .map_err(|e| format!("recompile (admin): {e}"))?;
        let again = registry
            .claim_next_build()
            .await
            .map_err(|e| format!("second claim: {e}"))?
            .ok_or("nothing claimed the second time")?;
        registry
            .mark_build_failed(again.id, "probe failure")
            .await
            .map_err(|e| format!("failure: {e}"))?;
        Ok::<_, String>(())
    }
    .await;

    build_db.close();
    tenant_admin_db.close();
    let _ = package_id;
    drop_tenants(&admin, &[&tenant]).await;
    admin_db.close();

    outcome.expect("every compiler query succeeds as the build role");
}

/// CLOACINA-T-1087: the compiler's schema check at start, as the build
/// role: it reads the applied migrations and the release row of its schema,
/// and finds nothing pending on a tenant the server made.
#[tokio::test]
#[serial]
async fn the_build_role_reads_the_schema_check_of_a_compiler() {
    use cloacina::database::schema_release::{RELEASE, schema_status};
    let admin_db = admin_database();
    admin_db.run_migrations().await.expect("public migrations");
    let admin = DatabaseAdmin::new(admin_db.clone());
    let tenant = make_tenant(&admin, "status").await;
    let url = build_url(&admin, &tenant).await;

    let compiler_db = cloacina::Database::try_new_with_schema(&url, "", 1, Some(&tenant))
        .expect("build role database");
    let status = schema_status(&compiler_db).await;
    compiler_db.close();
    drop_tenants(&admin, &[&tenant]).await;
    admin_db.close();

    let status = status.expect("the build role reads its schema");
    assert_eq!(status.pending, 0);
    assert_eq!(status.written_by.as_deref(), Some(RELEASE));
}

/// The build role of one tenant reaches no other tenant's schema, no table
/// of `public`, no table of its own schema outside the build tables, and can
/// make nothing. A table that its schema lacks is refused, not read from
/// `public`.
#[tokio::test]
#[serial]
async fn the_build_role_reaches_only_its_own_build_tables() {
    let admin_db = admin_database();
    admin_db.run_migrations().await.expect("public migrations");
    let admin = DatabaseAdmin::new(admin_db.clone());
    let tenant = make_tenant(&admin, "mine").await;
    let other = make_tenant(&admin, "other").await;

    let url = build_url(&admin, &tenant).await;
    let mut conn = build_connection(&url, &tenant);

    // Its own build tables.
    for table in [
        "workflow_packages",
        "workflow_registry",
        "package_providers",
        "package_artifacts",
    ] {
        try_sql(&mut conn, &format!("SELECT COUNT(*) FROM {table}"))
            .unwrap_or_else(|e| panic!("{table}: {e}"));
    }
    // No more than its verbs.
    refused(&mut conn, "DELETE FROM workflow_packages");
    refused(&mut conn, "DELETE FROM workflow_registry");
    // Not the other tables of its own schema.
    refused(
        &mut conn,
        &format!("SELECT COUNT(*) FROM {tenant}.api_keys"),
    );
    refused(&mut conn, &format!("SELECT COUNT(*) FROM {tenant}.secrets"));
    // Another tenant's schema.
    refused(
        &mut conn,
        &format!("SELECT COUNT(*) FROM {other}.workflow_packages"),
    );
    refused(&mut conn, &format!("DELETE FROM {other}.workflow_packages"));
    // `public`'s tables, the build tables too.
    refused(&mut conn, "SELECT COUNT(*) FROM public.api_keys");
    refused(&mut conn, "SELECT COUNT(*) FROM public.local_accounts");
    refused(&mut conn, "SELECT COUNT(*) FROM public.workflow_packages");
    // It makes nothing, in its schema or in `public`.
    refused(&mut conn, "CREATE TABLE probe_t1063 (x INT)");
    refused(&mut conn, "CREATE TABLE public.probe_t1063 (x INT)");

    // A table the schema lacks: the name is found in `public`, and Postgres
    // refuses it; nothing of `public` is read.
    admin_connection()
        .batch_execute(&format!("DROP TABLE {tenant}.audit_events"))
        .expect("drop a table of the tenant");
    refused(&mut conn, "SELECT COUNT(*) FROM audit_events");

    drop(conn);
    drop_tenants(&admin, &[&tenant, &other]).await;
    admin_db.close();
}

/// `public` is a tenant like the others: the start makes its build role,
/// which reaches the four build tables of `public` and nothing else there,
/// and no named tenant's schema.
#[tokio::test]
#[serial]
async fn the_start_makes_the_build_role_of_public() {
    let admin_db = admin_database();
    let admin = DatabaseAdmin::new(admin_db.clone());
    let tenant = make_tenant(&admin, "named").await;

    crate::startup_migrations::migrate_at_start(
        &admin_db,
        crate::startup_migrations::TenantSchemas::Named(&[]),
    )
    .await
    .expect("start migrations");
    assert_eq!(
        role_count("public$build"),
        1,
        "the start makes public$build"
    );

    let url = build_url(&admin, "public").await;
    let mut conn = PgConnection::establish(&url).expect("public$build logs in");
    let outcome = (|| {
        for table in [
            "workflow_packages",
            "workflow_registry",
            "package_providers",
            "package_artifacts",
        ] {
            try_sql(&mut conn, &format!("SELECT COUNT(*) FROM public.{table}"))
                .map_err(|e| format!("{table}: {e}"))?;
        }
        for sql in [
            "SELECT COUNT(*) FROM public.api_keys".to_string(),
            "SELECT COUNT(*) FROM public.secrets".to_string(),
            "SELECT COUNT(*) FROM public.audit_events".to_string(),
            format!("SELECT COUNT(*) FROM {tenant}.workflow_packages"),
        ] {
            match try_sql(&mut conn, &sql) {
                Err(e) if e.contains("permission denied") => {}
                other => return Err(format!("{sql}: not refused: {other:?}")),
            }
        }
        Ok::<_, String>(())
    })();
    drop(conn);
    // The password goes: the next start of public's compiler sets a new one.
    admin_connection()
        .batch_execute("ALTER ROLE \"public$build\" WITH PASSWORD NULL")
        .expect("clear the password");
    drop_tenants(&admin, &[&tenant]).await;
    admin_db.close();
    outcome.expect("public$build");
}

/// Every migration of this version, oldest first.
fn all_migrations() -> Vec<Box<dyn Migration<Pg>>> {
    let mut migrations =
        MigrationSource::<Pg>::migrations(&POSTGRES_MIGRATIONS).expect("migrations");
    migrations.sort_by(|a, b| a.name().version().cmp(&b.name().version()));
    migrations
}

/// A tenant made by a version before `package_providers` (migration 039)
/// gets its build role at the next start; the table that the migration makes
/// at that start reaches the role.
#[tokio::test]
#[serial]
async fn a_table_made_by_a_later_migration_reaches_the_build_role() {
    let admin_db = admin_database();
    admin_db.run_migrations().await.expect("public migrations");
    let admin = DatabaseAdmin::new(admin_db.clone());

    // A tenant schema at the level before 039, made by hand, with no build
    // role: an older server made it.
    let tenant = unique("old");
    {
        let mut conn = admin_connection();
        conn.batch_execute(&format!(
            "CREATE SCHEMA {tenant}; SET search_path TO {tenant}, public;"
        ))
        .expect("schema");
        conn.applied_migrations().expect("migrations table");
        for migration in all_migrations()
            .iter()
            .take_while(|m| m.name().to_string() != "039_create_package_providers")
        {
            conn.run_migration(migration.as_ref())
                .expect("older migration");
        }
    }
    assert_eq!(role_count(&build_role_name(&tenant)), 0);

    // Before the migration: the role, made with the grants on the tables
    // that exist, has no `package_providers` to reach.
    admin.ensure_build_role(&tenant).await.expect("build role");
    let url = build_url(&admin, &tenant).await;
    let before = try_sql(
        &mut build_connection(&url, &tenant),
        &format!("SELECT COUNT(*) FROM {tenant}.package_providers"),
    );

    // The start migrates the tenant schema, then grants again.
    let migrated = admin.migrate_tenant_schema(&tenant).await;
    let after = try_sql(
        &mut build_connection(&url, &tenant),
        &format!("SELECT COUNT(*) FROM {tenant}.package_providers"),
    );

    drop_tenants(&admin, &[&tenant]).await;
    admin_db.close();

    assert!(
        before.as_ref().is_err_and(|e| e.contains("does not exist")),
        "before the migration the table is not there: {before:?}"
    );
    assert!(
        migrated.expect("migrate") > 0,
        "the tenant had migrations to run"
    );
    after.expect("the table made by the later migration reaches the build role");
}

/// The build role has no password until the server sets one, and tenant
/// delete drops it. A second delete finds nothing to do.
#[tokio::test]
#[serial]
async fn tenant_create_makes_the_build_role_and_tenant_delete_drops_it() {
    let admin_db = admin_database();
    let admin = DatabaseAdmin::new(admin_db.clone());
    let tenant = make_tenant(&admin, "gone").await;
    let role = build_role_name(&tenant);
    let made = role_count(&role);

    // No password yet: no login (read as the superuser of the dev database).
    let no_password = diesel::sql_query(
        "SELECT COUNT(*) AS n FROM pg_authid WHERE rolname = $1 AND rolpassword IS NULL",
    )
    .bind::<diesel::sql_types::Text, _>(&role)
    .get_result::<Count>(&mut admin_connection())
    .expect("pg_authid")
    .n;
    let login_before_issue = no_password == 0;

    let first = admin.remove_tenant(&tenant, &tenant).await;
    let after = role_count(&role);
    let second = admin.remove_tenant(&tenant, &tenant).await;
    drop_tenants(&admin, &[&tenant]).await;
    admin_db.close();

    assert_eq!(made, 1, "tenant create makes the build role");
    assert!(
        !login_before_issue,
        "no login before the server sets a password"
    );
    first.expect("delete");
    assert_eq!(after, 0, "tenant delete drops the build role");
    second.expect("a second delete");
}
