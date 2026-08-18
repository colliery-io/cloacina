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

//! Per-reactor ownership via a single long-lived advisory-lock session
//! (CLOACI-T-0851, [`ADR CLOACI-A-0012`]).
//!
//! # The shape, and why it differs from the fleet control loop
//!
//! `cloacina-server::autoscaler::leader` takes an advisory lock, does one
//! tick's work, and unlocks — all on one pooled connection held briefly. Reactor
//! ownership is not tick-shaped: a replica owns a reactor for as long as it runs
//! it. Copying the fleet pattern per reactor would pin one pooled connection per
//! owned reactor for the process lifetime and exhaust the pool as reactor count
//! grows.
//!
//! Instead this holds **one dedicated connection per replica** — the *ownership
//! session* — which carries ALL of that replica's reactor locks. A Postgres
//! session can hold many advisory locks, so the connection cost is O(1) in
//! reactor count. If the replica dies, the session ends and Postgres releases
//! every one of its reactor locks at once, which is exactly the failover
//! behaviour we want.
//!
//! **Operator note:** this permanently reserves one connection per replica from
//! the pool. Size the pool with that in mind (maintainer decision 2026-08-16:
//! take one and document it, rather than raising the default).
//!
//! # Why there is a liveness check even though locks are session-scoped
//!
//! A-0012 records that session-scoped locks need "no lease/heartbeat
//! bookkeeping" because a crashed replica's locks auto-release. That is true for
//! *failover*, and it is NOT sufficient here — long-held ownership has a failure
//! mode the fleet loop cannot have:
//!
//! > The ownership connection drops (network blip, PgBouncer recycle, database
//! > restart) while the replica keeps running. Postgres releases every lock the
//! > session held. Another replica legitimately claims the reactor. **The
//! > original replica never notices and keeps running it too.** Two replicas,
//! > one reactor, no error raised anywhere.
//!
//! The fleet loop is immune because it re-acquires every tick, so a dropped
//! connection simply means it stops being leader. Ownership that is *assumed*
//! rather than *re-established* has to be re-verified, or the split-brain this
//! whole mechanism exists to prevent comes back through the side door.
//!
//! So [`OwnershipSession::verify_owned`] re-asserts the locks this replica
//! believes it holds. This is loss DETECTION, not lease renewal: there is no
//! TTL, no clock assumption, and no bookkeeping row. Postgres stays the single
//! source of truth; we are only asking it whether what we believe is still true.
//! Callers must stop the affected reactors on loss before attempting re-claim —
//! see [`OwnershipSession::verify_owned`]'s contract.

use std::collections::HashSet;

use crate::computation_graph::reactor_lock_key::reactor_lock_key;

/// Identifies a reactor for ownership purposes: the tenant scope it was loaded
/// under (`None` for single-tenant / admin-owned) plus its name.
///
/// This mirrors how `ComputationGraphScheduler` keys its `reactors` map, so an
/// ownership claim and the in-process reactor it guards cannot drift apart.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReactorId {
    pub tenant: Option<String>,
    pub name: String,
}

impl ReactorId {
    pub fn new(tenant: Option<impl Into<String>>, name: impl Into<String>) -> Self {
        Self {
            tenant: tenant.map(Into::into),
            name: name.into(),
        }
    }

    /// The database-wide advisory-lock key for this reactor.
    pub fn lock_key(&self) -> i64 {
        reactor_lock_key(self.tenant.as_deref(), &self.name)
    }
}

// `ReactorId` and `TenantKey` are the same `(tenant, name)` pair. Conversions
// rather than a parallel type: `TenantKey`'s own docs warn that a deployment
// must never have "two spellings of the same scope", and an ownership claim
// keyed differently from the scheduler's `reactors` map is precisely that —
// it would take a lock for one reactor while guarding another.
impl From<&crate::TenantKey> for ReactorId {
    fn from(k: &crate::TenantKey) -> Self {
        Self {
            tenant: k.tenant_id.clone(),
            name: k.name.clone(),
        }
    }
}

