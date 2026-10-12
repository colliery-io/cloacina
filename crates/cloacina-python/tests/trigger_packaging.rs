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

//! Integration tests for packaged trigger round-trip.
//!
//! Tests that triggers are correctly:
//! - Registered/deregistered in the global trigger registry
//! - Discovered for Python packages via `@cloaca.trigger`
//!
//! (CLOACI-T-0918: the `Manifest` schema fixtures + validation tests that
//! used to live here were deleted along with the test-only
//! `packaging::manifest_schema` module — shipped packages carry
//! `package.toml`, not that `manifest.json` format.)

use serial_test::serial;

// The `Trigger` trait + its associated error/result types live in
// cloacina-workflow (the leaf crate that packaged cdylibs depend on).
// `cloacina::trigger::TriggerError` is the *engine-side* error, broader
// than the trait error. Implementations of `Trigger` MUST use the
// leaf-crate variant.
use cloacina_workflow::{Trigger, TriggerError, TriggerResult};

/// A simple test trigger for registry round-trip tests.
#[derive(Debug, Clone)]
struct TestTrigger {
    name: String,
}

#[async_trait::async_trait]
impl Trigger for TestTrigger {
    fn name(&self) -> &str {
        &self.name
    }
    fn poll_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(5)
    }
    fn allow_concurrent(&self) -> bool {
        false
    }
    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        Ok(TriggerResult::Skip)
    }
}

// ---------------------------------------------------------------------------
// Tests — trigger registry register/deregister lifecycle
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn trigger_register_verify_deregister_roundtrip() {
    let name = "integration_test_trigger_roundtrip";

    // Simulate what the reconciler does: register a trigger constructor on a
    // scoped runtime.
    let runtime = cloacina::Runtime::empty();
    runtime.register_trigger(name.to_string(), {
        let name = name.to_string();
        move || {
            std::sync::Arc::new(TestTrigger { name: name.clone() }) as std::sync::Arc<dyn Trigger>
        }
    });

    // Verify it's registered (reconciler's verification step)
    assert!(runtime.get_trigger(name).is_some());

    // Get the trigger and verify it works
    let trigger = runtime.get_trigger(name).unwrap();
    assert_eq!(trigger.name(), name);
    assert_eq!(trigger.poll_interval(), std::time::Duration::from_secs(5));
    assert!(!trigger.allow_concurrent());

    // Deregister (reconciler's unload step)
    assert!(runtime.unregister_trigger(name));
    assert!(runtime.get_trigger(name).is_none());
}

#[test]
#[serial]
fn multiple_triggers_register_and_deregister_independently() {
    let names = [
        "integration_multi_trigger_a",
        "integration_multi_trigger_b",
        "integration_multi_trigger_c",
    ];

    let runtime = cloacina::Runtime::empty();

    // Register all
    for name in &names {
        runtime.register_trigger(name.to_string(), {
            let name = name.to_string();
            move || {
                std::sync::Arc::new(TestTrigger { name: name.clone() })
                    as std::sync::Arc<dyn Trigger>
            }
        });
    }

    // All registered
    for name in &names {
        assert!(
            runtime.get_trigger(name).is_some(),
            "{} should be registered",
            name
        );
    }

    // Deregister middle one
    assert!(runtime.unregister_trigger(names[1]));
    assert!(runtime.get_trigger(names[0]).is_some());
    assert!(runtime.get_trigger(names[1]).is_none());
    assert!(runtime.get_trigger(names[2]).is_some());

    // Deregister rest
    assert!(runtime.unregister_trigger(names[0]));
    assert!(runtime.unregister_trigger(names[2]));
    for name in &names {
        assert!(
            runtime.get_trigger(name).is_none(),
            "{} should be deregistered",
            name
        );
    }
}

// ---------------------------------------------------------------------------
// Tests — Python trigger via @cloaca.trigger decorator
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn python_trigger_decorator_registers_and_wraps() {
    pyo3::prepare_freethreaded_python();

    // Drain any leftover triggers from other tests
    cloacina_python::trigger::drain_python_triggers();

    pyo3::Python::with_gil(|py| {
        // Ensure cloaca module is available
        cloacina_python::loader::ensure_cloaca_module(py).unwrap();

        // Define a trigger using @cloaca.trigger decorator
        py.run(
            pyo3::ffi::c_str!(
                "from cloaca import trigger, TriggerResult\n\n@trigger(name='test_inbox_check', poll_interval='10s')\ndef check_inbox():\n    return TriggerResult.skip()\n"
            ),
            None,
            None,
        )
        .unwrap();
    });

    // Drain the registry — this is what import_and_register_python_workflow does
    let triggers = cloacina_python::trigger::drain_python_triggers();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].name, "test_inbox_check");
    assert_eq!(
        triggers[0].poll_interval,
        std::time::Duration::from_secs(10)
    );
    assert!(!triggers[0].allow_concurrent);

    // Wrap and register on a scoped runtime — same as the loader does
    let runtime = cloacina::Runtime::empty();
    let wrapper = std::sync::Arc::new(cloacina_python::trigger::PythonTriggerWrapper::new(
        &triggers[0],
    ));
    let wrapper_clone = wrapper.clone();
    runtime.register_trigger("test_inbox_check".to_string(), move || {
        wrapper_clone.clone() as std::sync::Arc<dyn Trigger>
    });

    // Verify it's in the runtime registry
    assert!(runtime.get_trigger("test_inbox_check").is_some());

    let trigger = runtime.get_trigger("test_inbox_check").unwrap();
    assert_eq!(trigger.name(), "test_inbox_check");
    assert_eq!(trigger.poll_interval(), std::time::Duration::from_secs(10));

    // Cleanup
    runtime.unregister_trigger("test_inbox_check");
}

#[tokio::test]
#[serial]
async fn python_trigger_poll_returns_result() {
    pyo3::prepare_freethreaded_python();
    cloacina_python::trigger::drain_python_triggers();

    pyo3::Python::with_gil(|py| {
        cloacina_python::loader::ensure_cloaca_module(py).unwrap();

        // Define a trigger that fires
        py.run(
            pyo3::ffi::c_str!(
                "from cloaca import trigger, TriggerResult, Context\n\n@trigger(name='test_fire_trigger', poll_interval='1s')\ndef fire_trigger():\n    ctx = Context()\n    ctx.set('key', 'value')\n    return TriggerResult.fire(ctx)\n"
            ),
            None,
            None,
        )
        .unwrap();
    });

    let triggers = cloacina_python::trigger::drain_python_triggers();
    assert_eq!(triggers.len(), 1);

    let wrapper = cloacina_python::trigger::PythonTriggerWrapper::new(&triggers[0]);

    // Poll the trigger — should fire
    let result = wrapper.poll().await.unwrap();
    assert!(result.should_fire());

    // Verify context was passed through
    let context = result.into_context().unwrap();
    assert_eq!(context.get("key").unwrap(), &serde_json::json!("value"));
}
