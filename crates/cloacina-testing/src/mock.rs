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

//! Mock data connection for testing continuous tasks.
//!
//! This module provides [`MockDataConnection`] for testing tasks that
//! interact with external data sources. Available when the `continuous`
//! feature is enabled.
//!
//! **Note**: The `DataConnection` trait is not yet available in `cloacina`.
//! This module provides a standalone mock that will implement the real trait
//! once CLOACI-I-0023 lands.

use serde_json::Value;
use std::any::Any;

/// Descriptor for a mock data connection.
#[derive(Debug, Clone)]
pub struct ConnectionDescriptor {
    /// The type of system (e.g., "postgres", "kafka").
    pub system_type: String,
    /// The location or address of the system.
    pub location: String,
}

/// A mock data connection that returns a user-provided handle.
///
/// Wraps a user-provided value and returns it from `connect()`,
/// allowing tests to inject mock database pools, clients, etc.
///
/// # Example
///
/// ```rust,ignore
/// use cloacina_testing::MockDataConnection;
///
/// let mock = MockDataConnection::new(
///     my_test_pool.clone(),
///     ConnectionDescriptor {
///         system_type: "postgres".into(),
///         location: "test".into(),
///     },
/// );
///
/// let handle = mock.connect();
/// ```
pub struct MockDataConnection<T: Any + Send + Sync + Clone> {
    handle: T,
    descriptor: ConnectionDescriptor,
}

impl<T: Any + Send + Sync + Clone> MockDataConnection<T> {
    /// Create a new mock connection with the given handle and descriptor.
    pub fn new(handle: T, descriptor: ConnectionDescriptor) -> Self {
        Self { handle, descriptor }
    }

    /// Get a clone of the underlying handle.
    pub fn connect(&self) -> T {
        self.handle.clone()
    }

    /// Get the connection descriptor.
    pub fn descriptor(&self) -> &ConnectionDescriptor {
        &self.descriptor
    }

    /// Get system metadata (returns empty JSON object for mocks).
    pub fn system_metadata(&self) -> Value {
        serde_json::json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_connection_connect() {
        let mock = MockDataConnection::new(
            42_i32,
            ConnectionDescriptor {
                system_type: "test".into(),
                location: "memory".into(),
            },
        );

        assert_eq!(mock.connect(), 42);
    }

    #[test]
    fn test_mock_connection_descriptor() {
        let mock = MockDataConnection::new(
            "handle".to_string(),
            ConnectionDescriptor {
                system_type: "postgres".into(),
                location: "localhost:5432".into(),
            },
        );

        assert_eq!(mock.descriptor().system_type, "postgres");
        assert_eq!(mock.descriptor().location, "localhost:5432");
    }

    #[test]
    fn test_mock_connection_metadata() {
        let mock = MockDataConnection::new(
            vec![1, 2, 3],
            ConnectionDescriptor {
                system_type: "kafka".into(),
                location: "broker:9092".into(),
            },
        );

        assert_eq!(mock.system_metadata(), serde_json::json!({}));
    }
}
