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

//! Wave-gated placeholders. Each carries the page's real title so nav e2e
//! specs bind now; the view arrives with its wave (T-0933/T-0934/T-0935)
//! and deletes its stub.

use aurora_leptos::components::{Empty, PageHeader};
use leptos::prelude::*;

macro_rules! stub {
    ($fn_name:ident, $title:expr, $wave:expr) => {
        #[component]
        pub fn $fn_name() -> impl IntoView {
            view! {
                <PageHeader title=$title />
                <Empty message=concat!("This view arrives with ", $wave, ".") />
            }
        }
    };
}

/// Unmatched in-app path.
#[component]
pub fn NotFound() -> impl IntoView {
    view! {
        <PageHeader title="Not found" />
        <Empty message="No such page." />
    }
}