impl From<&ReactorId> for crate::TenantKey {
    fn from(id: &ReactorId) -> Self {
        Self {
            tenant_id: id.tenant.clone(),
            name: id.name.clone(),
        }
    }
}

/// What a liveness check found. Deliberately not a bool: "we lost some locks" is
/// a different situation from "the check itself could not run", and conflating
/// them is how a replica ends up either needlessly stopping healthy reactors or
/// confidently running unowned ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipCheck {
    /// Every reactor this session believes it owns is still locked by it.
    AllHeld,
    /// These reactors are no longer owned. The caller MUST stop them locally
    /// before attempting to re-claim — another replica may already be running
    /// them.
    Lost(Vec<ReactorId>),
    /// The check could not be completed (connection down, query failed). The
    /// caller must treat this as *unknown*, not as healthy: a session that
    /// cannot reach Postgres cannot know whether it still holds anything.
    Indeterminate(String),
}

/// Tracks which reactors this replica believes it owns.
///
/// The database is the source of truth; this set is only what we *believe*, and
/// [`OwnershipCheck`] exists precisely because belief and truth can diverge.
#[derive(Debug, Default)]
pub struct OwnershipState {
    held: HashSet<ReactorId>,
}

impl OwnershipState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful claim.
    pub fn record_claimed(&mut self, id: ReactorId) {
        self.held.insert(id);
    }

    /// Forget a reactor — after an explicit release, or after a liveness check
    /// reported it lost.
    pub fn record_released(&mut self, id: &ReactorId) {
        self.held.remove(id);
    }

    pub fn believes_owned(&self, id: &ReactorId) -> bool {
        self.held.contains(id)
    }

    pub fn owned(&self) -> impl Iterator<Item = &ReactorId> {
        self.held.iter()
    }

    pub fn len(&self) -> usize {
        self.held.len()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// Given the set of keys Postgres reports this session still holds, return
    /// the reactors we believe we own but do not.
    ///
    /// Kept as a pure function so the divergence logic — the part that decides
    /// whether reactors get stopped — is testable without a database.
    pub fn diff_against_held_keys(&self, held_keys: &HashSet<i64>) -> Vec<ReactorId> {
        let mut lost: Vec<ReactorId> = self
            .held
            .iter()
            .filter(|id| !held_keys.contains(&id.lock_key()))
            .cloned()
            .collect();
        // Deterministic order so logs and tests do not depend on HashSet
        // iteration order.
        lost.sort_by(|a, b| (&a.tenant, &a.name).cmp(&(&b.tenant, &b.name)));
        lost
    }
}

/// What the watchdog decided a caller should do after one liveness check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchdogAction {
    /// Ownership confirmed (or nothing owned). Keep running.
    Continue,
    /// These reactors are provably no longer ours. Stop them locally.
    StopReactors(Vec<ReactorId>),
    /// We have been unable to verify for too long. Stop everything we believed
    /// we owned and treat it as lost. See [`OwnershipWatchdog`] for why this is
    /// the safe direction.
    StopAllPresumedLost(Vec<ReactorId>),
}

/// Turns a sequence of [`OwnershipCheck`]s into decisions.
///
/// The interesting case is [`OwnershipCheck::Indeterminate`], which the ADR
/// amendment deliberately leaves as a policy question. Both naive answers are
/// wrong:
///
/// * **Treat it as healthy** → a replica partitioned from Postgres keeps running
///   its reactors forever. But a partition is exactly what KILLS the ownership
///   session, and Postgres releases the locks the moment that connection drops,
///   so another replica will legitimately take over. Optimism here produces the
///   split-brain in its worst form: indefinite, and invisible.
/// * **Treat it as loss immediately** → one dropped query, one restarting
///   connection pool, and healthy reactors stop for no reason. Reactive
///   workloads would flap on ordinary transient errors.
///
/// So indeterminacy is tolerated, but only for a bounded number of consecutive
/// checks. Cross the threshold and we presume loss and stop, because the longer
/// we cannot verify, the likelier it is that our session is already gone and
/// someone else owns these reactors. Fail toward stopping: a stopped reactor is
/// recoverable by re-claiming, while two live reactors silently double-process.
#[derive(Debug)]
pub struct OwnershipWatchdog {
    consecutive_indeterminate: u32,
    max_indeterminate: u32,
}

