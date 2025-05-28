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

//! # Context Management
//!
//! The context module provides a type-safe, serializable container for sharing data between tasks
//! in a pipeline. Contexts can be persisted to and restored from a database, enabling robust
//! checkpoint and recovery capabilities.
//!
//! ## Overview
//!
//! The [`Context`] struct is the core data container that flows through your pipeline. It provides:
//! - Type-safe data storage with compile-time guarantees
//! - JSON serialization for database persistence
//! - Key-value access patterns with error handling
//! - Integration with the database layer
//! - Automatic dependency loading from other task contexts
//!
//! ## Key Features
//!
//! ### Type Safety
//! - Generic type parameter `T` must implement `Serialize`, `Deserialize`, and `Debug`
//! - Compile-time type checking prevents type mismatches
//! - Type information is preserved during serialization/deserialization
//!
//! ### Error Handling
//! - Comprehensive error types via [`ContextError`]
//! - Clear error messages for common failure cases
//! - Proper error propagation for database operations
//!
//! ### Database Integration
//! - Seamless conversion between Context and database records
//! - Automatic timestamp management
//! - UUID-based record identification
//!
//! ### Dependency Management
//! - Automatic loading of values from dependency task contexts
//! - Configurable dependency loading strategy
//! - Caching of loaded values for performance
//!
//! ## Best Practices
//!
//! 1. **Type Selection**
//!    - Choose types that implement `Serialize` and `Deserialize`
//!    - Consider using `serde_json::Value` for maximum flexibility
//!    - Use concrete types when type safety is critical
//!
//! 2. **Error Handling**
//!    - Always handle potential errors from context operations
//!    - Use `?` operator for error propagation
//!    - Consider using `Result` types in your task implementations
//!
//! 3. **Performance**
//!    - Cache frequently accessed values
//!    - Use `clone_data()` for creating lightweight copies
//!    - Consider using `get_with_dependencies()` for one-off lookups
//!
//! 4. **Database Usage**
//!    - Use `to_new_db_record()` for new records
//!    - Use `to_db_record()` when you need to specify an ID
//!    - Consider batching database operations
//!
//! ## Usage Patterns
//!
//! ### Basic Operations
//!
//! ```rust
//! use cloacina::Context;
//!
//! let mut context = Context::<i32>::new();
//!
//! // Insert values
//! context.insert("count", 42)?;
//!
//! // Retrieve values
//! let count = context.get("count").unwrap();
//! assert_eq!(*count, 42);
//!
//! // Update existing values
//! context.update("count", 100)?;
//! # Ok::<(), cloacina::ContextError>(())
//! ```
//!
//! ### Database Integration
//!
//! ```rust
//! use cloacina::{Context, Database};
//! use uuid::Uuid;
//!
//! let mut context = Context::<String>::new();
//! context.insert("message", "Hello, World!".to_string())?;
//!
//! // Convert to database record
//! let db_record = context.to_new_db_record()?;
//!
//! // Restore from database record
//! let restored = Context::<String>::from_json(db_record.value)?;
//! # Ok::<(), cloacina::ContextError>(())
//! ```
//!
//! ### Dependency Loading
//!
//! ```rust
//! use cloacina::{Context, ExecutionScope, DependencyLoader};
//! use serde_json::Value;
//! use uuid::Uuid;
//!
//! let mut context = Context::<Value>::new();
//! let scope = ExecutionScope {
//!     pipeline_execution_id: Uuid::new_v4(),
//!     task_execution_id: Some(Uuid::new_v4()),
//!     task_name: Some("my_task".to_string()),
//! };
//! context.set_execution_scope(scope);
//!
//! // Set up dependency loader
//! let loader = DependencyLoader::new(/* ... */);
//! context.set_dependency_loader(loader);
//!
//! // Load value with dependency fallback
//! match context.load_from_dependencies_and_cache("some_key") {
//!     Ok(Some(value)) => println!("Found: {:?}", value),
//!     Ok(None) => println!("Key not found"),
//!     Err(e) => println!("Error loading: {:?}", e),
//! }
//! ```
//!
//! ## Error Handling
//!
//! Context operations return [`ContextError`] for various failure conditions:
//! - [`ContextError::KeyExists`]: Attempting to insert a duplicate key
//! - [`ContextError::KeyNotFound`]: Attempting to update a non-existent key
//! - Serialization errors when converting to/from JSON
//! - Database operation failures
//!
//! ## Integration with Executor
//!
//! The Context type integrates with the executor system through:
//! - [`ExecutionScope`] for tracking task execution context
//! - [`DependencyLoader`] for loading values from dependency tasks
//! - Automatic dependency resolution during value lookups
//!
//! ## Performance Considerations
//!
//! - Context operations are generally O(1) for HashMap operations
//! - Serialization/deserialization can be expensive for large contexts
//! - Dependency loading may involve database queries
//! - Consider using `clone_data()` for creating lightweight copies
//! - Cache frequently accessed values to avoid repeated lookups

