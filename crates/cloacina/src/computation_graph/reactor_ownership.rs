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
