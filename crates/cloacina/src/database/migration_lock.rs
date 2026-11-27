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

//! Pending migrations under a database lock (CLOACINA-T-1042).
//!
//! diesel takes no lock for migrations. `run_pending_migrations` reads the
//! applied versions once and then runs each pending migration in its own
//! transaction, with the insert of its version into
//! `__diesel_schema_migrations`. Two processes that start together on a
//! database with pending migrations both read the same list. The second one
//! then fails, on the DDL ("already exists") or on the primary key of the
//! version row. Its transaction rolls back, so no migration is left half
//! applied, but the process gets an error and exits.
//!
//! On PostgreSQL the pending migrations therefore run in one transaction
//! that first takes `pg_advisory_xact_lock(MIGRATION_LOCK_KEY)`:
//!
//! - A second process waits on the lock. When it gets the lock, the first
//!   has committed, so its read of the applied versions finds nothing
//!   pending.
//! - The lock is a transaction lock: PostgreSQL releases it at commit, at
//!   rollback and when the connection is lost. No unlock call can be missed.
//! - The pending migrations are applied together or not at all. A migration
//!   that fails leaves the schema as it was before the call.
//!
//! The lock is one key for the database. Advisory locks have no schema
//! boundary, so the migrations of two tenant schemas also run one after the
//! other. Inside an open transaction (tenant creation), the call is a
//! savepoint and the lock is held to the end of that transaction.
//!
//! Each migration must be able to run in a transaction (no
//! `run_in_transaction = false` in a `metadata.toml`).
//!
//! SQLite has one writer and one process: its migrations take no lock.

/// The advisory-lock key of the migrations. Every process that migrates the
/// database must use the same key. It differs from the other fixed keys
/// (fleet control `8_110_127`, retention `8_111_023`) and is positive, so it
/// is not a reactor key (those have the sign bit set).
pub const MIGRATION_LOCK_KEY: i64 = 8_111_042;

/// What the locked run needs from a database connection. PostgreSQL
/// implements it; the tests implement it with no database.
#[cfg(any(feature = "postgres", test))]
pub(crate) trait LockedMigrationTarget {
    /// Runs `body` in one transaction: committed when it returns `Ok`,
    /// rolled back when it returns `Err`.
    fn in_one_transaction<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String>;

    /// Takes the lock `key` and waits for it when another session holds it.
    /// The lock lasts until the end of the transaction.
    fn take_transaction_lock(&mut self, key: i64) -> Result<(), String>;

    /// Reads the applied versions and runs each pending migration. Returns
    /// how many ran.
    fn run_pending(&mut self) -> Result<usize, String>;
}

/// The order that makes two concurrent runs safe: open the transaction, take
/// the lock, and only then read what is pending and run it.
#[cfg(any(feature = "postgres", test))]
pub(crate) fn run_pending_locked<T: LockedMigrationTarget>(
    target: &mut T,
    key: i64,
) -> Result<usize, String> {
    target.in_one_transaction(|target| {
        target.take_transaction_lock(key)?;
        target.run_pending()
    })
}

#[cfg(feature = "postgres")]
mod postgres {
    use super::LockedMigrationTarget;
    use diesel::pg::PgConnection;
    use diesel::prelude::*;
    use diesel_migrations::MigrationHarness;

    /// The error of the transaction closure: diesel needs `From` of its own
    /// error for the begin, the commit and the rollback.
    struct Failed(String);

    impl From<diesel::result::Error> for Failed {
        fn from(e: diesel::result::Error) -> Self {
            Failed(e.to_string())
        }
    }

    impl LockedMigrationTarget for PgConnection {
        fn in_one_transaction<T>(
            &mut self,
            body: impl FnOnce(&mut Self) -> Result<T, String>,
        ) -> Result<T, String> {
            self.transaction::<T, Failed, _>(|conn| body(conn).map_err(Failed))
                .map_err(|Failed(message)| message)
        }

        fn take_transaction_lock(&mut self, key: i64) -> Result<(), String> {
            diesel::sql_query("SELECT pg_advisory_xact_lock($1)")
                .bind::<diesel::sql_types::BigInt, _>(key)
                .execute(self)
                .map(|_| ())
                .map_err(|e| format!("Failed to take the migration lock: {}", e))
        }

        fn run_pending(&mut self) -> Result<usize, String> {
            self.run_pending_migrations(crate::database::POSTGRES_MIGRATIONS)
                .map(|versions| versions.len())
                .map_err(|e| format!("Failed to run PostgreSQL migrations: {}", e))
        }
    }

    /// Runs the pending PostgreSQL migrations of the schema that the
    /// connection's `search_path` names, under the migration lock (see the
    /// module docs). Returns how many ran: zero when another process applied
    /// them while this one waited.
    pub fn run_pending_postgres_migrations_locked(
        conn: &mut PgConnection,
    ) -> Result<usize, String> {
        super::run_pending_locked(conn, super::MIGRATION_LOCK_KEY)
    }
}

