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

//! Runtime config (CLOACI-I-0117 / OQ-5), semantics identical to the React
//! `config.ts`: a deploy container may inject `window.__CLOACINA_CONFIG__`;
//! otherwise, embedded serving means the API is this same origin. Debug
//! builds (`trunk serve`) prefill — and auto-connect — the compose demo
//! stack credentials; `cfg!(debug_assertions)` is the `import.meta.env.DEV`
//! analogue and is false in any release build.

pub struct RuntimeConfig {
    pub default_server_url: String,
    pub demo_api_key: String,
    pub demo_tenant: String,
    pub demo_auto_connect: bool,
}

/// Read `window.__CLOACINA_CONFIG__.defaultServerUrl`, if injected.
fn injected_server_url() -> Option<String> {
    let window = web_sys::window()?;
    let cfg = js_sys::Reflect::get(&window, &"__CLOACINA_CONFIG__".into()).ok()?;
    if cfg.is_undefined() || cfg.is_null() {
        return None;
    }
    let url = js_sys::Reflect::get(&cfg, &"defaultServerUrl".into()).ok()?;
    let s = url.as_string()?;
    // The deploy container may inject an empty string — treat as absent
    // (the React code used `||`, not `??`).
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn runtime_config() -> RuntimeConfig {
    let dev = cfg!(debug_assertions);
    let default_server_url = injected_server_url().unwrap_or_else(|| {
        if dev {
            "http://localhost:8080".to_string()
        } else {
            // Embedded from cloacina-server: the API is this origin
            // (CLOACI-I-0130 / T-0848). Stays editable on the connect gate.
            web_sys::window()
                .and_then(|w| w.location().origin().ok())
                .unwrap_or_default()
        }
    });
    RuntimeConfig {
        default_server_url,
        demo_api_key: if dev {
            "clk_demo_bootstrap_key_0001".to_string()
        } else {
            String::new()
        },
        demo_tenant: if dev {
            "public".to_string()
        } else {
            String::new()
        },
        demo_auto_connect: dev,
    }
}

/// The app version baked at compile time (parity with `__APP_VERSION__`,
/// which vite injected from package.json — here it's the crate version).
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
