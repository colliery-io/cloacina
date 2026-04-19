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
//! Tests that trigger definitions in Manifest are correctly:
//! - Validated via `Manifest::validate()`
//! - Registered/deregistered in the global trigger registry
//! - Discovered for Python packages via `@cloaca.trigger`

use chrono::Utc;
use serial_test::serial;

use cloacina::packaging::{
    Manifest, PackageInfo, PackageLanguage, PythonRuntime, RustRuntime, TaskDefinition,
    TriggerDefinition,
};
use cloacina::trigger::{
    deregister_trigger, is_trigger_registered, register_trigger_constructor, Trigger, TriggerError,
    TriggerResult,
};

fn rust_manifest_with_triggers() -> Manifest {
    Manifest {
        format_version: "2".to_string(),
        package: PackageInfo {
            name: "trigger-test-pkg".to_string(),
            version: "0.1.0".to_string(),
            description: Some("Test package with triggers".to_string()),
            fingerprint: "sha256:test".to_string(),
            targets: vec!["linux-x86_64".to_string(), "macos-arm64".to_string()],
        },
        language: PackageLanguage::Rust,
        python: None,
        rust: Some(RustRuntime {
            library_path: "lib/libtrigger_test.so".to_string(),
        }),
        tasks: vec![TaskDefinition {
            id: "process".to_string(),
            function: "execute_task".to_string(),
            dependencies: vec![],
            description: Some("Process data".to_string()),
            retries: 0,
            timeout_seconds: None,
        }],
        triggers: vec![
            TriggerDefinition {
                name: "file_watcher".to_string(),
                trigger_type: "rust".to_string(),
                workflow: "trigger-test-pkg".to_string(),
                poll_interval: "5s".to_string(),
                allow_concurrent: false,
                config: Some(serde_json::json!({"path": "/inbox/"})),
            },
            TriggerDefinition {
                name: "api_poller".to_string(),
                trigger_type: "http_poll".to_string(),
                workflow: "trigger-test-pkg".to_string(),
                poll_interval: "1m".to_string(),
                allow_concurrent: true,
                config: Some(serde_json::json!({"url": "https://example.com/status"})),
            },
        ],
        created_at: Utc::now(),
        signature: None,
    }
}

#[allow(dead_code)]
fn rust_manifest_no_triggers() -> Manifest {
    Manifest {
        format_version: "2".to_string(),
        package: PackageInfo {
            name: "no-trigger-pkg".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            fingerprint: "sha256:def".to_string(),
            targets: vec!["linux-x86_64".to_string()],
        },
        language: PackageLanguage::Rust,
        python: None,
        rust: Some(RustRuntime {
            library_path: "lib/libworkflow.so".to_string(),
        }),
        tasks: vec![TaskDefinition {
            id: "task1".to_string(),
            function: "execute_task".to_string(),
            dependencies: vec![],
            description: None,
            retries: 0,
            timeout_seconds: None,
        }],
        triggers: vec![],
        created_at: Utc::now(),
        signature: None,
    }
}

#[allow(dead_code)]
fn python_manifest_with_trigger() -> Manifest {
    Manifest {
        format_version: "2".to_string(),
        package: PackageInfo {
            name: "py-trigger-pkg".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            fingerprint: "sha256:pytrig".to_string(),
            targets: vec!["linux-x86_64".to_string(), "macos-arm64".to_string()],
        },
        language: PackageLanguage::Python,
        python: Some(PythonRuntime {
            requires_python: ">=3.10".to_string(),
            entry_module: "workflow.tasks".to_string(),
        }),
        rust: None,
        tasks: vec![TaskDefinition {
            id: "process".to_string(),
            function: "workflow.tasks:process".to_string(),
            dependencies: vec![],
            description: None,
            retries: 0,
            timeout_seconds: None,
        }],
        triggers: vec![TriggerDefinition {
            name: "check_inbox".to_string(),
            trigger_type: "python".to_string(),
            workflow: "py-trigger-pkg".to_string(),
            poll_interval: "30s".to_string(),
            allow_concurrent: false,
            config: None,
        }],
        created_at: Utc::now(),
        signature: None,
    }
}

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

    // Simulate what the reconciler does: register a trigger constructor
    register_trigger_constructor(name, {
        let name = name.to_string();
        move || std::sync::Arc::new(TestTrigger { name: name.clone() })
    });

    // Verify it's registered (reconciler's verification step)
    assert!(is_trigger_registered(name));

    // Get the trigger and verify it works
    let trigger = cloacina::trigger::get_trigger(name).unwrap();
    assert_eq!(trigger.name(), name);
    assert_eq!(trigger.poll_interval(), std::time::Duration::from_secs(5));
    assert!(!trigger.allow_concurrent());

    // Deregister (reconciler's unload step)
    assert!(deregister_trigger(name));
    assert!(!is_trigger_registered(name));
}

