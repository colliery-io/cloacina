/*
 *  Copyright 2025 Colliery Software
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

//! # Trigger Registry
//!
//! Global registry for trigger constructors, similar to the task registry.
//! Triggers registered here are available for use by the TriggerScheduler.

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::Trigger;

/// Type alias for the trigger constructor function stored in the global registry
type TriggerConstructor = Box<dyn Fn() -> Arc<dyn Trigger> + Send + Sync>;

/// Type alias for the global trigger registry
type GlobalTriggerRegistry = Arc<RwLock<HashMap<String, TriggerConstructor>>>;

/// Global registry for automatically registering triggers created with the `#[trigger]` macro
static GLOBAL_TRIGGER_REGISTRY: Lazy<GlobalTriggerRegistry> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Register a trigger constructor function globally.
///
/// This is used internally by the `#[trigger]` macro to automatically register triggers.
/// Most users won't call this directly.
///
/// # Arguments
///
/// * `name` - Unique name for the trigger
/// * `constructor` - Function that creates a new instance of the trigger
///
/// # Example
///
/// ```rust,ignore
/// use cloacina::trigger::{register_trigger_constructor, Trigger};
/// use std::sync::Arc;
///
/// register_trigger_constructor("my_trigger", || {
///     Arc::new(MyTrigger::new())
/// });
/// ```
pub fn register_trigger_constructor<F>(name: impl Into<String>, constructor: F)
where
    F: Fn() -> Arc<dyn Trigger> + Send + Sync + 'static,
{
    let name = name.into();
    let mut registry = GLOBAL_TRIGGER_REGISTRY.write();
    registry.insert(name.clone(), Box::new(constructor));
    tracing::debug!("Registered trigger constructor: {}", name);
}

/// Register a trigger instance directly.
///
/// This is a convenience function for registering a single trigger instance.
///
/// # Arguments
///
/// * `trigger` - The trigger to register
pub fn register_trigger<T: Trigger + Clone + 'static>(trigger: T) {
    let name = trigger.name().to_string();
    register_trigger_constructor(name, move || Arc::new(trigger.clone()));
}

/// Get a trigger instance from the global registry by name.
///
/// # Arguments
///
/// * `name` - The name of the trigger to retrieve
///
/// # Returns
///
/// * `Some(Arc<dyn Trigger>)` - If the trigger exists
/// * `None` - If no trigger with that name is registered
pub fn get_trigger(name: &str) -> Option<Arc<dyn Trigger>> {
    let registry = GLOBAL_TRIGGER_REGISTRY.read();
    registry.get(name).map(|constructor| constructor())
}

/// Get the global trigger registry.
///
/// This provides access to the global trigger registry used by the macro system.
/// Most users won't need to call this directly.
pub fn global_trigger_registry() -> GlobalTriggerRegistry {
    GLOBAL_TRIGGER_REGISTRY.clone()
}

/// Get all registered trigger names.
///
/// # Returns
///
/// A vector of all trigger names currently registered.
pub fn list_triggers() -> Vec<String> {
    let registry = GLOBAL_TRIGGER_REGISTRY.read();
    registry.keys().cloned().collect()
}

/// Get all registered triggers.
///
/// # Returns
///
/// A vector of all trigger instances currently registered.
pub fn get_all_triggers() -> Vec<Arc<dyn Trigger>> {
    let registry = GLOBAL_TRIGGER_REGISTRY.read();
    registry.values().map(|constructor| constructor()).collect()
}

/// Check if a trigger is registered.
///
/// # Arguments
///
/// * `name` - The name of the trigger to check
///
/// # Returns
///
/// `true` if the trigger is registered, `false` otherwise.
pub fn is_trigger_registered(name: &str) -> bool {
    let registry = GLOBAL_TRIGGER_REGISTRY.read();
    registry.contains_key(name)
}

/// Clear all registered triggers.
///
/// This is primarily useful for testing to reset the registry state.
#[cfg(test)]
pub fn clear_triggers() {
    let mut registry = GLOBAL_TRIGGER_REGISTRY.write();
    registry.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigger::{TriggerError, TriggerResult};
    use async_trait::async_trait;
    use serial_test::serial;
    use std::time::Duration;

    #[derive(Debug, Clone)]
    struct TestTrigger {
        name: String,
    }

    impl TestTrigger {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl Trigger for TestTrigger {
        fn name(&self) -> &str {
            &self.name
        }

        fn poll_interval(&self) -> Duration {
            Duration::from_secs(1)
        }

        fn allow_concurrent(&self) -> bool {
            false
        }

        async fn poll(&self) -> Result<TriggerResult, TriggerError> {
            Ok(TriggerResult::Skip)
        }
    }

    // Tests that access the global registry should run serially
    // to avoid interference with each other.

    #[test]
    #[serial]
    fn test_register_and_get_trigger() {
        // Use unique name to avoid conflicts with parallel tests
        let name = "test_register_and_get_trigger_unique_12345";
        let trigger = TestTrigger::new(name);
        register_trigger(trigger);

        assert!(is_trigger_registered(name));
        assert!(!is_trigger_registered("definitely_nonexistent_trigger_xyz"));

        let retrieved = get_trigger(name);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), name);
    }

    #[test]
    #[serial]
    fn test_register_constructor() {
        let name = "test_register_constructor_unique_12345";
        register_trigger_constructor(name, move || Arc::new(TestTrigger::new(name)));

        let trigger = get_trigger(name);
        assert!(trigger.is_some());
        assert_eq!(trigger.unwrap().name(), name);
    }

    #[test]
    #[serial]
    fn test_list_triggers() {
        // Register triggers with unique names
        let name_a = "test_list_triggers_a_unique_12345";
        let name_b = "test_list_triggers_b_unique_12345";

        register_trigger(TestTrigger::new(name_a));
        register_trigger(TestTrigger::new(name_b));

        let names = list_triggers();
        // Just verify our triggers are in the list (other tests may have added more)
        assert!(names.contains(&name_a.to_string()));
        assert!(names.contains(&name_b.to_string()));
    }

    #[test]
    #[serial]
    fn test_get_all_triggers() {
        // Register triggers with unique names
        let name_1 = "test_get_all_triggers_1_unique_12345";
        let name_2 = "test_get_all_triggers_2_unique_12345";

        register_trigger(TestTrigger::new(name_1));
        register_trigger(TestTrigger::new(name_2));

        let triggers = get_all_triggers();
        // Verify our triggers are in the list
        let trigger_names: Vec<_> = triggers.iter().map(|t| t.name()).collect();
        assert!(trigger_names.contains(&name_1));
        assert!(trigger_names.contains(&name_2));
    }

    #[test]
    #[serial]
    fn test_clear_triggers() {
        // This test verifies clear_triggers works. We run it serially
        // to avoid interference with other tests that use the global registry.
        let name = "test_clear_triggers_unique_12345";

        // Register our trigger
        register_trigger(TestTrigger::new(name));

        // Verify it's registered
        assert!(
            is_trigger_registered(name),
            "Trigger should be registered after register_trigger"
        );

        // Clear all triggers
        clear_triggers();

        // Verify our trigger is no longer registered
        assert!(
            !is_trigger_registered(name),
            "Trigger should not be registered after clear_triggers"
        );
    }
}
