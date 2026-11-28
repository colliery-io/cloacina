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

//! The release that last migrated a schema, and the guards that read it
//! (CLOACINA-T-1087).
//!
//! Each schema (`public`, each tenant schema, a SQLite file) has the table
//! `cloacina_schema_release` with one row: `written_by`, the release of the
//! binary that last ran a migration on it. That binary writes the row in the
//! same transaction as its migrations, under the migration lock, and only
//! raises it.
//!
//! Before it migrates, a binary reads the row and compares the minor of
//! `written_by` with its own:
//!
//! - its own minor or older, or no row (a schema from before the table):
//!   it migrates, and writes the row;
//! - exactly one minor ahead: it runs no migration and goes on, with a
//!   warning that names both releases. This is the previous minor's server
//!   during a rolling upgrade; each migration of a minor leaves the previous
//!   minor's server working;
//! - more than one minor ahead, another major, or a row it cannot read: it
//!   refuses, and the error names both releases.
//!
//! The compiler never migrates (see [`compiler_check`]): it waits for the
//! server while the schema is behind its binary, and refuses a schema that is
//! too new.

use std::fmt;

/// The release of this binary, as written into `written_by`.
pub const RELEASE: &str = env!("CARGO_PKG_VERSION");

/// The table of the release row.
pub const TABLE: &str = "cloacina_schema_release";

/// A release `major.minor.patch`. A pre-release or build suffix is ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Release {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Release {
    /// Reads `0.13.2`, `0.13.2-rc.1` or `0.13.2+build`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let core = text
            .trim()
            .split(['-', '+'])
            .next()
            .unwrap_or_default()
            .to_string();
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("'{text}' is not a release (major.minor.patch)"));
        }
        let number = |part: &str| {
            part.parse::<u64>()
                .map_err(|_| format!("'{text}' is not a release (major.minor.patch)"))
        };
        Ok(Self {
            major: number(parts[0])?,
            minor: number(parts[1])?,
            patch: number(parts[2])?,
        })
    }

    /// `0.13`.
    pub fn minor_label(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }

    /// The next minor of the same major.
    fn next_minor(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
        }
    }

    fn same_minor(&self, other: &Self) -> bool {
        self.major == other.major && self.minor == other.minor
    }
}

impl fmt::Display for Release {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// What a migrating binary does with a schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Run the pending migrations. `write` is true when the row must be
    /// written after them: no row, or a lower release.
    Migrate { write: bool },
    /// The schema is exactly one minor ahead: run no migration, go on.
    PreviousMinor { written_by: String },
}

/// The guard: what a binary of release `own` does with a schema that
/// `written_by` last migrated. `Err` is the refusal, with both releases.
pub fn check(own: &str, written_by: Option<&str>) -> Result<Verdict, String> {
    let own_release = Release::parse(own)?;
    let Some(written_by) = written_by else {
        return Ok(Verdict::Migrate { write: true });
    };
    let written = Release::parse(written_by).map_err(|e| {
        format!(
            "the schema's release row cannot be read ({e}); this binary is {own_release} and \
             does not use the schema"
        )
    })?;
    if written.same_minor(&own_release) || written < own_release {
        return Ok(Verdict::Migrate {
            write: written < own_release,
        });
    }
    if written.same_minor(&own_release.next_minor()) {
        return Ok(Verdict::PreviousMinor {
            written_by: written.to_string(),
        });
    }
    Err(too_new_message(&own_release, &written))
}

fn too_new_message(own: &Release, written: &Release) -> String {
    format!(
        "the schema was last migrated by release {written}; this binary is {own} and works on \
         schemas of {} and {} only: upgrade this binary to {} or later",
        own.minor_label(),
        own.next_minor().minor_label(),
        written.minor_label()
    )
}

