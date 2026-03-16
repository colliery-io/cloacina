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

//! Data source types for continuous scheduling.
//!
//! A `DataSource` is a named handle to an external dataset with a connection
//! implementation, change detection, and metadata for lineage tracking.
//!
//! See CLOACI-S-0003 for the full specification.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;

/// Trait for connecting to external data systems.
///
/// Implementations provide typed access to their specific system while
/// exposing generic metadata for framework-level lineage.
pub trait DataConnection: Send + Sync {
    /// Connect to or get a usable handle to the data source.
    fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError>;

    /// Generic lineage descriptor — enough for framework-level graphs (no secrets).
    fn descriptor(&self) -> ConnectionDescriptor;

    /// System-specific metadata as structured Value for detailed lineage.
    fn system_metadata(&self) -> serde_json::Value;
}

/// Error type for data connection operations.
#[derive(Debug, thiserror::Error)]
pub enum DataConnectionError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("configuration error: {0}")]
    ConfigurationError(String),
}

/// Generic lineage descriptor for a data connection.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionDescriptor {
    /// The type of system (e.g., "postgres", "kafka", "s3", "http").
    pub system_type: String,
    /// Human-readable canonical identifier (e.g., "localhost:5432/mydb.public.events").
    pub location: String,
}

impl fmt::Display for ConnectionDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.system_type, self.location)
    }
}

/// Metadata for lineage tracking on a data source.
#[derive(Debug, Clone, Default)]
pub struct DataSourceMetadata {
    /// Human-readable description of the data source.
    pub description: Option<String>,
    /// Owner of the data source (team, person, etc.).
    pub owner: Option<String>,
    /// Tags for categorization and discovery.
    pub tags: Vec<String>,
}

/// A named handle to an external dataset.
///
/// Carries a connection implementation, change detection workflow reference,
/// and metadata for lineage tracking.
pub struct DataSource {
    /// Unique name for this data source.
    pub name: String,
    /// Connection to the external system.
    pub connection: Box<dyn DataConnection>,
    /// Workflow name that detects changes on this data source.
    /// Must produce `DetectorOutput` in output context.
    pub detector_workflow: String,
    /// Lineage metadata.
    pub lineage: DataSourceMetadata,
}

impl fmt::Debug for DataSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataSource")
            .field("name", &self.name)
            .field("descriptor", &self.connection.descriptor())
            .field("detector_workflow", &self.detector_workflow)
            .field("lineage", &self.lineage)
            .finish()
    }
}

/// Errors that can occur during graph operations.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    /// A requested data source was not found.
    #[error("data source not found: '{0}'")]
    SourceNotFound(String),

    /// The connection type does not match the expected type.
    #[error("connection type mismatch for source '{source_name}': expected {expected}")]
    ConnectionTypeMismatch {
        source_name: String,
        expected: String,
    },

    /// A data connection error occurred.
    #[error("data connection error: {0}")]
    ConnectionError(#[from] DataConnectionError),
}

/// A map of data sources provided to continuous tasks at execution time.
///
/// Tasks receive a `&DataSourceMap` and use the typed `connection<T>()` helper
/// to get handles to their input data sources.
pub struct DataSourceMap {
    sources: HashMap<String, DataSource>,
}

impl DataSourceMap {
    /// Create a new empty data source map.
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    /// Insert a data source into the map.
    pub fn insert(&mut self, source: DataSource) {
        self.sources.insert(source.name.clone(), source);
    }

    /// Get a data source by name.
    pub fn get(&self, name: &str) -> Option<&DataSource> {
        self.sources.get(name)
    }

    /// Get a typed connection handle, with a clear error on wiring mismatch.
    ///
    /// This connects to the data source and downcasts the returned handle to
    /// the expected type `T`. Returns a clear error if the source doesn't
    /// exist or the type doesn't match.
    pub fn connection<T: 'static>(&self, name: &str) -> Result<Box<T>, GraphError> {
        let source = self
            .sources
            .get(name)
            .ok_or_else(|| GraphError::SourceNotFound(name.to_string()))?;

        let handle = source.connection.connect()?;

        handle
            .downcast::<T>()
            .map_err(|_| GraphError::ConnectionTypeMismatch {
                source_name: name.to_string(),
                expected: std::any::type_name::<T>().to_string(),
            })
    }

    /// Get the number of data sources in the map.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Iterate over all data source names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.sources.keys().map(|s| s.as_str())
    }
}

impl Default for DataSourceMap {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DataSourceMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataSourceMap")
            .field("sources", &self.sources.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple test connection that returns a String handle.
    struct TestStringConnection {
        value: String,
    }

    impl DataConnection for TestStringConnection {
        fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
            Ok(Box::new(self.value.clone()))
        }

        fn descriptor(&self) -> ConnectionDescriptor {
            ConnectionDescriptor {
                system_type: "test".to_string(),
                location: "memory".to_string(),
            }
        }

        fn system_metadata(&self) -> serde_json::Value {
            serde_json::json!({"value": self.value})
        }
    }

    /// Test connection that returns an i32 handle.
    struct TestIntConnection {
        value: i32,
    }

    impl DataConnection for TestIntConnection {
        fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
            Ok(Box::new(self.value))
        }

