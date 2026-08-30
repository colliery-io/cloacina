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

//! Route map (CLOACI-I-0117 IA) — identical paths to the React SPA so
//! bookmarks, the e2e suite, and the embedded-serving SPA fallback all keep
//! working across the swap.

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Redirect, Route, Router, Routes};
use leptos_router::path;

use crate::auth::{provide_auth, use_auth};
use crate::routes::connect::Connect;
use crate::routes::execution_detail::ExecutionDetail;
use crate::routes::executions::Executions;
use crate::routes::overview::Overview;
use crate::routes::stubs::*;
use crate::routes::workflow_detail::WorkflowDetail;
use crate::routes::triggers::Triggers;
use crate::routes::graphs::Graphs;
use crate::routes::graph_detail::GraphDetail;
use crate::routes::trigger_detail::TriggerDetail;
use crate::routes::operations::Operations;
use crate::routes::workflow_upload::WorkflowUpload;
use crate::routes::workflows::Workflows;
use crate::shell::Shell;

/// Gate: no active connection → the connect screen.
#[component]
fn RequireAuth() -> impl IntoView {
    let auth = use_auth();
    view! {
        <Show
            when=move || auth.connection().is_some()
            fallback=|| view! { <Redirect path="/connect" /> }
        >
            <Shell />
        </Show>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_auth();
    view! {
        // The full Aurora Dark stylesheet, injected at mount. index.html
        // carries a one-rule critical style (the --bg surface) so the
        // pre-wasm blank page doesn't flash light.
        <aurora_leptos::AuroraStyles />
        <Router>
            <Routes fallback=NotFound>
                <Route path=path!("/connect") view=Connect />
                <ParentRoute path=path!("") view=RequireAuth>
                    <Route path=path!("") view=Overview />
                    <Route path=path!("workflows") view=Workflows />
                    <Route path=path!("workflows/upload") view=WorkflowUpload />
                    <Route path=path!("workflows/:name") view=WorkflowDetail />
                    <Route path=path!("executions") view=Executions />
                    <Route path=path!("executions/:id") view=ExecutionDetail />
                    <Route path=path!("triggers") view=Triggers />
                    <Route path=path!("triggers/:name") view=TriggerDetail />
                    <Route path=path!("graphs") view=Graphs />
                    <Route path=path!("graphs/:name") view=GraphDetail />
                    <Route path=path!("operations") view=Operations />
                    <Route path=path!("keys") view=Keys />
                    <Route path=path!("secrets") view=Secrets />
                    <Route path=path!("accounts") view=Accounts />
                    <Route path=path!("fleet") view=Fleet />
                    <Route path=path!("settings") view=Settings />
                </ParentRoute>
            </Routes>
        </Router>
    }
}