#[test]
#[serial]
fn multiple_triggers_register_and_deregister_independently() {
    let names = [
        "integration_multi_trigger_a",
        "integration_multi_trigger_b",
        "integration_multi_trigger_c",
    ];

    // Register all
    for name in &names {
        register_trigger_constructor(*name, {
            let name = name.to_string();
            move || std::sync::Arc::new(TestTrigger { name: name.clone() })
        });
    }

    // All registered
    for name in &names {
        assert!(is_trigger_registered(name), "{} should be registered", name);
    }

    // Deregister middle one
    assert!(deregister_trigger(names[1]));
    assert!(is_trigger_registered(names[0]));
    assert!(!is_trigger_registered(names[1]));
    assert!(is_trigger_registered(names[2]));

    // Deregister rest
    assert!(deregister_trigger(names[0]));
    assert!(deregister_trigger(names[2]));
    for name in &names {
        assert!(
            !is_trigger_registered(name),
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
                "from cloaca import trigger, TriggerResult\n\n@trigger(name='test_inbox_check', poll_interval='10s')\ndef check_inbox():\n    return TriggerResult(should_fire=False)\n"
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

    // Wrap and register — same as the loader does
    let wrapper = std::sync::Arc::new(cloacina_python::trigger::PythonTriggerWrapper::new(
        &triggers[0],
    ));
    let wrapper_clone = wrapper.clone();
    register_trigger_constructor("test_inbox_check", move || wrapper_clone.clone());

    // Verify it's in the global registry
    assert!(is_trigger_registered("test_inbox_check"));

    let trigger = cloacina::trigger::get_trigger("test_inbox_check").unwrap();
    assert_eq!(trigger.name(), "test_inbox_check");
    assert_eq!(trigger.poll_interval(), std::time::Duration::from_secs(10));

    // Cleanup
    deregister_trigger("test_inbox_check");
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
                "from cloaca import trigger, TriggerResult\n\n@trigger(name='test_fire_trigger', poll_interval='1s')\ndef fire_trigger():\n    return TriggerResult(should_fire=True, context={'key': 'value'})\n"
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

// ---------------------------------------------------------------------------
// Tests — manifest validation with triggers
// ---------------------------------------------------------------------------

#[test]
fn manifest_with_triggers_validates_successfully() {
    let manifest = rust_manifest_with_triggers();
    assert!(manifest.validate().is_ok());
}

#[test]
fn manifest_trigger_referencing_package_name_is_valid() {
    let manifest = rust_manifest_with_triggers();
    // triggers reference "trigger-test-pkg" which is the package name
    assert!(manifest.validate().is_ok());
}

#[test]
fn manifest_trigger_referencing_task_id_is_valid() {
    let mut manifest = rust_manifest_with_triggers();
    manifest.triggers[0].workflow = "process".to_string(); // task id
    assert!(manifest.validate().is_ok());
}

#[test]
fn manifest_trigger_referencing_unknown_workflow_fails() {
    let mut manifest = rust_manifest_with_triggers();
    manifest.triggers[0].workflow = "nonexistent".to_string();
    assert!(manifest.validate().is_err());
}

#[test]
fn manifest_duplicate_trigger_names_fails() {
    let mut manifest = rust_manifest_with_triggers();
    manifest.triggers[1].name = "file_watcher".to_string(); // duplicate
    assert!(manifest.validate().is_err());
}

#[test]
fn manifest_trigger_invalid_poll_interval_fails() {
    let mut manifest = rust_manifest_with_triggers();
    manifest.triggers[0].poll_interval = "not_a_duration".to_string();
    assert!(manifest.validate().is_err());
}
