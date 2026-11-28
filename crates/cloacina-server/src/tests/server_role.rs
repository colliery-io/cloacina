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

//! The server role of a tenant pool (CLOACINA-T-1080), on Postgres. Each
//! test opens a tenant's pool the way the server does
//! (`TenantDatabaseCache::resolve`) and shows what its connections reach.
//!
//! The database is shared: each test makes tenants with names of its own and
//! removes them at the end.

use super::*;

use cloacina::database::{DatabaseAdmin, TenantConfig};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;

#[derive(QueryableByName)]
struct Name {
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
}

fn admin_database() -> cloacina::Database {
    cloacina::Database::new(TEST_DB_URL, "cloacina", 2)
}

fn unique(label: &str) -> String {
    format!(
        "t1080_{label}_{}",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    )
}

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

/// `Ok` when Postgres runs `sql` on a connection of `db`, else the error.
async fn try_sql(db: &cloacina::Database, sql: &str) -> Result<(), String> {
    let conn = db
        .get_postgres_connection()
        .await
        .map_err(|e| format!("checkout: {e}"))?;
    let sql = sql.to_string();
    conn.interact(move |conn| conn.batch_execute(&sql))
        .await
        .map_err(|e| format!("interact: {e}"))?
        .map_err(|e| e.to_string())
}

async fn current_user(db: &cloacina::Database) -> Result<String, String> {
    let conn = db
        .get_postgres_connection()
        .await
        .map_err(|e| format!("checkout: {e}"))?;
    conn.interact(|conn| {
        diesel::sql_query("SELECT current_user::text AS name").get_result::<Name>(conn)
    })
    .await
    .map_err(|e| format!("interact: {e}"))?
    .map(|row| row.name)
    .map_err(|e| e.to_string())
}

async fn remove_tenants(admin: &DatabaseAdmin, tenants: &[&str]) {
    for tenant in tenants {
        let _ = admin.remove_tenant(tenant, tenant).await;
    }
}

/// A tenant's server connection cannot read another tenant's table, and its
/// current user is the tenant's server role, not the admin user.
#[tokio::test]
#[serial]
async fn a_tenants_server_connection_cannot_read_another_tenants_table() {
    let admin_db = admin_database();
    let admin = DatabaseAdmin::new(admin_db.clone());
    let tenant = make_tenant(&admin, "mine").await;
    let other = make_tenant(&admin, "other").await;

    let cache = TenantDatabaseCache::new(TEST_DB_URL.to_string());
    let outcome = async {
        let db = cache
            .resolve(&tenant, &admin_db)
            .await
            .map_err(|e| format!("resolve: {e}"))?;
        let user = current_user(&db).await?;
        let own = try_sql(&db, "SELECT COUNT(*) FROM workflow_packages").await;
        let theirs = try_sql(
            &db,
            &format!("SELECT COUNT(*) FROM {other}.workflow_packages"),
        )
        .await;
        db.close();
        Ok::<_, String>((user, own, theirs))
    }
    .await;
    cache.evict(&tenant).await;
    remove_tenants(&admin, &[&tenant, &other]).await;
    admin_db.close();

    let (user, own, theirs) = outcome.expect("tenant pool");
    assert_eq!(user, format!("{tenant}$server"), "the tenant's server role");
    own.expect("the tenant's own table");
    let err = theirs.expect_err("another tenant's table is refused");
    assert!(err.contains("permission denied"), "refused: {err}");
}

fn refused(result: Result<(), String>, what: &str) {
    match result {
        Err(e) if e.contains("permission denied") => {}
        other => panic!("{what}: not refused: {other:?}"),
    }
}

