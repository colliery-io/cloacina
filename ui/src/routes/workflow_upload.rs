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

//! Workflow package upload (T-0657 / UC-3), parity port of
//! `WorkflowUpload.tsx`: pick a `.cloacina` file → upload (multipart over the
//! wasm client) → result, behind the write gate. Busy state rather than a
//! byte-progress bar, same as the React SPA.

use aurora_leptos::components::Alert;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::auth::{client_for, use_auth};

const MONO: &str = "'IBM Plex Mono', monospace";

async fn file_bytes(file: &web_sys::File) -> Result<Vec<u8>, String> {
    let buf = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
        .await
        .map_err(|_| "could not read the selected file".to_string())?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

#[component]
pub fn WorkflowUpload() -> impl IntoView {
    let auth = use_auth();
    let file = RwSignal::new(Option::<web_sys::File>::None);
    let uploading = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let uploaded = RwSignal::new(Option::<String>::None); // package name for the View link
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let on_pick = move |_| {
        if let Some(input) = input_ref.get_untracked() {
            let el: &web_sys::HtmlInputElement = &input;
            file.set(el.files().and_then(|l| l.get(0)));
            error.set(String::new());
            uploaded.set(None);
        }
    };

    let do_upload = move |_| {
        let Some(f) = file.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        uploading.set(true);
        error.set(String::new());
        leptos::task::spawn_local(async move {
            let result = async {
                let bytes = file_bytes(&f).await?;
                let client = client_for(&conn)?;
                client
                    .upload_workflow(bytes, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            uploading.set(false);
            match result {
                Ok(res) => uploaded.set(Some(res.package_id)),
                Err(e) => error.set(e),
            }
        });
    };

    view! {
        <div style:max-width="560px" style:display="flex" style:flex-direction="column" style:gap="14px">
            <div>
                <a href="/workflows" style:font-size="11.5px" style:color="var(--muted)" style:text-decoration="none">
                    "← Workflows"
                </a>
                <h1 style:font-size="22px" style:font-weight="600" style:color="var(--fg-bright)" style:margin="2px 0 0">
                    "Upload workflow"
                </h1>
                <div style:font-family=MONO style:font-size="11px" style:color="var(--faint)" style:margin-top="2px">
                    "Register a compiled .cloacina package for this tenant."
                </div>
            </div>

            <Show
                when=move || auth.can_write()
                fallback=|| view! {
                    <Alert title="Write access required" color="var(--gold)">
                        "You need write access to upload packages."
                    </Alert>
                }
            >
                <div
                    style:background="var(--panel)"
                    style:border="1px solid var(--border)"
                    style:border-radius="10px"
                    style:padding="16px 18px"
                    style:display="flex"
                    style:flex-direction="column"
                    style:gap="14px"
                >
                    <input
                        node_ref=input_ref
                        type="file"
                        accept=".cloacina"
                        style:display="none"
                        on:change=on_pick
                    />
                    <div
                        style:border="1px dashed var(--border-control)"
                        style:border-radius="10px"
                        style:background="var(--inset)"
                        style:padding="22px 16px"
                        style:text-align="center"
                        style:cursor="pointer"
                        on:click=move |_| {
                            if let Some(input) = input_ref.get_untracked() {
                                input.unchecked_ref::<web_sys::HtmlElement>().click();
                            }
                        }
                    >
                        <div style:font-size="13px" style:color="var(--fg-2)">
                            {move || if file.get().is_some() { "Selected file" } else { "Choose a .cloacina package" }}
                        </div>
                        <div
                            style:font-family=MONO
                            style:font-size="12px"
                            style:margin-top="4px"
                            style:color=move || if file.get().is_some() { "var(--ice)" } else { "var(--faint)" }
                        >
                            {move || file.get().map(|f| f.name()).unwrap_or_else(|| "click to browse".into())}
                        </div>
                    </div>

                    <button
                        class="cl-btn cl-btn--filled"
                        disabled=move || file.get().is_none() || uploading.get()
                        on:click=do_upload
                    >
                        {move || if uploading.get() { "Uploading…" } else { "↑ Upload" }}
                    </button>

                    <Show when=move || !error.get().is_empty()>
                        <Alert color="var(--bad)">{move || error.get()}</Alert>
                    </Show>

                    <Show when=move || uploaded.get().is_some()>
                        <Alert title="Uploaded" color="var(--ok)">
                            "Package registered. "
                            <a
                                href=move || format!(
                                    "/workflows/{}",
                                    urlencoding::encode(&uploaded.get().unwrap_or_default())
                                )
                                style:color="var(--ice)"
                            >
                                "View"
                            </a>
                        </Alert>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
