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

//! Authenticated shell — the Aurora Dark sidebar (CLOACI-I-0129): a fixed
//! 232px rail with the brand + server badge, a Run-workflow primary, grouped
//! nav, and a connection footer. Wraps every in-app route.
//!
//! Live nav counts and the app-level ops-metrics stream arrive with the
//! Wave-2 data layer (CLOACI-T-0933); the rail's structure and labels are
//! already at parity so navigation e2e specs bind to stable text.

use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::hooks::{use_location, use_navigate};

use crate::auth::use_auth;
use crate::brand::BrandMark;
use crate::config::APP_VERSION;

const MONO: &str = "'IBM Plex Mono', monospace";

#[component]
fn NavItem(
    #[prop(into)] to: String,
    #[prop(into)] label: String,
    #[prop(optional)] end: bool,
    /// A colored square marker (the orchestration trio) instead of an icon.
    #[prop(optional, into)]
    square: Option<String>,
) -> impl IntoView {
    let location = use_location();
    let to_for_match = to.clone();
    let is_active = Memo::new(move |_| {
        let path = location.pathname.get();
        if end {
            path == to_for_match
        } else {
            path == to_for_match || path.starts_with(&format!("{to_for_match}/"))
        }
    });
    view! {
        <a
            href=to
            style:display="flex"
            style:align-items="center"
            style:gap="10px"
            style:padding="7px 10px"
            style:border-radius="8px"
            style:margin-bottom="2px"
            style:font-size="13px"
            style:font-weight="500"
            style:text-decoration="none"
            style:color=move || if is_active.get() { "var(--fg)" } else { "var(--muted)" }
            style:background=move || {
                if is_active.get() { "rgba(127,178,255,.13)" } else { "transparent" }
            }
            style:box-shadow=move || {
                if is_active.get() { "inset 2px 0 0 #7fb2ff" } else { "none" }
            }
        >
            {square.map(|c| view! {
                <span
                    style:width="9px"
                    style:height="9px"
                    style:border-radius="2px"
                    style:background=c
                    style:flex="none"
                ></span>
            })}
            <span style:flex="1" style:min-width="0">{label}</span>
        </a>
    }
}

#[component]
fn GroupLabel(children: Children) -> impl IntoView {
    view! {
        <div
            style:font-family=MONO
            style:font-size="10px"
            style:letter-spacing=".1em"
            style:text-transform="uppercase"
            style:color="var(--faint)"
            style:padding="14px 10px 6px"
        >
            {children()}
        </div>
    }
}

/// Sidebar + main-content scaffold around the routed outlet.
#[component]
pub fn Shell() -> impl IntoView {
    let auth = use_auth();
    let navigate = use_navigate();
    let nav_run = navigate.clone();
    let nav_disconnect = navigate;

    let server_url = move || auth.connection().map(|c| c.server_url).unwrap_or_default();

    view! {
        <div
            style:display="flex"
            style:min-height="100vh"
        >
            // ---- 232px rail ----
            <nav
                style:width="232px"
                style:flex="none"
                style:background="var(--sidebar)"
                style:border-right="1px solid var(--border-soft)"
                style:display="flex"
                style:flex-direction="column"
                style:position="sticky"
                style:top="0"
                style:height="100vh"
            >
                // Brand
                <div style:padding="18px 14px 10px">
                    <div style:display="flex" style:align-items="center" style:gap="9px">
                        <BrandMark />
                        <span
                            style:font-size="16px"
                            style:font-weight="600"
                            style:color="var(--fg-bright)"
                        >
                            "Cloacina"
                        </span>
                    </div>
                    <div
                        style:display="flex"
                        style:align-items="center"
                        style:gap="6px"
                        style:margin-top="8px"
                        style:font-family=MONO
                        style:font-size="11px"
                        style:color="var(--muted)"
                    >
                        <span
                            style:width="7px"
                            style:height="7px"
                            style:border-radius="50%"
                            style:background=token::OK
                        ></span>
                        {format!("server · v{APP_VERSION}")}
                    </div>
                </div>

                // Run workflow primary
                <div style:padding="6px 14px 8px">
                    <button
                        class="cl-btn cl-btn--filled"
                        style:width="100%"
                        on:click=move |_| nav_run("/workflows", Default::default())
                    >
                        "▸ Run workflow"
                    </button>
                </div>

                // Nav
                <div style:flex="1" style:overflow-y="auto" style:padding="4px 8px">
                    <NavItem to="/" label="Overview" end=true />
                    <NavItem to="/executions" label="Executions" />

                    <GroupLabel>"Orchestration"</GroupLabel>
                    <NavItem to="/workflows" label="Workflows" square=token::ICE />
                    <NavItem to="/triggers" label="Triggers" square=token::VIOLET />
                    <NavItem to="/graphs" label="Graphs" square=token::TEAL />

                    <GroupLabel>"System"</GroupLabel>
                    <NavItem to="/operations" label="Operations" />
                    <NavItem to="/fleet" label="Agent fleet" />
                    <NavItem to="/keys" label="API Keys" />
                    <NavItem to="/secrets" label="Secrets" />
                    <NavItem to="/accounts" label="Accounts" />
                    <NavItem to="/settings" label="Settings" />
                </div>

                // Connection footer
                <div
                    style:padding="12px 14px"
                    style:border-top="1px solid var(--border-soft)"
                >
                    <div
                        style:font-family=MONO
                        style:font-size="10px"
                        style:letter-spacing=".1em"
                        style:text-transform="uppercase"
                        style:color="var(--faint)"
                        style:margin-bottom="6px"
                    >
                        "Connection"
                    </div>
                    <crate::shell::TenantSwitcher />
                    <div
                        style:font-family=MONO
                        style:font-size="10.5px"
                        style:color="var(--faint)"
                        style:margin-top="3px"
                        style:overflow="hidden"
                        style:text-overflow="ellipsis"
                        style:white-space="nowrap"
                        title=server_url
                    >
                        {server_url}
                    </div>
                    <button
                        style:margin-top="8px"
                        style:background="none"
                        style:border="none"
                        style:padding="0"
                        style:cursor="pointer"
                        style:font-size="11.5px"
                        style:color="var(--muted)"
                        on:click=move |_| {
                            auth.disconnect();
                            nav_disconnect("/connect", Default::default());
                        }
                    >
                        "Disconnect ↗"
                    </button>
                </div>
            </nav>

            // ---- main ----
            <main style:flex="1" style:background="var(--bg)" style:min-width="0">
                <div style:padding="22px 28px">
                    <Outlet />
                </div>
            </main>
        </div>
    }
}

