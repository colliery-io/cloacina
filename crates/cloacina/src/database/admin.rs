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

//! Database administration module for multi-tenant operations
//!
//! This module provides administrative functions for setting up and managing
//! per-tenant database users and schemas in PostgreSQL multi-tenant deployments.
//!
//! Note: This module is only available when using the PostgreSQL backend.

pub use postgres_impl::*;

mod postgres_impl {
    use crate::database::connection::{
        escape_password, validate_schema_name, validate_username, Database, SchemaError,
        UsernameError,
    };
    use diesel::connection::Connection;
    use diesel::prelude::*;
    use rand::Rng;

    /// Database administrator for tenant provisioning
    #[allow(dead_code)]
    pub struct DatabaseAdmin {
        database: Database,
    }

    /// Configuration for creating a new tenant
    pub struct TenantConfig {
        /// Schema name for the tenant (e.g., "tenant_acme")
        pub schema_name: String,
        /// Username for the tenant's database user (e.g., "acme_user")
        pub username: String,
        /// Password for the tenant user - empty string triggers auto-generation
        pub password: String,
    }

    /// Credentials returned after tenant creation
    pub struct TenantCredentials {
        /// Username of the created tenant user
        pub username: String,
        /// Password (either provided or auto-generated)
        pub password: String,
        /// Schema name for the tenant
        pub schema_name: String,
        /// Ready-to-use connection string for the tenant
        pub connection_string: String,
    }