impl OwnershipWatchdog {
    /// `max_indeterminate` is how many consecutive unverifiable checks to
    /// tolerate before presuming loss. Must be >= 1; 0 is clamped to 1 so a
    /// misconfiguration cannot make every transient blip stop reactors.
    pub fn new(max_indeterminate: u32) -> Self {
        Self {
            consecutive_indeterminate: 0,
            max_indeterminate: max_indeterminate.max(1),
        }
    }

    pub fn consecutive_indeterminate(&self) -> u32 {
        self.consecutive_indeterminate
    }

    /// Feed one check result; get the action to take.
    ///
    /// `believed_owned` is what we would have to stop if we presume total loss.
    pub fn observe(
        &mut self,
        check: OwnershipCheck,
        believed_owned: &[ReactorId],
    ) -> WatchdogAction {
        match check {
            OwnershipCheck::AllHeld => {
                // A successful verification clears the streak — transient
                // failures must not accumulate across healthy checks.
                self.consecutive_indeterminate = 0;
                WatchdogAction::Continue
            }
            OwnershipCheck::Lost(lost) => {
                // We got a definitive answer, so the streak is irrelevant.
                self.consecutive_indeterminate = 0;
                WatchdogAction::StopReactors(lost)
            }
            OwnershipCheck::Indeterminate(_) => {
                self.consecutive_indeterminate += 1;
                if self.consecutive_indeterminate >= self.max_indeterminate {
                    WatchdogAction::StopAllPresumedLost(believed_owned.to_vec())
                } else {
                    WatchdogAction::Continue
                }
            }
        }
    }
}

/// SQL asking Postgres which advisory locks the CURRENT session holds.
///
/// `pg_locks.pid = pg_backend_pid()` restricts to this session, which is the
/// whole point: another replica holding the same lock must NOT read as "we
/// still own it". The 64-bit key is reassembled from the split
/// `classid`/`objid` columns (see `reactor_lock_key` and the k8s-leader lane's
/// `advisory_lock_parts` for the same split on the test side).
pub const SESSION_HELD_LOCKS_SQL: &str = "SELECT (classid::bigint << 32) | objid::bigint AS key \
     FROM pg_locks \
     WHERE locktype = 'advisory' AND objsubid = 1 AND granted AND pid = pg_backend_pid()";

#[cfg(feature = "postgres")]
mod session {
    use super::*;
    use deadpool_diesel::postgres::Manager as PgManager;
    use tracing::warn;

    #[derive(diesel::QueryableByName)]
    struct AdvisoryLockRow {
        #[diesel(sql_type = diesel::sql_types::Bool)]
        locked: bool,
    }

    #[derive(diesel::QueryableByName)]
    struct HeldKeyRow {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        key: i64,
    }

    /// A replica's reactor-ownership session: ONE pooled connection carrying all
    /// of this replica's reactor advisory locks.
    ///
    /// Dropping this returns the connection to the pool, which ends the session
    /// and releases every lock — so it must outlive the reactors it guards.
    pub struct OwnershipSession {
        conn: deadpool::managed::Object<PgManager>,
        state: OwnershipState,
    }

    impl OwnershipSession {
        /// Take the dedicated connection. See the module note on pool sizing:
        /// this reserves one connection per replica for the process lifetime.
        pub async fn connect(
            db: &crate::database::Database,
        ) -> Result<Self, deadpool::managed::PoolError<deadpool_diesel::Error>> {
            Ok(Self {
                conn: db.get_postgres_connection().await?,
                state: OwnershipState::new(),
            })
        }

        pub fn state(&self) -> &OwnershipState {
            &self.state
        }

