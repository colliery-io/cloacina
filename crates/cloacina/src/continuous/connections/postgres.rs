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

//! PostgreSQL `DataConnection` implementation.

use crate::continuous::datasource::{ConnectionDescriptor, DataConnection, DataConnectionError};
use serde_json::json;
use std::any::Any;

/// A PostgreSQL data connection for continuous scheduling.
///
/// Provides connection information for a specific table in a PostgreSQL database.
/// The `connect()` method returns the connection URL as a `String` handle.
/// Tasks can use this to establish their own connection pools.
#[derive(Debug, Clone)]
pub struct PostgresConnection {
    /// Database host.
    pub host: String,
    /// Database port.
    pub port: u16,
    /// Database name.
    pub database: String,
    /// Schema name.
    pub schema: String,
    /// Table name.
    pub table: String,
    /// Optional username for connection URL.
    pub username: Option<String>,
}

impl PostgresConnection {
    /// Create a new PostgresConnection.
    pub fn new(host: &str, port: u16, database: &str, schema: &str, table: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            database: database.to_string(),
            schema: schema.to_string(),
            table: table.to_string(),
            username: None,
        }
    }

    /// Set the username for the connection URL.
    pub fn with_username(mut self, username: &str) -> Self {
        self.username = Some(username.to_string());
        self
    }

    /// Build the connection URL.
    pub fn connection_url(&self) -> String {
        match &self.username {
            Some(user) => format!(
                "postgres://{}@{}:{}/{}",
                user, self.host, self.port, self.database
            ),
            None => format!("postgres://{}:{}/{}", self.host, self.port, self.database),
        }
    }
}

impl DataConnection for PostgresConnection {
    fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
        // Returns the connection URL as a String handle.
        // Tasks use this to configure their own connection pools.
        Ok(Box::new(self.connection_url()))
    }

    fn descriptor(&self) -> ConnectionDescriptor {
        ConnectionDescriptor {
            system_type: "postgres".to_string(),
            location: format!("{}:{}/{}.{}", self.host, self.port, self.schema, self.table),
        }
    }

    fn system_metadata(&self) -> serde_json::Value {
        json!({
            "host": self.host,
            "port": self.port,
            "database": self.database,
            "schema": self.schema,
            "table": self.table,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_connection_descriptor() {
        let conn = PostgresConnection::new("localhost", 5432, "mydb", "public", "events");
        let desc = conn.descriptor();
        assert_eq!(desc.system_type, "postgres");
        assert_eq!(desc.location, "localhost:5432/public.events");
    }

    #[test]
    fn test_postgres_connection_metadata() {
        let conn = PostgresConnection::new("db.example.com", 5432, "analytics", "raw", "clicks");
        let meta = conn.system_metadata();
        assert_eq!(meta["host"], "db.example.com");
        assert_eq!(meta["database"], "analytics");
        assert_eq!(meta["schema"], "raw");
        assert_eq!(meta["table"], "clicks");
    }

    #[test]
    fn test_postgres_connection_connect() {
        let conn = PostgresConnection::new("localhost", 5432, "mydb", "public", "events");
        let handle = conn.connect().unwrap();
        let url = handle.downcast::<String>().unwrap();
        assert_eq!(*url, "postgres://localhost:5432/mydb");
    }

    #[test]
    fn test_postgres_connection_with_username() {
        let conn = PostgresConnection::new("localhost", 5432, "mydb", "public", "events")
            .with_username("admin");
        let handle = conn.connect().unwrap();
        let url = handle.downcast::<String>().unwrap();
        assert_eq!(*url, "postgres://admin@localhost:5432/mydb");
    }

    #[test]
    fn test_postgres_connection_url() {
        let conn = PostgresConnection::new("host", 5433, "db", "schema", "table");
        assert_eq!(conn.connection_url(), "postgres://host:5433/db");
    }
}
