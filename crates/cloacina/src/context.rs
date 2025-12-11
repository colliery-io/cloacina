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
//! - Integration with the database layer via [`ContextDbExt`]
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
//! - Seamless conversion between Context and database records via [`ContextDbExt`]
//! - Automatic timestamp management
//! - UUID-based record identification
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
//! ```rust,ignore
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
//! ```rust,ignore
//! use cloacina::{Context, ContextDbExt};
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

// Re-export the core Context type from cloacina_workflow
// This ensures type compatibility between macro-generated code and runtime
pub use cloacina_workflow::Context;

use crate::error::ContextError;
use crate::models::context::{DbContext, NewDbContext};
use crate::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tracing::debug;
use uuid::Uuid;

/// Extension trait providing database operations for Context.
///
/// This trait adds methods for converting contexts to and from database records,
/// enabling persistent storage of execution state.
///
/// # Examples
///
/// ```rust,ignore
/// use cloacina::{Context, ContextDbExt};
/// use uuid::Uuid;
///
/// let mut context = Context::<String>::new();
/// context.insert("key", "value".to_string()).unwrap();
///
/// // Convert to new database record
/// let new_record = context.to_new_db_record().unwrap();
///
/// // Convert to full database record with ID
/// let full_record = context.to_db_record(Uuid::new_v4()).unwrap();
/// ```
pub trait ContextDbExt<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Debug,
{
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
    fn from_db_record(db_context: &DbContext) -> Result<Context<T>, ContextError>;

    /// Converts this context into a new database record for insertion.
    ///
    /// Creates a [`NewDbContext`] that can be inserted into the database.
    /// The ID and timestamps will be generated by the database.
    ///
    /// # Returns
    ///
    /// * `Ok(NewDbContext)` - The database record ready for insertion
    /// * `Err(ContextError)` - If serialization fails
    fn to_new_db_record(&self) -> Result<NewDbContext, ContextError>;

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
    fn to_db_record(&self, id: Uuid) -> Result<DbContext, ContextError>;
}

impl<T> ContextDbExt<T> for Context<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Debug,
{
    fn from_db_record(db_context: &DbContext) -> Result<Context<T>, ContextError> {
        debug!("Creating context from database record");
        Ok(Context::from_json(db_context.value.clone())?)
    }

    fn to_new_db_record(&self) -> Result<NewDbContext, ContextError> {
        debug!("Converting context to database record");
        let json = self.to_json()?;
        Ok(NewDbContext { value: json })
    }

    fn to_db_record(&self, id: Uuid) -> Result<DbContext, ContextError> {
        debug!("Converting context to full database record");
        let json = self.to_json()?;
        let now = chrono::Utc::now();
        Ok(DbContext {
            id: UniversalUuid(id),
            value: json,
            created_at: UniversalTimestamp(now),
            updated_at: UniversalTimestamp(now),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_logging;
    use chrono::{TimeZone, Utc};

    fn setup_test_context() -> Context<i32> {
        init_test_logging();
        Context::new()
    }

    #[test]
    fn test_context_operations() {
        let mut context = setup_test_context();

        // Test empty context
        assert!(context.data().is_empty());

        // Test insert and get
        context.insert("test", 42).unwrap();
        assert_eq!(context.get("test"), Some(&42));

        // Test duplicate insert fails
        assert!(matches!(
            context.insert("test", 43),
            Err(cloacina_workflow::ContextError::KeyExists(_))
        ));

        // Test update
        context.update("test", 43).unwrap();
        assert_eq!(context.get("test"), Some(&43));

        // Test update nonexistent key fails
        assert!(matches!(
            context.update("nonexistent", 42),
            Err(cloacina_workflow::ContextError::KeyNotFound(_))
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
            id: UniversalUuid(id),
            value: json,
            created_at: UniversalTimestamp(Utc.from_utc_datetime(&now)),
            updated_at: UniversalTimestamp(Utc.from_utc_datetime(&now)),
        };

        // Test conversion from DB record
        let deserialized = Context::<i32>::from_db_record(&db_context).unwrap();
        assert_eq!(deserialized.get("test"), Some(&42));

        // Test conversion to new DB record
        let new_record = context.to_new_db_record().unwrap();
        assert!(!new_record.value.is_empty());

        // Test conversion to full DB record
        let full_record = context.to_db_record(id).unwrap();
        assert_eq!(full_record.id, UniversalUuid(id));
        assert!(!full_record.value.is_empty());

        // Verify roundtrip conversion
        let roundtrip = Context::<i32>::from_db_record(&full_record).unwrap();
        assert_eq!(roundtrip.get("test"), Some(&42));
    }
}