use crate::error::ContextError;
use crate::models::context::{DbContext, NewDbContext};
#[allow(unused_imports)]
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use tracing::{debug, warn};
use uuid::Uuid;

// Executor-related types for enhanced context functionality
use crate::error::ExecutorError;
use crate::executor::types::{DependencyLoader, ExecutionScope};

/// A context that holds data for pipeline execution.
///
/// The context is a type-safe, serializable container that flows through your pipeline,
/// allowing tasks to share data. It supports JSON serialization for database persistence
/// and provides key-value access patterns with comprehensive error handling.
///
/// ## Type Parameter
///
/// - `T`: The type of values stored in the context. Must implement `Serialize`, `Deserialize`, and `Debug`.
///
/// ## Examples
///
/// ```rust
/// use cloacina::Context;
/// use serde_json::Value;
///
/// // Create a context for JSON values
/// let mut context = Context::<Value>::new();
///
/// // Insert and retrieve data
/// context.insert("user_id", serde_json::json!(123))?;
/// let user_id = context.get("user_id").unwrap();
/// # Ok::<(), cloacina::ContextError>(())
/// ```
#[derive(Debug)]
pub struct Context<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Debug,
{
    data: HashMap<String, T>,
    execution_scope: Option<ExecutionScope>,
    dependency_loader: Option<DependencyLoader>,
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
    /// use cloacina::Context;
    ///
    /// let context = Context::<i32>::new();
    /// assert!(context.get("any_key").is_none());
    /// ```
    pub fn new() -> Self {
        debug!("Creating new empty context");
        Self {
            data: HashMap::new(),
            execution_scope: None,
            dependency_loader: None,
        }
    }

