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
