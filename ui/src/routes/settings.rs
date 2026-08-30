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

//! Settings (Aurora Dark spec 13), parity port of `Settings.tsx`:
//! Connection (from the live session), Server (read-only, server-managed
//! placeholders), Appearance (dark-only pack, light "soon").

use aurora_leptos::tokens::token;
use leptos::prelude::*;

use crate::auth::use_auth;

const MONO: &str = "'IBM Plex Mono', monospace";

#[component]
fn Section(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <div>
            <div
                style:font-size="14px"
                style:font-weight="600"
                style:color="var(--fg)"
                style:border-bottom="1px solid var(--border-soft)"
                style:padding-bottom="8px"
                style:margin-bottom="12px"
            >
                {title}
            </div>
            {children()}
        </div>
    }
}

#[component]
fn ConfigCard(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(optional, into)] color: Option<String>,
) -> impl IntoView {
    view! {
        <div
            style:background="var(--panel)"
            style:border="1px solid var(--border)"
            style:border-radius="11px"
            style:padding="13px 16px"
        >
            <div
                style:font-family=MONO
                style:font-size="10.5px"
                style:letter-spacing=".06em"
                style:text-transform="uppercase"
                style:color="var(--faint)"
                style:margin-bottom="6px"
            >
                {label}
            </div>
            <div
                style:font-family=MONO
                style:font-size="12.5px"
                style:color=color.unwrap_or_else(|| "var(--fg)".into())
            >
                {value}
            </div>
        </div>
    }
}

#[component]
pub fn Settings() -> impl IntoView {
    let auth = use_auth();
    let tenant = move || auth.connection().map(|c| c.tenant).unwrap_or_else(|| "—".into());
    let server = move || {
        auth.connection()
            .map(|c| c.server_url)
            .unwrap_or_else(|| "—".into())
    };

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="22px">
            <h1 style:font-size="22px" style:font-weight="600" style:color="var(--fg-bright)" style:margin="0">
                "Settings"
            </h1>

            <Section title="Connection">
                <div style:display="grid" style:grid-template-columns="1fr 1fr" style:gap="13px">
                    <ConfigCard label="Tenant" value=tenant() />
                    <ConfigCard label="Server URL" value=server() />
                </div>
            </Section>

            <Section title="Server">
                <div style:display="grid" style:grid-template-columns="1fr 1fr" style:gap="13px">
                    <ConfigCard label="CLOACINA_BIND_ADDR" value="server-managed" color="var(--faint)" />
                    <ConfigCard label="DATABASE_URL" value="server-managed" color="var(--faint)" />
                    <ConfigCard label="SECRET_KEY" value="set · credentials encrypted" color=token::OK />
                    <ConfigCard label="SCHEDULER" value="enabled" />
                </div>
            </Section>

            <Section title="Appearance">
                <div style:display="grid" style:grid-template-columns="1fr 1fr" style:gap="13px">
                    <div
                        style:background="var(--panel)"
                        style:border=format!("1px solid {}7a", token::ICE)
                        style:border-radius="11px"
                        style:padding="14px 16px"
                        style:display="flex"
                        style:justify-content="space-between"
                        style:align-items="center"
                    >
                        <span style:display="inline-flex" style:gap="9px" style:align-items="center">
                            <span
                                style:width="8px"
                                style:height="8px"
                                style:border-radius="50%"
                                style:background=token::ICE
                            ></span>
                            <span style:font-size="13px" style:font-weight="500" style:color="var(--fg)">
                                "Aurora dark"
                            </span>
                        </span>
                        <span style:font-family=MONO style:font-size="10.5px" style:color=token::ICE>
                            "active"
                        </span>
                    </div>
                    <div
                        style:background="var(--panel)"
                        style:border="1px solid var(--border)"
                        style:border-radius="11px"
                        style:padding="14px 16px"
                        style:opacity="0.6"
                        style:display="flex"
                        style:justify-content="space-between"
                        style:align-items="center"
                    >
                        <span style:display="inline-flex" style:gap="9px" style:align-items="center">
                            <span
                                style:width="8px"
                                style:height="8px"
                                style:border-radius="50%"
                                style:background=token::MUTED
                            ></span>
                            <span style:font-size="13px" style:font-weight="500" style:color="var(--muted)">
                                "Light"
                            </span>
                        </span>
                        <span style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                            "soon"
                        </span>
                    </div>
                </div>
            </Section>
        </div>
    }
}
