// CLOACI-T-0822 constructor contract — TASK constructor guest fixture.
//
// Proves the TASK-constructor contract shape end-to-end: a SYNCHRONOUS fidius
// constructor implementing `TaskConstructor` (the per-primitive sync trait), compiled
// to a wasm32-wasip2 component, configured once and invoked across the sandbox.
//
// Contract recap (T-0821 constraints):
//   * methods are SYNCHRONOUS — the guest has no async runtime.
//   * config binds ONCE via `configure` (macro-emitted `fidius-configure`).
//   * single-arg method, bincode/JSON wire: here `JSON(TaskInvocation)` in ->
//     `JSON(TaskOutcome)` out (String boundary, matching the de-risked spike and
//     cloacina's existing context-as-JSON-string FFI).

use fidius_macro::{plugin_impl, plugin_interface};
use constructor_contract::{TaskInvocation, TaskOutcome};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Per-instance configuration, bound once at load via `configure`.
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Prefix applied to the context's `name` to produce `result`.
    pub prefix: String,
}

/// The TASK-constructor contract. ONE sync method: take the serialized task
/// invocation, return the serialized outcome. This is the WASM-compatible
/// analogue of the async `cloacina_workflow::Task::execute`.
///
/// Declared with `crate = "fidius_guest"` so the macro emits the guest-side
/// export + descriptor; the host re-declares the SAME trait with
/// `crate = "fidius_core"` to get a matching descriptor + interface hash.
#[plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_guest")]
pub trait TaskConstructor: Send + Sync {
    /// `JSON(TaskInvocation)` in -> `JSON(TaskOutcome)` out.
    fn execute(&self, invocation_json: String) -> String;
}

pub struct Configured {
    cfg: Config,
}

#[plugin_impl(TaskConstructor, crate = "fidius_guest", config = Config)]
impl TaskConstructor for Configured {
    fn execute(&self, invocation_json: String) -> String {
        let outcome = self.run(&invocation_json);
        // The boundary moves a JSON string; never panic across the sandbox.
        serde_json::to_string(&outcome)
            .unwrap_or_else(|e| format!(r#"{{"success":false,"error":"encode: {e}"}}"#))
    }
}

impl Configured {
    fn configure(cfg: Config) -> Self {
        Self { cfg }
    }

    /// The actual task body: read `name` from the context, write
    /// `result = "{prefix}{name}"`, hand the updated context back.
    fn run(&self, invocation_json: &str) -> TaskOutcome {
        let inv: TaskInvocation = match serde_json::from_str(invocation_json) {
            Ok(v) => v,
            Err(e) => return TaskOutcome::err(format!("decode invocation: {e}")),
        };
        let mut ctx: Map<String, Value> = match serde_json::from_str(&inv.context_json) {
            Ok(Value::Object(m)) => m,
            Ok(_) => return TaskOutcome::err("context_json is not a JSON object"),
            Err(e) => return TaskOutcome::err(format!("decode context: {e}")),
        };
        let name = match ctx.get("name").and_then(Value::as_str) {
            Some(s) => s.to_string(),
            None => return TaskOutcome::err("context missing required `name`"),
        };
        ctx.insert(
            "result".to_string(),
            Value::String(format!("{}{}", self.cfg.prefix, name)),
        );
        match serde_json::to_string(&Value::Object(ctx)) {
            Ok(updated) => TaskOutcome::ok(updated),
            Err(e) => TaskOutcome::err(format!("encode context: {e}")),
        }
    }
}