#[cfg(feature = "postgres")]
pub use postgres::run_pending_postgres_migrations_locked;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Duration;

    /// A stand-in for one database: which versions are applied, who holds
    /// the lock, and what happened in which order.
    #[derive(Default)]
    struct FakeDatabase {
        state: Mutex<FakeState>,
        lock_free: Condvar,
    }

    #[derive(Default)]
    struct FakeState {
        applied: Vec<&'static str>,
        lock_holder: Option<&'static str>,
        events: Vec<String>,
    }

    const MIGRATIONS: [&str; 3] = ["058", "059", "060"];

    /// One session of the stand-in. Without the lock it behaves as diesel
    /// does: it reads the applied versions, and a version that another
    /// session applied in the meantime is an error.
    struct FakeSession {
        name: &'static str,
        database: Arc<FakeDatabase>,
        /// Versions applied in the open transaction, for a rollback.
        in_transaction: Vec<&'static str>,
        holds_lock: bool,
        /// The migration that fails, if any.
        fails_at: Option<&'static str>,
        /// A pause between the read of the applied versions and the run.
        pause_after_read: Duration,
    }

    impl FakeSession {
        fn new(name: &'static str, database: &Arc<FakeDatabase>) -> Self {
            Self {
                name,
                database: database.clone(),
                in_transaction: Vec::new(),
                holds_lock: false,
                fails_at: None,
                pause_after_read: Duration::ZERO,
            }
        }

        fn end_transaction(&mut self, commit: bool) {
            let mut state = self.database.state.lock().unwrap();
            if !commit {
                state.applied.retain(|v| !self.in_transaction.contains(v));
            }
            self.in_transaction.clear();
            if self.holds_lock {
                self.holds_lock = false;
                state.lock_holder = None;
                state
                    .events
                    .push(format!("{} releases the lock", self.name));
                self.database.lock_free.notify_all();
            }
        }
    }

    impl LockedMigrationTarget for FakeSession {
        fn in_one_transaction<T>(
            &mut self,
            body: impl FnOnce(&mut Self) -> Result<T, String>,
        ) -> Result<T, String> {
            let result = body(self);
            self.end_transaction(result.is_ok());
            result
        }

        fn take_transaction_lock(&mut self, _key: i64) -> Result<(), String> {
            let mut state = self.database.state.lock().unwrap();
            while state.lock_holder.is_some() {
                state = self.database.lock_free.wait(state).unwrap();
            }
            state.lock_holder = Some(self.name);
            state.events.push(format!("{} takes the lock", self.name));
            self.holds_lock = true;
            Ok(())
        }

        fn run_pending(&mut self) -> Result<usize, String> {
            let pending: Vec<&'static str> = {
                let state = self.database.state.lock().unwrap();
                MIGRATIONS
                    .iter()
                    .copied()
                    .filter(|v| !state.applied.contains(v))
                    .collect()
            };
            std::thread::sleep(self.pause_after_read);
            for version in &pending {
                if self.fails_at == Some(*version) {
                    return Err(format!("migration {version} failed"));
                }
                let mut state = self.database.state.lock().unwrap();
                if state.applied.contains(version) {
                    return Err(format!("{version} is already applied"));
                }
                state.applied.push(version);
                state
                    .events
                    .push(format!("{} applies {version}", self.name));
                self.in_transaction.push(version);
            }
            Ok(pending.len())
        }
    }

    /// Two processes start together. The second waits for the lock and then
    /// finds nothing to do; neither fails.
    #[test]
    fn a_second_runner_waits_and_then_finds_nothing_pending() {
        let database = Arc::new(FakeDatabase::default());
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let run = |name: &'static str| {
            let database = database.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut session = FakeSession::new(name, &database);
                session.pause_after_read = Duration::from_millis(100);
                barrier.wait();
                run_pending_locked(&mut session, MIGRATION_LOCK_KEY)
            })
        };
        let (first, second) = (run("a"), run("b"));
        let mut ran = [
            first.join().unwrap().expect("first"),
            second.join().unwrap().expect("second"),
        ];
        ran.sort();
        assert_eq!(ran, [0, 3], "one runs the three, the other none");

        let state = database.state.lock().unwrap();
        assert_eq!(state.applied, MIGRATIONS.to_vec());
        // The lock is held from before the first migration until after the
        // last: take, apply x3, release, take, release.
        let shape: Vec<&str> = state
            .events
            .iter()
            .map(|event| event.split_once(' ').unwrap().1)
            .collect();
        assert_eq!(
            shape,
            vec![
                "takes the lock",
                "applies 058",
                "applies 059",
                "applies 060",
                "releases the lock",
                "takes the lock",
                "releases the lock",
            ]
        );
    }

    /// The stand-in without the lock shows the race that the lock removes:
    /// the same two runs, each reading the applied versions before the other
    /// has written, and one of them fails.
    #[test]
    fn without_the_lock_the_second_runner_fails() {
        let database = Arc::new(FakeDatabase::default());
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let run = |name: &'static str| {
            let database = database.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut session = FakeSession::new(name, &database);
                session.pause_after_read = Duration::from_millis(100);
                barrier.wait();
                session.in_one_transaction(|session| session.run_pending())
            })
        };
        let (first, second) = (run("a"), run("b"));
        let results = [first.join().unwrap(), second.join().unwrap()];
        assert!(
            results.iter().any(|result| result.is_err()),
            "with no lock one runner hits a version that the other applied: {results:?}"
        );
    }

    /// A migration that fails rolls the whole run back and frees the lock:
    /// the database is as before, and the next run can apply everything.
    #[test]
    fn a_failed_migration_leaves_nothing_applied_and_frees_the_lock() {
        let database = Arc::new(FakeDatabase::default());

        let mut failing = FakeSession::new("a", &database);
        failing.fails_at = Some("060");
        let error = run_pending_locked(&mut failing, MIGRATION_LOCK_KEY).unwrap_err();
        assert_eq!(error, "migration 060 failed");
        {
            let state = database.state.lock().unwrap();
            assert!(state.applied.is_empty(), "058 and 059 are rolled back");
            assert_eq!(state.lock_holder, None);
        }

        let mut next = FakeSession::new("b", &database);
        assert_eq!(run_pending_locked(&mut next, MIGRATION_LOCK_KEY), Ok(3));
    }
}
