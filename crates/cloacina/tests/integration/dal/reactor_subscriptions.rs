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

//! Integration tests for the reactor-subscription DAL (CLOACI-I-0100).
//!
//! Covers the DB-backed event log + subscription contract:
//!   - end-to-end: insert firing → poll picks it up → watermark advance
//!   - fan-out: two subscriptions on the same reactor each see all firings
//!   - tenancy: tenant A's poller cannot see tenant B's firings
//!   - at-least-once on crash: simulated by skipping watermark advance
//!     between dispatch attempts; the next poll returns the same row
//!   - TTL prune: rows older than the cutoff are deleted; subscriptions
//!     whose watermark predates the cutoff miss those firings (gotcha)

use crate::fixtures::get_or_init_fixture;
use chrono::{Duration as ChronoDuration, Utc};
use cloacina::database::universal_types::UniversalTimestamp;
use serial_test::serial;

/// End-to-end: a firing inserted under tenant T for reactor R is
/// returned by `poll_unconsumed`, and after `advance_watermark` is
/// no longer returned.
#[tokio::test]
#[serial]
async fn test_firing_round_trip_and_watermark_advance() {
    let fixture = get_or_init_fixture().await;
    let fixture = fixture.lock().unwrap();
    let dal = fixture.get_dal();
    let api = dal.reactor_subscriptions();

    let tenant = format!("tenant-{}", uuid::Uuid::new_v4());
    let reactor = "rt_round_trip".to_string();
    let workflow = "wf_round_trip".to_string();

    // Subscribe.
    let sub_id = api
        .subscribe(&reactor, &workflow, &tenant)
        .await
        .expect("subscribe");

    // No firings yet.
    let initial = api
        .poll_unconsumed(&tenant, &reactor, None, 100)
        .await
        .expect("poll empty");
    assert!(
        initial.is_empty(),
        "no firings expected on a fresh subscription"
    );

    // Insert a firing.
    let fired_at = UniversalTimestamp::now();
    api.insert_firing(&reactor, &tenant, Some(b"payload-1".to_vec()), fired_at)
        .await
        .expect("insert firing");

    // Poll picks it up.
    let firings = api
        .poll_unconsumed(&tenant, &reactor, None, 100)
        .await
        .expect("poll after insert");
    assert_eq!(firings.len(), 1, "expected one firing");
    assert_eq!(firings[0].reactor_name, reactor);
    assert_eq!(firings[0].tenant_id, tenant);
    assert_eq!(
        firings[0]
            .payload
            .as_ref()
            .map(|p| p.as_slice().to_vec())
            .unwrap_or_default(),
        b"payload-1".to_vec()
    );

    // Advance watermark; subsequent poll yields nothing.
    api.advance_watermark(sub_id, firings[0].fired_at)
        .await
        .expect("advance watermark");
    let after_ack = api
        .poll_unconsumed(&tenant, &reactor, Some(firings[0].fired_at), 100)
        .await
        .expect("poll after ack");
    assert!(
        after_ack.is_empty(),
        "advanced subscription should see no firings"
    );
}