/// The tenant pool reaches each table of the list in its own schema, and no
/// global table: not `public`'s, not the copy in its own schema. A table that
/// its schema lacks is refused, not read from `public`. It makes nothing.
#[tokio::test]
#[serial]
async fn the_tenant_pool_reaches_only_the_tenant_tables_of_its_schema() {
    let admin_db = admin_database();
    admin_db.run_migrations().await.expect("public migrations");
    let admin = DatabaseAdmin::new(admin_db.clone());
    let tenant = make_tenant(&admin, "reach").await;
    let cache = TenantDatabaseCache::new(TEST_DB_URL.to_string());

    let db = cache.resolve(&tenant, &admin_db).await.expect("resolve");
    let mut results = Vec::new();
    for table in cloacina::database::SERVER_ROLE_TABLES {
        let read = try_sql(&db, &format!("SELECT COUNT(*) FROM {table}")).await;
        let delete = try_sql(&db, &format!("DELETE FROM {table} WHERE false")).await;
        results.push((format!("{table}: read"), read.and(delete)));
    }
    let mut global = Vec::new();
    for table in cloacina::database::SERVER_GLOBAL_TABLES
        .iter()
        .chain(cloacina::database::SERVER_BOOKKEEPING_TABLES)
    {
        global.push((
            format!("public.{table}"),
            try_sql(&db, &format!("SELECT COUNT(*) FROM public.{table}")).await,
        ));
        global.push((
            format!("{tenant}.{table}"),
            try_sql(&db, &format!("SELECT COUNT(*) FROM {tenant}.{table}")).await,
        ));
    }
    let make = try_sql(&db, "CREATE TABLE probe_t1080 (x INT)").await;
    let public_tenant_table = try_sql(&db, "SELECT COUNT(*) FROM public.workflow_packages").await;
    // A table the schema lacks: the name is found in `public`, and refused.
    let mut conn = PgConnection::establish(TEST_DB_URL).expect("connect");
    conn.batch_execute(&format!("DROP TABLE {tenant}.schedule_executions"))
        .expect("drop a table of the tenant");
    let fall_through = try_sql(&db, "SELECT COUNT(*) FROM schedule_executions").await;

    db.close();
    cache.evict(&tenant).await;
    remove_tenants(&admin, &[&tenant]).await;
    admin_db.close();

    for (what, result) in results {
        result.unwrap_or_else(|e| panic!("{what}: {e}"));
    }
    for (what, result) in global {
        refused(result, &what);
    }
    refused(make, "CREATE TABLE");
    refused(public_tenant_table, "public.workflow_packages");
    refused(fall_through, "a table the schema lacks");
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}

fn role_count(role: &str) -> i64 {
    let mut conn = PgConnection::establish(TEST_DB_URL).expect("connect");
    diesel::sql_query("SELECT COUNT(*) AS n FROM pg_roles WHERE rolname = $1")
        .bind::<diesel::sql_types::Text, _>(role)
        .get_result::<Count>(&mut conn)
        .expect("count")
        .n
}

/// Each table of a fresh tenant schema is in one list: the tenant tables of
/// the server role, or the global tables. A table that a new migration makes
/// fails here until it is put in one.
#[tokio::test]
#[serial]
async fn each_table_of_a_tenant_schema_is_in_one_list() {
    let admin_db = admin_database();
    let admin = DatabaseAdmin::new(admin_db.clone());
    let tenant = make_tenant(&admin, "tables").await;
    let mut conn = PgConnection::establish(TEST_DB_URL).expect("connect");
    let tables: Vec<String> = diesel::sql_query(
        "SELECT tablename::text AS name FROM pg_tables WHERE schemaname = $1 ORDER BY 1",
    )
    .bind::<diesel::sql_types::Text, _>(&tenant)
    .load::<Name>(&mut conn)
    .expect("tables")
    .into_iter()
    .map(|row| row.name)
    .collect();
    drop(conn);
    remove_tenants(&admin, &[&tenant]).await;
    admin_db.close();

    let tenant_tables = cloacina::database::SERVER_ROLE_TABLES;
    let global_tables: Vec<&str> = cloacina::database::SERVER_GLOBAL_TABLES
        .iter()
        .chain(cloacina::database::SERVER_BOOKKEEPING_TABLES)
        .copied()
        .collect();
    let in_neither: Vec<&String> = tables
        .iter()
        .filter(|t| !tenant_tables.contains(&t.as_str()) && !global_tables.contains(&t.as_str()))
        .collect();
    let in_both: Vec<&&str> = tenant_tables
        .iter()
        .filter(|t| global_tables.contains(t))
        .collect();
    let missing: Vec<&&str> = tenant_tables
        .iter()
        .chain(global_tables.iter())
        .filter(|t| !tables.iter().any(|name| name == *t))
        .collect();
    assert!(
        in_neither.is_empty(),
        "tables in neither list: {in_neither:?}"
    );
    assert!(in_both.is_empty(), "tables in both lists: {in_both:?}");
    assert!(missing.is_empty(), "listed tables not made: {missing:?}");
}

