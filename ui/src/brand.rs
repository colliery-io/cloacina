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

//! The Cloacina "confluence" brand mark — three strokes flowing down into one
//! node. App branding is supplied downstream of the design pack by contract
//! (the pack ships no logo).

use leptos::prelude::*;

/// The mark draws from the Aurora hue tokens through CSS classes
/// (`style/app.css`), so it follows the light and dark themes. SVG
/// presentation attributes do not take `var(--…)`, so the colours are not
/// set as `stroke=`/`fill=` attributes.
#[component]
pub fn BrandMark(#[prop(default = 22)] size: u32) -> impl IntoView {
    view! {
        <svg class="app-brandmark" width=size height=size viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path class="app-brandmark__ice" d="M5 4 C5 12, 12 12, 12 19" />
            <path class="app-brandmark__teal" d="M12 4 C12 12, 12 12, 12 19" />
            <path class="app-brandmark__violet" d="M19 4 C19 12, 12 12, 12 19" />
            <circle class="app-brandmark__ice" cx="5" cy="4" r="1.8" />
            <circle class="app-brandmark__teal" cx="12" cy="4" r="1.8" />
            <circle class="app-brandmark__violet" cx="19" cy="4" r="1.8" />
            <circle class="app-brandmark__node" cx="12" cy="20" r="2" />
        </svg>
    }
}

/// The brand lock-up: the mark and the product name.
#[component]
pub fn Brand(#[prop(default = 22)] size: u32, #[prop(optional)] large: bool) -> impl IntoView {
    view! {
        <span class="app-brand" class:app-brand--lg=large>
            <BrandMark size=size />
            <span class="app-brand__name">"Cloacina"</span>
        </span>
    }
}
