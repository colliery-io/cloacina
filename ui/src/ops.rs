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

//! App-level live ops-metrics stream (CLOACI-T-0718; warm-up T-0774) —
//! parity port of `OpsMetricsProvider`: ONE `ops_metrics:global` delivery
//! subscription connected right after login, warm for the whole session, so
//! live pages never cold-start. The latest snapshot is a raw JSON value;
//! views pull the fields they show. Never reset on reconnect (stale-while-
//! revalidate, not a "down" flash); reset only when the connection changes.

use futures_util::StreamExt;
use leptos::prelude::*;

use crate::auth::{client_for, use_auth};

#[derive(Clone, Copy)]
pub struct OpsMetrics(pub RwSignal<Option<serde_json::Value>>);

/// Install the ops-metrics context + the per-connection subscription loop.
/// Call once under the auth guard (shell mount).
pub fn provide_ops_metrics() {
    let auth = use_auth();
    let latest = RwSignal::new(Option::<serde_json::Value>::None);
    provide_context(OpsMetrics(latest));

    // Generation counter kills the previous loop when the active connection
    // changes (the React version keyed the effect on `client`).
    let generation = StoredValue::new(0u64);

    Effect::new(move |_| {
        let Some(conn) = auth.connection() else {
            latest.set(None);
            generation.update_value(|g| *g += 1);
            return;
        };
        let my_gen = generation.with_value(|g| g + 1);
        generation.set_value(my_gen);
        latest.set(None); // new tenant → don't show the old tenant's snapshot
        leptos::task::spawn_local(async move {
            let Ok(client) = client_for(&conn) else {
                return;
            };
            let stream = client.subscribe_delivery("ops_metrics:global", Default::default());
            let mut stream = std::pin::pin!(stream);
            while let Some(push) = stream.next().await {
                if generation.with_value(|g| *g) != my_gen {
                    return; // superseded by a newer connection
                }
                let Ok(push) = push else { continue };
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&push.payload) {
                    latest.set(Some(v));
                }
            }
        });
    });
}

/// The latest ops snapshot (None only before the first frame after connect).
pub fn use_ops_metrics() -> RwSignal<Option<serde_json::Value>> {
    use_context::<OpsMetrics>()
        .expect("OpsMetrics installed under the shell")
        .0
}
