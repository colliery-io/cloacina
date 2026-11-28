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
//!
//! The form also takes the package's detached signature (CLOACINA-T-1120):
//! the `<archive>.sig` file that `cloacinactl package pack --sign` writes. Its
//! text goes as the `signature` field of the upload (CLOACINA-A-0018). The
//! UI does not sign: the private key stays on the publisher's machine.

use aurora_leptos::components::{Alert, Button, PageHeader};
use aurora_leptos::tokens::token;
use cloacina_client::ClientError;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::auth::{client_for, use_auth};

async fn file_bytes(file: &web_sys::File) -> Result<Vec<u8>, String> {
    let buf = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
        .await
        .map_err(|_| "could not read the selected file".to_string())?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// The upload the form sends: with a signature, the `signature` field goes
/// beside the archive; without one, the archive only, as before.
async fn send_upload(
    client: &cloacina_client::Client,
    package: Vec<u8>,
    signature: Option<&str>,
) -> Result<cloacina_api_types::WorkflowUploadedResponse, ClientError> {
    match signature {
        Some(sig) => client.upload_workflow_signed(package, sig, None).await,
        None => client.upload_workflow(package, None).await,
    }
}

/// The signature check's refusal codes (the upload route in
/// `crates/cloacina-server/src/routes/workflows.rs`), each with what to do.
const REFUSALS: [(&str, &str); 6] = [
    (
        "signature_required",
        "This server requires signed packages. Choose the package's .sig file (written by `cloacinactl package pack --sign`) and upload again.",
    ),
    (
        "malformed_signature",
        "The signature file is not a detached signature. Choose the .sig file that `cloacinactl package pack --sign` wrote for this package.",
    ),
    (
        "package_tampered",
        "The signature file was made for other bytes than this package. Choose the .sig file written with this archive, or sign the archive again.",
    ),
    (
        "untrusted_signer",
        "The signing key is not trusted by this tenant. A tenant admin adds its public key with `cloacinactl trusted-key add`, or sign with a key the tenant trusts.",
    ),
    (
        "signer_revoked",
        "The signing key was revoked. Sign the package with a live trusted key and upload again.",
    ),
    (
        "invalid_signature",
        "The signature does not verify with the trusted key. Sign the package again with the key the tenant trusts.",
    ),
];

/// The refusal code of a failed upload, when it is one of [`REFUSALS`].
///
/// The client keeps the server's code for every status
/// ([`ClientError::code`], CLOACINA-T-1141), 400 and 403 included.
fn refusal_code(e: &ClientError) -> Option<&'static str> {
    let code = e.code()?;
    REFUSALS.iter().map(|(c, _)| *c).find(|c| *c == code)
}

/// What the form shows for a failed upload: a title (the refusal code, when
/// the signature check refused it) and the text.
fn upload_error(e: &ClientError) -> (Option<&'static str>, String) {
    match refusal_code(e) {
        Some(code) => {
            let advice = REFUSALS
                .iter()
                .find(|(c, _)| *c == code)
                .map(|(_, a)| *a)
                .unwrap_or_default();
            // The server's own words, without the client's "authentication:"
            // prefix: a 403 here is the signature check, not the key.
            let detail = match e {
                ClientError::Auth(m) | ClientError::InvalidRequest(m) => m.to_string(),
                other => other.to_string(),
            };
            (Some(code), format!("{advice} Server: {detail}"))
        }
        None => (None, e.to_string()),
    }
}

