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

//! Connection identity + session (CLOACI-I-0117 REQ-001 / T-0779 / T-0800 /
//! T-0803), semantics 1:1 with the React `AuthContext`:
//!
//! * the credential is a bearer key in `sessionStorage` (NFR-005) — cleared
//!   on tab close; storage keys `cloacina.connections` / `cloacina.active`
//!   are UNCHANGED so a session survives the React→Leptos swap;
//! * the app holds a LIST of labeled connections (one per tenant) with an
//!   active one (T-0779) — the tenant switcher flips between them;
//! * minted (local/OIDC) keys are silently re-minted every 10 minutes via
//!   `/auth/refresh` (T-0800); a non-refreshable pasted key just stops the
//!   loop;
//! * `can_write` / `can_admin` derive from the whoami-resolved role and fail
//!   CLOSED while the role is unknown (T-0803).

use base64::Engine;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use cloacina_client::{Client, ClientBuilder};

const STORE_KEY: &str = "cloacina.connections";
const ACTIVE_KEY: &str = "cloacina.active";
const LEGACY_KEY: &str = "cloacina.connection";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    #[serde(default)]
    pub label: String,
    #[serde(rename = "serverUrl")]
    pub server_url: String,
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub tenant: String,
    /// `read` | `write` | `admin`, resolved via `/auth/whoami`. `None` for a
    /// session restored before the role was known — resolved on next load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// God-mode (cross-tenant platform admin).
    #[serde(rename = "isAdmin", default, skip_serializing_if = "Option::is_none")]
    pub is_admin: Option<bool>,
}

/// A minted tenant membership from an OIDC login (T-0800).
#[derive(Debug, Clone, Deserialize)]
pub struct Membership {
    pub key: String,
    pub tenant: String,
    #[serde(default)]
    pub role: String,
}

fn session_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.session_storage().ok().flatten()
}