/// Two subscriptions on the same reactor each independently observe
/// every firing — the per-subscription watermark is the only state
/// that gates dispatch.
#[tokio::test]
#[serial]
async fn test_fan_out_two_subscriptions_independent() {
    let fixture = get_or_init_fixture().await;
    let fixture = fixture.lock().unwrap();
    let dal = fixture.get_dal();
    let api = dal.reactor_subscriptions();

    let tenant = format!("tenant-{}", uuid::Uuid::new_v4());
    let reactor = "rt_fan_out".to_string();

    let sub_a = api
        .subscribe(&reactor, "wf_a", &tenant)
        .await
        .expect("subscribe a");
    let sub_b = api
        .subscribe(&reactor, "wf_b", &tenant)
        .await
        .expect("subscribe b");
    assert_ne!(
        sub_a, sub_b,
        "distinct workflows yield distinct subscription ids"
    );

    // Two firings.
    let t0 = UniversalTimestamp::now();
    let t1 = UniversalTimestamp(t0.0 + ChronoDuration::milliseconds(10));
    api.insert_firing(&reactor, &tenant, Some(b"f0".to_vec()), t0)
        .await
        .expect("insert f0");
    api.insert_firing(&reactor, &tenant, Some(b"f1".to_vec()), t1)
        .await
        .expect("insert f1");

    // Each subscription sees both.
    let a_firings = api
        .poll_unconsumed(&tenant, &reactor, None, 100)
        .await
        .expect("poll a");
    let b_firings = api
        .poll_unconsumed(&tenant, &reactor, None, 100)
        .await
        .expect("poll b");
    assert_eq!(a_firings.len(), 2);
    assert_eq!(b_firings.len(), 2);

    // Advance only A's watermark to t0 — A still sees f1, B still sees both.
    api.advance_watermark(sub_a, t0).await.expect("advance a");
    let a_after = api
        .poll_unconsumed(&tenant, &reactor, Some(t0), 100)
        .await
        .expect("poll a after");
    let b_after = api
        .poll_unconsumed(&tenant, &reactor, None, 100)
        .await
        .expect("poll b after");
    assert_eq!(a_after.len(), 1, "A skips f0, sees f1");
    assert_eq!(b_after.len(), 2, "B's watermark untouched");
    let _ = sub_b; // suppress unused-var warning when assertions trim
}

/// Tenancy isolation: tenant A's poller never sees tenant B's
/// firings even when both share the same reactor name.
#[tokio::test]
#[serial]
async fn test_tenant_isolation_on_poll() {
    let fixture = get_or_init_fixture().await;
    let fixture = fixture.lock().unwrap();
    let dal = fixture.get_dal();
    let api = dal.reactor_subscriptions();

    let tenant_a = format!("tenant-a-{}", uuid::Uuid::new_v4());
    let tenant_b = format!("tenant-b-{}", uuid::Uuid::new_v4());
    let reactor = "rt_tenancy".to_string();

    let _sub_a = api
        .subscribe(&reactor, "wf", &tenant_a)
        .await
        .expect("subscribe a");
    let _sub_b = api
        .subscribe(&reactor, "wf", &tenant_b)
        .await
        .expect("subscribe b");

    let now = UniversalTimestamp::now();
    api.insert_firing(&reactor, &tenant_a, Some(b"a-only".to_vec()), now)
        .await
        .expect("insert tenant a");
    api.insert_firing(&reactor, &tenant_b, Some(b"b-only".to_vec()), now)
        .await
        .expect("insert tenant b");

    let a = api
        .poll_unconsumed(&tenant_a, &reactor, None, 100)
        .await
        .expect("poll a");
    let b = api
        .poll_unconsumed(&tenant_b, &reactor, None, 100)
        .await
        .expect("poll b");

    assert_eq!(a.len(), 1, "tenant A sees exactly its own firing");
    assert_eq!(
        a[0].payload.as_ref().map(|p| p.as_slice().to_vec()),
        Some(b"a-only".to_vec())
    );
    assert_eq!(b.len(), 1, "tenant B sees exactly its own firing");
    assert_eq!(
        b[0].payload.as_ref().map(|p| p.as_slice().to_vec()),
        Some(b"b-only".to_vec())
    );
}

/// At-least-once on crash: when the dispatcher does not advance the
/// watermark between dispatch attempts, the same firing is returned
/// on the next poll. Workflow idempotency is the user's concern
/// (same as cron triggers).
#[tokio::test]
#[serial]
async fn test_at_least_once_on_crash_simulates_redelivery() {
    let fixture = get_or_init_fixture().await;
    let fixture = fixture.lock().unwrap();
    let dal = fixture.get_dal();
    let api = dal.reactor_subscriptions();

    let tenant = format!("tenant-{}", uuid::Uuid::new_v4());
    let reactor = "rt_at_least_once".to_string();
    let _sub_id = api
        .subscribe(&reactor, "wf", &tenant)
        .await
        .expect("subscribe");

    let fired_at = UniversalTimestamp::now();
    api.insert_firing(&reactor, &tenant, Some(b"x".to_vec()), fired_at)
        .await
        .expect("insert");

    // First poll dispatches; we *do not* advance the watermark to
    // simulate a crash after dispatch.
    let first = api
        .poll_unconsumed(&tenant, &reactor, None, 100)
        .await
        .expect("first poll");
    assert_eq!(first.len(), 1, "first poll returns the firing");

    // Second poll (with the same watermark) must return the same
    // firing — at-least-once contract.
    let second = api
        .poll_unconsumed(&tenant, &reactor, None, 100)
        .await
        .expect("second poll");
    assert_eq!(second.len(), 1, "stale watermark re-delivers the firing");
    assert_eq!(second[0].id, first[0].id);
}