        fn descriptor(&self) -> ConnectionDescriptor {
            ConnectionDescriptor {
                system_type: "test_int".to_string(),
                location: "memory".to_string(),
            }
        }

        fn system_metadata(&self) -> serde_json::Value {
            serde_json::json!({"value": self.value})
        }
    }

    fn make_test_source(name: &str, conn: impl DataConnection + 'static) -> DataSource {
        DataSource {
            name: name.to_string(),
            connection: Box::new(conn),
            detector_workflow: format!("detect_{}", name),
            lineage: DataSourceMetadata::default(),
        }
    }

    #[test]
    fn test_datasource_map_typed_access() {
        let mut map = DataSourceMap::new();
        map.insert(make_test_source(
            "events",
            TestStringConnection {
                value: "hello".to_string(),
            },
        ));

        let handle = map.connection::<String>("events").unwrap();
        assert_eq!(*handle, "hello");
    }

    #[test]
    fn test_datasource_map_type_mismatch() {
        let mut map = DataSourceMap::new();
        map.insert(make_test_source(
            "events",
            TestStringConnection {
                value: "hello".to_string(),
            },
        ));

        let result = map.connection::<i32>("events");
        assert!(result.is_err());
        match result.unwrap_err() {
            GraphError::ConnectionTypeMismatch { source_name, .. } => {
                assert_eq!(source_name, "events");
            }
            other => panic!("expected ConnectionTypeMismatch, got: {:?}", other),
        }
    }

    #[test]
    fn test_datasource_map_missing_source() {
        let map = DataSourceMap::new();
        let result = map.connection::<String>("nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            GraphError::SourceNotFound(name) => assert_eq!(name, "nonexistent"),
            other => panic!("expected SourceNotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_datasource_map_multiple_sources() {
        let mut map = DataSourceMap::new();
        map.insert(make_test_source(
            "events",
            TestStringConnection {
                value: "event_pool".to_string(),
            },
        ));
        map.insert(make_test_source("metrics", TestIntConnection { value: 42 }));

        assert_eq!(map.len(), 2);
        assert_eq!(*map.connection::<String>("events").unwrap(), "event_pool");
        assert_eq!(*map.connection::<i32>("metrics").unwrap(), 42);
    }

    #[test]
    fn test_connection_descriptor() {
        let conn = TestStringConnection {
            value: "test".to_string(),
        };
        let desc = conn.descriptor();
        assert_eq!(desc.system_type, "test");
        assert_eq!(desc.location, "memory");
        assert_eq!(desc.to_string(), "test:memory");
    }

    #[test]
    fn test_datasource_debug() {
        let source = make_test_source(
            "events",
            TestStringConnection {
                value: "test".to_string(),
            },
        );
        let debug = format!("{:?}", source);
        assert!(debug.contains("events"));
    }

    #[test]
    fn test_datasource_map_new_is_empty() {
        let map = DataSourceMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_datasource_map_default_is_empty() {
        let map = DataSourceMap::default();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_datasource_map_insert_and_len() {
        let mut map = DataSourceMap::new();
        assert!(map.is_empty());

        map.insert(make_test_source(
            "events",
            TestStringConnection {
                value: "ev".to_string(),
            },
        ));
        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());

        map.insert(make_test_source("metrics", TestIntConnection { value: 7 }));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_datasource_map_get_returns_source() {
        let mut map = DataSourceMap::new();
        map.insert(make_test_source(
            "events",
            TestStringConnection {
                value: "hello".to_string(),
            },
        ));

        let source = map.get("events");
        assert!(source.is_some());
        let source = source.unwrap();
        assert_eq!(source.name, "events");
        assert_eq!(source.detector_workflow, "detect_events");
    }

    #[test]
    fn test_datasource_map_get_returns_none_for_missing() {
        let map = DataSourceMap::new();
        assert!(map.get("nonexistent").is_none());
    }

    #[test]
    fn test_datasource_map_names() {
        let mut map = DataSourceMap::new();
        map.insert(make_test_source(
            "alpha",
            TestStringConnection {
                value: "a".to_string(),
            },
        ));
        map.insert(make_test_source("beta", TestIntConnection { value: 2 }));
        map.insert(make_test_source(
            "gamma",
            TestStringConnection {
                value: "g".to_string(),
            },
        ));

        let mut names: Vec<&str> = map.names().collect();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn test_datasource_map_names_empty() {
        let map = DataSourceMap::new();
        let names: Vec<&str> = map.names().collect();
        assert!(names.is_empty());
    }

    #[test]
    fn test_datasource_map_insert_overwrites_existing() {
        let mut map = DataSourceMap::new();
        map.insert(make_test_source(
            "events",
            TestStringConnection {
                value: "v1".to_string(),
            },
        ));
        map.insert(make_test_source(
            "events",
            TestStringConnection {
                value: "v2".to_string(),
            },
        ));

        assert_eq!(map.len(), 1);
        let handle = map.connection::<String>("events").unwrap();
        assert_eq!(*handle, "v2");
    }

    #[test]
    fn test_datasource_map_debug_format() {
        let mut map = DataSourceMap::new();
        map.insert(make_test_source(
            "events",
            TestStringConnection {
                value: "x".to_string(),
            },
        ));
        let debug = format!("{:?}", map);
        assert!(debug.contains("DataSourceMap"));
        assert!(debug.contains("events"));
    }
}