        /// Try to claim `id`. `Ok(false)` means another replica owns it — an
        /// ordinary outcome, not an error.
        ///
        /// A failed claim must NOT be recorded as owned; that is the difference
        /// between "we did not get it" and believing we did.
        pub async fn claim(&mut self, id: &ReactorId) -> Result<bool, String> {
            let sql = format!("SELECT pg_try_advisory_lock({}) AS locked", id.lock_key());
            let acquired = self.run_lock_sql(sql).await?;
            if acquired {
                self.state.record_claimed(id.clone());
            }
            Ok(acquired)
        }

        /// Release `id`, forgetting it locally regardless of what Postgres says.
        ///
        /// If the unlock reports false the lock was not held on this session —
        /// which means we had already lost it, so continuing to believe we own
        /// it is the dangerous option. We warn and forget either way.
        pub async fn release(&mut self, id: &ReactorId) -> Result<(), String> {
            let sql = format!("SELECT pg_advisory_unlock({}) AS locked", id.lock_key());
            let released = self.run_lock_sql(sql).await;
            self.state.record_released(id);
            match released {
                Ok(true) => Ok(()),
                Ok(false) => {
                    warn!(
                        reactor = %id.name,
                        tenant = ?id.tenant,
                        "pg_advisory_unlock returned false — ownership had already been lost; \
                         forgetting it locally"
                    );
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }

        /// Re-assert that Postgres still reports every lock we believe we hold.
        ///
        /// See the module docs for why this exists at all. On
        /// [`OwnershipCheck::Lost`] the caller MUST stop those reactors locally
        /// BEFORE attempting to re-claim — another replica may already be
        /// running them.
        ///
        /// [`OwnershipCheck::Indeterminate`] is NOT a pass. A session that
        /// cannot reach Postgres does not know what it holds; treating that as
        /// healthy is how a partitioned replica keeps running unowned reactors.
        pub async fn verify_owned(&mut self) -> OwnershipCheck {
            if self.state.is_empty() {
                return OwnershipCheck::AllHeld;
            }

            let rows = self
                .conn
                .interact(|conn| {
                    use diesel::RunQueryDsl;
                    diesel::sql_query(SESSION_HELD_LOCKS_SQL).load::<HeldKeyRow>(conn)
                })
                .await;

            let held: HashSet<i64> = match rows {
                Ok(Ok(rows)) => rows.into_iter().map(|r| r.key).collect(),
                Ok(Err(e)) => return OwnershipCheck::Indeterminate(format!("query failed: {e}")),
                Err(e) => return OwnershipCheck::Indeterminate(format!("interact failed: {e}")),
            };

            let lost = self.state.diff_against_held_keys(&held);
            if lost.is_empty() {
                OwnershipCheck::AllHeld
            } else {
                // Forget them immediately: from here on we must not believe we
                // own these, whatever the caller does about stopping them.
                for id in &lost {
                    self.state.record_released(id);
                }
                OwnershipCheck::Lost(lost)
            }
        }

        async fn run_lock_sql(&self, sql: String) -> Result<bool, String> {
            match self
                .conn
                .interact(move |conn| {
                    use diesel::RunQueryDsl;
                    diesel::sql_query(sql).get_result::<AdvisoryLockRow>(conn)
                })
                .await
            {
                Ok(Ok(row)) => Ok(row.locked),
                Ok(Err(e)) => Err(format!("advisory lock query failed: {e}")),
                Err(e) => Err(format!("interact failed: {e}")),
            }
        }
    }
}

#[cfg(feature = "postgres")]
pub use session::OwnershipSession;

#[cfg(test)]
mod tests {
    use super::*;

    fn id(tenant: Option<&str>, name: &str) -> ReactorId {
        ReactorId::new(tenant, name)
    }

    #[test]
    fn believed_ownership_tracks_claim_and_release() {
        let mut state = OwnershipState::new();
        let a = id(Some("t1"), "r1");
        assert!(!state.believes_owned(&a));

        state.record_claimed(a.clone());
        assert!(state.believes_owned(&a));
        assert_eq!(state.len(), 1);

        state.record_released(&a);
        assert!(!state.believes_owned(&a));
        assert!(state.is_empty());
    }