/// The warning of a schema one minor ahead.
pub fn previous_minor_message(own: &str, written_by: &str) -> String {
    format!(
        "the schema was last migrated by release {written_by}, one minor ahead of this binary \
         ({own}); no migration runs. This is the previous minor during an upgrade: upgrade this \
         binary to {written_by}"
    )
}

/// What the compiler does with a schema (CLOACINA-T-1087). The compiler runs
/// no migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerSchema {
    /// The schema has each migration of this binary: start.
    Ready,
    /// The schema lacks `pending` migrations of this binary: the server has
    /// not migrated it yet. Wait for it.
    Behind { pending: usize, message: String },
    /// The schema is too new for this binary: exit at once.
    TooNew(String),
}

/// The compiler's guard: `pending` is the number of migrations of this
/// binary that the schema lacks.
pub fn compiler_check(own: &str, written_by: Option<&str>, pending: usize) -> CompilerSchema {
    match check(own, written_by) {
        Err(message) => CompilerSchema::TooNew(message),
        Ok(Verdict::PreviousMinor { .. }) => CompilerSchema::Ready,
        Ok(Verdict::Migrate { .. }) if pending == 0 => CompilerSchema::Ready,
        Ok(Verdict::Migrate { .. }) => CompilerSchema::Behind {
            pending,
            message: format!(
                "the schema lacks {pending} migration(s) of this compiler ({own}); it was last \
                 migrated by {}. The compiler runs no migration: start or upgrade the server, \
                 which migrates the schema",
                written_by.unwrap_or("a release from before the release row")
            ),
        },
    }
}

/// What the compiler reads of a schema, with no write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaStatus {
    /// The release row, if the table and the row exist.
    pub written_by: Option<String>,
    /// The migrations of this binary that the schema lacks.
    pub pending: usize,
}

#[derive(diesel::QueryableByName)]
struct Present {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    present: bool,
}

#[derive(diesel::QueryableByName)]
struct WrittenBy {
    #[diesel(sql_type = diesel::sql_types::Text)]
    written_by: String,
}

#[derive(diesel::QueryableByName)]
struct AppliedVersion {
    #[diesel(sql_type = diesel::sql_types::Text)]
    version: String,
}

/// PostgreSQL. Each function reads or writes the schema that the
/// connection's `search_path` names first (`current_schema()`), never a
/// table of `public` found through the path.
#[cfg(feature = "postgres")]
pub(crate) mod postgres {
    use super::*;
    use diesel::migration::MigrationSource;
    use diesel::pg::{Pg, PgConnection};
    use diesel::prelude::*;

    fn present(conn: &mut PgConnection, table: &str) -> Result<bool, String> {
        diesel::sql_query(
            "SELECT to_regclass(quote_ident(current_schema()) || '.' || $1) IS NOT NULL \
             AS present",
        )
        .bind::<diesel::sql_types::Text, _>(table)
        .get_result::<Present>(conn)
        .map(|row| row.present)
        .map_err(|e| format!("Failed to look up {table}: {e}"))
    }

    /// The release row, or `None` with no table or no row.
    pub fn read(conn: &mut PgConnection) -> Result<Option<String>, String> {
        if !present(conn, TABLE)? {
            return Ok(None);
        }
        diesel::sql_query("SELECT written_by FROM cloacina_schema_release WHERE id = 1")
            .get_results::<WrittenBy>(conn)
            .map(|rows| rows.into_iter().next().map(|row| row.written_by))
            .map_err(|e| format!("Failed to read the schema's release row: {e}"))
    }