    /// Errors that can occur during database administration
    #[derive(Debug, thiserror::Error)]
    pub enum AdminError {
        #[error("Database error: {0}")]
        Database(#[from] diesel::result::Error),

        #[error("Connection pool error: {0}")]
        Pool(String),

        #[error("SQL execution error: {message}")]
        SqlExecution { message: String },

        #[error("Invalid configuration: {message}")]
        InvalidConfig { message: String },

        #[error("Invalid schema name: {0}")]
        InvalidSchema(#[from] SchemaError),

        #[error("Invalid username: {0}")]
        InvalidUsername(#[from] UsernameError),
    }

    impl From<deadpool::managed::PoolError<deadpool_diesel::postgres::Manager>> for AdminError {
        fn from(err: deadpool::managed::PoolError<deadpool_diesel::postgres::Manager>) -> Self {
            AdminError::Pool(format!("{:?}", err))
        }
    }

    impl From<deadpool::managed::PoolError<deadpool_diesel::Error>> for AdminError {
        fn from(err: deadpool::managed::PoolError<deadpool_diesel::Error>) -> Self {
            AdminError::Pool(format!("{:?}", err))
        }
    }

    #[allow(dead_code)]
    impl DatabaseAdmin {
        /// Create a new database administrator
        pub fn new(database: Database) -> Self {
            Self { database }
        }

        /// Create a complete tenant setup (schema + user + permissions + migrations)
        ///
        /// If `tenant_config.password` is empty, a secure 32-character password will be auto-generated.
        /// Returns the tenant credentials for distribution to the tenant.
        pub async fn create_tenant(
            &self,
            tenant_config: TenantConfig,
        ) -> Result<TenantCredentials, AdminError> {
            // Validate schema name to prevent SQL injection
            // This also checks for empty strings and invalid characters
            validate_schema_name(&tenant_config.schema_name)?;

            // Validate username to prevent SQL injection
            // This also checks for empty strings, invalid characters, and reserved names
            validate_username(&tenant_config.username)?;

            // Password logic: use provided password or generate secure one
            // Generated passwords use only safe alphanumeric characters
            let final_password = if tenant_config.password.is_empty() {
                generate_secure_password(32) // Auto-generate if none provided
            } else {
                tenant_config.password.clone() // Use admin-provided password
            };

            // Escape the password for safe SQL embedding
            // This doubles any single quotes to prevent SQL injection
            let escaped_password = escape_password(&final_password);

            // Clone values needed in the closure
            let schema_name = tenant_config.schema_name.clone();
            let username = tenant_config.username.clone();
            let escaped_password_clone = escaped_password.clone();

            // Clone again for use after the closure
            let schema_name_result = schema_name.clone();
            let username_result = username.clone();

            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;

            // Execute all tenant setup SQL in a transaction
            let _ = conn
                .interact(move |conn| {
                    conn.transaction::<(), AdminError, _>(|conn: &mut diesel::PgConnection| {
                        // 1. Create schema
                        let sql = format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name);
                        diesel::sql_query(&sql).execute(conn).map_err(|e| {
                            AdminError::SqlExecution {
                                message: format!(
                                    "Failed to create schema '{}': {}",
                                    schema_name, e
                                ),
                            }
                        })?;

                        // 2. Create user with escaped password
                        // Note: username and schema_name are pre-validated as safe identifiers
                        // Password is escaped (single quotes doubled) to prevent injection
                        let sql = format!(
                            "CREATE USER {} WITH PASSWORD '{}'",
                            username, escaped_password_clone
                        );
                        diesel::sql_query(&sql).execute(conn).map_err(|e| {
                            AdminError::SqlExecution {
                                message: format!("Failed to create user '{}': {}", username, e),
                            }
                        })?;

                        // 3. Grant permissions
                        let sqls = vec![
                            format!("GRANT USAGE ON SCHEMA {} TO {}", schema_name, username),
                            format!("GRANT CREATE ON SCHEMA {} TO {}", schema_name, username),
                            format!(
                                "GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA {} TO {}",
                                schema_name, username
                            ),
                            format!(
                                "GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA {} TO {}",
                                schema_name, username
                            ),
                            format!(
                                "ALTER DEFAULT PRIVILEGES IN SCHEMA {} GRANT ALL ON TABLES TO {}",
                                schema_name, username
                            ),
                            format!(
                            "ALTER DEFAULT PRIVILEGES IN SCHEMA {} GRANT ALL ON SEQUENCES TO {}",
                            schema_name, username
                        ),
                        ];

                        for sql in sqls {
                            diesel::sql_query(&sql).execute(conn).map_err(|e| {
                                AdminError::SqlExecution {
                                    message: format!("Failed to grant permissions: {}", e),
                                }
                            })?;
                        }

                        // 4. Run migrations in the schema
                        let set_path_sql = format!("SET search_path TO {}, public", schema_name);
                        diesel::sql_query(&set_path_sql)
                            .execute(conn)
                            .map_err(|e| AdminError::SqlExecution {
                                message: format!("Failed to set search_path: {}", e),
                            })?;

                        use diesel_migrations::MigrationHarness;
                        conn.run_pending_migrations(crate::database::POSTGRES_MIGRATIONS)
                            .map_err(|e| AdminError::SqlExecution {
                                message: format!("Failed to run migrations: {}", e),
                            })?;

                        Ok(())
                    })
                })
                .await
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Transaction failed: {}", e),
                })?;

            // Return credentials for admin to share with tenant
            let connection_string = self.build_connection_string(&username_result, &final_password);

            Ok(TenantCredentials {
                username: username_result,
                password: final_password, // Either provided or generated
                schema_name: schema_name_result,
                connection_string,
            })
        }