#[component]
pub fn WorkflowUpload() -> impl IntoView {
    let auth = use_auth();
    let file = RwSignal::new(Option::<web_sys::File>::None);
    let sig_file = RwSignal::new(Option::<web_sys::File>::None);
    let uploading = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let error_code = RwSignal::new(Option::<&'static str>::None);
    let uploaded = RwSignal::new(Option::<String>::None); // package name for the View link
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let sig_input_ref = NodeRef::<leptos::html::Input>::new();

    let clear_result = move || {
        error.set(String::new());
        error_code.set(None);
        uploaded.set(None);
    };

    let on_pick = move |_| {
        if let Some(input) = input_ref.get_untracked() {
            let el: &web_sys::HtmlInputElement = &input;
            file.set(el.files().and_then(|l| l.get(0)));
            clear_result();
        }
    };

    let on_pick_sig = move |_| {
        if let Some(input) = sig_input_ref.get_untracked() {
            let el: &web_sys::HtmlInputElement = &input;
            sig_file.set(el.files().and_then(|l| l.get(0)));
            clear_result();
        }
    };

    let clear_sig = move |_| {
        sig_file.set(None);
        if let Some(input) = sig_input_ref.get_untracked() {
            let el: &web_sys::HtmlInputElement = &input;
            el.set_value("");
        }
        clear_result();
    };

    let do_upload = move || {
        let Some(f) = file.get_untracked() else {
            return;
        };
        let sig = sig_file.get_untracked();
        let Some(conn) = auth.connection() else {
            return;
        };
        uploading.set(true);
        clear_result();
        leptos::task::spawn_local(async move {
            let result = async {
                let bytes = file_bytes(&f).await.map_err(|e| (None, e))?;
                let signature = match &sig {
                    Some(s) => Some(
                        String::from_utf8(file_bytes(s).await.map_err(|e| (None, e))?)
                            .map_err(|_| (None, "the signature file is not text".to_string()))?,
                    ),
                    None => None,
                };
                let client = client_for(&conn).map_err(|e| (None, e))?;
                send_upload(&client, bytes, signature.as_deref())
                    .await
                    .map_err(|e| upload_error(&e))
            }
            .await;
            uploading.set(false);
            match result {
                Ok(res) => uploaded.set(Some(res.package_id)),
                Err((code, text)) => {
                    error_code.set(code);
                    error.set(text);
                }
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

                    // The detached signature: the .sig file of `pack --sign`.
                    <input
                        node_ref=sig_input_ref
                        type="file"
                        accept=".sig,application/json"
                        class="cl-sr-only"
                        tabindex="-1"
                        aria-hidden="true"
                        on:change=on_pick_sig
                    />
                    <button
                        type="button"
                        class="app-dropzone"
                        on:click=move |_| {
                            if let Some(input) = sig_input_ref.get_untracked() {
                                input.unchecked_ref::<web_sys::HtmlElement>().click();
                            }
                        }
                    >
                        <span class="app-text app-fg2 app-block">
                            {move || if sig_file.get().is_some() { "Signature file" } else { "Signature file (.sig), if the package is signed" }}
                        </span>
                        <span class="app-dropzone__file" class:app-dropzone__file--set=move || sig_file.get().is_some()>
                            {move || sig_file.get().map(|f| f.name()).unwrap_or_else(|| "optional: click to browse".into())}
                        </span>
                    </button>
                    <Show when=move || sig_file.get().is_some()>
                        <button type="button" class="app-link" on:click=clear_sig>
                            "Remove the signature file"
                        </button>
                    </Show>

                    <Button
                        loading=uploading
                        loading_label="Uploading…"
                        disabled=Signal::derive(move || file.get().is_none())
                        on_click=Callback::new(move |_| do_upload())
                    >
                        "↑ Upload"
                    </Button>

                    <Show when=move || !error.get().is_empty()>
                        <Alert
                            // Built each time the alert shows: an upload
                            // clears the error before it starts.
                            title=error_code
                                .get_untracked()
                                .map(|c| format!("Upload refused: {c}"))
                                .unwrap_or_else(|| "Upload failed".to_string())
                            color=token::BAD
                        >
                            {move || error.get()}
                        </Alert>
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Serve one upload on a local port and give back the raw request.
    async fn capture_upload(signature: Option<&str>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.expect("accept");
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = sock.read(&mut chunk).await.expect("read");
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
                if buf.ends_with(b"--\r\n") || buf.ends_with(b"0\r\n\r\n") {
                    break;
                }
            }
            let body = r#"{"package_id":"p-1","tenant_id":"public"}"#;
            let reply = format!(
                "HTTP/1.1 201 Created\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            sock.write_all(reply.as_bytes()).await.expect("write");
            String::from_utf8_lossy(&buf).into_owned()
        });
        let client = cloacina_client::ClientBuilder::new(format!("http://{addr}"))
            .api_key("test-key")
            .build()
            .expect("client");
        let uploaded = send_upload(&client, b"package bytes".to_vec(), signature)
            .await
            .expect("upload");
        assert_eq!(uploaded.package_id, "p-1");
        server.await.expect("server")
    }

    #[tokio::test]
    async fn a_signature_file_is_sent_as_the_signature_field() {
        let sig = r#"{"version":1,"package_hash":"abc"}"#;
        let request = capture_upload(Some(sig)).await;
        assert!(
            request.contains(r#"name="signature""#),
            "the upload has no signature field:\n{request}"
        );
        assert!(
            request.contains(sig),
            "the field does not hold the file's text"
        );
        assert!(request.contains(r#"name="file""#));
    }

    /// Each refusal as the client gives it: the status and the server's
    /// message, copied from the upload route.
    #[test]
    fn each_refusal_of_the_signature_check_is_named_with_what_to_do() {
        let cases = [
            (
                ClientError::from_status(
                    403,
                    serde_json::json!({"code": "signature_required", "error": "this server requires signed packages and the upload has no 'signature' field; sign the package and send its signature"}),
                ),
                "signature_required",
            ),
            (
                ClientError::from_status(
                    400,
                    serde_json::json!({"code": "malformed_signature", "error": "the 'signature' field is not a detached signature: expected value at line 1 column 1"}),
                ),
                "malformed_signature",
            ),
            (
                ClientError::from_status(
                    400,
                    serde_json::json!({"code": "malformed_signature", "error": "the 'signature' field is not a signature: bad base64"}),
                ),
                "malformed_signature",
            ),
            (
                ClientError::from_status(
                    403,
                    serde_json::json!({"code": "package_tampered", "error": "package contents do not match the signed hash"}),
                ),
                "package_tampered",
            ),
            (
                ClientError::from_status(
                    403,
                    serde_json::json!({"code": "untrusted_signer", "error": "package signed by untrusted key: ab12"}),
                ),
                "untrusted_signer",
            ),
            (
                ClientError::from_status(
                    403,
                    serde_json::json!({"code": "signer_revoked", "error": "package signed by a revoked key: ab12"}),
                ),
                "signer_revoked",
            ),
            (
                ClientError::from_status(
                    403,
                    serde_json::json!({"code": "invalid_signature", "error": "cryptographic signature verification failed"}),
                ),
                "invalid_signature",
            ),
        ];
        for (err, code) in cases {
            let (title, text) = upload_error(&err);
            assert_eq!(title, Some(code), "{err}");
            let advice = REFUSALS.iter().find(|(c, _)| *c == code).unwrap().1;
            assert!(text.starts_with(advice), "{text}");
            let (ClientError::Auth(message) | ClientError::InvalidRequest(message)) = &err else {
                panic!("a 400 or 403: {err}");
            };
            assert!(text.ends_with(&format!("Server: {message}")), "{text}");
        }
    }

    #[test]
    fn package_tampered_names_the_mismatch() {
        let advice = REFUSALS
            .iter()
            .find(|(c, _)| *c == "package_tampered")
            .unwrap()
            .1;
        assert!(advice.contains("other bytes than this package"));
    }

    #[test]
    fn other_failures_are_shown_as_they_are() {
        let err = ClientError::from_status(
            400,
            serde_json::json!({"code": "invalid_request", "error": "no 'file' field in multipart request"}),
        );
        assert_eq!(upload_error(&err), (None, err.to_string()));
        let err = ClientError::from_status(
            403,
            serde_json::json!({"code": "forbidden", "error": "write role required"}),
        );
        assert_eq!(upload_error(&err).0, None);
        let err = ClientError::from_status(
            500,
            serde_json::json!({"code": "untrusted_signer", "error": "x"}),
        );
        assert_eq!(
            upload_error(&err).0,
            Some("untrusted_signer"),
            "a kept code is read as is"
        );
    }

    #[tokio::test]
    async fn no_signature_file_sends_the_archive_only() {
        let request = capture_upload(None).await;
        assert!(request.contains(r#"name="file""#));
        assert!(!request.contains(r#"name="signature""#));
    }
}