    /// Writes `release` into the row. With no table, nothing.
    pub fn write(conn: &mut PgConnection, release: &str) -> Result<(), String> {
        if !present(conn, TABLE)? {
            return Ok(());
        }
        diesel::sql_query(
            "INSERT INTO cloacina_schema_release (id, written_by, written_at) \
             VALUES (1, $1, CURRENT_TIMESTAMP) \
             ON CONFLICT (id) DO UPDATE SET written_by = EXCLUDED.written_by, \
             written_at = EXCLUDED.written_at",
        )
        .bind::<diesel::sql_types::Text, _>(release)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| format!("Failed to write the schema's release row: {e}"))
    }

    /// The release row and the number of this binary's migrations that the
    /// schema lacks. Reads only: no table is made, so a role that may only
    /// read (a compiler's build role) can call it.
    pub fn status(conn: &mut PgConnection) -> Result<SchemaStatus, String> {
        let written_by = read(conn)?;
        let applied: Vec<String> = if present(conn, "__diesel_schema_migrations")? {
            diesel::sql_query("SELECT version::text AS version FROM __diesel_schema_migrations")
                .load::<AppliedVersion>(conn)
                .map_err(|e| format!("Failed to read the applied migrations: {e}"))?
                .into_iter()
                .map(|row| row.version)
                .collect()
        } else {
            Vec::new()
        };
        let known = MigrationSource::<Pg>::migrations(&crate::database::POSTGRES_MIGRATIONS)
            .map_err(|e| format!("Failed to list the migrations: {e}"))?;
        let pending = known
            .iter()
            .filter(|m| !applied.contains(&m.name().version().to_string()))
            .count();
        Ok(SchemaStatus {
            written_by,
            pending,
        })
    }
}

/// SQLite: one file, one process.
#[cfg(feature = "sqlite")]
pub(crate) mod sqlite {
    use super::*;
    use diesel::migration::MigrationSource;
    use diesel::prelude::*;
    use diesel::sqlite::{Sqlite, SqliteConnection};
    use diesel_migrations::MigrationHarness;
    use tracing::warn;

    fn present(conn: &mut SqliteConnection, table: &str) -> Result<bool, String> {
        diesel::sql_query(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?) \
             AS present",
        )
        .bind::<diesel::sql_types::Text, _>(table)
        .get_result::<Present>(conn)
        .map(|row| row.present)
        .map_err(|e| format!("Failed to look up {table}: {e}"))
    }

    pub fn read(conn: &mut SqliteConnection) -> Result<Option<String>, String> {
        if !present(conn, TABLE)? {
            return Ok(None);
        }
        diesel::sql_query("SELECT written_by FROM cloacina_schema_release WHERE id = 1")
            .get_results::<WrittenBy>(conn)
            .map(|rows| rows.into_iter().next().map(|row| row.written_by))
            .map_err(|e| format!("Failed to read the database's release row: {e}"))
    }

    pub fn write(conn: &mut SqliteConnection, release: &str) -> Result<(), String> {
        if !present(conn, TABLE)? {
            return Ok(());
        }
        diesel::sql_query(
            "INSERT INTO cloacina_schema_release (id, written_by, written_at) \
             VALUES (1, ?, CURRENT_TIMESTAMP) \
             ON CONFLICT (id) DO UPDATE SET written_by = excluded.written_by, \
             written_at = excluded.written_at",
        )
        .bind::<diesel::sql_types::Text, _>(release)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| format!("Failed to write the database's release row: {e}"))
    }

    pub fn status(conn: &mut SqliteConnection) -> Result<SchemaStatus, String> {
        let written_by = read(conn)?;
        let applied: Vec<String> = if present(conn, "__diesel_schema_migrations")? {
            diesel::sql_query("SELECT version FROM __diesel_schema_migrations")
                .load::<AppliedVersion>(conn)
                .map_err(|e| format!("Failed to read the applied migrations: {e}"))?
                .into_iter()
                .map(|row| row.version)
                .collect()
        } else {
            Vec::new()
        };
        let known = MigrationSource::<Sqlite>::migrations(&crate::database::SQLITE_MIGRATIONS)
            .map_err(|e| format!("Failed to list the migrations: {e}"))?;
        let pending = known
            .iter()
            .filter(|m| !applied.contains(&m.name().version().to_string()))
            .count();
        Ok(SchemaStatus {
            written_by,
            pending,
        })
    }

    /// The guarded migration of a SQLite database, as release `own`: read
    /// the row, refuse a database that is too new, run nothing on one minor
    /// ahead, else run the pending migrations and write the row. Returns how
    /// many ran.
    pub fn run_pending_guarded_as(conn: &mut SqliteConnection, own: &str) -> Result<usize, String> {
        let written_by = read(conn)?;
        match check(own, written_by.as_deref())? {
            Verdict::PreviousMinor { written_by } => {
                warn!(
                    schema_release = %written_by,
                    binary_release = %own,
                    "{}",
                    previous_minor_message(own, &written_by)
                );
                Ok(0)
            }
            Verdict::Migrate { write: raise } => {
                let ran = conn
                    .run_pending_migrations(crate::database::SQLITE_MIGRATIONS)
                    .map(|versions| versions.len())
                    .map_err(|e| format!("Failed to run SQLite migrations: {e}"))?;
                if raise {
                    write(conn, own)?;
                }
                Ok(ran)
            }
        }
    }
}

