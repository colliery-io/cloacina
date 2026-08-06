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

//! Unified login-throttle DAL — brute-force defense for local login
//! (CLOACI-T-0923, closes I-0118 OQ-13 "required before production").
//!
//! Argon2id makes each guess expensive for the **server**, not for the
//! attacker. Nothing previously bounded sustained guessing against a known
//! username, so `/v1/auth/local/login` was an open credential-stuffing target.
//!
//! **Persisted, not per-replica.** A counter in one replica's memory is evaded
//! by spraying attempts across replicas behind a load balancer, and a lockout
//! decided on replica A is invisible to replica B. Same lesson (and same DAL
//! shape) as the T-0916 `ws_tickets` move.
//!
//! # Why dual-keyed
//!
//! Every failed attempt increments **two** independent counters, and a login is
//! refused if **either** is locked:
//!
//! | key | catches | threshold |
//! |-----|---------|-----------|
//! | `u:<tenant-or-_>/<username>` | an attacker rotating source IPs against one account | low (5) |
//! | `ip:<source>` | one host spraying many usernames, none of which alone trips | high (50) |
//!
//! Neither scope alone is sufficient: username-only lets a single host walk an
//! entire user list; IP-only is defeated by a botnet or an open proxy pool. The
//! obvious third option — a composite `(username, ip)` key — is strictly the
//! worst, because it resets on every new source IP and therefore stops neither
//! attack. The IP threshold is deliberately an order of magnitude higher than
//! the username one so a shared-NAT office does not lock itself out over a
//! handful of typos.
//!
//! # Not an enumeration oracle
//!
//! The caller records a failure for an **unknown** username exactly as for a
//! real one, so a locked key's `429` says only "this key has been failing",
//! never "this account exists". See `routes/local_auth.rs`.
//!
//! # Decay and reset
//!
//! A counter whose last failure is older than [`ThrottlePolicy::decay`] starts
//! over. A successful login clears the *username* key outright (a legitimate
//! user who finally remembers their password is not left in a hole). The IP key
//! is deliberately **not** cleared on success — otherwise an attacker with one
//! valid account of their own could zero the spray counter at will — and is
//! left to decay instead.

use super::models::{NewUnifiedLoginThrottle, UnifiedLoginThrottle};
use super::DAL;
use crate::database::schema::unified::login_throttle;
use crate::database::universal_types::UniversalTimestamp;
use crate::error::ValidationError;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use diesel::prelude::*;

/// Scope label stored on a row (also the metric/audit label).
pub const SCOPE_USERNAME: &str = "username";
/// Scope label for source-IP rows.
pub const SCOPE_IP: &str = "ip";

/// Build the username-scoped throttle key. `tenant` is the login request's
/// tenant selector; `None` (a global account) collapses to `_` so the two
/// namespaces never collide.
pub fn username_key(tenant: Option<&str>, username: &str) -> String {
    format!("u:{}/{}", tenant.unwrap_or("_"), username)
}

/// Build the source-IP-scoped throttle key.
pub fn ip_key(ip: &str) -> String {
    format!("ip:{ip}")
}

/// Tunables for one throttle scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThrottlePolicy {
    /// Consecutive failures tolerated before the key locks.
    pub threshold: i32,
    /// Lock duration for the first failure past the threshold. Each further
    /// failure doubles it, up to `max_lock`.
    pub base_lock: ChronoDuration,
    /// Ceiling on the exponential backoff.
    pub max_lock: ChronoDuration,
    /// Idle period after which a counter is considered stale and starts over.
    pub decay: ChronoDuration,
}

impl ThrottlePolicy {
    /// Default username-scoped policy: 5 strikes, then 30s doubling to a 15min
    /// cap, counters forgotten after 15 idle minutes.
    pub fn username_default() -> Self {
        Self {
            threshold: 5,
            base_lock: ChronoDuration::seconds(30),
            max_lock: ChronoDuration::minutes(15),
            decay: ChronoDuration::minutes(15),
        }
    }

    /// Default source-IP policy: an order of magnitude looser than the username
    /// policy so shared NAT does not self-inflict a lockout, but still bounds a
    /// single host spraying a user list.
    pub fn ip_default() -> Self {
        Self {
            threshold: 50,
            base_lock: ChronoDuration::minutes(15),
            max_lock: ChronoDuration::minutes(15),
            decay: ChronoDuration::minutes(15),
        }
    }

