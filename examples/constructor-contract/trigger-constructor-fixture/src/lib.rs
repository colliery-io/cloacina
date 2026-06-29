// CLOACI-T-0824 constructor contract — TRIGGER constructor guest fixture.
//
// Proves the TRIGGER-constructor contract shape end-to-end: a SYNCHRONOUS fidius
// constructor implementing `TriggerConstructor` (the per-primitive sync trait),
// compiled to a wasm32-wasip2 component, configured once and polled across the
// sandbox. This is the WASM-compatible analogue of the async
// `cloacina_workflow::Trigger::poll`.
//
// Contract recap (T-0821 constraints):
//   * methods are SYNCHRONOUS — the guest has no async runtime.
//   * config binds ONCE via `configure` (macro-emitted `fidius-configure`).
//   * single-arg method, JSON wire: here `JSON(TriggerInvocation)` in ->
//     `JSON(PollOutcome)` out (String boundary).
//
// The config decides Fire vs Skip, so two loads with different configs yield a
// firing trigger and a skipping one — proving the poll outcome is config-bound.

use fidius_macro::{plugin_impl, plugin_interface};
use constructor_contract::{PollOutcome, TriggerInvocation};
use serde::{Deserialize, Serialize};

/// Per-instance configuration, bound once at load via `configure`.
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Whether this trigger instance fires when polled.
    pub should_fire: bool,
    /// The message written into the fired context's `reason` key.
    pub message: String,
}

/// The TRIGGER-constructor contract. ONE sync method: take the serialized poll
/// invocation, return the serialized outcome.
///
/// Declared with `crate = "fidius_guest"` so the macro emits the guest-side
/// export + descriptor; the host re-declares the SAME trait with
/// `crate = "fidius_core"` to get a matching descriptor + interface hash.
#[plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_guest")]
pub trait TriggerConstructor: Send + Sync {
    /// `JSON(TriggerInvocation)` in -> `JSON(PollOutcome)` out.
    fn poll(&self, invocation_json: String) -> String;
}

pub struct Configured {
    cfg: Config,
}

#[plugin_impl(TriggerConstructor, crate = "fidius_guest", config = Config)]
impl TriggerConstructor for Configured {
    fn poll(&self, invocation_json: String) -> String {
        let outcome = self.run(&invocation_json);
        // The boundary moves a JSON string; never panic across the sandbox.
        serde_json::to_string(&outcome)
            .unwrap_or_else(|e| format!(r#"{{"fire":false,"error":"encode: {e}"}}"#))
    }
}

impl Configured {
    fn configure(cfg: Config) -> Self {
        Self { cfg }
    }

    /// The actual poll body: when `should_fire`, return `Fire` with a context
    /// carrying `reason = message`; otherwise `Skip`.
    fn run(&self, invocation_json: &str) -> PollOutcome {
        // Decode the (currently-empty) invocation envelope — proves the wire
        // shape even though the body ignores it.
        if let Err(e) = serde_json::from_str::<TriggerInvocation>(invocation_json) {
            return PollOutcome::err(format!("decode invocation: {e}"));
        }

        if !self.cfg.should_fire {
            return PollOutcome::skip();
        }

        let ctx = serde_json::json!({ "reason": self.cfg.message });
        match serde_json::to_string(&ctx) {
            Ok(context_json) => PollOutcome::fire(Some(context_json)),
            Err(e) => PollOutcome::err(format!("encode context: {e}")),
        }
    }
}
