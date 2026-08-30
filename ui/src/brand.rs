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

use aurora_leptos::tokens::token;
use leptos::prelude::*;

#[component]
pub fn BrandMark(#[prop(default = 22)] size: u32) -> impl IntoView {
    view! {
        <svg width=size height=size viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M5 4 C5 12, 12 12, 12 19" stroke=token::ICE stroke-width="1.6" stroke-linecap="round" />
            <path d="M12 4 C12 12, 12 12, 12 19" stroke=token::TEAL stroke-width="1.6" stroke-linecap="round" />
            <path d="M19 4 C19 12, 12 12, 12 19" stroke=token::VIOLET stroke-width="1.6" stroke-linecap="round" />
            <circle cx="5" cy="4" r="1.8" fill=token::ICE />
            <circle cx="12" cy="4" r="1.8" fill=token::TEAL />
            <circle cx="19" cy="4" r="1.8" fill=token::VIOLET />
            <circle cx="12" cy="20" r="2" fill="#8fbcff" />
        </svg>
    }
}