    /// Lock window for the `n`-th consecutive failure (1-based). Failures at or
    /// below `threshold` do not lock.
    fn lock_for(&self, failure_count: i32) -> Option<ChronoDuration> {
        let over = failure_count - self.threshold;
        if over <= 0 {
            return None;
        }
        // 2^(over-1), saturating well before ChronoDuration overflows.
        let shift = (over - 1).clamp(0, 20) as u32;
        let scaled = self
            .base_lock
            .checked_mul(1i32 << shift)
            .unwrap_or(self.max_lock);
        Some(if scaled > self.max_lock {
            self.max_lock
        } else {
            scaled
        })
    }
}

/// The post-write state of a throttle key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThrottleState {
    pub scope: String,
    /// Consecutive failures after this write.
    pub failure_count: i32,
    /// Set when the key is locked; the wall past which attempts are accepted
    /// again.
    pub locked_until: Option<DateTime<Utc>>,
    /// True when *this* write is what pushed the key into a lock (the edge the
    /// caller audits + counts, so a lockout is logged once, not once per
    /// subsequent attempt).
    pub newly_locked: bool,
}

impl ThrottleState {
    /// Whether the key is locked as of `now`.
    pub fn is_locked_at(&self, now: DateTime<Utc>) -> bool {
        self.locked_until.map(|t| t > now).unwrap_or(false)
    }
}

/// Data access layer for failed-login counters.
#[derive(Clone)]
pub struct LoginThrottleDAL<'a> {
    dal: &'a DAL,
}

