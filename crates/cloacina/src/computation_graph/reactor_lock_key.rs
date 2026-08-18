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

//! Advisory-lock key derivation for per-reactor leadership (CLOACI-T-0851,
//! [`ADR CLOACI-A-0012`]).
//!
//! # Why this needs its own module and its own tests
//!
//! Reactor ownership is claimed with a Postgres **session-level advisory
//! lock**, mirroring the fleet control loop
//! (`cloacina-server::autoscaler::leader`). Advisory locks live in a single
//! **database-wide** key space: they are NOT scoped by schema.
//!
//! Cloacina isolates tenants by **schema within one database**
//! (`SET LOCAL search_path TO <tenant>`; see `database::admin`). Every DAL
//! table is therefore tenant-separated for free — but advisory locks are not.
//! Deriving a reactor's lock key from the graph name alone would mean two
//! tenants running a same-named graph contend for one lock, and exactly one of
//! them would silently never run its reactor. That is a cross-tenant
//! correctness bug that no single-tenant test could ever surface, which is why
//! the key derivation is isolated here with tests rather than inlined at the
//! lock call site.
//!
//! Note that `save_reactor_state` keys checkpoints by graph name alone and is
//! safe precisely BECAUSE the DAL is schema-scoped. That asymmetry is the trap:
//! the neighbouring persistence code looks like a precedent for
//! name-only keying, and it is not one for locks.
//!
//! # Why not `DefaultHasher`
//!
//! `std::collections::hash_map::DefaultHasher` (and anything built on
//! `RandomState`) is seeded randomly **per process**. Two replicas hashing the
//! same `(tenant, reactor)` would compute DIFFERENT keys, so both would acquire
//! "the lock" and both would run the reactor — the precise split-brain this
//! mechanism exists to prevent, presenting as an intermittent duplicate rather
//! than an error. The hash here is therefore a hand-rolled FNV-1a over a
//! delimited encoding: fixed constants, no seed, stable across processes,
//! releases, and architectures.

/// Reserved high bit pattern distinguishing reactor-ownership keys from other
/// subsystems' advisory locks in the same database-wide key space.
///
/// `cloacina-server::autoscaler::leader::FLEET_CONTROL_LOCK_KEY` is the small
/// positive integer `8_110_127`. Reactor keys are forced negative so they can
/// never collide with that key, nor with any other small-integer key a future
/// subsystem picks by hand.
const REACTOR_LOCK_KEY_SIGN_BIT: u64 = 1 << 63;

