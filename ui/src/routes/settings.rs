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

//! Settings (Aurora spec 13), parity port of `Settings.tsx`: Connection
//! (from the live session), Server (read-only, server-managed
//! placeholders), and Appearance — the Aurora light / dark / system choice
//! (the same setting as the top-bar toggle).

use aurora_leptos::components::{Card, DetailList, Dot, KeyValue, PageHeader, Panel};
use aurora_leptos::theme::{use_theme, Theme};
use aurora_leptos::tokens::token;
use leptos::prelude::*;

use crate::auth::use_auth;

/// One theme choice as a selectable card.
#[component]
fn ThemeCard(theme: Theme, #[prop(into)] note: String) -> impl IntoView {
    let ctx = use_theme();
    let selected = Signal::derive(move || ctx.choice.get() == theme);
    view! {
        <Card
            on_click=Callback::new(move |_| ctx.set(theme))
            selected=selected
            label=format!("{} theme", theme.label())
        >
            <div class="app-row app-row--between">
                <span class="app-row app-row--tight">
                    {move || view! { <Dot color=if selected.get() { token::ICE } else { token::MUTED } /> }}
                    <span class="app-text app-strong">{theme.label()}</span>
                </span>
                <Show when=move || selected.get()>
                    <span class="app-meta app-ice">"active"</span>
                </Show>
            </div>
            <div class="app-meta app-theme-note">{note.clone()}</div>
        </Card>
    }
}

#[component]
pub fn Settings() -> impl IntoView {
    let auth = use_auth();
    let tenant = move || {
        auth.connection()
            .map(|c| c.tenant)
            .unwrap_or_else(|| "—".into())
    };
    let server = move || {
        auth.connection()
            .map(|c| c.server_url)
            .unwrap_or_else(|| "—".into())
    };

    view! {
        <div class="app-page app-page--loose">
            <PageHeader title="Settings" />

            <Panel title="Connection">
                <DetailList mono=true stacked=true>
                    <KeyValue label="Tenant">{tenant}</KeyValue>
                    <KeyValue label="Server URL">{server}</KeyValue>
                </DetailList>
            </Panel>

            <Panel title="Server" caption="read-only · managed by the server">
                <DetailList mono=true stacked=true>
                    <KeyValue label="CLOACINA_BIND_ADDR"><span class="app-faint">"server-managed"</span></KeyValue>
                    <KeyValue label="DATABASE_URL"><span class="app-faint">"server-managed"</span></KeyValue>
                    <KeyValue label="SECRET_KEY"><span class="app-ok">"set · credentials encrypted"</span></KeyValue>
                    <KeyValue label="SCHEDULER">"enabled"</KeyValue>
                </DetailList>
            </Panel>

            <Panel title="Appearance" caption="stored in this browser · same as the top-bar toggle">
                <div class="app-grid-3">
                    <ThemeCard theme=Theme::Light note="Aurora light" />
                    <ThemeCard theme=Theme::Dark note="Aurora dark" />
                    <ThemeCard theme=Theme::System note="follow the operating system" />
                </div>
            </Panel>
        </div>
    }
}