fn load_state() -> (Vec<Connection>, Option<String>) {
    let Some(store) = session_storage() else {
        return (Vec::new(), None);
    };
    let mut connections: Vec<Connection> = store
        .get_item(STORE_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    // Migrate a pre-T-0779 single connection.
    if connections.is_empty() {
        if let Some(raw) = store.get_item(LEGACY_KEY).ok().flatten() {
            if let Ok(mut c) = serde_json::from_str::<Connection>(&raw) {
                if c.label.is_empty() {
                    c.label = c.tenant.clone();
                }
                connections = vec![c];
                let _ = store.remove_item(LEGACY_KEY);
            }
        }
    }
    let mut active = store.get_item(ACTIVE_KEY).ok().flatten();
    if active
        .as_ref()
        .map(|a| !connections.iter().any(|c| &c.label == a))
        .unwrap_or(true)
    {
        active = connections.first().map(|c| c.label.clone());
    }
    if connections.is_empty() {
        active = None;
    }
    (connections, active)
}

fn persist(connections: &[Connection], active: Option<&str>) {
    let Some(store) = session_storage() else {
        return;
    };
    if let Ok(raw) = serde_json::to_string(connections) {
        let _ = store.set_item(STORE_KEY, &raw);
    }
    match active {
        Some(a) => {
            let _ = store.set_item(ACTIVE_KEY, a);
        }
        None => {
            let _ = store.remove_item(ACTIVE_KEY);
        }
    }
}

/// Build a client for a connection. Cheap; call per use.
pub fn client_for(conn: &Connection) -> Result<Client, String> {
    ClientBuilder::new(&conn.server_url)
        .api_key(&conn.api_key)
        .tenant(&conn.tenant)
        .build()
        .map_err(|e| e.to_string())
}

#[derive(Clone, Copy)]
pub struct Auth {
    pub connections: RwSignal<Vec<Connection>>,
    pub active: RwSignal<Option<String>>,
}

impl Auth {
    pub fn connection(&self) -> Option<Connection> {
        let active = self.active.get()?;
        self.connections
            .get()
            .into_iter()
            .find(|c| c.label == active)
    }

    /// The authenticated client for the active connection. Only call under
    /// the auth guard. (Consumed by the Wave-2+ data layer, T-0933.)
    #[allow(dead_code)]
    pub fn client(&self) -> Option<Client> {
        self.connection().and_then(|c| client_for(&c).ok())
    }

    /// T-0803 role gates — unknown role fails closed. (Consumed by the
    /// role-gated views from Wave 2 on.)
    #[allow(dead_code)]
    pub fn can_write(&self) -> bool {
        self.connection()
            .map(|c| {
                c.is_admin.unwrap_or(false)
                    || matches!(c.role.as_deref(), Some("admin") | Some("write"))
            })
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn can_admin(&self) -> bool {
        self.connection()
            .map(|c| c.is_admin.unwrap_or(false) || c.role.as_deref() == Some("admin"))
            .unwrap_or(false)
    }

    fn upsert(&self, conn: Connection, make_active: bool) {
        self.connections.update(|list| {
            list.retain(|c| c.label != conn.label);
            list.push(conn.clone());
        });
        if make_active {
            self.active.set(Some(conn.label.clone()));
        }
        persist(
            &self.connections.get_untracked(),
            self.active.get_untracked().as_deref(),
        );
    }

    /// Validate a connection against the server, then save it and make it
    /// active. Validation = health (server up) + listWorkflows (key works for
    /// this tenant, surfaces 401/403) + whoami (role for gating; soft-fails).
    pub async fn connect(&self, mut conn: Connection) -> Result<(), String> {
        let probe = client_for(&conn)?;
        probe.health().await.map_err(|e| e.to_string())?;
        probe
            .list_workflows(None)
            .await
            .map_err(|e| e.to_string())?;
        if let Ok(me) = probe.whoami().await {
            conn.role = me.get("role").and_then(|v| v.as_str()).map(String::from);
            conn.is_admin = me.get("is_admin").and_then(|v| v.as_bool());
        }
        self.upsert(conn, true);
        Ok(())
    }

    /// Save several freshly-minted connections at once (an OIDC login
    /// granting multiple tenants) and enter one. Only the entered one is
    /// validated.
    pub async fn enter_memberships(
        &self,
        conns: Vec<Connection>,
        active_label: &str,
    ) -> Result<(), String> {
        let chosen = conns
            .iter()
            .find(|c| c.label == active_label)
            .or(conns.first())
            .cloned()
            .ok_or_else(|| "no memberships to enter".to_string())?;
        for c in &conns {
            if c.label != chosen.label {
                self.upsert(c.clone(), false);
            }
        }
        self.connect(chosen).await
    }

    /// Switch to a previously-saved connection (no re-validation, T-0779).
    pub fn switch_to(&self, label: &str) {
        if self
            .connections
            .get_untracked()
            .iter()
            .any(|c| c.label == label)
        {
            self.active.set(Some(label.to_string()));
            persist(&self.connections.get_untracked(), Some(label));
        }
    }

    /// Remove a saved connection; if it was active, fall back to another.
    pub fn remove_connection(&self, label: &str) {
        self.connections
            .update(|list| list.retain(|c| c.label != label));
        if self.active.get_untracked().as_deref() == Some(label) {
            let next = self
                .connections
                .get_untracked()
                .first()
                .map(|c| c.label.clone());
            self.active.set(next);
        }
        persist(
            &self.connections.get_untracked(),
            self.active.get_untracked().as_deref(),
        );
    }

    /// Clear ALL connections and return to the connect gate.
    pub fn disconnect(&self) {
        self.connections.set(Vec::new());
        self.active.set(None);
        if let Some(store) = session_storage() {
            let _ = store.remove_item(STORE_KEY);
            let _ = store.remove_item(ACTIVE_KEY);
            let _ = store.remove_item(LEGACY_KEY);
        }
    }

    fn patch(&self, label: &str, f: impl Fn(&mut Connection)) {
        self.connections.update(|list| {
            if let Some(c) = list.iter_mut().find(|c| c.label == label) {
                f(c);
            }
        });
        persist(
            &self.connections.get_untracked(),
            self.active.get_untracked().as_deref(),
        );
    }
}

/// Decode the `#memberships=<b64url json>` fragment payload (T-0800).
pub fn decode_memberships(raw: &str) -> Option<Vec<Membership>> {
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(raw))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(raw))
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Install the auth context + the two background loops (silent refresh,
/// restored-session role resolution). Call once at app root.
pub fn provide_auth() -> Auth {
    let (connections, active) = load_state();
    let auth = Auth {
        connections: RwSignal::new(connections),
        active: RwSignal::new(active),
    };
    provide_context(auth);

    // T-0800: silent refresh — every 10 min re-mint the ACTIVE connection's
    // key (minted TTL ~15m). A non-refreshable pasted key errors once; we
    // remember the label and stop asking for it.
    leptos::task::spawn_local(async move {
        let mut non_refreshable: Vec<String> = Vec::new();
        loop {
            gloo_timers::future::TimeoutFuture::new(10 * 60 * 1000).await;
            let Some(conn) = auth.connection() else {
                continue;
            };
            if non_refreshable.contains(&conn.label) {
                continue;
            }
            let Ok(client) = client_for(&conn) else {
                continue;
            };
            match client.refresh().await {
                Ok(res) => {
                    if let Some(key) = res.get("key").and_then(|v| v.as_str()) {
                        let key = key.to_string();
                        auth.patch(&conn.label, move |c| c.api_key = key.clone());
                    }
                }
                Err(_) => non_refreshable.push(conn.label.clone()),
            }
        }
    });

    // T-0803: a restored session may lack a role — resolve via whoami so
    // gating reflects the real role instead of staying failed-closed.
    Effect::new(move |_| {
        let Some(conn) = auth.connection() else {
            return;
        };
        if conn.role.is_some() {
            return;
        }
        leptos::task::spawn_local(async move {
            let Ok(client) = client_for(&conn) else {
                return;
            };
            if let Ok(me) = client.whoami().await {
                let role = me.get("role").and_then(|v| v.as_str()).map(String::from);
                let is_admin = me.get("is_admin").and_then(|v| v.as_bool());
                auth.patch(&conn.label, move |c| {
                    c.role = role.clone();
                    c.is_admin = is_admin;
                });
            }
        });
    });

    auth
}

pub fn use_auth() -> Auth {
    use_context::<Auth>().expect("Auth context installed at app root")
}
