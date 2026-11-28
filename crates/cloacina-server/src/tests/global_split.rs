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

//! The move of the global tables to `cloacina_global` and the pool of
//! `public` (CLOACINA-T-1080, notes n-10 to n-12). The move changes
//! `public`, so each test runs in a database of its own, made from the test
//! database's server and dropped at the end.

use super::*;

use cloacina::database::{
    DatabaseAdmin, GLOBAL_LAYOUT_TABLE, GLOBAL_SCHEMA, GlobalSplit, SERVER_GLOBAL_TABLES,
    TenantConfig, server_role_name,
};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}

#[derive(QueryableByName)]
struct Text {
    #[diesel(sql_type = diesel::sql_types::Text)]
    t: String,
}

/// A database of its own on the test database's server.
struct Scratch {
    name: String,
    url: String,
}

impl Scratch {
    fn new() -> Self {
        let name = format!(
            "t1080_split_{}",
            &uuid::Uuid::new_v4().simple().to_string()[..12]
        );
        PgConnection::establish(TEST_DB_URL)
            .expect("connect")
            .batch_execute(&format!("CREATE DATABASE {name}"))
            .expect("make the database");
        let (base, _) = TEST_DB_URL.rsplit_once('/').expect("a database in the url");
        let url = format!("{base}/{name}");
        Self { name, url }
    }

    fn connect(&self) -> PgConnection {
        PgConnection::establish(&self.url).expect("connect to the scratch database")
    }

    fn count(&self, sql: &str) -> i64 {
        diesel::sql_query(sql)
            .get_result::<Count>(&mut self.connect())
            .expect(sql)
            .n
    }

    fn admin_pool(&self) -> cloacina::Database {
        cloacina::Database::new(&self.url, "cloacina", 2)
            .with_search_path_head(GLOBAL_SCHEMA)
            .expect("admin pool")
    }