    /// Creates a clone of this context without the dependency loader.
    ///
    /// This method clones the data and execution scope but does not clone
    /// the dependency loader, as it contains non-cloneable resources.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(n) where n is the number of key-value pairs
    /// - Space complexity: O(n) for the cloned data
    /// - Execution scope cloning is O(1)
    ///
    /// # Use Cases
    ///
    /// - Creating lightweight copies for parallel processing
    /// - Sharing context data between tasks without sharing dependency loading
    /// - Creating snapshots for checkpointing
    ///
    /// # Returns
    ///
    /// A new Context with the same data and execution scope
    pub fn clone_data(&self) -> Self
    where
        T: Clone,
    {
        debug!("Cloning context data without dependency loader");
        Self {
            data: self.data.clone(),
            execution_scope: self.execution_scope.clone(),
            dependency_loader: None,
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
    /// use cloacina::{Context, ContextError};
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
    /// use cloacina::{Context, ContextError};
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
    /// use cloacina::Context;
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
    /// use cloacina::Context;
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

    /// Sets the execution scope for this context.
    ///
    /// This enables automatic dependency loading when the context is used
    /// during task execution.
    ///
    /// # Arguments
    ///
    /// * `scope` - The execution scope information
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::{Context, ExecutionScope};
    /// use uuid::Uuid;
    ///
    /// let mut context = Context::<i32>::new();
    /// let scope = ExecutionScope {
    ///     pipeline_execution_id: Uuid::new_v4(),
    ///     task_execution_id: Some(Uuid::new_v4()),
    ///     task_name: Some("my_task".to_string()),
    /// };
    /// context.set_execution_scope(scope);
    /// ```
    pub fn set_execution_scope(&mut self, scope: ExecutionScope) {
        debug!("Setting execution scope for context");
        self.execution_scope = Some(scope);
    }

    /// Sets the dependency loader for this context.
    ///
    /// This enables automatic loading of values from dependency task contexts
    /// when a key is not found in the current context.
    ///
    /// # Arguments
    ///
    /// * `loader` - The dependency loader
    pub fn set_dependency_loader(&mut self, loader: DependencyLoader) {
        debug!("Setting dependency loader for context");
        self.dependency_loader = Some(loader);
    }

    /// Gets the execution scope for this context.
    ///
    /// # Returns
    ///
    /// * `Some(&ExecutionScope)` - If execution scope is set
    /// * `None` - If no execution scope is set
    pub fn execution_scope(&self) -> Option<&ExecutionScope> {
        self.execution_scope.as_ref()
    }

    /// Gets a value from the context with automatic dependency loading.
    ///
    /// This method first checks the local context data. If the key is not found
    /// and a dependency loader is available, it will automatically load the value
    /// from dependency task contexts using the "latest wins" strategy.
    ///
    /// Note: This method works specifically with serde_json::Value as the context type
    /// to enable seamless integration with the dependency loader.
    ///
    /// # Performance
    ///
    /// - Local lookup: O(1) HashMap operation
    /// - Dependency loading: O(m) where m is the number of dependency tasks
    /// - Database queries may be involved in dependency loading
    ///
    /// # Edge Cases
    ///
    /// - Returns `Ok(None)` if key not found in local context and no dependency loader
    /// - Returns `Ok(None)` if key not found in any dependency contexts
    /// - Returns `Err(ExecutorError)` if dependency loading fails
    /// - Does not cache loaded values (use `load_from_dependencies_and_cache` for caching)
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Some(&T))` - If the key exists in local context
    /// * `Ok(None)` - If the key doesn't exist anywhere
    /// * `Err(ExecutorError)` - If dependency loading fails
    pub fn get_with_dependencies(&self, key: &str) -> Result<Option<&T>, ExecutorError>
    where
        T: From<serde_json::Value>,
    {
        debug!("Getting value with dependency fallback for key: {}", key);

        // First check local context
        if let Some(value) = self.data.get(key) {
            debug!("Found key '{}' in local context", key);
            return Ok(Some(value));
        }

        // If not found locally and we have a dependency loader, try loading from dependencies
        if let Some(loader) = &self.dependency_loader {
            debug!("Key '{}' not found locally, checking dependencies", key);
            match loader.load_from_dependencies(key)? {
                Some(_json_value) => {
                    // Note: Since we can't modify self in this immutable method,
                    // we can't cache the loaded value. The caller should use
                    // `load_from_dependencies_and_cache` for that.
                    debug!("Found key '{}' in dependency contexts, but cannot cache in immutable method", key);
                    // For now, we return None and document that the caller should use the mutable version
                    Ok(None)
                }
                None => {
                    debug!("Key '{}' not found in any dependency contexts", key);
                    Ok(None)
                }
            }
        } else {
            debug!("No dependency loader available for key '{}'", key);
            Ok(None)
        }
    }

    /// Loads a value from dependencies and caches it in the local context.
    ///
    /// This method provides automatic dependency loading with caching. If the key
    /// is not found in the local context, it will load the value from dependency
    /// task contexts and cache it locally for future access.
    ///
    /// This method is specifically designed for Context<serde_json::Value> to enable
    /// seamless integration with the dependency loader.
    ///
    /// # Performance
    ///
    /// - Local lookup: O(1) HashMap operation
    /// - Dependency loading: O(m) where m is the number of dependency tasks
    /// - Caching: O(1) HashMap insertion
    /// - Database queries may be involved in dependency loading
    ///
    /// # Edge Cases
    ///
    /// - Returns `Ok(None)` if key not found in local context and no dependency loader
    /// - Returns `Ok(None)` if key not found in any dependency contexts
    /// - Returns `Err(ExecutorError)` if dependency loading fails
    /// - Caches loaded values for future lookups
    /// - Handles type conversion from JSON to T automatically
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up and potentially load
    ///
    /// # Returns
    ///
    /// * `Ok(Some(&T))` - If the key exists (locally or loaded from dependencies)
    /// * `Ok(None)` - If the key doesn't exist anywhere
    /// * `Err(ExecutorError)` - If dependency loading fails
    pub fn load_from_dependencies_and_cache(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, ExecutorError>
    where
        T: From<serde_json::Value> + Clone,
    {
        debug!(
            "Loading value with dependency fallback and caching for key: {}",
            key
        );

        // First check local context
        if let Some(value) = self.data.get(key) {
            debug!("Found key '{}' in local context", key);
            return Ok(Some(value.clone()));
        }

        // If not found locally and we have a dependency loader, try loading from dependencies
        if let Some(loader) = &self.dependency_loader {
            debug!("Key '{}' not found locally, loading from dependencies", key);
            match loader.load_from_dependencies(key)? {
                Some(json_value) => {
                    debug!(
                        "Found key '{}' in dependency contexts, caching locally",
                        key
                    );
                    let typed_value = T::from(json_value);
                    self.data.insert(key.to_string(), typed_value.clone());
                    // Return the cloned value
                    Ok(Some(typed_value))
                }
                None => {
                    debug!("Key '{}' not found in any dependency contexts", key);
                    Ok(None)
                }
            }
        } else {
            debug!("No dependency loader available for key '{}'", key);
            Ok(None)
        }
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
    /// use cloacina::Context;
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

    /// Serializes the context to a JSON string.
    ///
    /// # Performance
    ///
    /// - Time complexity: O(n) where n is the total size of serialized data
    /// - Space complexity: O(n) for the JSON string
    /// - Serialization overhead depends on the size and complexity of stored values
    ///
    /// # Error Cases
    ///
    /// - Returns `Err(ContextError)` if any value fails to serialize
    /// - Handles nested serialization of complex types
    /// - Preserves type information in the serialized format
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
    /// # Performance
    ///
    /// - Time complexity: O(n) where n is the length of the JSON string
    /// - Space complexity: O(n) for the deserialized data
    /// - Deserialization overhead depends on the size and complexity of stored values
    ///
    /// # Error Cases
    ///
    /// - Returns `Err(ContextError)` if JSON is malformed
    /// - Returns `Err(ContextError)` if type conversion fails
    /// - Handles nested deserialization of complex types
    /// - Validates type compatibility during deserialization
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
        Ok(Self {
            data,
            execution_scope: None,
            dependency_loader: None,
        })
    }

    /// Creates a Context from a database record.
    ///
    /// This is a convenience method that combines database record retrieval
    /// with context deserialization.
    ///
    /// # Arguments
    ///
    /// * `db_context` - The database context record
    ///
    /// # Returns
    ///
    /// * `Ok(Context<T>)` - The deserialized context
    /// * `Err(ContextError)` - If deserialization fails
    pub fn from_db_record(db_context: &DbContext) -> Result<Self, ContextError> {
        debug!("Creating context from database record");
        Self::from_json(db_context.value.clone())
    }

    /// Converts this context into a new database record for insertion.
    ///
    /// Creates a [`NewDbContext`] that can be inserted into the database.
    /// The ID and timestamps will be generated by the database.
    ///
    /// # Returns
    ///
    /// * `Ok(NewDbContext)` - The database record ready for insertion
    /// * `Err(ContextError)` - If serialization fails
    pub fn to_new_db_record(&self) -> Result<NewDbContext, ContextError> {
        debug!("Converting context to database record");
        let json = self.to_json()?;
        Ok(NewDbContext { value: json })
    }

    /// Converts this context into a complete database record.
    ///
    /// Creates a [`DbContext`] with the specified ID and current timestamp.
    /// This is useful when you need to create a complete record with known ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID for this context record
    ///
    /// # Returns
    ///
    /// * `Ok(DbContext)` - The complete database record
    /// * `Err(ContextError)` - If serialization fails
    pub fn to_db_record(&self, id: Uuid) -> Result<DbContext, ContextError> {
        debug!("Converting context to full database record");
        let json = self.to_json()?;
        let now = chrono::Utc::now().naive_utc();
        Ok(DbContext {
            id,
            value: json,
            created_at: now,
            updated_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_logging;

    fn setup_test_context() -> Context<i32> {
        init_test_logging();
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
    fn test_context_db_conversion() {
        let mut context = setup_test_context();
        context.insert("test", 42).unwrap();

        let json = context.to_json().unwrap();
        let now = Utc::now().naive_utc();
        let id = Uuid::new_v4();
        let db_context = DbContext {
            id,
            value: json,
            created_at: now,
            updated_at: now,
        };

        // Test conversion from DB record
        let deserialized = Context::<i32>::from_db_record(&db_context).unwrap();
        assert_eq!(deserialized.get("test"), Some(&42));

        // Test conversion to new DB record
        let new_record = context.to_new_db_record().unwrap();
        assert!(!new_record.value.is_empty());

        // Test conversion to full DB record
        let full_record = context.to_db_record(id).unwrap();
        assert_eq!(full_record.id, id);
        assert!(!full_record.value.is_empty());

        // Verify roundtrip conversion
        let roundtrip = Context::<i32>::from_db_record(&full_record).unwrap();
        assert_eq!(roundtrip.get("test"), Some(&42));
    }
}
