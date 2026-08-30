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

//! Cloacina control-plane UI — Leptos CSR (CLOACI-I-0141).
//!
//! The Rust/WASM successor to the React SPA (I-0117/I-0129/I-0130): same
//! routes, same auth/session semantics, same-origin API via the
//! contract-tested `cloacina-client` (wasm transport, T-0932), styled by the
//! Aurora Dark design pack (`colliery-io-aurora`).

mod app;
mod auth;
mod brand;
mod components;
mod config;
mod data;
mod ops;
mod routes;
mod shell;
mod util;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
