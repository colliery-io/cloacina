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

//! Connection gate (Aurora Dark spec 14) — parity port of the React
//! `Connect.tsx` (CLOACI-T-0796/0798/0800): pasted API key, self-managed
//! username/password login (mints a short-TTL key), or the OIDC browser
//! flow. An SSO callback returning multiple tenant memberships shows the
//! tenant picker; `?add=1` keeps the gate open while already connected
//! (add-a-tenant from the switcher).

use aurora_leptos::components::{
    Alert, AuthCard, Button, CenterScreen, PasswordInput, SegmentedControl, TextInput,
};
use aurora_leptos::theme::ThemeToggle;
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::auth::{client_for, decode_memberships, use_auth, Connection, Membership};
use crate::brand::Brand;
use crate::config::{runtime_config, APP_VERSION};

const SSO_SERVER_KEY: &str = "cloacina.sso.server";

fn session_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.session_storage().ok().flatten()
}

/// Pull `#memberships=<b64url>` off the URL, stripping the fragment from the
/// URL bar / history immediately (the keys never reach a server or a log).
fn take_sso_fragment() -> Option<(String, Vec<Membership>)> {
    let window = web_sys::window()?;
    let hash = window.location().hash().ok()?;
    if !hash.contains("memberships=") {
        return None;
    }
    let params = web_sys::UrlSearchParams::new_with_str(hash.trim_start_matches('#')).ok()?;
    let raw = params.get("memberships");
    let path = window
        .location()
        .pathname()
        .unwrap_or_else(|_| "/connect".into());
    let _ = window.history().ok()?.replace_state_with_url(
        &wasm_bindgen::JsValue::NULL,
        "",
        Some(&path),
    );
    let raw = raw?;
    let server = session_storage()
        .and_then(|s| s.get_item(SSO_SERVER_KEY).ok().flatten())
        .unwrap_or_default();
    if let Some(s) = session_storage() {
        let _ = s.remove_item(SSO_SERVER_KEY);
    }
    let memberships = decode_memberships(&raw)?;
    if memberships.is_empty() {
        return None;
    }
    Some((server, memberships))
}

