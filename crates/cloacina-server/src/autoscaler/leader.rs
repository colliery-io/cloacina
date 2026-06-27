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

//! Leader election for the fleet control loop (CLOACI-T-0811).
//!
//! The autoscale + reconcile control loop must run on exactly ONE replica at a
//! time (NFR-003): two replicas scaling/actuating the same tenant concurrently
//! would double-apply deltas and overshoot. We get single-writer semantics from a
//! **Postgres session-level advisory lock** on a fixed key.
//!
//! Per tick the leader-elect helper runs `pg_try_advisory_lock($K)`:
//! - acquired → this replica is the leader for the tick: run `work`, then
//!   `pg_advisory_unlock($K)`.
//! - not acquired → another replica holds the lock; skip the tick.
//!
//! ## Failover
//! A session-level advisory lock is bound to the Postgres *session* (connection)
//! that took it. If the leader replica crashes, its TCP connection drops,
//! Postgres ends the session and **auto-releases** the lock — the next replica's
//! `pg_try_advisory_lock` then succeeds. No lease/heartbeat bookkeeping needed.
//!
//! ## Why we hold one pooled connection for the tick
//! Advisory locks are *session*-scoped: `pg_advisory_unlock` only succeeds on the
//! same session that took the lock. Our DB access goes through a `deadpool`
//! connection pool, so to lock and unlock the same session we hold one pooled
//! connection (`Object`) for the duration of the tick — lock on it, run `work`,
//! unlock on it, then return it to the pool. This is brief (one control tick),
//! NOT a dedicated server-lifetime leader connection. `work`'s own DB calls draw
//! *other* connections from the pool, so they are unaffected.

use cloacina::database::Database;
use std::future::Future;
use tracing::warn;

/// Fixed advisory-lock key for "cloacina fleet control" (CLOACI-T-0811 /
/// CLOACI-I-0127). Any value works as long as every replica agrees; this one
/// encodes the ticket + initiative numbers (`0811`, `0127`) so it's greppable
/// and unlikely to collide with another subsystem's advisory lock.
pub const FLEET_CONTROL_LOCK_KEY: i64 = 8_110_127;

#[cfg(feature = "postgres")]
#[derive(diesel::QueryableByName)]
struct AdvisoryLockRow {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    locked: bool,
}

/// Run `work` only if this replica wins fleet leadership for the call, ALWAYS
/// releasing the lock afterwards.
///
/// Returns `Some(work's output)` when leadership was acquired and `work` ran;
/// `None` when another replica holds the lock, or when the lock query itself
/// failed (fail-safe: a replica that can't talk to Postgres must not assume it
/// is the leader).
///
/// `work` is only constructed/run on the leader path, so callers can put the
/// (potentially expensive) control-loop body behind it without paying for it on
/// follower replicas.
#[cfg(feature = "postgres")]
pub async fn with_fleet_leadership<F, Fut, T>(db: &Database, work: F) -> Option<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    // Hold ONE pooled connection for the whole tick so lock + unlock land on the
    // same Postgres session (see module docs).
    let conn = match db.get_postgres_connection().await {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "fleet leadership: failed to acquire a connection; skipping tick");
            return None;
        }
    };

    // Try to take the lock. A non-blocking try: if another replica holds it we
    // get `false` and bow out rather than queueing.
    let lock_sql = format!("SELECT pg_try_advisory_lock({FLEET_CONTROL_LOCK_KEY}) AS locked");
    let acquired = match conn
        .interact(move |conn| {
            use diesel::RunQueryDsl;
            diesel::sql_query(lock_sql).get_result::<AdvisoryLockRow>(conn)
        })
        .await
    {
        Ok(Ok(row)) => row.locked,
        Ok(Err(e)) => {
            warn!(error = %e, "fleet leadership: pg_try_advisory_lock query failed; skipping tick");
            return None;
        }
        Err(e) => {
            warn!(error = %e, "fleet leadership: interact failed during lock; skipping tick");
            return None;
        }
    };

    if !acquired {
        // Another replica leads this tick. Drop the connection back to the pool.
        return None;
    }

    // We are the leader. Run the work, then ALWAYS unlock.
    let out = work().await;

    let unlock_sql = format!("SELECT pg_advisory_unlock({FLEET_CONTROL_LOCK_KEY}) AS locked");
    match conn
        .interact(move |conn| {
            use diesel::RunQueryDsl;
            diesel::sql_query(unlock_sql).get_result::<AdvisoryLockRow>(conn)
        })
        .await
    {
        Ok(Ok(row)) if row.locked => {}
        Ok(Ok(_)) => warn!(
            "fleet leadership: pg_advisory_unlock returned false (lock was not held on this \
             session); the lock will free when this connection is recycled"
        ),
        Ok(Err(e)) => warn!(error = %e, "fleet leadership: pg_advisory_unlock query failed"),
        Err(e) => warn!(error = %e, "fleet leadership: interact failed during unlock"),
    }

    Some(out)
}
