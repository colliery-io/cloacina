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

//! The reactive state of a tenant's graphs (CLOACINA-T-1080, notes n-9 and
//! n-11). The server's one graph scheduler gives each reactor and accumulator
//! the DAL of `state_dal_for(tenant)`; the reactor writes its firings and its
//! state through it, and the tenant's runner reads firings through its own
//! pool. The scheduler here is built as `run` builds it.

use super::*;

use cloacina::computation_graph::scheduler::ComputationGraphScheduler;
use cloacina::database::{DatabaseAdmin, TenantConfig};
use diesel::prelude::*;

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

/// The scheduler of a server: the admin DAL, and each tenant's own DAL.
fn server_scheduler(
    admin_db: &cloacina::Database,
    cache: &Arc<TenantDatabaseCache>,
) -> ComputationGraphScheduler {
    ComputationGraphScheduler::with_dal(
        EndpointRegistry::requiring_tenants(),
        cloacina::dal::unified::DAL::new(admin_db.clone()),
    )
    .with_tenant_dals(Arc::new(ServerTenantDals::new(
        cache.clone(),
        admin_db.clone(),
    )))
}

/// A firing of a named tenant's reactor reaches the subscription of that
/// tenant's runner.
#[tokio::test]
#[serial]
async fn a_named_tenants_reactor_firing_reaches_its_runners_subscription() {
    let admin_db = cloacina::Database::new(TEST_DB_URL, "cloacina", 2);
    admin_db.run_migrations().await.expect("public migrations");
    let admin = DatabaseAdmin::new(admin_db.clone());
    let tenant = make_tenant(&admin, "firing").await;
    let cache = Arc::new(TenantDatabaseCache::new(TEST_DB_URL.to_string()));
    let tenant_db = cache.resolve(&tenant, &admin_db).await.expect("resolve");
    let scheduler = server_scheduler(&admin_db, &cache);

    // The tenant's runner holds a subscription (as
    // `subscribe_workflow_to_reactor` on that runner writes it).
    let runner_dal = cloacina::dal::unified::DAL::new(tenant_db.clone());
    let reactor = "price_reactor";
    let subscribed = runner_dal
        .reactor_subscriptions()
        .subscribe(reactor, "alert", &tenant, None)
        .await;

    // The reactor fires through the DAL its scheduler gave it.
    let fired = match scheduler.state_dal_for(Some(&tenant)).await {
        Some(dal) => dal
            .reactor_subscriptions()
            .insert_firing(
                reactor,
                &tenant,
                None,
                cloacina::database::universal_types::UniversalTimestamp::now(),
            )
            .await
            .map(|_| ())
            .map_err(|e| e.to_string()),
        None => Err("no state DAL".to_string()),
    };

    // The runner's poll, through its own pool.
    let seen_by_runner = runner_dal
        .reactor_subscriptions()
        .poll_unconsumed(&tenant, reactor, None, 10)
        .await;
    let in_admin = cloacina::dal::unified::DAL::new(admin_db.clone())
        .reactor_subscriptions()
        .poll_unconsumed(&tenant, reactor, None, 10)
        .await;

    let mut conn = diesel::PgConnection::establish(TEST_DB_URL).expect("connect");
    let _ = diesel::connection::SimpleConnection::batch_execute(
        &mut conn,
        &format!("DELETE FROM reactor_firings WHERE tenant_id = '{tenant}'"),
    );
    tenant_db.close();
    cache.evict(&tenant).await;
    let _ = admin.remove_tenant(&tenant, &tenant).await;
    admin_db.close();

    subscribed.expect("subscribe");
    fired.expect("fire");
    let in_admin = in_admin.map(|rows| rows.len()).unwrap_or_default();
    let seen_by_runner = seen_by_runner.expect("poll the tenant schema");
    assert_eq!(
        seen_by_runner.len(),
        1,
        "the tenant's runner sees its reactor's firing ({in_admin} row(s) in the admin \
         DAL's schema instead)"
    );
}

/// Two tenants with a graph of the same name keep separate reactive state.
#[tokio::test]
#[serial]
async fn two_tenants_with_a_graph_of_the_same_name_keep_separate_state() {
    let admin_db = cloacina::Database::new(TEST_DB_URL, "cloacina", 2);
    admin_db.run_migrations().await.expect("public migrations");
    let admin = DatabaseAdmin::new(admin_db.clone());
    let first = make_tenant(&admin, "state_a").await;
    let second = make_tenant(&admin, "state_b").await;
    let cache = Arc::new(TenantDatabaseCache::new(TEST_DB_URL.to_string()));
    let scheduler = server_scheduler(&admin_db, &cache);
    let graph = unique("graph");

    let outcome = async {
        for (tenant, cache_bytes) in [(&first, b"first".to_vec()), (&second, b"second".to_vec())] {
            let dal = scheduler
                .state_dal_for(Some(tenant))
                .await
                .ok_or("no state DAL")?;
            dal.checkpoint()
                .save_reactor_state(&graph, cache_bytes, Vec::new(), None)
                .await
                .map_err(|e| format!("save: {e}"))?;
        }
        let dal = scheduler
            .state_dal_for(Some(&first))
            .await
            .ok_or("no state DAL")?;
        let loaded = dal
            .checkpoint()
            .load_reactor_state(&graph)
            .await
            .map_err(|e| format!("load: {e}"))?;
        Ok::<_, String>(loaded.map(|(cache, _, _)| cache))
    }
    .await;

    let mut conn = diesel::PgConnection::establish(TEST_DB_URL).expect("connect");
    let _ = diesel::connection::SimpleConnection::batch_execute(
        &mut conn,
        &format!("DELETE FROM reactor_state WHERE graph_name = '{graph}'"),
    );
    for tenant in [&first, &second] {
        cache.evict(tenant).await;
        let _ = admin.remove_tenant(tenant, tenant).await;
    }
    admin_db.close();

    assert_eq!(
        outcome.expect("state"),
        Some(b"first".to_vec()),
        "the first tenant reads its own state, not the second tenant's"
    );
}