/// A tenant made before the server role existed gets it at the next start,
/// with no manual step; its pool then works. Tenant delete drops the role.
#[tokio::test]
#[serial]
async fn an_older_tenant_gets_its_server_role_at_start_and_delete_drops_it() {
    let admin_db = admin_database();
    let admin = DatabaseAdmin::new(admin_db.clone());
    let tenant = make_tenant(&admin, "older").await;
    let role = cloacina::database::server_role_name(&tenant);
    // As an older server left it: no server role.
    PgConnection::establish(TEST_DB_URL)
        .expect("connect")
        .batch_execute(&format!("DROP OWNED BY \"{role}\"; DROP ROLE \"{role}\";"))
        .expect("drop the server role");
    let before = role_count(&role);

    let migrated = crate::startup_migrations::migrate_at_start(
        &admin_db,
        crate::startup_migrations::TenantSchemas::Named(std::slice::from_ref(&tenant)),
    )
    .await;
    let after_start = role_count(&role);
    let cache = TenantDatabaseCache::new(TEST_DB_URL.to_string());
    let used = async {
        let db = cache
            .resolve(&tenant, &admin_db)
            .await
            .map_err(|e| format!("resolve: {e}"))?;
        let user = current_user(&db).await?;
        try_sql(&db, "SELECT COUNT(*) FROM workflow_executions").await?;
        db.close();
        Ok::<_, String>(user)
    }
    .await;
    cache.evict(&tenant).await;

    let removed = admin.remove_tenant(&tenant, &tenant).await;
    let after_delete = role_count(&role);
    remove_tenants(&admin, &[&tenant]).await;
    admin_db.close();

    assert_eq!(before, 0);
    migrated.expect("start migrations");
    assert_eq!(after_start, 1, "the start makes the server role");
    assert_eq!(used.expect("the pool works"), role);
    removed.expect("delete");
    assert_eq!(after_delete, 0, "tenant delete drops the server role");
}

/// The server's user need not be a superuser: a user with `CREATEROLE` that
/// owns the database makes the tenant, its server role and its own
/// membership, and its tenant pool takes the role. Run in a database of its
/// own with a user of its own, both dropped at the end.
#[tokio::test]
#[serial]
async fn a_server_user_with_createrole_only_takes_the_server_role() {
    let suffix = &uuid::Uuid::new_v4().simple().to_string()[..12];
    let user = format!("t1080_user_{suffix}");
    let database = format!("t1080_db_{suffix}");
    let mut root = PgConnection::establish(TEST_DB_URL).expect("connect");
    root.batch_execute(&format!(
        "CREATE ROLE {user} LOGIN CREATEROLE NOSUPERUSER PASSWORD 'scratch-t1080'"
    ))
    .expect("make the user");
    root.batch_execute(&format!("CREATE DATABASE {database} OWNER {user}"))
        .expect("make the database");
    let (base, _) = TEST_DB_URL.rsplit_once('/').expect("a database in the url");
    let url = cloacina::database::connection::inject_credentials(
        &format!("{base}/{database}"),
        &user,
        "scratch-t1080",
    )
    .expect("url");

    let outcome = async {
        let admin_db = cloacina::Database::new(&url, &database, 2);
        let admin = DatabaseAdmin::new(admin_db.clone());
        let tenant = unique("managed");
        let other = unique("managed_other");
        for name in [&tenant, &other] {
            admin
                .create_tenant(TenantConfig {
                    schema_name: name.clone(),
                    username: name.clone(),
                    password: String::new(),
                })
                .await
                .map_err(|e| format!("create tenant: {e}"))?;
        }
        let cache = TenantDatabaseCache::new(url.clone());
        let result = async {
            let db = cache
                .resolve(&tenant, &admin_db)
                .await
                .map_err(|e| format!("resolve: {e}"))?;
            let current = current_user(&db).await?;
            let own = try_sql(&db, "SELECT COUNT(*) FROM workflow_packages").await;
            let theirs = try_sql(
                &db,
                &format!("SELECT COUNT(*) FROM {other}.workflow_packages"),
            )
            .await;
            db.close();
            Ok::<_, String>((current, own, theirs))
        }
        .await;
        cache.evict(&tenant).await;
        let removed = (
            admin.remove_tenant(&tenant, &tenant).await,
            admin.remove_tenant(&other, &other).await,
        );
        admin_db.close();
        let (current, own, theirs) = result?;
        removed.0.map_err(|e| format!("remove: {e}"))?;
        removed.1.map_err(|e| format!("remove: {e}"))?;
        Ok::<_, String>((tenant, current, own, theirs))
    }
    .await;

    let _ = root.batch_execute(&format!("DROP DATABASE IF EXISTS {database} WITH (FORCE)"));
    let _ = root.batch_execute(&format!("DROP ROLE IF EXISTS {user}"));

    let (tenant, current, own, theirs) = outcome.expect("tenant as a CREATEROLE user");
    assert_eq!(current, cloacina::database::server_role_name(&tenant));
    own.expect("its own table");
    refused(theirs, "another tenant's table");
}