/// FNV-1a 64-bit offset basis and prime. Fixed constants — see the module note
/// on why a seeded hasher would be a correctness bug.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Derive the advisory-lock key identifying ownership of `reactor_name` within
/// `tenant`.
///
/// `tenant` is the tenant scope as the scheduler keys it — `None` for
/// single-tenant / admin-owned reactors, which is a distinct namespace from any
/// named tenant (see the `none_tenant_is_distinct_from_named_tenant` test).
///
/// The encoding is length-delimited rather than concatenated so that
/// `(tenant="a", reactor="bc")` and `(tenant="ab", reactor="c")` cannot collide.
/// Plain concatenation with a separator is not enough either, since a tenant or
/// reactor name may itself contain the separator.
pub fn reactor_lock_key(tenant: Option<&str>, reactor_name: &str) -> i64 {
    let mut buf = Vec::with_capacity(reactor_name.len() + 32);

    // Domain tag: keeps this hash space distinct from any other FNV use.
    buf.extend_from_slice(b"cloacina.reactor.ownership.v1\0");

    match tenant {
        // A named tenant is encoded with its length; `None` uses a marker that
        // no length-prefixed string can produce.
        Some(t) => {
            buf.push(1);
            buf.extend_from_slice(&(t.len() as u64).to_be_bytes());
            buf.extend_from_slice(t.as_bytes());
        }
        None => buf.push(0),
    }

    buf.extend_from_slice(&(reactor_name.len() as u64).to_be_bytes());
    buf.extend_from_slice(reactor_name.as_bytes());

    // Force the sign bit so reactor keys occupy the negative half of the i64
    // space, disjoint from hand-picked small positive keys like
    // FLEET_CONTROL_LOCK_KEY.
    (fnv1a(&buf) | REACTOR_LOCK_KEY_SIGN_BIT) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of the module: two tenants running a same-named reactor
    /// must not contend for one lock.
    #[test]
    fn same_reactor_in_different_tenants_gets_different_keys() {
        let a = reactor_lock_key(Some("tenant_a"), "orders_reactor");
        let b = reactor_lock_key(Some("tenant_b"), "orders_reactor");
        assert_ne!(
            a, b,
            "same-named reactors in different tenants must not share a lock key"
        );
    }

    #[test]
    fn different_reactors_in_same_tenant_get_different_keys() {
        let a = reactor_lock_key(Some("tenant_a"), "orders_reactor");
        let b = reactor_lock_key(Some("tenant_a"), "billing_reactor");
        assert_ne!(a, b);
    }

    /// Stability is a correctness requirement, not a nicety: replicas are
    /// separate processes, so a per-process seed would give each its own key and
    /// every replica would win "the lock" simultaneously. These literals are
    /// pinned so a change to the hash constants or the encoding fails loudly
    /// rather than silently splitting brains in production.
    #[test]
    fn keys_are_stable_across_processes() {
        assert_eq!(
            reactor_lock_key(Some("public"), "orders_reactor"),
            reactor_lock_key(Some("public"), "orders_reactor"),
        );
        // Pinned expectations — update ONLY with a deliberate migration plan,
        // since changing them means old and new replicas disagree about
        // ownership during a rolling deploy.
        assert_eq!(
            reactor_lock_key(Some("public"), "orders_reactor"),
            -798_654_939_832_276_275,
        );
        assert_eq!(
            reactor_lock_key(None, "orders_reactor"),
            -6_219_432_407_812_253_675,
        );
    }

    /// `None` (single-tenant / admin-owned) is its own namespace. If it
    /// collided with a tenant literally named e.g. "" or "public", an
    /// admin-owned reactor and a tenant reactor would fight over one lock.
    #[test]
    fn none_tenant_is_distinct_from_named_tenant() {
        let none_key = reactor_lock_key(None, "r");
        assert_ne!(none_key, reactor_lock_key(Some(""), "r"));
        assert_ne!(none_key, reactor_lock_key(Some("public"), "r"));
    }

    /// Length-delimiting matters: without it, ("a","bc") and ("ab","c") hash
    /// the same bytes and two unrelated reactors silently share ownership.
    #[test]
    fn field_boundaries_cannot_be_forged() {
        assert_ne!(
            reactor_lock_key(Some("a"), "bc"),
            reactor_lock_key(Some("ab"), "c"),
        );
        // A name containing the kind of separator a naive encoding would use.
        assert_ne!(
            reactor_lock_key(Some("a:b"), "c"),
            reactor_lock_key(Some("a"), "b:c"),
        );
    }

    /// Reactor keys must never collide with a hand-picked subsystem key.
    /// FLEET_CONTROL_LOCK_KEY (8_110_127) is small and positive; every reactor
    /// key is negative by construction.
    #[test]
    fn reactor_keys_never_collide_with_handpicked_subsystem_keys() {
        const FLEET_CONTROL_LOCK_KEY: i64 = 8_110_127;
        for name in ["a", "orders_reactor", "", "x".repeat(500).as_str()] {
            for tenant in [None, Some("public"), Some("tenant_a")] {
                let key = reactor_lock_key(tenant, name);
                assert!(
                    key < 0,
                    "reactor key must be negative to stay disjoint from small positive subsystem keys, got {key}"
                );
                assert_ne!(key, FLEET_CONTROL_LOCK_KEY);
            }
        }
    }

    /// Cheap spread check over a realistic population. Not a proof of
    /// collision-freedom — 64-bit birthday bounds make collisions negligible at
    /// this scale — but it catches an encoding bug that funnels many inputs to
    /// one key.
    #[test]
    fn no_collisions_across_a_realistic_population() {
        let mut keys = std::collections::HashSet::new();
        for t in 0..200 {
            for r in 0..50 {
                let tenant = format!("tenant_{t}");
                let reactor = format!("reactor_{r}");
                assert!(
                    keys.insert(reactor_lock_key(Some(&tenant), &reactor)),
                    "collision at tenant_{t}/reactor_{r}"
                );
            }
        }
        assert_eq!(keys.len(), 200 * 50);
    }
}