    #[test]
    fn diff_reports_nothing_lost_when_all_keys_still_held() {
        let mut state = OwnershipState::new();
        let a = id(Some("t1"), "r1");
        let b = id(Some("t2"), "r1");
        state.record_claimed(a.clone());
        state.record_claimed(b.clone());

        let held: HashSet<i64> = [a.lock_key(), b.lock_key()].into_iter().collect();
        assert!(state.diff_against_held_keys(&held).is_empty());
    }

    /// The failure this module exists for: the session dropped its locks while
    /// the replica kept believing it owned them.
    #[test]
    fn diff_reports_reactors_whose_locks_vanished() {
        let mut state = OwnershipState::new();
        let a = id(Some("t1"), "r1");
        let b = id(Some("t2"), "r1");
        state.record_claimed(a.clone());
        state.record_claimed(b.clone());

        // Postgres reports only `a` — `b`'s lock is gone.
        let held: HashSet<i64> = [a.lock_key()].into_iter().collect();
        assert_eq!(state.diff_against_held_keys(&held), vec![b]);
    }

    /// A totally dropped session (no locks at all) must report EVERY reactor as
    /// lost, not silently report none.
    #[test]
    fn diff_reports_everything_lost_when_session_holds_nothing() {
        let mut state = OwnershipState::new();
        let a = id(Some("t1"), "r1");
        let b = id(None, "r2");
        state.record_claimed(a.clone());
        state.record_claimed(b.clone());

        let lost = state.diff_against_held_keys(&HashSet::new());
        assert_eq!(
            lost.len(),
            2,
            "a dropped session loses everything: {lost:?}"
        );
    }

    /// Same-named reactors in different tenants are distinct ownership units.
    /// If they collapsed, losing one would look like losing both — or worse,
    /// holding one would mask the loss of the other.
    #[test]
    fn tenants_are_independent_ownership_units() {
        let mut state = OwnershipState::new();
        let t1 = id(Some("t1"), "orders");
        let t2 = id(Some("t2"), "orders");
        state.record_claimed(t1.clone());
        state.record_claimed(t2.clone());
        assert_eq!(state.len(), 2, "same name in two tenants must be two units");

        let held: HashSet<i64> = [t1.lock_key()].into_iter().collect();
        assert_eq!(state.diff_against_held_keys(&held), vec![t2]);
    }

    #[test]
    fn lost_ordering_is_deterministic() {
        let mut state = OwnershipState::new();
        for (t, n) in [
            (Some("t2"), "b"),
            (Some("t1"), "b"),
            (Some("t1"), "a"),
            (None, "z"),
        ] {
            state.record_claimed(id(t, n));
        }
        let lost = state.diff_against_held_keys(&HashSet::new());
        let seen: Vec<(Option<String>, String)> = lost
            .iter()
            .map(|r| (r.tenant.clone(), r.name.clone()))
            .collect();
        let mut expected = seen.clone();
        expected.sort();
        assert_eq!(seen, expected, "lost list must be sorted for stable logs");
    }

    #[test]
    fn watchdog_continues_while_ownership_is_confirmed() {
        let mut w = OwnershipWatchdog::new(3);
        assert_eq!(
            w.observe(OwnershipCheck::AllHeld, &[]),
            WatchdogAction::Continue
        );
        assert_eq!(w.consecutive_indeterminate(), 0);
    }

    #[test]
    fn watchdog_stops_exactly_the_reactors_reported_lost() {
        let mut w = OwnershipWatchdog::new(3);
        let a = id(Some("t1"), "r1");
        let owned = vec![a.clone(), id(Some("t1"), "r2")];
        assert_eq!(
            w.observe(OwnershipCheck::Lost(vec![a.clone()]), &owned),
            WatchdogAction::StopReactors(vec![a]),
            "a definitive loss must stop ONLY the lost reactors, not everything"
        );
    }