    fn public_pool(&self) -> cloacina::Database {
        cloacina::Database::new(&self.url, "cloacina", 2)
            .with_search_path_head("public")
            .and_then(|db| db.with_session_role(&server_role_name("public")))
            .expect("public pool")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = PgConnection::establish(TEST_DB_URL).map(|mut conn| {
            conn.batch_execute(&format!(
                "DROP DATABASE IF EXISTS {} WITH (FORCE)",
                self.name
            ))
        });
    }
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

async fn current_user(db: &cloacina::Database) -> String {
    let conn = db.get_postgres_connection().await.expect("checkout");
    conn.interact(|conn| {
        diesel::sql_query("SELECT current_user::text AS t").get_result::<Text>(conn)
    })
    .await
    .expect("interact")
    .expect("current_user")
    .t
}

fn outbox_row(recipient: &str) -> String {
    format!(
        "INSERT INTO public.delivery_outbox (recipient, kind, payload, created_at) \
         VALUES ('{recipient}', 'probe', '\\x00', now())"
    )
}

/// The start moves every global table to `cloacina_global`, keeps
/// `public`'s run-finished notices in `public.delivery_outbox`, moves every
/// other outbox row, writes the mark, and changes nothing the second time.
#[tokio::test]
#[serial]
async fn the_start_moves_the_global_tables_and_keeps_public_notices() {
    let scratch = Scratch::new();
    let admin_db = scratch.admin_pool();
    crate::startup_migrations::migrate_at_start(
        &admin_db,
        crate::startup_migrations::TenantSchemas::Named(&[]),
    )
    .await
    .expect("migrations");
    scratch
        .connect()
        .batch_execute(&format!(
            "{}; {}; {};",
            outbox_row(cloacina::run_alerts::RECIPIENT),
            outbox_row("agent:probe"),
            outbox_row("agent:probe")
        ))
        .expect("outbox rows");

    let first = crate::startup_migrations::split_global_at_start(&admin_db)
        .await
        .expect("split");
    let second = crate::startup_migrations::split_global_at_start(&admin_db)
        .await
        .expect("second split");
    // The alert sender after the restart: it lists `public` as a tenant with
    // an open notice, and reads the notice through `public`'s own pool.
    let sender_tenants = crate::alerts::tenants_with_open_notices(&admin_db).await;
    let public_db = scratch.public_pool();
    let sender_rows = cloacina::dal::unified::DAL::new(public_db.clone())
        .delivery_outbox()
        .list_open_for_recipient(cloacina::run_alerts::RECIPIENT, 10)
        .await
        .map(|rows| rows.len());
    public_db.close();
    // A later start migrates `public` again: nothing pending, the release
    // row of `public` is read in `public`.
    let again = crate::startup_migrations::migrate_at_start(
        &admin_db,
        crate::startup_migrations::TenantSchemas::Named(&[]),
    )
    .await;

    let in_public: Vec<(&str, i64)> = SERVER_GLOBAL_TABLES
        .iter()
        .map(|t| {
            (
                *t,
                scratch.count(&format!(
                    "SELECT COUNT(*) AS n FROM pg_tables WHERE schemaname = 'public' \
                     AND tablename = '{t}'"
                )),
            )
        })
        .filter(|(_, n)| *n > 0)
        .collect();
    let in_global = scratch.count(&format!(
        "SELECT COUNT(*) AS n FROM pg_tables WHERE schemaname = '{GLOBAL_SCHEMA}'"
    ));
    let public_notices = scratch.count("SELECT COUNT(*) AS n FROM public.delivery_outbox");
    let global_rows = scratch.count(&format!(
        "SELECT COUNT(*) AS n FROM {GLOBAL_SCHEMA}.delivery_outbox WHERE recipient = 'agent:probe'"
    ));
    let mark = diesel::sql_query(format!(
        "SELECT written_by AS t FROM {GLOBAL_SCHEMA}.{GLOBAL_LAYOUT_TABLE} WHERE id = 1"
    ))
    .get_result::<Text>(&mut scratch.connect())
    .map(|row| row.t);
    // A new global row through the admin pool goes to the global outbox.
    let admin_insert = try_sql(&admin_db, &outbox_row("agent:after").replace("public.", "")).await;
    let after = scratch.count(&format!(
        "SELECT COUNT(*) AS n FROM {GLOBAL_SCHEMA}.delivery_outbox WHERE recipient = 'agent:after'"
    ));
    admin_db.close();

    match first {
        GlobalSplit::Moved {
            tables,
            outbox_rows,
        } => {
            assert_eq!(tables.len(), SERVER_GLOBAL_TABLES.len(), "{tables:?}");
            assert_eq!(outbox_rows, 2);
        }
        other => panic!("first split: {other:?}"),
    }
    assert_eq!(
        second,
        GlobalSplit::Moved {
            tables: Vec::new(),
            outbox_rows: 0
        }
    );
    again.expect("a later start");
    assert!(in_public.is_empty(), "still in public: {in_public:?}");
    // The global tables, the global outbox and the mark.
    assert_eq!(in_global as usize, SERVER_GLOBAL_TABLES.len() + 2);
    assert_eq!(public_notices, 1, "public's notice stays in public");
    assert!(
        sender_tenants.iter().any(|t| t == "public"),
        "the alert sender finds public's open notice: {sender_tenants:?}"
    );
    assert_eq!(
        sender_rows.expect("the sender's read"),
        1,
        "the sender reads the notice through public's pool"
    );
    assert_eq!(global_rows, 2, "the other rows are global");
    assert_eq!(
        mark.expect("mark"),
        cloacina::database::schema_release::RELEASE
    );
    admin_insert.expect("insert through the admin pool");
    assert_eq!(after, 1, "the admin pool writes the global outbox");
}

/// `public`'s pool takes `public$server`: it reaches `public`'s tenant tables
/// and its own outbox, and no global table. The admin pool reads the global
/// tables.
#[tokio::test]
#[serial]
async fn the_pool_of_public_takes_its_server_role() {
    let scratch = Scratch::new();
    let admin_db = scratch.admin_pool();
    crate::startup_migrations::migrate_at_start(
        &admin_db,
        crate::startup_migrations::TenantSchemas::Named(&[]),
    )
    .await
    .expect("migrations");
    crate::startup_migrations::split_global_at_start(&admin_db)
        .await
        .expect("split");
    let public_db = scratch.public_pool();

    let user = current_user(&public_db).await;
    let mut own = Vec::new();
    for table in cloacina::database::SERVER_ROLE_TABLES {
        own.push((
            *table,
            try_sql(&public_db, &format!("SELECT COUNT(*) FROM {table}")).await,
        ));
    }
    let notice = try_sql(
        &public_db,
        &outbox_row(cloacina::run_alerts::RECIPIENT).replace("public.", ""),
    )
    .await;
    let mut global = Vec::new();
    for table in SERVER_GLOBAL_TABLES
        .iter()
        .chain(std::iter::once(&"delivery_outbox"))
    {
        global.push((
            *table,
            try_sql(
                &public_db,
                &format!("SELECT COUNT(*) FROM {GLOBAL_SCHEMA}.{table}"),
            )
            .await,
        ));
    }
    let unqualified_global = try_sql(&public_db, "SELECT COUNT(*) FROM api_keys").await;
    let admin_reads = try_sql(&admin_db, "SELECT COUNT(*) FROM api_keys").await;
    let admin_dal_keys = cloacina::dal::unified::DAL::new(admin_db.clone())
        .api_keys()
        .list_keys()
        .await;
    public_db.close();
    admin_db.close();

    assert_eq!(user, server_role_name("public"));
    for (table, result) in own {
        result.unwrap_or_else(|e| panic!("public.{table}: {e}"));
    }
    notice.expect("public writes its own notice");
    for (table, result) in global {
        match result {
            Err(e) if e.contains("permission denied") => {}
            other => panic!("{GLOBAL_SCHEMA}.{table}: not refused: {other:?}"),
        }
    }
    assert!(
        unqualified_global.is_err(),
        "a global table is not found from public's pool"
    );
    admin_reads.expect("the admin pool reads a global table");
    admin_dal_keys.expect("the admin DAL reads the keys");
}

/// The reserved name: tenant create refuses it, and the start refuses a
/// tenant schema of that name. A mark one minor ahead changes nothing; one
/// more than a minor ahead stops the start.
#[tokio::test]
#[serial]
async fn the_global_schema_name_is_reserved_and_the_mark_is_checked() {
    let scratch = Scratch::new();
    let admin_db = scratch.admin_pool();
    crate::startup_migrations::migrate_at_start(
        &admin_db,
        crate::startup_migrations::TenantSchemas::Named(&[]),
    )
    .await
    .expect("migrations");
    let admin = DatabaseAdmin::new(admin_db.clone());
    let create = admin
        .create_tenant(TenantConfig {
            schema_name: GLOBAL_SCHEMA.to_string(),
            username: GLOBAL_SCHEMA.to_string(),
            password: String::new(),
        })
        .await;

    // A schema of that name with migrations, made by hand as an older
    // server could have made a tenant.
    scratch
        .connect()
        .batch_execute(&format!(
            "CREATE SCHEMA {GLOBAL_SCHEMA}; \
             CREATE TABLE {GLOBAL_SCHEMA}.__diesel_schema_migrations (version TEXT)"
        ))
        .expect("a tenant schema of the reserved name");
    let refused_tenant = admin.split_global_schema().await;
    scratch
        .connect()
        .batch_execute(&format!(
            "DROP TABLE {GLOBAL_SCHEMA}.__diesel_schema_migrations"
        ))
        .expect("drop it");

    admin.split_global_schema().await.expect("split");
    let own = cloacina::database::schema_release::Release::parse(
        cloacina::database::schema_release::RELEASE,
    )
    .expect("release");
    let set_mark = |release: String| {
        scratch
            .connect()
            .batch_execute(&format!(
                "UPDATE {GLOBAL_SCHEMA}.{GLOBAL_LAYOUT_TABLE} SET written_by = '{release}'"
            ))
            .expect("set the mark");
    };
    set_mark(format!("{}.{}.0", own.major, own.minor + 1));
    let next_minor = admin.split_global_schema().await;
    set_mark(format!("{}.0.0", own.major + 1));
    let next_major = admin.split_global_schema().await;
    admin_db.close();

    let err = create
        .err()
        .expect("the reserved name is refused")
        .to_string();
    assert!(err.contains("reserved"), "{err}");
    let err = refused_tenant.expect_err("a tenant of that name stops the start");
    assert!(err.to_string().contains("reserved"), "{err}");
    assert!(
        matches!(next_minor, Ok(GlobalSplit::PreviousMinor { .. })),
        "{next_minor:?}"
    );
    assert!(next_major.is_err(), "{next_major:?}");
}
