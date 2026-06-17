/*
 *  Copyright 2025-2026 Colliery Software
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

//! # Minimal Context for Workflow Authoring
//!
//! This module provides a minimal `Context` type for sharing data between tasks.
//! It contains only the core data operations without runtime-specific features
//! like database persistence or dependency loading.

use crate::error::ContextError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use tracing::{debug, warn};

/// A context that holds data for pipeline execution.
///
/// The context is a type-safe, serializable container that flows through your pipeline,
/// allowing tasks to share data. It supports JSON serialization and provides key-value
/// access patterns with comprehensive error handling.
///
/// ## Type Parameter
///
/// - `T`: The type of values stored in the context. Must implement `Serialize`, `Deserialize`, and `Debug`.
///
/// ## Examples
///
/// ```rust
/// use cloacina_workflow::Context;
/// use serde_json::Value;
///
/// // Create a context for JSON values
/// let mut context = Context::<Value>::new();
///
/// // Insert and retrieve data
/// context.insert("user_id", serde_json::json!(123)).unwrap();
/// let user_id = context.get("user_id").unwrap();
/// ```
#[derive(Debug)]
pub struct Context<T = serde_json::Value>
where
    T: Serialize + for<'de> Deserialize<'de> + Debug,
{
    data: HashMap<String, T>,
}

impl<T> Context<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Debug,
{
    /// Creates a new empty context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina_workflow::Context;
    ///
    /// let context = Context::<i32>::new();
    /// assert!(context.get("any_key").is_none());
    /// ```
    pub fn new() -> Self {
        debug!("Creating new empty context");
        Self {
            data: HashMap::new(),
        }
    }

    /// Creates a clone of this context's data.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(n) where n is the number of key-value pairs
    /// - Space complexity: O(n) for the cloned data
    pub fn clone_data(&self) -> Self
    where
        T: Clone,
    {
        debug!("Cloning context data");
        Self {
            data: self.data.clone(),
        }
    }

    /// Inserts a value into the context.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to insert (can be any type that converts to String)
    /// * `value` - The value to store
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the insertion was successful
    /// * `Err(ContextError::KeyExists)` - If the key already exists
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina_workflow::{Context, ContextError};
    ///
    /// let mut context = Context::<i32>::new();
    ///
    /// // First insertion succeeds
    /// assert!(context.insert("count", 42).is_ok());
    ///
    /// // Duplicate insertion fails
    /// assert!(matches!(context.insert("count", 43), Err(ContextError::KeyExists(_))));
    /// ```
    pub fn insert(&mut self, key: impl Into<String>, value: T) -> Result<(), ContextError> {
        let key = key.into();
        if self.data.contains_key(&key) {
            warn!("Attempted to insert duplicate key: {}", key);
            return Err(ContextError::KeyExists(key));
        }
        debug!("Inserting value for key: {}", key);
        self.data.insert(key, value);
        Ok(())
    }

    /// Updates an existing value in the context.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to update
    /// * `value` - The new value
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the update was successful
    /// * `Err(ContextError::KeyNotFound)` - If the key doesn't exist
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina_workflow::{Context, ContextError};
    ///
    /// let mut context = Context::<i32>::new();
    /// context.insert("count", 42).unwrap();
    ///
    /// // Update existing key
    /// assert!(context.update("count", 100).is_ok());
    /// assert_eq!(context.get("count"), Some(&100));
    ///
    /// // Update non-existent key fails
    /// assert!(matches!(context.update("missing", 1), Err(ContextError::KeyNotFound(_))));
    /// ```
    pub fn update(&mut self, key: impl Into<String>, value: T) -> Result<(), ContextError> {
        let key = key.into();
        if !self.data.contains_key(&key) {
            warn!("Attempted to update non-existent key: {}", key);
            return Err(ContextError::KeyNotFound(key));
        }
        debug!("Updating value for key: {}", key);
        self.data.insert(key, value);
        Ok(())
    }

    /// Gets a reference to a value from the context.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Returns
    ///
    /// * `Some(&T)` - If the key exists
    /// * `None` - If the key doesn't exist
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina_workflow::Context;
    ///
    /// let mut context = Context::<String>::new();
    /// context.insert("message", "Hello".to_string()).unwrap();
    ///
    /// assert_eq!(context.get("message"), Some(&"Hello".to_string()));
    /// assert_eq!(context.get("missing"), None);
    /// ```
    pub fn get(&self, key: &str) -> Option<&T> {
        debug!("Getting value for key: {}", key);
        self.data.get(key)
    }

    /// Removes and returns a value from the context.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove
    ///
    /// # Returns
    ///
    /// * `Some(T)` - If the key existed and was removed
    /// * `None` - If the key didn't exist
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina_workflow::Context;
    ///
    /// let mut context = Context::<i32>::new();
    /// context.insert("temp", 42).unwrap();
    ///
    /// assert_eq!(context.remove("temp"), Some(42));
    /// assert_eq!(context.get("temp"), None);
    /// assert_eq!(context.remove("missing"), None);
    /// ```
    pub fn remove(&mut self, key: &str) -> Option<T> {
        debug!("Removing value for key: {}", key);
        self.data.remove(key)
    }

    /// Gets a reference to the underlying data HashMap.
    ///
    /// This method provides direct access to the internal data structure
    /// for advanced use cases that need to iterate over all key-value pairs.
    ///
    /// # Returns
    ///
    /// A reference to the HashMap containing all context data
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina_workflow::Context;
    ///
    /// let mut context = Context::<i32>::new();
    /// context.insert("a", 1).unwrap();
    /// context.insert("b", 2).unwrap();
    ///
    /// for (key, value) in context.data() {
    ///     println!("{}: {}", key, value);
    /// }
    /// ```
    pub fn data(&self) -> &HashMap<String, T> {
        &self.data
    }

    /// Consumes the context and returns the underlying data HashMap.
    ///
    /// # Returns
    ///
    /// The HashMap containing all context data
    pub fn into_data(self) -> HashMap<String, T> {
        self.data
    }

    /// Creates a Context from a HashMap.
    ///
    /// # Arguments
    ///
    /// * `data` - The HashMap to use as context data
    ///
    /// # Returns
    ///
    /// A new Context with the provided data
    pub fn from_data(data: HashMap<String, T>) -> Self {
        Self { data }
    }

    /// Serializes the context to a JSON string.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The JSON representation of the context
    /// * `Err(ContextError)` - If serialization fails
    pub fn to_json(&self) -> Result<String, ContextError> {
        debug!("Serializing context to JSON");
        let json = serde_json::to_string(&self.data)?;
        debug!("Context serialized successfully");
        Ok(json)
    }

    /// Deserializes a context from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `json` - The JSON string to deserialize
    ///
    /// # Returns
    ///
    /// * `Ok(Context<T>)` - The deserialized context
    /// * `Err(ContextError)` - If deserialization fails
    pub fn from_json(json: String) -> Result<Self, ContextError> {
        debug!("Deserializing context from JSON");
        let data = serde_json::from_str(&json)?;
        debug!("Context deserialized successfully");
        Ok(Self { data })
    }
}

/// Typed accessors for the task context (`Context<serde_json::Value>`).
///
/// Task bodies operate on a `Context<serde_json::Value>`, so reading an input
/// otherwise means `get(...).and_then(|v| v.as_*()).ok_or_else(...)?` plus a
/// `serde_json::from_value` round-trip, and writing means wrapping every value
/// in `serde_json::json!(...)`. These helpers fold that boilerplate and return
/// a [`TaskError`] so they compose with `?` in a task body (CLOACI-T-0733).
///
/// This mirrors the ergonomics Python authors already get from
/// `context.get(key, default)` / `context.set(key, value)`.
impl Context<serde_json::Value> {
    /// Get a value by key and deserialize it into `V`.
    ///
    /// Returns `Ok(None)` when the key is absent, `Ok(Some(value))` when it is
    /// present and deserializes cleanly, and `Err(TaskError::ValidationFailed)`
    /// when the stored JSON does not match `V` (the message names the key and
    /// target type).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina_workflow::Context;
    ///
    /// let mut ctx = Context::new();
    /// ctx.insert("count", serde_json::json!(7)).unwrap();
    /// let n: Option<i64> = ctx.get_as("count").unwrap();
    /// assert_eq!(n, Some(7));
    /// assert_eq!(ctx.get_as::<i64>("missing").unwrap(), None);
    /// ```
    pub fn get_as<V>(&self, key: &str) -> Result<Option<V>, crate::error::TaskError>
    where
        V: serde::de::DeserializeOwned,
    {
        match self.data.get(key) {
            None => Ok(None),
            Some(value) => serde_json::from_value(value.clone()).map(Some).map_err(|e| {
                crate::error::TaskError::ValidationFailed {
                    message: format!(
                        "context key '{}' could not be read as {}: {}",
                        key,
                        std::any::type_name::<V>(),
                        e
                    ),
                }
            }),
        }
    }

    /// Get a value by key, deserialize it into `V`, and error if the key is
    /// missing.
    ///
    /// `Err(TaskError::ValidationFailed)` when the key is absent or the stored
    /// JSON does not match `V`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina_workflow::Context;
    ///
    /// let mut ctx = Context::new();
    /// ctx.insert("name", serde_json::json!("ada")).unwrap();
    /// let name: String = ctx.get_required("name").unwrap();
    /// assert_eq!(name, "ada");
    /// assert!(ctx.get_required::<String>("missing").is_err());
    /// ```
    pub fn get_required<V>(&self, key: &str) -> Result<V, crate::error::TaskError>
    where
        V: serde::de::DeserializeOwned,
    {
        match self.get_as(key)? {
            Some(value) => Ok(value),
            None => Err(crate::error::TaskError::ValidationFailed {
                message: format!(
                    "required context key '{}' is missing (expected {})",
                    key,
                    std::any::type_name::<V>()
                ),
            }),
        }
    }

    /// Serialize a value and write it under `key`, **upserting** (insert or
    /// overwrite).
    ///
    /// Folds the `serde_json::json!(...)` / `to_value` wrapping — and the
    /// "exists? update : insert" dance — that every context write otherwise
    /// repeats. Upsert semantics mirror Python's `context.set(key, value)`
    /// (unlike the lower-level [`Context::insert`], which errors on an existing
    /// key). Errors with `TaskError::ValidationFailed` only if the value cannot
    /// be serialized.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina_workflow::Context;
    ///
    /// let mut ctx = Context::new();
    /// ctx.insert_as("total", 42u32).unwrap();
    /// assert_eq!(ctx.get_as::<u32>("total").unwrap(), Some(42));
    /// // Upserts — overwriting an existing key is fine.
    /// ctx.insert_as("total", 100u32).unwrap();
    /// assert_eq!(ctx.get_as::<u32>("total").unwrap(), Some(100));
    /// ```
    pub fn insert_as<V>(&mut self, key: impl Into<String>, value: V) -> Result<(), crate::error::TaskError>
    where
        V: serde::Serialize,
    {
        let key = key.into();
        let json = serde_json::to_value(value).map_err(|e| {
            crate::error::TaskError::ValidationFailed {
                message: format!("context key '{}' could not be serialized: {}", key, e),
            }
        })?;
        // Upsert: overwrite if present, insert otherwise.
        self.data.insert(key, json);
        Ok(())
    }
}

impl<T> Default for Context<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_context() -> Context<i32> {
        Context::new()
    }

    #[test]
    fn test_context_operations() {
        let mut context = setup_test_context();

        // Test empty context
        assert!(context.data.is_empty());

        // Test insert and get
        context.insert("test", 42).unwrap();
        assert_eq!(context.get("test"), Some(&42));

        // Test duplicate insert fails
        assert!(matches!(
            context.insert("test", 43),
            Err(ContextError::KeyExists(_))
        ));

        // Test update
        context.update("test", 43).unwrap();
        assert_eq!(context.get("test"), Some(&43));

        // Test update nonexistent key fails
        assert!(matches!(
            context.update("nonexistent", 42),
            Err(ContextError::KeyNotFound(_))
        ));
    }

    #[test]
    fn test_context_serialization() {
        let mut context = setup_test_context();
        context.insert("test", 42).unwrap();

        let json = context.to_json().unwrap();
        let deserialized = Context::<i32>::from_json(json).unwrap();

        assert_eq!(deserialized.get("test"), Some(&42));
    }

    #[test]
    fn test_context_clone_data() {
        let mut context = Context::<i32>::new();
        context.insert("a", 1).unwrap();
        context.insert("b", 2).unwrap();

        let cloned = context.clone_data();
        assert_eq!(cloned.get("a"), Some(&1));
        assert_eq!(cloned.get("b"), Some(&2));
    }

    #[test]
    fn test_context_from_data() {
        let mut data = HashMap::new();
        data.insert("key".to_string(), 42);

        let context = Context::from_data(data);
        assert_eq!(context.get("key"), Some(&42));
    }

    #[test]
    fn test_context_into_data() {
        let mut context = Context::<i32>::new();
        context.insert("key", 42).unwrap();

        let data = context.into_data();
        assert_eq!(data.get("key"), Some(&42));
    }

    // CLOACI-T-0733: typed accessors on Context<serde_json::Value>.
    #[test]
    fn test_typed_accessors_roundtrip() {
        let mut ctx = Context::new();
        ctx.insert_as("count", 7u32).unwrap();
        ctx.insert_as("name", "ada").unwrap();

        // get_as: present + absent
        assert_eq!(ctx.get_as::<u32>("count").unwrap(), Some(7));
        assert_eq!(ctx.get_as::<String>("missing").unwrap(), None);

        // get_required: present
        let name: String = ctx.get_required("name").unwrap();
        assert_eq!(name, "ada");

        // insert_as upserts (overwrites) without erroring
        ctx.insert_as("count", 100u32).unwrap();
        assert_eq!(ctx.get_as::<u32>("count").unwrap(), Some(100));
    }

    #[test]
    fn test_typed_accessor_errors_are_actionable() {
        let mut ctx = Context::new();
        ctx.insert("count", serde_json::json!("not-a-number")).unwrap();

        // Type mismatch names the key and target type.
        let err = ctx.get_as::<u32>("count").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("count"), "msg should name the key: {msg}");

        // Missing required key errors and names the key.
        let err = ctx.get_required::<u32>("absent").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("absent"), "msg should name the key: {msg}");
    }
}
