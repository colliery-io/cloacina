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

use aurora_leptos::components::{Alert, Button, PageHeader};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::auth::{client_for, use_auth};

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

    let do_upload = move || {
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
        <div class="app-page app-narrow-sm">
            <PageHeader
                title="Upload workflow"
                sub="Register a compiled .cloacina package for this tenant."
                back_href="/workflows"
                back_label="Workflows"
            />

            <Show
                when=move || auth.can_write()
                fallback=|| view! {
                    <Alert title="Write access required" color=token::GOLD>
                        "You need write access to upload packages."
                    </Alert>
                }
            >
                <div class="app-panel app-col app-col--loose">
                    // The native picker is hidden; the drop zone opens it.
                    <input
                        node_ref=input_ref
                        type="file"
                        accept=".cloacina"
                        class="cl-sr-only"
                        tabindex="-1"
                        aria-hidden="true"
                        on:change=on_pick
                    />
                    <button
                        type="button"
                        class="app-dropzone"
                        on:click=move |_| {
                            if let Some(input) = input_ref.get_untracked() {
                                input.unchecked_ref::<web_sys::HtmlElement>().click();
                            }
                        }
                    >
                        <span class="app-text app-fg2 app-block">
                            {move || if file.get().is_some() { "Selected file" } else { "Choose a .cloacina package" }}
                        </span>
                        <span class="app-dropzone__file" class:app-dropzone__file--set=move || file.get().is_some()>
                            {move || file.get().map(|f| f.name()).unwrap_or_else(|| "click to browse".into())}
                        </span>
                    </button>

                    <Button
                        loading=uploading
                        loading_label="Uploading…"
                        disabled=Signal::derive(move || file.get().is_none())
                        on_click=Callback::new(move |_| do_upload())
                    >
                        "↑ Upload"
                    </Button>

                    <Show when=move || !error.get().is_empty()>
                        <Alert color=token::BAD>{move || error.get()}</Alert>
                    </Show>

                    <Show when=move || uploaded.get().is_some()>
                        <Alert title="Uploaded" color=token::OK>
                            "Package registered. "
                            <a
                                class="app-link"
                                href=move || format!(
                                    "/workflows/{}",
                                    urlencoding::encode(&uploaded.get().unwrap_or_default())
                                )
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