/// Runs the pending SQLite migrations with the release guard, as this
/// binary's release. Returns how many ran.
#[cfg(feature = "sqlite")]
pub fn run_pending_sqlite_migrations_guarded(
    conn: &mut diesel::sqlite::SqliteConnection,
) -> Result<usize, String> {
    sqlite::run_pending_guarded_as(conn, RELEASE)
}

/// The release row and the pending migrations of the schema of `database`,
/// read with no write: the compiler's check (CLOACINA-T-1087).
pub async fn schema_status(database: &crate::Database) -> Result<SchemaStatus, String> {
    match database.backend() {
        #[cfg(feature = "postgres")]
        crate::database::BackendType::Postgres => {
            let conn = database
                .get_postgres_connection()
                .await
                .map_err(|e| e.to_string())?;
            conn.interact(postgres::status)
                .await
                .map_err(|e| e.to_string())?
        }
        #[cfg(feature = "sqlite")]
        crate::database::BackendType::Sqlite => {
            let conn = database
                .get_sqlite_connection()
                .await
                .map_err(|e| e.to_string())?;
            conn.interact(sqlite::status)
                .await
                .map_err(|e| e.to_string())?
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_is_read_with_its_suffix_ignored() {
        assert_eq!(
            Release::parse("0.13.2").unwrap(),
            Release {
                major: 0,
                minor: 13,
                patch: 2
            }
        );
        assert_eq!(
            Release::parse("0.13.2-rc.1").unwrap(),
            Release::parse("0.13.2").unwrap()
        );
        assert!(Release::parse("0.13").is_err());
        assert!(Release::parse("x.y.z").is_err());
    }

    #[test]
    fn no_row_migrates_and_writes_it() {
        assert_eq!(check("0.13.2", None), Ok(Verdict::Migrate { write: true }));
    }

    #[test]
    fn an_older_schema_migrates_and_raises_the_row() {
        assert_eq!(
            check("0.14.0", Some("0.13.2")),
            Ok(Verdict::Migrate { write: true })
        );
        assert_eq!(
            check("0.13.2", Some("0.10.0")),
            Ok(Verdict::Migrate { write: true })
        );
    }

    #[test]
    fn the_same_minor_migrates_and_never_lowers_the_row() {
        assert_eq!(
            check("0.13.1", Some("0.13.2")),
            Ok(Verdict::Migrate { write: false })
        );
        assert_eq!(
            check("0.13.2", Some("0.13.2")),
            Ok(Verdict::Migrate { write: false })
        );
        assert_eq!(
            check("0.13.3", Some("0.13.2")),
            Ok(Verdict::Migrate { write: true })
        );
    }

    #[test]
    fn one_minor_ahead_runs_no_migration() {
        assert_eq!(
            check("0.13.2", Some("0.14.1")),
            Ok(Verdict::PreviousMinor {
                written_by: "0.14.1".to_string()
            })
        );
    }

    #[test]
    fn two_minors_ahead_is_refused_with_both_releases() {
        let error = check("0.13.2", Some("0.15.0")).unwrap_err();
        assert!(error.contains("0.15.0"), "{error}");
        assert!(error.contains("0.13.2"), "{error}");
        assert!(error.contains("0.13 and 0.14 only"), "{error}");
    }

    #[test]
    fn another_major_ahead_is_refused() {
        assert!(check("0.13.2", Some("1.0.0")).is_err());
        assert!(check("1.4.0", Some("2.0.0")).is_err());
    }

    #[test]
    fn an_unreadable_row_is_refused() {
        let error = check("0.13.2", Some("garbage")).unwrap_err();
        assert!(error.contains("garbage"), "{error}");
    }

    #[test]
    fn the_compiler_waits_on_a_schema_behind_it() {
        match compiler_check("0.14.0", Some("0.13.2"), 3) {
            CompilerSchema::Behind { pending, message } => {
                assert_eq!(pending, 3);
                assert!(message.contains("0.14.0"), "{message}");
                assert!(message.contains("0.13.2"), "{message}");
            }
            other => panic!("expected Behind, got {other:?}"),
        }
        assert!(matches!(
            compiler_check("0.14.0", None, 72),
            CompilerSchema::Behind { pending: 72, .. }
        ));
    }

    #[test]
    fn the_compiler_starts_on_a_migrated_schema_and_one_minor_ahead() {
        assert_eq!(
            compiler_check("0.14.0", Some("0.14.0"), 0),
            CompilerSchema::Ready
        );
        assert_eq!(
            compiler_check("0.14.0", Some("0.15.0"), 0),
            CompilerSchema::Ready
        );
    }

    #[test]
    fn the_compiler_refuses_a_schema_too_new() {
        match compiler_check("0.13.0", Some("0.15.2"), 0) {
            CompilerSchema::TooNew(message) => {
                assert!(message.contains("0.15.2"), "{message}");
                assert!(message.contains("0.13.0"), "{message}");
            }
            other => panic!("expected TooNew, got {other:?}"),
        }
    }

    #[test]
    fn this_binarys_release_is_readable() {
        Release::parse(RELEASE).expect("CARGO_PKG_VERSION");
    }

    /// SQLite: a fresh database lacks every migration and has no row; after
    /// the migration at open it lacks none and the row is this release.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn sqlite_open_migrates_and_writes_the_release_row() {
        let db = crate::Database::new("sqlite://:memory:", "", 1);
        let before = schema_status(&db).await.expect("status");
        assert_eq!(before.written_by, None);
        assert!(before.pending > 40, "{before:?}");
        db.run_migrations().await.expect("migrations");
        let after = schema_status(&db).await.expect("status");
        assert_eq!(
            after,
            SchemaStatus {
                written_by: Some(RELEASE.to_string()),
                pending: 0
            }
        );
    }

    /// SQLite: an older binary on a database that a newer one migrated.
    /// Two minors ahead is refused with both releases; one minor ahead opens
    /// with no migration; the row is unchanged.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn sqlite_refuses_a_database_too_new() {
        let db = crate::Database::new("sqlite://:memory:", "", 1);
        db.run_migrations().await.expect("migrations");
        let conn = db.get_sqlite_connection().await.expect("connection");
        conn.interact(|conn| {
            sqlite::write(conn, "0.15.0").unwrap();
            let error = sqlite::run_pending_guarded_as(conn, "0.13.4").unwrap_err();
            assert!(
                error.contains("0.15.0") && error.contains("0.13.4"),
                "{error}"
            );
            assert_eq!(sqlite::run_pending_guarded_as(conn, "0.14.0"), Ok(0));
            assert_eq!(sqlite::read(conn).unwrap().as_deref(), Some("0.15.0"));
        })
        .await
        .expect("interact");
    }
}
