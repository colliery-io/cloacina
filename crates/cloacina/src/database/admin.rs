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
        Database, SchemaError, UsernameError, escape_password, validate_schema_name,
        validate_username,
    };
    use diesel::connection::Connection;
    use diesel::prelude::*;
    use rand::RngExt;

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

        /// The tenant's database user exists already (CLOACINA-T-1035).
        #[error("tenant '{name}' already exists")]
        TenantExists { name: String },
    }

    /// Row of the "does this role exist" check in `create_tenant`.
    #[derive(diesel::QueryableByName)]
    struct RoleCount {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        n: i64,
    }

    /// True when a Postgres error message says the object exists already.
    /// `CREATE USER` answers this when two creates of one tenant run at the
    /// same time and both pass the `pg_roles` check.
    pub(crate) fn is_already_exists_message(message: &str) -> bool {
        message.contains("already exists")
    }

    /// The error for a failed `CREATE USER` (CLOACINA-T-1035).
    pub(crate) fn create_user_error(username: &str, db_message: &str) -> AdminError {
        if is_already_exists_message(db_message) {
            AdminError::TenantExists {
                name: username.to_string(),
            }
        } else {
            AdminError::SqlExecution {
                message: format!("Failed to create user '{}': {}", username, db_message),
            }
        }
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
            let txn_result = conn
                .interact(move |conn| {
                    let result =
                        conn.transaction::<(), AdminError, _>(|conn: &mut diesel::PgConnection| {
                            // 0. Refuse a tenant that exists (CLOACINA-T-1035).
                            // The tenant's role is the mark: the schema is
                            // made with IF NOT EXISTS, the role is not.
                            let existing = diesel::sql_query(
                                "SELECT COUNT(*) AS n FROM pg_roles WHERE rolname = $1",
                            )
                            .bind::<diesel::sql_types::Text, _>(&username)
                            .get_result::<RoleCount>(conn)
                            .map_err(|e| AdminError::SqlExecution {
                                message: format!("Failed to check for user '{}': {}", username, e),
                            })?;
                            if existing.n > 0 {
                                return Err(AdminError::TenantExists {
                                    name: username.clone(),
                                });
                            }

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
                            diesel::sql_query(&sql)
                                .execute(conn)
                                .map_err(|e| create_user_error(&username, &e.to_string()))?;

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

                            // 4. Run migrations in the schema.
                            //
                            // SET LOCAL: the search_path change must die with this
                            // transaction. This connection belongs to the shared
                            // admin pool — a session-level SET here poisoned every
                            // later checkout, scattering admin writes (api_keys!)
                            // into whatever tenant schema was created last. Found
                            // by the T-0645 TS contract suite.
                            let set_path_sql =
                                format!("SET LOCAL search_path TO {}, public", schema_name);
                            diesel::sql_query(&set_path_sql)
                                .execute(conn)
                                .map_err(|e| AdminError::SqlExecution {
                                    message: format!("Failed to set search_path: {}", e),
                                })?;

                            // Under the migration lock (CLOACINA-T-1042).
                            // This is inside the transaction of the tenant
                            // creation: the lock is held until it ends.
                            crate::database::migration_lock::run_pending_postgres_migrations_locked(conn)
                                .map_err(|e| AdminError::SqlExecution {
                                    message: format!("Failed to run migrations: {}", e),
                                })?;

                            Ok(())
                        });
                    // Defense in depth: SET LOCAL above scopes the change to
                    // the transaction, but never let a pooled admin connection
                    // escape with a tenant search_path under any code path.
                    let _ = diesel::sql_query("SET search_path TO public").execute(conn);
                    result
                })
                .await
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Transaction failed: {}", e),
                })?;
            // Propagate tenant-setup SQL failures. The previous shape bound
            // this inner result to `_`, so a failed schema create / migration
            // run still returned Ok(credentials) for a half-created tenant.
            txn_result?;

            // Return credentials for admin to share with tenant
            let connection_string = self.build_connection_string(&username_result, &final_password);

            Ok(TenantCredentials {
                username: username_result,
                password: final_password, // Either provided or generated
                schema_name: schema_name_result,
                connection_string,
            })
        }

        /// Remove a tenant: its rows in the admin schema, its schema and its
        /// user.
        ///
        /// WARNING: This will permanently delete all data in the tenant's schema.
        ///
        /// Everything goes in one transaction, so a failure leaves the tenant
        /// as it was and the error reaches the caller (CLOACINA-T-1068). Each
        /// statement can run again: a second call after a failure finishes
        /// the removal, and a call for a tenant that is gone answers `Ok`.
        ///
        /// The admin-schema rows of the tenant (`schema_name`) are deleted
        /// first, see [`TENANT_ROWS`] (CLOACINA-T-1095): its API keys and
        /// their sessions, accounts, agent limits, pools and rows, signing
        /// keys and the rest. A table the admin schema does not have is
        /// skipped. The audit rows stay; while the schema exists, one row
        /// `tenant.removed` is added for the tenant, and the tenant's own audit
        /// reads give only rows after it. Each lock wait is bounded by
        /// [`REMOVE_LOCK_TIMEOUT`]: a lock held in the schema ends the call
        /// with an error, not a wait with no end.
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
            let org_id = crate::security::tenant_org_id(&schema_name).0;

            let txn_result = conn
                .interact(move |conn| {
                    conn.transaction::<(), AdminError, _>(|conn: &mut diesel::PgConnection| {
                        let sql = format!(
                            "SET LOCAL lock_timeout = '{}ms'",
                            REMOVE_LOCK_TIMEOUT.as_millis()
                        );
                        diesel::sql_query(&sql).execute(conn).map_err(|e| {
                            AdminError::SqlExecution {
                                message: format!("Failed to set the lock timeout: {}", e),
                            }
                        })?;

                        // 0. The tenant's rows in the admin schema
                        // (CLOACINA-T-1095), and the audit boundary.
                        remove_tenant_rows(conn, &schema_name, org_id)?;

                        // 1. Drop schema (with CASCADE to remove all objects).
                        // The schema goes first: its grants and the default
                        // privileges that `create_tenant` gave the user go
                        // with it. While they are there, Postgres refuses to
                        // drop the user.
                        let sql = format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name);
                        diesel::sql_query(&sql).execute(conn).map_err(|e| {
                            AdminError::SqlExecution {
                                message: format!("Failed to drop schema '{}': {}", schema_name, e),
                            }
                        })?;

                        // 2. Drop user
                        let sql = format!("DROP USER IF EXISTS {}", username);
                        diesel::sql_query(&sql).execute(conn).map_err(|e| {
                            AdminError::SqlExecution {
                                message: format!("Failed to drop user '{}': {}", username, e),
                            }
                        })?;

                        Ok(())
                    })
                })
                .await
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Transaction failed: {}", e),
                })?;
            // The result of the transaction is the result of the removal.
            // The previous shape bound it to `_`, so a drop that Postgres
            // refused still answered `Ok`.
            txn_result
        }

        /// Build the tenant's ready-to-use connection string.
        ///
        /// CLOACI-T-0888: this used to return a hardcoded
        /// `postgresql://{user}:{pass}@localhost:5432/cloacina` regardless of
        /// where the admin connection actually pointed, so every credential
        /// issued on a non-default deployment named the wrong host, port and
        /// database. It now derives those coordinates from the admin pool's own
        /// endpoint and substitutes only the credentials.
        ///
        /// The `localhost:5432` form survives ONLY as an unreachable fallback:
        /// this module is postgres-gated, so the endpoint is always present in
        /// practice. It is kept rather than panicking because issuing a
        /// best-effort string is better than failing tenant creation outright
        /// after the schema and role have already been committed.
        fn build_connection_string(&self, username: &str, password: &str) -> String {
            self.database
                .tenant_connection_string(username, password)
                .unwrap_or_else(|| {
                    format!(
                        "postgresql://{}:{}@localhost:5432/cloacina",
                        username, password
                    )
                })
        }

        /// List all non-system schemas (tenant schemas).
        pub async fn list_tenant_schemas(&self) -> Result<Vec<String>, AdminError> {
            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;

            let schemas: Vec<String> = conn
                .interact(|conn| {
                    use diesel::prelude::*;

                    #[derive(diesel::QueryableByName)]
                    struct SchemaRow {
                        #[diesel(sql_type = diesel::sql_types::Text)]
                        nspname: String,
                    }

                    diesel::sql_query(
                        "SELECT nspname::text FROM pg_catalog.pg_namespace \
                         WHERE nspname NOT LIKE 'pg_%' \
                         AND nspname NOT IN ('information_schema', 'public') \
                         ORDER BY nspname",
                    )
                    .load::<SchemaRow>(conn)
                    .map(|rows| rows.into_iter().map(|r| r.nspname).collect())
                })
                .await
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to list schemas: {}", e),
                })?
                .map_err(|e: diesel::result::Error| AdminError::SqlExecution {
                    message: format!("Failed to query schemas: {}", e),
                })?;

            Ok(schemas)
        }

        /// The tenant schemas that hold migrations: each non-system schema
        /// with a `__diesel_schema_migrations` table (CLOACINA-T-1056).
        ///
        /// `create_tenant` makes that table in the transaction that makes the
        /// schema, so every tenant has it. A schema without it is not a
        /// tenant of this server (another application's schema in the same
        /// database) and is not in the list.
        pub async fn list_migrated_tenant_schemas(&self) -> Result<Vec<String>, AdminError> {
            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;

            conn.interact(|conn| {
                diesel::sql_query(
                    "SELECT n.nspname::text AS nspname \
                     FROM pg_catalog.pg_namespace n \
                     JOIN pg_catalog.pg_class c ON c.relnamespace = n.oid \
                     WHERE c.relname = '__diesel_schema_migrations' \
                     AND c.relkind = 'r' \
                     AND n.nspname NOT LIKE 'pg_%' \
                     AND n.nspname NOT IN ('information_schema', 'public') \
                     ORDER BY n.nspname",
                )
                .load::<SchemaName>(conn)
                .map(|rows| rows.into_iter().map(|r| r.nspname).collect())
            })
            .await
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Failed to list schemas: {}", e),
            })?
            .map_err(|e: diesel::result::Error| AdminError::SqlExecution {
                message: format!("Failed to query schemas: {}", e),
            })
        }

        /// Runs the pending migrations of one tenant schema, under the
        /// migration lock (CLOACINA-T-1056). Returns how many ran: zero when
        /// the schema is up to date, or when another process applied them
        /// while this one waited for the lock.
        ///
        /// One transaction: the pending migrations of the schema are applied
        /// together or not at all, so a failure leaves the schema as it was.
        /// The `search_path` change is `SET LOCAL` and ends with the
        /// transaction: the connection belongs to the shared admin pool.
        pub async fn migrate_tenant_schema(&self, schema_name: &str) -> Result<usize, AdminError> {
            validate_schema_name(schema_name)?;
            let schema_name = schema_name.to_string();

            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;

            conn.interact(move |conn| {
                let result =
                    conn.transaction::<usize, AdminError, _>(|conn: &mut diesel::PgConnection| {
                        let set_path_sql =
                            format!("SET LOCAL search_path TO {}, public", schema_name);
                        diesel::sql_query(&set_path_sql)
                            .execute(conn)
                            .map_err(|e| AdminError::SqlExecution {
                                message: format!("Failed to set search_path: {}", e),
                            })?;
                        crate::database::migration_lock::run_pending_postgres_migrations_locked(
                            conn,
                        )
                        .map_err(|e| AdminError::SqlExecution { message: e })
                    });
                // As in `create_tenant`: a pooled admin connection never
                // leaves with a tenant search_path.
                let _ = diesel::sql_query("SET search_path TO public").execute(conn);
                result
            })
            .await
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Transaction failed: {}", e),
            })?
        }
    }

    /// Row of `list_migrated_tenant_schemas`.
    #[derive(diesel::QueryableByName)]
    struct SchemaName {
        #[diesel(sql_type = diesel::sql_types::Text)]
        nspname: String,
    }

    /// The longest wait for one lock in [`DatabaseAdmin::remove_tenant`]. A
    /// connection that holds a lock in the tenant's schema (a runner on
    /// another server) makes the removal fail after this time; a retry
    /// finishes it once the lock is gone.
    pub const REMOVE_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

    /// What [`DatabaseAdmin::remove_tenant`] deletes in the admin schema
    /// (CLOACINA-T-1095), in this order, then the rows of
    /// [`TENANT_FLEET_ROWS`]: the table, and the `WHERE` of the delete. `$1` is the tenant (the schema name), `$2` the tenant's org
    /// (`tenant_org_id`). The sessions go before the keys they name.
    ///
    /// Kept: `audit_events` (the record), `login_throttle` (counters that
    /// expire and grant nothing), `execution_events` and `fleet_result_inbox`
    /// (written in the tenant schema or with no tenant column, removed by age).
    pub const TENANT_ROWS: &[(&str, &str)] = &[
        (
            "oidc_sessions",
            "key_id IN (SELECT id FROM api_keys WHERE tenant_id = $1)",
        ),
        ("api_keys", "tenant_id = $1"),
        ("ws_tickets", "tenant_id = $1"),
        ("local_accounts", "tenant_id = $1"),
        ("delivery_outbox", "tenant_id = $1"),
        ("reactor_firings", "tenant_id = $1"),
        ("reactor_trigger_subscriptions", "tenant_id = $1"),
        ("reactor_owner_addresses", "tenant_id = $1"),
        ("trusted_keys", "org_id = $2::uuid"),
        ("signing_keys", "org_id = $2::uuid"),
        ("package_signatures", "org_id = $2::uuid"),
        (
            "key_trust_acls",
            "parent_org_id = $2::uuid OR child_org_id = $2::uuid",
        ),
        ("secrets", "org_id = $2::uuid"),
        ("tenant_data_keys", "org_id = $2::uuid"),
    ];

    #[derive(diesel::QueryableByName)]
    struct Present {
        #[diesel(sql_type = diesel::sql_types::Bool)]
        present: bool,
    }

    /// Whether the admin schema (the connection's search path) has `table`.
    fn has_table(conn: &mut diesel::PgConnection, table: &str) -> Result<bool, AdminError> {
        diesel::sql_query("SELECT to_regclass($1) IS NOT NULL AS present")
            .bind::<diesel::sql_types::Text, _>(table)
            .get_result::<Present>(conn)
            .map(|r| r.present)
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Failed to look up table '{}': {}", table, e),
            })
    }

    /// The rows that make the fleet loop run agents for a tenant: its limit,
    /// its desired count, its pools and its agents. Tenant removal deletes
    /// them before it drops the schema (`remove_tenant_fleet_rows`), so a
    /// removal that stops later leaves nothing for the loop to start again.
    pub const TENANT_FLEET_ROWS: &[(&str, &str)] = &[
        ("agent_capacity_limits", "tenant_id = $1"),
        ("agent_desired_counts", "tenant_id = $1"),
        ("agent_pools", "tenant_id = $1"),
        ("fleet_agents", "tenant_id = $1"),
    ];

    impl DatabaseAdmin {
        /// Delete the tenant's rows of [`TENANT_FLEET_ROWS`] in one
        /// transaction (CLOACINA-T-1095). Safe to run again. The server calls
        /// it during tenant removal, after the actuator has removed the
        /// tenant's agents and before [`DatabaseAdmin::remove_tenant`].
        pub async fn remove_tenant_fleet_rows(&self, schema_name: &str) -> Result<(), AdminError> {
            validate_schema_name(schema_name)?;
            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;
            let tenant = schema_name.to_string();
            conn.interact(move |conn| {
                conn.transaction::<(), AdminError, _>(|conn: &mut diesel::PgConnection| {
                    delete_rows(conn, &tenant, "", TENANT_FLEET_ROWS)
                })
            })
            .await
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Transaction failed: {}", e),
            })?
        }
    }

    /// Delete the rows of `rows` (table, filter) for `tenant` and `org`.
    /// A table the admin schema does not have is skipped.
    fn delete_rows(
        conn: &mut diesel::PgConnection,
        tenant: &str,
        org: &str,
        rows: &[(&str, &str)],
    ) -> Result<(), AdminError> {
        for (table, filter) in rows {
            if !has_table(conn, table)? {
                continue;
            }
            let sql = format!("DELETE FROM {} WHERE {}", table, filter);
            diesel::sql_query(&sql)
                .bind::<diesel::sql_types::Text, _>(tenant)
                .bind::<diesel::sql_types::Text, _>(org)
                .execute(conn)
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to delete the tenant's rows of '{}': {}", table, e),
                })?;
        }
        Ok(())
    }

    /// Delete the tenant's rows of [`TENANT_ROWS`] and, while its schema
    /// exists, add the `tenant.removed` audit row. Runs inside the
    /// transaction of `remove_tenant`. Safe to run again: a second run finds
    /// nothing to delete and, with the schema gone, adds no audit row.
    fn remove_tenant_rows(
        conn: &mut diesel::PgConnection,
        tenant: &str,
        org_id: uuid::Uuid,
    ) -> Result<(), AdminError> {
        delete_rows(conn, tenant, &org_id.to_string(), TENANT_ROWS)?;
        delete_rows(conn, tenant, "", TENANT_FLEET_ROWS)?;
        if has_table(conn, "audit_events")? {
            diesel::sql_query(
                "INSERT INTO audit_events \
                 (created_at, tenant_id, actor_key_id, actor_key_name, action, target_kind, \
                  target, outcome, detail) \
                 SELECT now() AT TIME ZONE 'UTC', $1, 'system', 'tenant-removal', $2, 'tenant', \
                        $1, 'ok', '{}' \
                 WHERE EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)",
            )
            .bind::<diesel::sql_types::Text, _>(tenant)
            .bind::<diesel::sql_types::Text, _>(
                crate::dal::unified::audit_events::ACTION_TENANT_REMOVED,
            )
            .execute(conn)
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Failed to write the tenant.removed audit row: {}", e),
            })?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn generate_secure_password(length: usize) -> String {
        // Use only alphanumeric characters to avoid URL/connection string issues
        let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                              abcdefghijklmnopqrstuvwxyz\
                              0123456789";
        let mut rng = rand::rng();
        (0..length)
            .map(|_| {
                let idx = rng.random_range(0..charset.len());
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

        /// CLOACINA-T-1035: a `CREATE USER` that fails because the role is
        /// there is "tenant exists", with no SQL text in the message.
        #[test]
        fn create_user_error_maps_an_existing_role_to_tenant_exists() {
            let e = create_user_error("team_a", "role \"team_a\" already exists");
            assert!(matches!(&e, AdminError::TenantExists { name } if name == "team_a"));
            assert_eq!(e.to_string(), "tenant 'team_a' already exists");
        }

        #[test]
        fn create_user_error_keeps_other_failures_as_sql_errors() {
            let e = create_user_error("team_a", "permission denied to create role");
            assert!(matches!(e, AdminError::SqlExecution { .. }));
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
