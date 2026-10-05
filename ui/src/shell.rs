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

//! Authenticated shell (CLOACI-I-0129, moved onto Aurora 0.4 by
//! CLOACINA-T-0943): Aurora's `AppShell` (sticky top bar, sidebar that is a
//! drawer below 768 px, one `<main>`) with a `SideNav` — a Run-workflow
//! primary, grouped links, and the connection footer (tenant switcher,
//! server URL, disconnect). The top bar carries the brand, the server badge
//! and the `ThemeToggle`.

use std::sync::Arc;

use aurora_leptos::components::{AppShell, Button, Dot, SideNav, SideNavGroup, SideNavLink};
use aurora_leptos::theme::ThemeToggle;
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::hooks::{use_location, use_navigate};

use crate::auth::use_auth;
use crate::brand::Brand;
use crate::config::APP_VERSION;

/// A sidebar link, active on its path (and, unless `end`, on sub-paths).
#[component]
fn NavItem(
    #[prop(into)] to: String,
    #[prop(into)] label: String,
    #[prop(optional)] end: bool,
    /// A coloured square marker (the orchestration trio): a hue token.
    #[prop(optional, into)]
    marker: String,
) -> impl IntoView {
    let location = use_location();
    let to_for_match = to.clone();
    let active = Signal::derive(move || {
        let path = location.pathname.get();
        if end {
            path == to_for_match
        } else {
            path == to_for_match || path.starts_with(&format!("{to_for_match}/"))
        }
    });
    view! {
        <SideNavLink href=to active=active marker=marker>
            {label}
        </SideNavLink>
    }
}

/// The top bar: server badge on the left, theme choice on the right.
#[component]
fn TopBar() -> impl IntoView {
    view! {
        <div class="app-topbar">
            <span class="app-topbar__server">
                <Dot color=token::OK size=7 />
                {format!("server · v{APP_VERSION}")}
            </span>
            <span class="app-topbar__spacer"></span>
            <ThemeToggle />
        </div>
    }
}

/// Sidebar + main-content scaffold around the routed outlet.
#[component]
pub fn Shell() -> impl IntoView {
    let auth = use_auth();
    // App-level data plumbing (T-0933): the shared poll tick and the warm
    // ops-metrics WS both live for the whole authenticated session.
    crate::data::provide_poll_tick();
    crate::ops::provide_ops_metrics();
    let navigate = use_navigate();

    let server_url = move || auth.connection().map(|c| c.server_url).unwrap_or_default();

    let navbar = move || {
        let navigate = navigate.clone();
        view! {
            <SideNav
                label="Main"
                footer=Box::new(move || {
                    view! {
                        <div class="app-conn">
                            <div class="app-conn__label">"Connection"</div>
                            <TenantSwitcher />
                            <div class="app-conn__url" title=server_url>{server_url}</div>
                            <button
                                type="button"
                                class="app-linkbtn"
                                on:click=move |_| {
                                    auth.disconnect();
                                    navigate("/connect", Default::default());
                                }
                            >
                                "Disconnect ↗"
                            </button>
                        </div>
                    }
                    .into_any()
                })
            >
                <div class="app-nav-cta">
                    <Button href="/workflows">"▸ Run workflow"</Button>
                </div>
                <SideNavGroup>
                    <NavItem to="/" label="Overview" end=true />
                    <NavItem to="/executions" label="Executions" />
                </SideNavGroup>
                <SideNavGroup label="Orchestration">
                    <NavItem to="/workflows" label="Workflows" marker=token::ICE />
                    <NavItem to="/triggers" label="Triggers" marker=token::VIOLET />
                    <NavItem to="/graphs" label="Graphs" marker=token::TEAL />
                </SideNavGroup>
                <SideNavGroup label="System">
                    <NavItem to="/operations" label="Operations" />
                    <NavItem to="/fleet" label="Agent fleet" />
                    <NavItem to="/keys" label="API Keys" />
                    <NavItem to="/secrets" label="Secrets" />
                    <NavItem to="/accounts" label="Accounts" />
                    <NavItem to="/settings" label="Settings" />
                </SideNavGroup>
            </SideNav>
        }
        .into_any()
    };

    view! {
        <AppShell
            brand=Arc::new(|| view! { <Brand /> }.into_any())
            header=Box::new(|| view! { <TopBar /> }.into_any())
            navbar=Box::new(navbar)
        >
            <Outlet />
        </AppShell>
    }
}

/// The tenant switcher (T-0779): the active connection plus a flip-open list
/// of the other saved tenants, an "add tenant" entry, and per-row remove.
/// (Aurora's `Menu` has no row with a second action, so this stays local.)
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
        <div class="app-tenant">
            <button
                type="button"
                class="app-tenant__trigger"
                aria-expanded=move || if open.get() { "true" } else { "false" }
                on:click=move |_| open.update(|v| *v = !*v)
            >
                <Dot color=token::OK size=7 />
                <span class="app-tenant__current">{active_label}</span>
                <span class="app-tenant__caret" aria-hidden="true">"▾"</span>
            </button>
            <Show when=move || open.get()>
                <div class="app-tenant__list">
                    <For
                        each=move || auth.connections.get()
                        key=|c| c.label.clone()
                        children=move |c| {
                            let label = c.label.clone();
                            let switch_label = label.clone();
                            let remove_label = label.clone();
                            view! {
                                <div class="app-tenant__row">
                                    <button
                                        type="button"
                                        class="app-tenant__item"
                                        on:click=move |_| {
                                            auth.switch_to(&switch_label);
                                            open.set(false);
                                        }
                                    >
                                        {label.clone()}
                                    </button>
                                    <button
                                        type="button"
                                        class="app-tenant__remove"
                                        title="Remove"
                                        aria-label=format!("Remove {label}")
                                        on:click=move |_| auth.remove_connection(&remove_label)
                                    >
                                        "×"
                                    </button>
                                </div>
                            }
                        }
                    />
                    <button
                        type="button"
                        class="app-tenant__add"
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