#[component]
pub fn Connect() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();
    let query = use_query_map();
    let add_mode = move || query.read().get("add").as_deref() == Some("1");

    let cfg = runtime_config();

    let submitting = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    // "Username & password" | "Key" | "SSO" — segmented control binds a label.
    let mode = RwSignal::new("Key".to_string());

    let server_url = RwSignal::new(cfg.default_server_url.clone());
    let api_key = RwSignal::new(cfg.demo_api_key.clone());
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let tenant = RwSignal::new(if cfg.demo_tenant.is_empty() {
        "public".to_string()
    } else {
        cfg.demo_tenant.clone()
    });

    // SSO multi-tenant picker state.
    let sso_picker = RwSignal::new(Option::<(String, Vec<Membership>)>::None);

    // Already connected and not adding → straight in.
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if auth.connection().is_some() && !add_mode() && sso_picker.get().is_none() {
                navigate("/", Default::default());
            }
        });
    }

    let do_connect = {
        let navigate = navigate.clone();
        move || {
            let navigate = navigate.clone();
            submitting.set(true);
            error.set(String::new());
            let mode_v = mode.get_untracked();
            let server = server_url.get_untracked().trim_end_matches('/').to_string();
            let tenant_v = tenant.get_untracked().trim().to_string();
            leptos::task::spawn_local(async move {
                let result: Result<(), String> = async {
                    if !server.starts_with("http://") && !server.starts_with("https://") {
                        return Err("Server URL must start with http:// or https://".into());
                    }
                    if tenant_v.is_empty() {
                        return Err("Tenant is required".into());
                    }
                    let key = if mode_v == "Username & password" {
                        let user = username.get_untracked().trim().to_string();
                        let pass = password.get_untracked();
                        if user.is_empty() || pass.is_empty() {
                            return Err("Username and password are required".into());
                        }
                        // Public login — mint a short-TTL key, then connect with it.
                        let login = client_for(&Connection {
                            label: tenant_v.clone(),
                            server_url: server.clone(),
                            api_key: String::new(),
                            tenant: tenant_v.clone(),
                            role: None,
                            is_admin: None,
                        })?;
                        let res = login
                            .local_login(&user, &pass, Some(&tenant_v))
                            .await
                            .map_err(|e| e.to_string())?;
                        res.get("key")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .ok_or_else(|| "login response carried no key".to_string())?
                    } else {
                        let k = api_key.get_untracked().trim().to_string();
                        if k.is_empty() {
                            return Err("API key is required".into());
                        }
                        k
                    };
                    auth.connect(Connection {
                        label: tenant_v.clone(),
                        server_url: server,
                        api_key: key,
                        tenant: tenant_v,
                        role: None,
                        is_admin: None,
                    })
                    .await
                }
                .await;
                submitting.set(false);
                match result {
                    Ok(()) => navigate("/", Default::default()),
                    Err(e) => error.set(e),
                }
            });
        }
    };

    // OIDC start: stash the server URL, full-page navigate to the server's
    // login route (which 302s to the identity provider).
    let start_sso = move || {
        let server = server_url.get_untracked().trim_end_matches('/').to_string();
        if !server.starts_with("http://") && !server.starts_with("https://") {
            error.set("Server URL must start with http:// or https://".into());
            return;
        }
        submitting.set(true);
        if let Some(s) = session_storage() {
            let _ = s.set_item(SSO_SERVER_KEY, &server);
        }
        if let Some(w) = web_sys::window() {
            let _ = w
                .location()
                .set_href(&format!("{server}/v1/auth/oidc/login"));
        }
    };

    // OIDC callback pickup — one membership connects straight in; several
    // show the picker.
    {
        let navigate = navigate.clone();
        let default_server = cfg.default_server_url.clone();
        if let Some((stashed_server, memberships)) = take_sso_fragment() {
            let server = if stashed_server.is_empty() {
                default_server.trim_end_matches('/').to_string()
            } else {
                stashed_server
            };
            if memberships.len() == 1 {
                let m = memberships.into_iter().next().expect("len checked");
                submitting.set(true);
                let navigate = navigate.clone();
                let server_for_conn = server.clone();
                leptos::task::spawn_local(async move {
                    let res = auth
                        .connect(Connection {
                            label: m.tenant.clone(),
                            server_url: server_for_conn,
                            api_key: m.key.clone(),
                            tenant: m.tenant.clone(),
                            role: None,
                            is_admin: None,
                        })
                        .await;
                    submitting.set(false);
                    match res {
                        Ok(()) => navigate("/", Default::default()),
                        Err(e) => error.set(e),
                    }
                });
            } else {
                sso_picker.set(Some((server, memberships)));
            }
        }
    }

    // Dev auto-connect (compose demo stack).
    if cfg.demo_auto_connect
        && auth.connection().is_none()
        && !api_key.get_untracked().trim().is_empty()
    {
        do_connect();
    }

    let pick_membership = {
        let navigate = navigate.clone();
        move |tenant_pick: String| {
            let Some((server, memberships)) = sso_picker.get_untracked() else {
                return;
            };
            submitting.set(true);
            error.set(String::new());
            let conns: Vec<Connection> = memberships
                .iter()
                .map(|m| Connection {
                    label: m.tenant.clone(),
                    server_url: server.clone(),
                    api_key: m.key.clone(),
                    tenant: m.tenant.clone(),
                    role: None,
                    is_admin: None,
                })
                .collect();
            let navigate = navigate.clone();
            leptos::task::spawn_local(async move {
                let res = auth.enter_memberships(conns, &tenant_pick).await;
                submitting.set(false);
                match res {
                    Ok(()) => navigate("/", Default::default()),
                    Err(e) => error.set(e),
                }
            });
        }
    };

    let mode_hint = move || match mode.get().as_str() {
        "Username & password" => "Sign in with your username and password.",
        "SSO" => "Sign in through your identity provider.",
        _ => "Enter a server URL and a tenant API key.",
    };

    let submit_word = move || {
        if mode.get() == "Username & password" {
            "Sign in"
        } else {
            "Connect"
        }
    };

    let do_connect_click = do_connect.clone();

    let footer_line = format!("cloacina v{APP_VERSION} · tenant-scoped control plane");

    view! {
        <div class="app-connect">
            // The theme choice is on the gate too (there is no top bar here).
            <div class="app-connect__theme"><ThemeToggle /></div>
            <CenterScreen>
                <Show
                    when=move || sso_picker.get().is_some()
                    fallback=move || {
                        let do_connect = do_connect_click.clone();
                        let on_submit = do_connect.clone();
                        let footer_line = footer_line.clone();
                        view! {
                            <AuthCard
                                title="Connect to a server"
                                brand=Box::new(|| view! { <Brand size=26 large=true /> }.into_any())
                                footer=Box::new(move || view! { <span class="app-meta">{footer_line}</span> }.into_any())
                            >
                                <p class="app-hint app-connect__hint">{mode_hint}</p>

                                <SegmentedControl
                                    options=vec![
                                        "Username & password".to_string(),
                                        "Key".to_string(),
                                        "SSO".to_string(),
                                    ]
                                    value=mode
                                />

                                <form
                                    class="app-col app-col--loose"
                                    on:submit=move |ev| {
                                        ev.prevent_default();
                                        on_submit();
                                    }
                                >
                                    <TextInput
                                        label="Server URL"
                                        placeholder="http://localhost:8080"
                                        value=server_url
                                        autocomplete="url"
                                    />

                                    <Show
                                        when=move || mode.get() == "SSO"
                                        fallback=move || {
                                            view! {
                                                <Show when=move || mode.get() == "Username & password">
                                                    <TextInput label="Username" placeholder="alice" value=username autocomplete="username" />
                                                    <PasswordInput label="Password" value=password autocomplete="current-password" />
                                                </Show>
                                                <Show when=move || mode.get() == "Key">
                                                    <PasswordInput label="API key" placeholder="clk_…" value=api_key autocomplete="off" />
                                                </Show>
                                                <TextInput label="Tenant" placeholder="public" value=tenant />
                                            }
                                        }
                                    >
                                        <span></span>
                                    </Show>

                                    <Show when=move || !error.get().is_empty()>
                                        <Alert color=token::BAD>{move || error.get()}</Alert>
                                    </Show>

                                    <Show
                                        when=move || mode.get() == "SSO"
                                        fallback=move || {
                                            view! {
                                                <div class="app-connect__submit">
                                                    <Button button_type="submit" loading=submitting>
                                                        {submit_word}
                                                    </Button>
                                                </div>
                                            }
                                        }
                                    >
                                        // Reactive: the button follows `submitting` (it
                                        // took a one-time snapshot before Aurora 0.4).
                                        <div class="app-connect__submit">
                                            <Button
                                                button_type="button"
                                                loading=submitting
                                                on_click=Callback::new(move |_| start_sso())
                                            >
                                                "Continue with SSO"
                                            </Button>
                                        </div>
                                    </Show>
                                </form>
                            </AuthCard>
                        }
                    }
                >
                    // ---- SSO tenant picker ----
                    {
                        let pick_membership = pick_membership.clone();
                        view! {
                            <AuthCard
                                title="Choose a tenant"
                                sub="Your sign-in grants access to multiple tenants. Pick one to enter — the rest stay one click away in the tenant switcher."
                                brand=Box::new(|| view! { <Brand size=26 large=true /> }.into_any())
                            >
                                <For
                                    each=move || {
                                        sso_picker.get().map(|(_, m)| m).unwrap_or_default()
                                    }
                                    key=|m| m.tenant.clone()
                                    children={
                                        let pick = pick_membership.clone();
                                        move |m| {
                                            let pick = pick.clone();
                                            let tenant_name = m.tenant.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    class="app-tenant-pick"
                                                    disabled=move || submitting.get()
                                                    on:click=move |_| pick(tenant_name.clone())
                                                >
                                                    <span>{m.tenant.clone()}</span>
                                                    <span class="app-meta app-meta--sm app-muted">{m.role.clone()}</span>
                                                </button>
                                            }
                                        }
                                    }
                                />
                                <Show when=move || !error.get().is_empty()>
                                    <Alert color=token::BAD>{move || error.get()}</Alert>
                                </Show>
                            </AuthCard>
                        }
                    }
                </Show>
            </CenterScreen>
        </div>
    }
}