    /// Transient failures must not stop healthy reactors — reactive workloads
    /// would flap on any ordinary blip.
    #[test]
    fn watchdog_tolerates_indeterminate_below_the_threshold() {
        let mut w = OwnershipWatchdog::new(3);
        let owned = vec![id(Some("t1"), "r1")];
        for i in 1..3 {
            assert_eq!(
                w.observe(OwnershipCheck::Indeterminate("blip".into()), &owned),
                WatchdogAction::Continue,
                "indeterminate #{i} is below the threshold and must be tolerated"
            );
        }
    }

    /// The safety property: sustained inability to verify is PRESUMED LOSS.
    /// If we cannot reach Postgres, our session is likely already gone — and
    /// Postgres released our locks the moment it dropped, so another replica
    /// may already own these. Failing toward "stop" is recoverable; failing
    /// toward "keep running" is a silent double-processing split-brain.
    #[test]
    fn watchdog_presumes_loss_after_sustained_indeterminacy() {
        let mut w = OwnershipWatchdog::new(3);
        let owned = vec![id(Some("t1"), "r1"), id(None, "r2")];
        w.observe(OwnershipCheck::Indeterminate("1".into()), &owned);
        w.observe(OwnershipCheck::Indeterminate("2".into()), &owned);
        assert_eq!(
            w.observe(OwnershipCheck::Indeterminate("3".into()), &owned),
            WatchdogAction::StopAllPresumedLost(owned.clone()),
            "crossing the threshold must stop everything we believed we owned"
        );
    }

    /// A recovered check clears the streak, so intermittent failures spread
    /// across healthy checks never accumulate into a spurious stop.
    #[test]
    fn watchdog_streak_resets_on_a_successful_check() {
        let mut w = OwnershipWatchdog::new(3);
        let owned = vec![id(Some("t1"), "r1")];
        w.observe(OwnershipCheck::Indeterminate("1".into()), &owned);
        w.observe(OwnershipCheck::Indeterminate("2".into()), &owned);
        assert_eq!(w.consecutive_indeterminate(), 2);

        w.observe(OwnershipCheck::AllHeld, &owned);
        assert_eq!(
            w.consecutive_indeterminate(),
            0,
            "success must clear the streak"
        );

        // Two more must still be tolerated — proving the reset was real.
        assert_eq!(
            w.observe(OwnershipCheck::Indeterminate("3".into()), &owned),
            WatchdogAction::Continue
        );
        assert_eq!(
            w.observe(OwnershipCheck::Indeterminate("4".into()), &owned),
            WatchdogAction::Continue
        );
    }

    /// A definitive Lost also clears the streak: we got an answer, so prior
    /// unverifiable checks say nothing about what happens next.
    #[test]
    fn watchdog_streak_resets_on_a_definitive_loss() {
        let mut w = OwnershipWatchdog::new(2);
        let a = id(Some("t1"), "r1");
        w.observe(OwnershipCheck::Indeterminate("1".into()), &[a.clone()]);
        w.observe(OwnershipCheck::Lost(vec![a.clone()]), &[a.clone()]);
        assert_eq!(w.consecutive_indeterminate(), 0);
    }

    /// A zero threshold would stop reactors on the very first transient error.
    #[test]
    fn watchdog_threshold_is_clamped_to_at_least_one() {
        let mut w = OwnershipWatchdog::new(0);
        let owned = vec![id(Some("t1"), "r1")];
        assert_eq!(
            w.observe(OwnershipCheck::Indeterminate("x".into()), &owned),
            WatchdogAction::StopAllPresumedLost(owned),
            "clamped to 1: still stops, but on a defined threshold rather than \
             a divide-by-zero-ish 0"
        );
    }

    /// The session-scoping predicate is the difference between "we still hold
    /// it" and "somebody holds it". Losing it would make every liveness check
    /// pass while another replica owned the reactor.
    #[test]
    fn held_locks_sql_is_scoped_to_this_session() {
        assert!(
            SESSION_HELD_LOCKS_SQL.contains("pid = pg_backend_pid()"),
            "liveness check MUST be scoped to this session, else another \
             replica's lock reads as our own"
        );
        assert!(SESSION_HELD_LOCKS_SQL.contains("granted"));
        assert!(SESSION_HELD_LOCKS_SQL.contains("objsubid = 1"));
    }
}
