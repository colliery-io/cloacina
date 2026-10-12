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

// CLOACI-T-0821 WASM-operator spike — author fixture.
//
// A minimal, SYNCHRONOUS fidius operator. The existing cloacina
// `CloacinaPlugin` interface is async/tokio (WASM guests have no runtime),
// so the operator contract probed here is deliberately one method with
// primitives in/out. The host binds `Config` once via the macro-emitted
// `fidius-configure` export; `apply` then uses `self.cfg.op` without the
// caller re-passing it. N differently-configured instances coexist.

use fidius_macro::{plugin_impl, plugin_interface};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub op: String,
}

#[plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_guest")]
pub trait MinimalOperator: Send + Sync {
    fn apply(&self, input: String) -> String;
}

pub struct Configured {
    cfg: Config,
}

#[plugin_impl(MinimalOperator, crate = "fidius_guest", config = Config)]
impl MinimalOperator for Configured {
    fn apply(&self, input: String) -> String {
        format!("{}: {}", self.cfg.op, input)
    }
}

impl Configured {
    fn configure(cfg: Config) -> Self {
        Self { cfg }
    }
}