/// The tenant switcher (T-0779): the active connection plus a flip-open list
/// of the other saved tenants, an "add tenant" entry, and per-row remove.
#[component]
pub fn TenantSwitcher() -> impl IntoView {
    let auth = use_auth();
    let open = RwSignal::new(false);
    let navigate = use_navigate();

    let active_label = move || {
        auth.connection()
            .map(|c| c.label)
            .unwrap_or_else(|| "—".to_string())
    };

    view! {
        <div style:position="relative">
            <button
                style:display="flex"
                style:align-items="center"
                style:gap="6px"
                style:width="100%"
                style:background="var(--control)"
                style:border="1px solid var(--border-control)"
                style:border-radius="7px"
                style:padding="5px 8px"
                style:cursor="pointer"
                style:color="var(--fg)"
                style:font-family=MONO
                style:font-size="11.5px"
                on:click=move |_| open.update(|v| *v = !*v)
            >
                <span
                    style:width="7px"
                    style:height="7px"
                    style:border-radius="50%"
                    style:background=token::OK
                    style:flex="none"
                ></span>
                <span
                    style:flex="1"
                    style:text-align="left"
                    style:overflow="hidden"
                    style:text-overflow="ellipsis"
                >
                    {active_label}
                </span>
                <span style:color="var(--faint)">"▾"</span>
            </button>
            <Show when=move || open.get()>
                <div
                    style:position="absolute"
                    style:bottom="110%"
                    style:left="0"
                    style:right="0"
                    style:background="var(--panel)"
                    style:border="1px solid var(--border)"
                    style:border-radius="8px"
                    style:padding="4px"
                    style:z-index="30"
                >
                    <For
                        each=move || auth.connections.get()
                        key=|c| c.label.clone()
                        children=move |c| {
                            let label = c.label.clone();
                            let switch_label = label.clone();
                            let remove_label = label.clone();
                            view! {
                                <div style:display="flex" style:align-items="center">
                                    <button
                                        style:flex="1"
                                        style:background="none"
                                        style:border="none"
                                        style:text-align="left"
                                        style:padding="5px 7px"
                                        style:cursor="pointer"
                                        style:color="var(--fg)"
                                        style:font-family=MONO
                                        style:font-size="11.5px"
                                        on:click=move |_| {
                                            auth.switch_to(&switch_label);
                                            open.set(false);
                                        }
                                    >
                                        {label.clone()}
                                    </button>
                                    <button
                                        title="Remove"
                                        style:background="none"
                                        style:border="none"
                                        style:cursor="pointer"
                                        style:color="var(--faint)"
                                        style:padding="0 6px"
                                        on:click=move |_| auth.remove_connection(&remove_label)
                                    >
                                        "×"
                                    </button>
                                </div>
                            }
                        }
                    />
                    <button
                        style:width="100%"
                        style:background="none"
                        style:border="none"
                        style:border-top="1px solid var(--border-soft)"
                        style:text-align="left"
                        style:padding="6px 7px 4px"
                        style:cursor="pointer"
                        style:color="var(--muted)"
                        style:font-size="11.5px"
                        on:click={
                            let navigate = navigate.clone();
                            move |_| {
                                open.set(false);
                                navigate("/connect?add=1", Default::default());
                            }
                        }
                    >
                        "+ Add tenant"
                    </button>
                </div>
            </Show>
        </div>
    }
}
