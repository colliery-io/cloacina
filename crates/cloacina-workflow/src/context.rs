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
pub struct Context<T>
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
}