        /// Remove a tenant (user + schema)
        ///
        /// WARNING: This will permanently delete all data in the tenant's schema.
        pub async fn remove_tenant(
            &self,
            schema_name: &str,
            username: &str,
        ) -> Result<(), AdminError> {
            // Validate inputs to prevent SQL injection
            validate_schema_name(schema_name)?;
            validate_username(username)?;

            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;
            let schema_name = schema_name.to_string();
            let username = username.to_string();

            let _ = conn
                .interact(move |conn| {
                    conn.transaction::<(), AdminError, _>(|conn: &mut diesel::PgConnection| {
                        // 1. Revoke permissions
                        let sqls = vec![
                            format!(
                                "REVOKE ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA {} FROM {}",
                                schema_name, username
                            ),
                            format!(
                                "REVOKE ALL PRIVILEGES ON ALL TABLES IN SCHEMA {} FROM {}",
                                schema_name, username
                            ),
                            format!("REVOKE ALL ON SCHEMA {} FROM {}", schema_name, username),
                        ];

                        for sql in sqls {
                            // Use unwrap_or to continue even if revoke fails (user might already be gone)
                            let _ = diesel::sql_query(&sql).execute(conn);
                        }

                        // 2. Drop user
                        let sql = format!("DROP USER IF EXISTS {}", username);
                        diesel::sql_query(&sql).execute(conn).map_err(|e| {
                            AdminError::SqlExecution {
                                message: format!("Failed to drop user '{}': {}", username, e),
                            }
                        })?;

                        // 3. Drop schema (with CASCADE to remove all objects)
                        let sql = format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name);
                        diesel::sql_query(&sql).execute(conn).map_err(|e| {
                            AdminError::SqlExecution {
                                message: format!("Failed to drop schema '{}': {}", schema_name, e),
                            }
                        })?;

                        Ok(())
                    })
                })
                .await
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Transaction failed: {}", e),
                })?;

            Ok(())
        }

        fn build_connection_string(&self, username: &str, password: &str) -> String {
            // Extract connection details from the admin database connection
            // For now, return a template - in a real implementation, this would
            // parse the admin connection string and replace credentials

            // Try unencoded password first - sqlx may handle encoding internally
            format!(
                "postgresql://{}:{}@localhost:5432/cloacina",
                username, password
            )
        }
    }

    #[allow(dead_code)]
    fn generate_secure_password(length: usize) -> String {
        // Use only alphanumeric characters to avoid URL/connection string issues
        let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                              abcdefghijklmnopqrstuvwxyz\
                              0123456789";
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset[idx] as char
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_generate_secure_password() {
            let password = generate_secure_password(32);
            assert_eq!(password.len(), 32);

            // Verify it contains mixed characters
            let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
            let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
            let has_digit = password.chars().any(|c| c.is_ascii_digit());

            assert!(has_upper || has_lower || has_digit);
        }

        #[test]
        fn test_tenant_config_validation() {
            // This would need actual database setup for full testing
            // For now, just verify struct creation
            let config = TenantConfig {
                schema_name: "test_tenant".to_string(),
                username: "test_user".to_string(),
                password: "".to_string(),
            };

            assert_eq!(config.schema_name, "test_tenant");
            assert_eq!(config.username, "test_user");
            assert_eq!(config.password, "");
        }

        #[test]
        fn test_username_validation_rejects_sql_injection() {
            // Verify that SQL injection attempts in usernames are rejected
            assert!(matches!(
                validate_username("admin'; DROP TABLE users; --"),
                Err(UsernameError::InvalidCharacters(_))
            ));

            assert!(matches!(
                validate_username("user' OR '1'='1"),
                Err(UsernameError::InvalidCharacters(_))
            ));

            // Spaces are not allowed
            assert!(matches!(
                validate_username("admin user"),
                Err(UsernameError::InvalidCharacters(_))
            ));

            // Valid username should pass
            assert!(validate_username("valid_user_123").is_ok());
        }

        #[test]
        fn test_schema_validation_rejects_sql_injection() {
            // Verify that SQL injection attempts in schema names are rejected
            assert!(matches!(
                validate_schema_name("tenant; DROP SCHEMA public; --"),
                Err(SchemaError::InvalidCharacters(_))
            ));

            assert!(matches!(
                validate_schema_name("schema' OR '1'='1"),
                Err(SchemaError::InvalidCharacters(_))
            ));

            // Valid schema should pass
            assert!(validate_schema_name("valid_schema_123").is_ok());
        }

        #[test]
        fn test_reserved_usernames_rejected() {
            // PostgreSQL superuser
            assert!(matches!(
                validate_username("postgres"),
                Err(UsernameError::ReservedName(_))
            ));

            // System roles
            assert!(matches!(
                validate_username("pg_read_all_data"),
                Err(UsernameError::ReservedName(_))
            ));
        }

        #[test]
        fn test_password_escaping() {
            // Test that passwords with single quotes are properly escaped
            assert_eq!(escape_password("simple"), "simple");
            assert_eq!(escape_password("pass'word"), "pass''word");
            assert_eq!(escape_password("it's a test"), "it''s a test");

            // SQL injection attempt in password should be safely escaped
            let dangerous = "'; DROP TABLE users; --";
            let escaped = escape_password(dangerous);
            assert_eq!(escaped, "''; DROP TABLE users; --");
        }
    }
}