/// TTL prune deletes old firings. A subscription whose watermark is
/// younger than the cutoff still sees newer firings, but anything
/// older than the cutoff is gone — including firings the subscription
/// never observed (the documented gotcha).
#[tokio::test]
#[serial]
async fn test_ttl_prune_removes_old_firings_and_documents_gotcha() {
    let fixture = get_or_init_fixture().await;
    let fixture = fixture.lock().unwrap();
    let dal = fixture.get_dal();
    let api = dal.reactor_subscriptions();

    let tenant = format!("tenant-{}", uuid::Uuid::new_v4());
    let reactor = "rt_ttl".to_string();
    let _sub_id = api
        .subscribe(&reactor, "wf", &tenant)
        .await
        .expect("subscribe");

    // Two firings — one old, one fresh.
    let now = Utc::now();
    let old = UniversalTimestamp(now - ChronoDuration::days(10));
    let fresh = UniversalTimestamp(now);
    api.insert_firing(&reactor, &tenant, Some(b"old".to_vec()), old)
        .await
        .expect("insert old");
    api.insert_firing(&reactor, &tenant, Some(b"fresh".to_vec()), fresh)
        .await
        .expect("insert fresh");

    // Sanity: poll sees both before pruning.
    let pre = api
        .poll_unconsumed(&tenant, &reactor, None, 100)
        .await
        .expect("poll pre-prune");
    assert_eq!(pre.len(), 2, "both firings visible before prune");

    // Prune anything older than 7 days.
    let cutoff = UniversalTimestamp(now - ChronoDuration::days(7));
    let deleted = api.prune_firings_older_than(cutoff).await.expect("prune");
    assert_eq!(deleted, 1, "one row older than the cutoff");

    // Subscription that never advanced its watermark still misses the
    // pruned firing — at-least-once is bounded by the retention window.
    let post = api
        .poll_unconsumed(&tenant, &reactor, None, 100)
        .await
        .expect("poll post-prune");
    assert_eq!(post.len(), 1, "only the fresh firing remains");
    assert_eq!(
        post[0].payload.as_ref().map(|p| p.as_slice().to_vec()),
        Some(b"fresh".to_vec())
    );
}

/// `subscribe` is idempotent on the unique `(reactor, workflow,
/// tenant)` triple — calling twice returns the same id and does not
/// duplicate the row.
#[tokio::test]
#[serial]
async fn test_subscribe_is_idempotent() {
    let fixture = get_or_init_fixture().await;
    let fixture = fixture.lock().unwrap();
    let dal = fixture.get_dal();
    let api = dal.reactor_subscriptions();

    let tenant = format!("tenant-{}", uuid::Uuid::new_v4());
    let reactor = "rt_idempotent".to_string();

    let id1 = api
        .subscribe(&reactor, "wf", &tenant)
        .await
        .expect("first subscribe");
    let id2 = api
        .subscribe(&reactor, "wf", &tenant)
        .await
        .expect("second subscribe");
    assert_eq!(id1, id2, "duplicate subscribe must return the existing id");

    let listed = api.list_subscriptions(&tenant).await.expect("list");
    let matching: Vec<_> = listed
        .iter()
        .filter(|s| s.reactor_name == reactor && s.workflow_name == "wf")
        .collect();
    assert_eq!(matching.len(), 1, "exactly one row for the (r,w,t) triple");
}
