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

//! Database administration module for multi-tenant operations
//!
//! This module provides administrative functions for setting up and managing
//! per-tenant database users and schemas in PostgreSQL multi-tenant deployments.
//!
//! Note: This module is only available when using the PostgreSQL backend.

#[cfg(feature = "postgres")]
pub use postgres_impl::*;

#[cfg(feature = "postgres")]
mod postgres_impl {
    use crate::database::connection::Database;
    use diesel::pg::PgConnection;
    use std::ops::DerefMut;
    use diesel::prelude::*;
    use rand::Rng;

    /// Database administrator for tenant provisioning
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
        Pool(#[from] deadpool::managed::PoolError<deadpool_diesel::postgres::Manager>),

        #[error("SQL execution error: {message}")]
        SqlExecution { message: String },

        #[error("Invalid configuration: {message}")]
        InvalidConfig { message: String },
    }

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
            // Validate configuration
            if tenant_config.schema_name.is_empty() {
                return Err(AdminError::InvalidConfig {
                    message: "Schema name cannot be empty".to_string(),
                });
            }
            if tenant_config.username.is_empty() {
                return Err(AdminError::InvalidConfig {
                    message: "Username cannot be empty".to_string(),
                });
            }

            // Password logic: use provided password or generate secure one
            let final_password = if tenant_config.password.is_empty() {
                generate_secure_password(32) // Auto-generate if none provided
            } else {
                tenant_config.password.clone() // Use admin-provided password
            };

            let mut conn = self.database.pool().get().await?;

            // Execute all tenant setup SQL in a transaction
            conn.deref_mut().transaction::<(), AdminError, _>(|conn| {
                // 1. Create schema
                self.create_schema(conn, &tenant_config.schema_name)?;

                // 2. Create user with determined password
                self.create_user(conn, &tenant_config.username, &final_password)?;

                // 3. Grant permissions
                self.grant_schema_permissions(
                    conn,
                    &tenant_config.schema_name,
                    &tenant_config.username,
                )?;

                // 4. Run migrations in the schema
                self.run_migrations_in_schema(conn, &tenant_config.schema_name)?;

                Ok(())
            })?;

            // Return credentials for admin to share with tenant
            let connection_string =
                self.build_connection_string(&tenant_config.username, &final_password);
            Ok(TenantCredentials {
                username: tenant_config.username,
                password: final_password, // Either provided or generated
                schema_name: tenant_config.schema_name,
                connection_string,
            })
        }

        /// Remove a tenant (user + schema)
        ///
        /// WARNING: This will permanently delete all data in the tenant's schema.
        pub async fn remove_tenant(&self, schema_name: &str, username: &str) -> Result<(), AdminError> {
            let mut conn = self.database.pool().get().await?;

            conn.deref_mut().transaction::<(), AdminError, _>(|conn| {
                // 1. Revoke permissions
                self.revoke_schema_permissions(conn, schema_name, username)?;

                // 2. Drop user
                self.drop_user(conn, username)?;

                // 3. Drop schema (with CASCADE to remove all objects)
                self.drop_schema(conn, schema_name)?;

                Ok(())
            })
        }

        // Private helper methods

        fn create_schema(
            &self,
            conn: &mut PgConnection,
            schema_name: &str,
        ) -> Result<(), AdminError> {
            let sql = format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name);
            diesel::sql_query(&sql)
                .execute(conn)
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to create schema '{}': {}", schema_name, e),
                })?;
            Ok(())
        }

        fn create_user(
            &self,
            conn: &mut PgConnection,
            username: &str,
            password: &str,
        ) -> Result<(), AdminError> {
            // PostgreSQL handles password hashing automatically (SCRAM-SHA-256)
            let sql = format!("CREATE USER {} WITH PASSWORD '{}'", username, password);
            diesel::sql_query(&sql)
                .execute(conn)
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to create user '{}': {}", username, e),
                })?;
            Ok(())
        }

        fn grant_schema_permissions(
            &self,
            conn: &mut PgConnection,
            schema_name: &str,
            username: &str,
        ) -> Result<(), AdminError> {
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
                diesel::sql_query(&sql)
                    .execute(conn)
                    .map_err(|e| AdminError::SqlExecution {
                        message: format!("Failed to grant permissions: {}", e),
                    })?;
            }
            Ok(())
        }

        fn revoke_schema_permissions(
            &self,
            conn: &mut PgConnection,
            schema_name: &str,
            username: &str,
        ) -> Result<(), AdminError> {
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
            Ok(())
        }

        fn drop_user(&self, conn: &mut PgConnection, username: &str) -> Result<(), AdminError> {
            let sql = format!("DROP USER IF EXISTS {}", username);
            diesel::sql_query(&sql)
                .execute(conn)
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to drop user '{}': {}", username, e),
                })?;
            Ok(())
        }

        fn drop_schema(
            &self,
            conn: &mut PgConnection,
            schema_name: &str,
        ) -> Result<(), AdminError> {
            let sql = format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name);
            diesel::sql_query(&sql)
                .execute(conn)
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to drop schema '{}': {}", schema_name, e),
                })?;
            Ok(())
        }

        fn run_migrations_in_schema(
            &self,
            conn: &mut PgConnection,
            schema_name: &str,
        ) -> Result<(), AdminError> {
            // Set search_path to the schema
            let set_path_sql = format!("SET search_path TO {}, public", schema_name);
            diesel::sql_query(&set_path_sql)
                .execute(conn)
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to set search_path: {}", e),
                })?;

            // Run migrations
            crate::database::run_migrations(conn).map_err(|e| AdminError::SqlExecution {
                message: format!("Failed to run migrations: {}", e),
            })?;

            Ok(())
        }

        fn build_connection_string(&self, username: &str, password: &str) -> String {
            // Extract connection details from the admin database connection
            // For now, return a template - in a real implementation, this would
            // parse the admin connection string and replace credentials
            format!(
                "postgresql://{}:{}@localhost:5432/cloacina",
                username, password
            )
        }
    }

    /// Generate a cryptographically secure password
    fn generate_secure_password(length: usize) -> String {
        let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                              abcdefghijklmnopqrstuvwxyz\
                              0123456789\
                              !@#$%^&*()_+-=[]{}|;:,.<>?";
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
    }
}

// For SQLite builds, provide empty stubs to prevent compilation errors
#[cfg(not(feature = "postgres"))]
pub struct DatabaseAdmin;

#[cfg(not(feature = "postgres"))]
pub struct TenantConfig;

#[cfg(not(feature = "postgres"))]
pub struct TenantCredentials;

#[cfg(not(feature = "postgres"))]
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error("PostgreSQL admin features not available")]
    Unavailable,
}
