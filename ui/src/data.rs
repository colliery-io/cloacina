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

//! The data layer (CLOACI-T-0933) — the Leptos analogue of the react-query
//! convention (T-0651):
//!
//! * all server access rides the authenticated `cloacina-client` (wasm);
//! * wasm client futures are `!Send`, so everything is a [`LocalResource`];
//! * resources derive from the ACTIVE connection, so a tenant switch
//!   re-fetches naturally (the react-query keys were tenant-scoped for the
//!   same reason);
//! * polling views bump a shared tick (the react-query
//!   `refetchInterval` analogue): [`poll_tick`] re-runs every resource that
//!   reads it;
//! * a fetch retries ONCE, and only for transient failures (server/network)
//!   — auth/validation/not-found never resolve by retrying (queryClient.ts
//!   parity, classified by aurora's `ApiError`).

use aurora_leptos::tokens::ApiError;
use leptos::prelude::*;

use cloacina_client::{Client, ClientError};

use crate::auth::use_auth;

/// Map the client's error onto the pack's [`ApiError`] shape (the app-side
/// half of the aurora `classify` contract).
pub fn map_client_error(e: &ClientError) -> ApiError {
    match e {
        ClientError::Transport(_) => ApiError::Network,
        ClientError::Auth(m) => ApiError::Http {
            status: 401,
            message: m.clone(),
            code: None,
        },
        ClientError::NotFound(m) => ApiError::Http {
            status: 404,
            message: m.clone(),
            code: None,
        },
        ClientError::InvalidRequest(m) => ApiError::Http {
            status: 400,
            message: m.clone(),
            code: None,
        },
        ClientError::Server { status, body } => ApiError::Http {
            status: *status,
            message: body["error"].as_str().unwrap_or("server error").to_string(),
            code: body["code"].as_str().map(String::from),
        },
        other => ApiError::Unknown(other.to_string()),
    }
}

/// Transient = worth one retry (server 5xx / network), per queryClient.ts.
fn is_transient(e: &ClientError) -> bool {
    matches!(
        e,
        ClientError::Transport(_)
            | ClientError::Server {
                status: 500..=u16::MAX,
                ..
            }
    )
}

/// Poll cadence for live lists (matches the React views' refetchInterval).
pub const POLL_MS: u32 = 5_000;

/// A monotonically-increasing tick every [`POLL_MS`]. A resource that reads
/// this re-fetches on cadence. One interval for the whole app (installed at
/// shell mount).
#[derive(Clone, Copy)]
pub struct PollTick(pub RwSignal<u64>);

pub fn provide_poll_tick() {
    let tick = RwSignal::new(0u64);
    provide_context(PollTick(tick));
    leptos::task::spawn_local(async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(POLL_MS).await;
            tick.update(|t| *t += 1);
        }
    });
}

pub fn poll_tick() -> RwSignal<u64> {
    use_context::<PollTick>()
        .expect("PollTick installed under the shell")
        .0
}

/// One fetch with the shared retry policy: retry once on transient
/// (server/network) failures only.
pub async fn fetch_with_retry<T, F, Fut>(f: F) -> Result<T, String>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ClientError>>,
{
    match f().await {
        Ok(v) => Ok(v),
        Err(first) if is_transient(&first) => f().await.map_err(|e| e.to_string()),
        Err(first) => Err(first.to_string()),
    }
}

/// A polling [`LocalResource`] bound to the active connection: re-fetches on
/// the app tick and whenever the active connection (tenant) changes.
///
/// The fetcher gets a fresh authenticated [`Client`] per run.
pub fn poll_resource<T, F, Fut>(f: F) -> LocalResource<Result<T, String>>
where
    T: 'static,
    F: Fn(Client) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<T, cloacina_client::ClientError>> + 'static,
{
    let auth = use_auth();
    let tick = poll_tick();
    let f = std::rc::Rc::new(f);
    LocalResource::new(move || {
        tick.get(); // subscribe to the cadence
        let conn = auth.connection();
        let f = f.clone();
        async move {
            let Some(conn) = conn else {
                return Err("disconnected".to_string());
            };
            let client = crate::auth::client_for(&conn)?;
            fetch_with_retry(move || f(client.clone())).await
        }
    })
}

/// A one-shot [`LocalResource`] bound to the active connection (no polling) —
/// for detail views that refresh on navigation, plus explicit `refetch()`.
pub fn once_resource<T, F, Fut>(f: F) -> LocalResource<Result<T, String>>
where
    T: 'static,
    F: Fn(Client) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<T, cloacina_client::ClientError>> + 'static,
{
    let auth = use_auth();
    let f = std::rc::Rc::new(f);
    LocalResource::new(move || {
        let conn = auth.connection();
        let f = f.clone();
        async move {
            let Some(conn) = conn else {
                return Err("disconnected".to_string());
            };
            let client = crate::auth::client_for(&conn)?;
            fetch_with_retry(move || f(client.clone())).await
        }
    })
}