impl<'a> LoginThrottleDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Is `key` currently locked? Returns the lock expiry when it is.
    ///
    /// Read-only and cheap — this is the pre-password-verify gate, so it must
    /// not itself become the expensive part of a login attempt.
    pub async fn locked_until(&self, key: &str) -> Result<Option<DateTime<Utc>>, ValidationError> {
        let key = key.to_string();
        let now = UniversalTimestamp::now();
        let row: Option<UnifiedLoginThrottle> = crate::interact_on_backend!(self.dal, |conn| {
            login_throttle::table
                .filter(login_throttle::throttle_key.eq(&key))
                .first::<UnifiedLoginThrottle>(conn)
                .optional()
        })
        .map_err(ValidationError::from)?;
        Ok(row
            .and_then(|r| r.locked_until)
            .filter(|t| t.0 > now.0)
            .map(|t| t.0))
    }

    /// Record one failed attempt against `key` and return the resulting state.
    ///
    /// Read-modify-write inside a single transaction so two concurrent
    /// attempts (on any replica) cannot both read the same count and write the
    /// same increment. A counter idle longer than `policy.decay` restarts at 1.
    pub async fn record_failure(
        &self,
        key: &str,
        scope: &str,
        policy: ThrottlePolicy,
    ) -> Result<ThrottleState, ValidationError> {
        let key = key.to_string();
        let scope = scope.to_string();
        let now = UniversalTimestamp::now();

        let state: ThrottleState = crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<ThrottleState, diesel::result::Error, _>(|conn| {
                let existing: Option<UnifiedLoginThrottle> = login_throttle::table
                    .filter(login_throttle::throttle_key.eq(&key))
                    .first::<UnifiedLoginThrottle>(conn)
                    .optional()?;

                // Was the key already locked before this write? Used to report
                // the lock *edge* exactly once.
                let was_locked = existing
                    .as_ref()
                    .and_then(|r| r.locked_until)
                    .map(|t| t.0 > now.0)
                    .unwrap_or(false);

                let (failure_count, first_failure_at) = match &existing {
                    // Stale counter — the attacker (or the forgetful human)
                    // went quiet long enough; start over.
                    Some(r) if now.0 - r.last_failure_at.0 > policy.decay => (1, now),
                    Some(r) => (r.failure_count.saturating_add(1), r.first_failure_at),
                    None => (1, now),
                };

                let locked_until = policy
                    .lock_for(failure_count)
                    .map(|d| UniversalTimestamp(now.0 + d));

                let row = NewUnifiedLoginThrottle {
                    throttle_key: key.clone(),
                    scope: scope.clone(),
                    failure_count,
                    first_failure_at,
                    last_failure_at: now,
                    locked_until,
                };

                if existing.is_some() {
                    diesel::update(
                        login_throttle::table.filter(login_throttle::throttle_key.eq(&key)),
                    )
                    .set(&row)
                    .execute(conn)?;
                } else {
                    diesel::insert_into(login_throttle::table)
                        .values(&row)
                        .execute(conn)?;
                }

                Ok(ThrottleState {
                    scope: scope.clone(),
                    failure_count,
                    locked_until: locked_until.map(|t| t.0),
                    newly_locked: locked_until.is_some() && !was_locked,
                })
            })
        })
        .map_err(ValidationError::from)?;

        Ok(state)
    }

    /// Forget `key` entirely — called on a successful login so a legitimate
    /// user's counter never carries over. Returns true if a row was removed.
    pub async fn clear(&self, key: &str) -> Result<bool, ValidationError> {
        let key = key.to_string();
        let n: usize = crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(login_throttle::table.filter(login_throttle::throttle_key.eq(&key)))
                .execute(conn)
        })
        .map_err(ValidationError::from)?;
        Ok(n > 0)
    }

    /// Drop counters idle for longer than `older_than` **and** not currently
    /// locked. Keeps the table bounded without a dedicated sweeper; the login
    /// route calls it opportunistically.
    pub async fn prune_idle(&self, older_than: ChronoDuration) -> Result<usize, ValidationError> {
        let now = UniversalTimestamp::now();
        let cutoff = UniversalTimestamp(now.0 - older_than);
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                login_throttle::table
                    .filter(login_throttle::last_failure_at.lt(cutoff))
                    .filter(
                        login_throttle::locked_until
                            .is_null()
                            .or(login_throttle::locked_until.lt(now)),
                    ),
            )
            .execute(conn)
        })
        .map_err(ValidationError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    #[test]
    fn keys_are_scoped_and_do_not_collide() {
        assert_eq!(username_key(Some("acme"), "bob"), "u:acme/bob");
        assert_eq!(username_key(None, "bob"), "u:_/bob");
        assert_ne!(username_key(Some("acme"), "bob"), ip_key("acme/bob"));
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        let p = ThrottlePolicy::username_default();
        assert_eq!(p.lock_for(1), None, "under threshold must not lock");
        assert_eq!(p.lock_for(5), None, "at threshold must not lock");
        assert_eq!(p.lock_for(6), Some(ChronoDuration::seconds(30)));
        assert_eq!(p.lock_for(7), Some(ChronoDuration::seconds(60)));
        assert_eq!(p.lock_for(8), Some(ChronoDuration::seconds(120)));
        // Capped, and never overflows however far the attacker pushes.
        assert_eq!(p.lock_for(100), Some(ChronoDuration::minutes(15)));
        assert_eq!(p.lock_for(i32::MAX), Some(ChronoDuration::minutes(15)));
    }

    #[test]
    fn ip_policy_is_far_looser_than_username_policy() {
        // A shared-NAT office must not lock out because five people fat-finger
        // their passwords.
        assert!(
            ThrottlePolicy::ip_default().threshold
                > ThrottlePolicy::username_default().threshold * 5
        );
    }

    #[cfg(feature = "sqlite")]
    fn shared_url() -> String {
        format!(
            "file:login_throttle_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        )
    }

    #[cfg(feature = "sqlite")]
    async fn dal_for(url: &str) -> DAL {
        let db = Database::new(url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    #[cfg(feature = "sqlite")]
    fn fast_policy() -> ThrottlePolicy {
        ThrottlePolicy {
            threshold: 3,
            base_lock: ChronoDuration::milliseconds(150),
            max_lock: ChronoDuration::milliseconds(150),
            decay: ChronoDuration::minutes(15),
        }
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn locks_after_threshold_then_expires() {
        let dal = dal_for(&shared_url()).await;
        let t = dal.login_throttle();
        let key = username_key(Some("acme"), "bob");
        let p = fast_policy();

        for _ in 0..3 {
            let s = t.record_failure(&key, SCOPE_USERNAME, p).await.unwrap();
            assert!(s.locked_until.is_none(), "must not lock at/below threshold");
        }
        assert!(t.locked_until(&key).await.unwrap().is_none());

        let s = t.record_failure(&key, SCOPE_USERNAME, p).await.unwrap();
        assert_eq!(s.failure_count, 4);
        assert!(s.newly_locked, "the crossing attempt reports the edge");
        assert!(t.locked_until(&key).await.unwrap().is_some());

        // A further failure while locked is not a *new* lockout (so the audit
        // event / metric fires once per lock, not once per attempt).
        let s = t.record_failure(&key, SCOPE_USERNAME, p).await.unwrap();
        assert!(!s.newly_locked);

        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        assert!(
            t.locked_until(&key).await.unwrap().is_none(),
            "lock must expire on its own"
        );
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn success_clears_the_counter() {
        let dal = dal_for(&shared_url()).await;
        let t = dal.login_throttle();
        let key = username_key(None, "carol");
        let p = fast_policy();

        t.record_failure(&key, SCOPE_USERNAME, p).await.unwrap();
        t.record_failure(&key, SCOPE_USERNAME, p).await.unwrap();
        assert!(t.clear(&key).await.unwrap(), "row existed");

        // Next failure starts from scratch, so the user is not one strike from
        // a lockout after finally logging in.
        let s = t.record_failure(&key, SCOPE_USERNAME, p).await.unwrap();
        assert_eq!(s.failure_count, 1);
    }

    /// The multi-replica contract (mirrors the T-0916 ws-ticket test): failures
    /// recorded through one DAL handle are seen — and the lock enforced — by a
    /// DIFFERENT handle over the same database.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn throttle_state_is_shared_across_handles() {
        let url = shared_url();
        let dal_a = dal_for(&url).await;
        let dal_b = DAL::new(Database::new(&url, "", 5));
        let key = username_key(Some("acme"), "dave");
        let p = ThrottlePolicy {
            threshold: 3,
            base_lock: ChronoDuration::seconds(60),
            max_lock: ChronoDuration::seconds(60),
            decay: ChronoDuration::minutes(15),
        };

        // Replica A takes two strikes, replica B takes the next two.
        dal_a
            .login_throttle()
            .record_failure(&key, SCOPE_USERNAME, p)
            .await
            .unwrap();
        dal_a
            .login_throttle()
            .record_failure(&key, SCOPE_USERNAME, p)
            .await
            .unwrap();
        let s = dal_b
            .login_throttle()
            .record_failure(&key, SCOPE_USERNAME, p)
            .await
            .unwrap();
        assert_eq!(s.failure_count, 3, "B must continue A's count, not restart");
        let s = dal_b
            .login_throttle()
            .record_failure(&key, SCOPE_USERNAME, p)
            .await
            .unwrap();
        assert!(s.newly_locked);

        // ...and the lock decided on B is enforced on A.
        assert!(dal_a
            .login_throttle()
            .locked_until(&key)
            .await
            .unwrap()
            .is_some());

        // Clearing on A releases B too.
        dal_a.login_throttle().clear(&key).await.unwrap();
        assert!(dal_b
            .login_throttle()
            .locked_until(&key)
            .await
            .unwrap()
            .is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn stale_counters_decay_and_prune() {
        let dal = dal_for(&shared_url()).await;
        let t = dal.login_throttle();
        let key = username_key(None, "erin");
        let p = ThrottlePolicy {
            threshold: 3,
            base_lock: ChronoDuration::milliseconds(50),
            max_lock: ChronoDuration::milliseconds(50),
            decay: ChronoDuration::milliseconds(100),
        };

        t.record_failure(&key, SCOPE_USERNAME, p).await.unwrap();
        t.record_failure(&key, SCOPE_USERNAME, p).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let s = t.record_failure(&key, SCOPE_USERNAME, p).await.unwrap();
        assert_eq!(s.failure_count, 1, "idle counter restarts");

        // Idle + unlocked rows are pruned; a live lock is not.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert_eq!(
            t.prune_idle(ChronoDuration::milliseconds(100))
                .await
                .unwrap(),
            1
        );
    }
}
