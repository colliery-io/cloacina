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
            // CLOACINA-T-1080: the schema of the global tables is no tenant.
            if tenant_config
                .schema_name
                .eq_ignore_ascii_case(GLOBAL_SCHEMA)
            {
                return Err(AdminError::InvalidConfig {
                    message: format!(
                        "'{GLOBAL_SCHEMA}' is reserved: it holds the server's global tables"
                    ),
                });
            }

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

                            // 5. The tenant's build role (CLOACINA-T-1063),
                            // still under the migration lock.
                            ensure_build_role_in(conn, &schema_name)?;

                            // 6. The role of the server's pool for the tenant
                            // (CLOACINA-T-1080).
                            ensure_server_role_in(conn, &schema_name)?;

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

                        // 3. Drop the build role (CLOACINA-T-1063) and the
                        // server role (CLOACINA-T-1080). Their grants went
                        // with the schema.
                        for role in [
                            build_role_name(&schema_name),
                            server_role_name(&schema_name),
                        ] {
                            let sql = format!("DROP ROLE IF EXISTS \"{}\"", role);
                            diesel::sql_query(&sql).execute(conn).map_err(|e| {
                                AdminError::SqlExecution {
                                    message: format!("Failed to drop role '{}': {}", role, e),
                                }
                            })?;
                        }

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
                         AND nspname NOT IN ('information_schema', 'public', 'cloacina_global') \
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

        /// Whether `schema_name` holds migrations: a schema with a
        /// `__diesel_schema_migrations` table, as in
        /// [`Self::list_migrated_tenant_schemas`] (CLOACINA-T-1093).
        pub async fn is_migrated_tenant_schema(
            &self,
            schema_name: &str,
        ) -> Result<bool, AdminError> {
            // A bound parameter: any name is safe here, and a name that is
            // no schema is `false`.
            let schema_name = schema_name.to_string();
            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;

            conn.interact(move |conn| {
                diesel::sql_query(
                    "SELECT n.nspname::text AS nspname \
                     FROM pg_catalog.pg_namespace n \
                     JOIN pg_catalog.pg_class c ON c.relnamespace = n.oid \
                     WHERE c.relname = '__diesel_schema_migrations' \
                     AND c.relkind = 'r' \
                     AND n.nspname = $1",
                )
                .bind::<diesel::sql_types::Text, _>(schema_name)
                .load::<SchemaName>(conn)
                .map(|rows| !rows.is_empty())
            })
            .await
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Failed to look up the schema: {}", e),
            })?
            .map_err(|e: diesel::result::Error| AdminError::SqlExecution {
                message: format!("Failed to query the schema: {}", e),
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
                        let ran =
                            crate::database::migration_lock::run_pending_postgres_migrations_locked(
                                conn,
                            )
                            .map_err(|e| AdminError::SqlExecution { message: e })?;
                        // The build role and its grants follow each
                        // migration, whatever role ran it (CLOACINA-T-1063).
                        ensure_build_role_in(conn, &schema_name)?;
                        // So does the server role (CLOACINA-T-1080). A tenant
                        // made before it gets it here, at the next start.
                        ensure_server_role_in(conn, &schema_name)?;
                        Ok(ran)
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

    /// The tables a tenant's compiler uses, all in the tenant's own schema,
    /// and the verbs its build role gets on each (CLOACINA-T-1063). Nothing
    /// else is granted: no other table, no `CREATE`, no default privileges.
    ///
    /// A table that a later migration adds for the compiler is one more line
    /// here: the grants are given again after each migration.
    pub const BUILD_ROLE_GRANTS: &[(&str, &str)] = &[
        ("workflow_packages", "SELECT, UPDATE"),
        ("workflow_registry", "SELECT"),
        ("package_providers", "SELECT, INSERT, DELETE"),
        ("package_artifacts", "SELECT, INSERT, DELETE"),
        // The compiler's schema check at start reads which migrations are
        // applied and which release wrote the schema (CLOACINA-T-1087).
        ("__diesel_schema_migrations", "SELECT"),
        ("cloacina_schema_release", "SELECT"),
    ];

    /// The name of a tenant's build role (CLOACINA-T-1063): `<tenant>$build`,
    /// lower case. A tenant name cannot hold `$`, so no tenant role has this
    /// name. A name that would pass Postgres' 63 bytes is its first 40
    /// characters, `$build$` and 16 hex digits of the SHA-256 of the name.
    /// `public` gets `public$build`.
    ///
    /// The caller passes a schema name that is already checked.
    pub fn build_role_name(schema_name: &str) -> String {
        suffixed_role_name(schema_name, "build")
    }

    /// The name of the role of a tenant's server pool (CLOACINA-T-1080):
    /// `<tenant>$server`, lower case, shortened as [`build_role_name`] is.
    /// No tenant role has this name: a tenant name cannot hold `$`.
    ///
    /// The caller passes a schema name that is already checked.
    pub fn server_role_name(schema_name: &str) -> String {
        suffixed_role_name(schema_name, "server")
    }

    /// `<schema>$<suffix>`, or, past 63 bytes, the start of the name,
    /// `$<suffix>$` and 16 hex digits of the SHA-256 of the name, 63 bytes in
    /// all (the start is 40 characters for `build`, 39 for `server`).
    fn suffixed_role_name(schema_name: &str, suffix: &str) -> String {
        use sha2::Digest;
        let name = schema_name.to_lowercase();
        if name.len() + 1 + suffix.len() <= 63 {
            format!("{name}${suffix}")
        } else {
            let digest = sha2::Sha256::digest(name.as_bytes());
            let short = hex::encode(&digest[..8]);
            let keep = 63 - 2 - suffix.len() - short.len();
            format!("{}${suffix}${}", &name[..keep], short)
        }
    }

    /// A schema that can have a build role: a tenant schema, or `public`.
    fn validate_build_schema(schema_name: &str) -> Result<(), AdminError> {
        if schema_name == "public" {
            return Ok(());
        }
        validate_schema_name(schema_name)?;
        Ok(())
    }

    /// A build role and the password the server just gave it. The password
    /// is in no log and no `Debug` output.
    pub struct BuildCredential {
        pub role: String,
        pub password: String,
    }

    impl std::fmt::Debug for BuildCredential {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("BuildCredential")
                .field("role", &self.role)
                .field("password", &"<redacted>")
                .finish()
        }
    }

    #[derive(diesel::QueryableByName)]
    struct TablePresent {
        #[diesel(sql_type = diesel::sql_types::Bool)]
        present: bool,
    }

    /// Takes the migration lock for the rest of the transaction
    /// (CLOACINA-T-1042). Taking it again in one session is allowed.
    fn take_migration_lock(conn: &mut diesel::PgConnection) -> Result<(), AdminError> {
        diesel::sql_query("SELECT pg_advisory_xact_lock($1)")
            .bind::<diesel::sql_types::BigInt, _>(
                crate::database::migration_lock::MIGRATION_LOCK_KEY,
            )
            .execute(conn)
            .map(|_| ())
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Failed to take the migration lock: {}", e),
            })
    }

    /// Makes the build role of `schema_name` if it is absent and gives it the
    /// grants of [`BUILD_ROLE_GRANTS`] on each of those tables that exists
    /// (CLOACINA-T-1063). Safe to run again. Runs inside a transaction that
    /// holds the migration lock, so two servers never grant at once.
    fn ensure_build_role_in(
        conn: &mut diesel::PgConnection,
        schema_name: &str,
    ) -> Result<(), AdminError> {
        let role = build_role_name(schema_name);
        let existing = diesel::sql_query("SELECT COUNT(*) AS n FROM pg_roles WHERE rolname = $1")
            .bind::<diesel::sql_types::Text, _>(&role)
            .get_result::<RoleCount>(conn)
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Failed to check for role '{}': {}", role, e),
            })?;
        if existing.n == 0 {
            // No password: no one logs in until the server sets one.
            let sql = format!("CREATE ROLE \"{}\" LOGIN NOINHERIT", role);
            diesel::sql_query(&sql)
                .execute(conn)
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to create role '{}': {}", role, e),
                })?;
        }
        let mut grants = vec![format!(
            "GRANT USAGE ON SCHEMA {} TO \"{}\"",
            schema_name, role
        )];
        for (table, verbs) in BUILD_ROLE_GRANTS {
            let qualified = format!("{}.{}", schema_name, table);
            let present = diesel::sql_query("SELECT to_regclass($1) IS NOT NULL AS present")
                .bind::<diesel::sql_types::Text, _>(&qualified)
                .get_result::<TablePresent>(conn)
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to look up table '{}': {}", qualified, e),
                })?
                .present;
            if present {
                grants.push(format!("GRANT {} ON {} TO \"{}\"", verbs, qualified, role));
            }
        }
        for sql in grants {
            diesel::sql_query(&sql)
                .execute(conn)
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to grant to role '{}': {}", role, e),
                })?;
        }
        Ok(())
    }

    /// The tables that a tenant's server pool reads and writes, in the
    /// tenant's own schema (CLOACINA-T-1080). Its role gets
    /// [`SERVER_ROLE_VERBS`] on each, `USAGE` on their sequences and `USAGE`
    /// on the schema: no other table, no `CREATE`, no default privileges.
    /// The list is the same for every tenant, `public` included.
    ///
    /// The global tables ([`SERVER_GLOBAL_TABLES`]) are not here: they are in
    /// the schema [`GLOBAL_SCHEMA`], and the server reads them through the
    /// admin pool only. `delivery_outbox` is here for the schema's own copy:
    /// the run-finished notices of the tenant's runs (the global outbox is
    /// `cloacina_global.delivery_outbox`). The reactive state, the firings and
    /// the reactor subscriptions of a tenant are in its own schema.
    ///
    /// A table that a later migration adds is one more line here, in
    /// [`SERVER_GLOBAL_TABLES`] or in [`SERVER_BOOKKEEPING_TABLES`]: the grants
    /// are given again after each migration, and a test fails on a table in
    /// no list.
    pub const SERVER_ROLE_TABLES: &[&str] = &[
        "accumulator_boundaries",
        "accumulator_checkpoints",
        "contexts",
        "delivery_outbox",
        "execution_events",
        "package_artifacts",
        "package_providers",
        "package_signatures",
        "reactor_firings",
        "reactor_state",
        "reactor_trigger_subscriptions",
        "recovery_events",
        "schedule_executions",
        "schedules",
        "secrets",
        "state_accumulator_buffers",
        "task_attempt_errors",
        "task_execution_metadata",
        "task_executions",
        "tenant_data_keys",
        "workflow_execution_package_pins",
        "workflow_executions",
        "workflow_packages",
        "workflow_registry",
    ];

    /// The verbs of the server role on each table of [`SERVER_ROLE_TABLES`].
    pub const SERVER_ROLE_VERBS: &str = "SELECT, INSERT, UPDATE, DELETE";

    /// The schema of the global tables (CLOACINA-T-1080). A tenant cannot
    /// have this name.
    pub const GLOBAL_SCHEMA: &str = "cloacina_global";

    /// The table in [`GLOBAL_SCHEMA`] whose one row says which release last
    /// moved the global tables (CLOACINA-T-1080): `id = 1`, `written_by`,
    /// `moved_at`. The release check of the schema (CLOACINA-T-1087) reads
    /// `written_by` before the move.
    pub const GLOBAL_LAYOUT_TABLE: &str = "cloacina_global_layout";

    /// The global tables (CLOACINA-T-1080): rows of every tenant, read and
    /// written by the server through the admin pool only. On a server they
    /// are in [`GLOBAL_SCHEMA`]; the server's start moves them there from
    /// `public` ([`DatabaseAdmin::split_global_schema`]). Every schema that
    /// the migrations made has a copy of each; no server role reaches one.
    /// `delivery_outbox` is not here: its global rows move to
    /// `cloacina_global.delivery_outbox`, and each schema keeps its own copy.
    pub const SERVER_GLOBAL_TABLES: &[&str] = &[
        "agent_capacity_limits",
        "agent_desired_counts",
        "agent_pool_scale",
        "agent_pools",
        "api_keys",
        "audit_events",
        "fleet_agents",
        "fleet_result_inbox",
        "key_trust_acls",
        "local_accounts",
        "login_throttle",
        "oidc_login_flows",
        "oidc_sessions",
        "reactor_owner_addresses",
        "signing_keys",
        "trusted_keys",
        "ws_tickets",
    ];

    /// The bookkeeping of each schema (CLOACINA-T-1080): its applied
    /// migrations and its release row (CLOACINA-T-1087). They stay in each
    /// schema; only the admin user writes them, and no server role reaches
    /// them.
    pub const SERVER_BOOKKEEPING_TABLES: &[&str] =
        &["__diesel_schema_migrations", "cloacina_schema_release"];

    #[derive(diesel::QueryableByName)]
    struct SequenceName {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }

    #[derive(diesel::QueryableByName)]
    struct ServerVersion {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        version: i32,
    }

    /// Makes the server role of `schema_name` if it is absent, gives it the
    /// grants of [`SERVER_ROLE_TABLES`] on each of those tables that exists,
    /// and makes the connected user a member that may `SET ROLE` to it
    /// (CLOACINA-T-1080). Safe to run again. Runs inside a transaction that
    /// holds the migration lock, so two servers never grant at once.
    ///
    /// The role is `NOLOGIN`: no password is issued or stored. The server's
    /// pool connects as the admin user and runs `SET ROLE` at each checkout.
    /// The membership needs the admin option on the role: Postgres 16 gives
    /// it to the user that makes the role (with `CREATEROLE`).
    fn ensure_server_role_in(
        conn: &mut diesel::PgConnection,
        schema_name: &str,
    ) -> Result<(), AdminError> {
        let role = server_role_name(schema_name);
        let sql_error = |what: &str, e: diesel::result::Error| AdminError::SqlExecution {
            message: format!("{what} (role '{role}'): {e}"),
        };
        let existing = diesel::sql_query("SELECT COUNT(*) AS n FROM pg_roles WHERE rolname = $1")
            .bind::<diesel::sql_types::Text, _>(&role)
            .get_result::<RoleCount>(conn)
            .map_err(|e| sql_error("Failed to check for the server role", e))?;
        if existing.n == 0 {
            let sql = format!("CREATE ROLE \"{}\" NOLOGIN NOINHERIT", role);
            diesel::sql_query(&sql)
                .execute(conn)
                .map_err(|e| sql_error("Failed to create the server role", e))?;
        }

        let mut grants = vec![format!(
            "GRANT USAGE ON SCHEMA {} TO \"{}\"",
            schema_name, role
        )];
        let mut present_tables = Vec::new();
        for table in SERVER_ROLE_TABLES {
            let qualified = format!("{}.{}", schema_name, table);
            let present = diesel::sql_query("SELECT to_regclass($1) IS NOT NULL AS present")
                .bind::<diesel::sql_types::Text, _>(&qualified)
                .get_result::<TablePresent>(conn)
                .map_err(|e| sql_error(&format!("Failed to look up table '{qualified}'"), e))?
                .present;
            if present {
                grants.push(format!(
                    "GRANT {} ON {} TO \"{}\"",
                    SERVER_ROLE_VERBS, qualified, role
                ));
                present_tables.push(table.to_string());
            }
        }
        // The sequences of those tables (a serial or identity column).
        let sequences = diesel::sql_query(
            "SELECT s.relname::text AS name \
             FROM pg_catalog.pg_class s \
             JOIN pg_catalog.pg_depend d ON d.objid = s.oid \
              AND d.classid = 'pg_catalog.pg_class'::regclass \
              AND d.refclassid = 'pg_catalog.pg_class'::regclass \
             JOIN pg_catalog.pg_class t ON t.oid = d.refobjid \
             JOIN pg_catalog.pg_namespace n ON n.oid = t.relnamespace \
             WHERE s.relkind = 'S' AND n.nspname = $1 AND t.relname = ANY($2) \
             ORDER BY 1",
        )
        .bind::<diesel::sql_types::Text, _>(schema_name)
        .bind::<diesel::sql_types::Array<diesel::sql_types::Text>, _>(&present_tables)
        .load::<SequenceName>(conn)
        .map_err(|e| sql_error("Failed to list the sequences", e))?;
        for sequence in sequences {
            grants.push(format!(
                "GRANT USAGE ON SEQUENCE {}.\"{}\" TO \"{}\"",
                schema_name, sequence.name, role
            ));
        }

        // The connected user may `SET ROLE` to it. On Postgres 16 and later
        // it does not inherit the role's rights; before 16 a plain grant is
        // the only form (the role holds no right the user lacks).
        let version =
            diesel::sql_query("SELECT current_setting('server_version_num')::integer AS version")
                .get_result::<ServerVersion>(conn)
                .map_err(|e| sql_error("Failed to read the server version", e))?
                .version;
        grants.push(if version >= 160000 {
            format!(
                "GRANT \"{}\" TO CURRENT_USER WITH INHERIT FALSE, SET TRUE",
                role
            )
        } else {
            format!("GRANT \"{}\" TO CURRENT_USER", role)
        });

        for sql in grants {
            diesel::sql_query(&sql)
                .execute(conn)
                .map_err(|e| sql_error("Failed to grant", e))?;
        }
        Ok(())
    }

    /// What [`DatabaseAdmin::split_global_schema`] did.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum GlobalSplit {
        /// The tables of [`SERVER_GLOBAL_TABLES`] that moved from `public`,
        /// and the outbox rows that moved to `cloacina_global.delivery_outbox`.
        Moved {
            tables: Vec<String>,
            outbox_rows: usize,
        },
        /// The mark is one minor ahead of this binary: nothing changed.
        PreviousMinor { written_by: String },
    }

    #[derive(diesel::QueryableByName)]
    struct LayoutRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        written_by: String,
    }

    fn present(conn: &mut diesel::PgConnection, qualified: &str) -> Result<bool, AdminError> {
        diesel::sql_query("SELECT to_regclass($1) IS NOT NULL AS present")
            .bind::<diesel::sql_types::Text, _>(qualified)
            .get_result::<TablePresent>(conn)
            .map(|row| row.present)
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Failed to look up table '{qualified}': {e}"),
            })
    }

    fn run_sql(conn: &mut diesel::PgConnection, sql: &str) -> Result<usize, AdminError> {
        diesel::sql_query(sql)
            .execute(conn)
            .map_err(|e| AdminError::SqlExecution {
                message: format!("{sql}: {e}"),
            })
    }

    /// The move of [`DatabaseAdmin::split_global_schema`], inside its
    /// transaction, as release `own`.
    pub(crate) fn split_global_schema_in(
        conn: &mut diesel::PgConnection,
        own: &str,
    ) -> Result<GlobalSplit, AdminError> {
        use crate::database::schema_release::{Verdict, check};
        take_migration_lock(conn)?;
        if present(conn, &format!("{GLOBAL_SCHEMA}.__diesel_schema_migrations"))? {
            return Err(AdminError::InvalidConfig {
                message: format!(
                    "a tenant schema is named '{GLOBAL_SCHEMA}'; that name is reserved for the \
                     server's global tables. Rename or remove that tenant, then start again"
                ),
            });
        }
        run_sql(
            conn,
            &format!("CREATE SCHEMA IF NOT EXISTS {GLOBAL_SCHEMA}"),
        )?;
        run_sql(
            conn,
            &format!(
                "CREATE TABLE IF NOT EXISTS {GLOBAL_SCHEMA}.{GLOBAL_LAYOUT_TABLE} (\
                 id INTEGER PRIMARY KEY CHECK (id = 1), \
                 written_by TEXT NOT NULL, \
                 moved_at TIMESTAMPTZ NOT NULL DEFAULT now())"
            ),
        )?;
        let written_by = diesel::sql_query(format!(
            "SELECT written_by FROM {GLOBAL_SCHEMA}.{GLOBAL_LAYOUT_TABLE} WHERE id = 1"
        ))
        .get_results::<LayoutRow>(conn)
        .map_err(|e| AdminError::SqlExecution {
            message: format!("Failed to read the global layout mark: {e}"),
        })?
        .into_iter()
        .next()
        .map(|row| row.written_by);
        let write = match check(own, written_by.as_deref())
            .map_err(|message| AdminError::InvalidConfig { message })?
        {
            Verdict::PreviousMinor { written_by } => {
                return Ok(GlobalSplit::PreviousMinor { written_by });
            }
            Verdict::Migrate { write } => write,
        };

        let mut moved = Vec::new();
        for table in SERVER_GLOBAL_TABLES {
            let in_public = present(conn, &format!("public.{table}"))?;
            let in_global = present(conn, &format!("{GLOBAL_SCHEMA}.{table}"))?;
            match (in_public, in_global) {
                (true, false) => {
                    run_sql(
                        conn,
                        &format!("ALTER TABLE public.{table} SET SCHEMA {GLOBAL_SCHEMA}"),
                    )?;
                    moved.push(table.to_string());
                }
                (true, true) => {
                    return Err(AdminError::InvalidConfig {
                        message: format!(
                            "'{table}' is in both public and {GLOBAL_SCHEMA}; the server does \
                             not choose between them"
                        ),
                    });
                }
                _ => {}
            }
        }

        // The outbox: `public` keeps its own run-finished notices; every
        // other row is global.
        let mut outbox_rows = 0;
        if present(conn, "public.delivery_outbox")?
            && !present(conn, &format!("{GLOBAL_SCHEMA}.delivery_outbox"))?
        {
            let global = format!("{GLOBAL_SCHEMA}.delivery_outbox");
            for sql in [
                format!("CREATE TABLE {global} (LIKE public.delivery_outbox INCLUDING ALL)"),
                format!("CREATE SEQUENCE {global}_id_seq OWNED BY {global}.id"),
                format!(
                    "ALTER TABLE {global} ALTER COLUMN id SET DEFAULT \
                     nextval('{global}_id_seq'::regclass)"
                ),
                format!(
                    "SELECT setval('{global}_id_seq'::regclass, \
                     COALESCE((SELECT max(id) FROM public.delivery_outbox), 0) + 1, false)"
                ),
                format!(
                    "CREATE TRIGGER delivery_outbox_notify_trg AFTER INSERT ON {global} \
                     FOR EACH ROW EXECUTE FUNCTION public.delivery_outbox_notify()"
                ),
            ] {
                run_sql(conn, &sql)?;
            }
            outbox_rows = diesel::sql_query(format!(
                "INSERT INTO {global} SELECT * FROM public.delivery_outbox WHERE recipient <> $1"
            ))
            .bind::<diesel::sql_types::Text, _>(crate::run_alerts::RECIPIENT)
            .execute(conn)
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Failed to move the global outbox rows: {e}"),
            })?;
            diesel::sql_query("DELETE FROM public.delivery_outbox WHERE recipient <> $1")
                .bind::<diesel::sql_types::Text, _>(crate::run_alerts::RECIPIENT)
                .execute(conn)
                .map_err(|e| AdminError::SqlExecution {
                    message: format!("Failed to move the global outbox rows: {e}"),
                })?;
        }

        if write {
            diesel::sql_query(format!(
                "INSERT INTO {GLOBAL_SCHEMA}.{GLOBAL_LAYOUT_TABLE} (id, written_by, moved_at) \
                 VALUES (1, $1, now()) \
                 ON CONFLICT (id) DO UPDATE SET written_by = EXCLUDED.written_by, \
                 moved_at = EXCLUDED.moved_at"
            ))
            .bind::<diesel::sql_types::Text, _>(own)
            .execute(conn)
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Failed to write the global layout mark: {e}"),
            })?;
        }
        Ok(GlobalSplit::Moved {
            tables: moved,
            outbox_rows,
        })
    }

    impl DatabaseAdmin {
        /// Moves the global tables out of `public` into [`GLOBAL_SCHEMA`]
        /// (CLOACINA-T-1080). The server runs it at start, after the
        /// migrations of `public`; embedded use and the daemon never do.
        ///
        /// One transaction under the migration lock, safe to run again:
        /// - a schema `cloacina_global` with migrations (a tenant of that
        ///   name) stops it;
        /// - the mark [`GLOBAL_LAYOUT_TABLE`] is read with the release check
        ///   of CLOACINA-T-1087: a mark more than one minor ahead is refused,
        ///   one minor ahead changes nothing;
        /// - each table of [`SERVER_GLOBAL_TABLES`] still in `public` moves;
        /// - `cloacina_global.delivery_outbox` is made once, and every row of
        ///   `public.delivery_outbox` that is not a run-finished notice moves
        ///   to it;
        /// - the mark is raised to this release.
        pub async fn split_global_schema(&self) -> Result<GlobalSplit, AdminError> {
            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;
            conn.interact(|conn| {
                conn.transaction::<GlobalSplit, AdminError, _>(|conn| {
                    split_global_schema_in(conn, crate::database::schema_release::RELEASE)
                })
            })
            .await
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Transaction failed: {}", e),
            })?
        }

        /// Runs the pending migrations of `public` with the search path
        /// `public, cloacina_global` (CLOACINA-T-1080): the migration table
        /// and a new table are in `public`, and a later migration that
        /// changes a global table finds it in `cloacina_global` once the
        /// server has moved it. Under the migration lock and the release
        /// check, as any schema. Returns how many ran.
        pub async fn migrate_public_schema(&self) -> Result<usize, AdminError> {
            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;
            conn.interact(|conn| {
                let result =
                    conn.transaction::<usize, AdminError, _>(|conn: &mut diesel::PgConnection| {
                        run_sql(
                            conn,
                            &format!("SET LOCAL search_path TO public, {GLOBAL_SCHEMA}"),
                        )?;
                        crate::database::migration_lock::run_pending_postgres_migrations_locked(
                            conn,
                        )
                        .map_err(|message| AdminError::SqlExecution { message })
                    });
                result
            })
            .await
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Transaction failed: {}", e),
            })?
        }
    }

    impl DatabaseAdmin {
        /// Makes the server role of `schema_name` if it is absent, gives it
        /// its grants and the connected user's membership, under the
        /// migration lock (CLOACINA-T-1080). A tenant schema gets it at
        /// tenant create and with each migration run. The server does not
        /// make one for `public`: the pool of `public` is the admin pool.
        pub async fn ensure_server_role(&self, schema_name: &str) -> Result<(), AdminError> {
            validate_build_schema(schema_name)?;
            let schema_name = schema_name.to_string();
            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;
            conn.interact(move |conn| {
                conn.transaction::<(), AdminError, _>(|conn: &mut diesel::PgConnection| {
                    take_migration_lock(conn)?;
                    ensure_server_role_in(conn, &schema_name)
                })
            })
            .await
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Transaction failed: {}", e),
            })?
        }
    }

    impl DatabaseAdmin {
        /// Makes the build role of `schema_name` if it is absent and gives it
        /// its grants, under the migration lock (CLOACINA-T-1063). For
        /// `public` and for a tenant schema. The server runs it for `public`
        /// at start; a tenant schema gets it with each migration run.
        pub async fn ensure_build_role(&self, schema_name: &str) -> Result<(), AdminError> {
            validate_build_schema(schema_name)?;
            let schema_name = schema_name.to_string();
            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;
            conn.interact(move |conn| {
                conn.transaction::<(), AdminError, _>(|conn: &mut diesel::PgConnection| {
                    take_migration_lock(conn)?;
                    ensure_build_role_in(conn, &schema_name)
                })
            })
            .await
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Transaction failed: {}", e),
            })?
        }

        /// Gives the build role of `schema_name` a new random password and
        /// returns it (CLOACINA-T-1063). The role is made first if absent.
        /// The server keeps the password nowhere: the caller writes it only
        /// where the tenant's compiler reads it, and sets a new one at each
        /// start of that compiler.
        pub async fn issue_build_password(
            &self,
            schema_name: &str,
        ) -> Result<BuildCredential, AdminError> {
            validate_build_schema(schema_name)?;
            let schema_name = schema_name.to_string();
            let password = generate_secure_password(32);
            let role = build_role_name(&schema_name);
            let conn = self
                .database
                .get_postgres_connection()
                .await
                .map_err(|e| AdminError::Pool(e.to_string()))?;
            let sql_role = role.clone();
            let sql = format!(
                "ALTER ROLE \"{}\" WITH LOGIN PASSWORD '{}'",
                role,
                escape_password(&password)
            );
            conn.interact(move |conn| {
                conn.transaction::<(), AdminError, _>(|conn: &mut diesel::PgConnection| {
                    take_migration_lock(conn)?;
                    ensure_build_role_in(conn, &schema_name)?;
                    // The statement holds the password: its error text is
                    // not passed on.
                    diesel::sql_query(&sql).execute(conn).map_err(|_| {
                        AdminError::SqlExecution {
                            message: format!("Failed to set the password of role '{}'", sql_role),
                        }
                    })?;
                    Ok(())
                })
            })
            .await
            .map_err(|e| AdminError::SqlExecution {
                message: format!("Transaction failed: {}", e),
            })??;
            Ok(BuildCredential { role, password })
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
        ("agent_pool_scale", "tenant_id = $1"),
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

        /// CLOACINA-T-1063: the build role's name never is a tenant role's
        /// name, and fits Postgres' 63 bytes.
        #[test]
        fn build_role_name_is_no_tenant_name_and_fits() {
            assert_eq!(build_role_name("acme"), "acme$build");
            assert_eq!(build_role_name("Acme"), "acme$build");
            assert_eq!(build_role_name("public"), "public$build");
            assert!(validate_username(&build_role_name("acme")).is_err());

            let long_a = format!("{}a", "x".repeat(60));
            let long_b = format!("{}b", "x".repeat(60));
            let a = build_role_name(&long_a);
            let b = build_role_name(&long_b);
            assert!(a.len() <= 63, "{a}");
            assert_ne!(a, b, "two long names with one prefix differ");
            assert!(a.starts_with(&"x".repeat(40)));
            // 57 characters still fit with the plain suffix.
            let fits = "y".repeat(57);
            assert_eq!(build_role_name(&fits), format!("{fits}$build"));
        }

        #[test]
        fn server_role_name_is_no_tenant_name_and_fits() {
            assert_eq!(server_role_name("acme"), "acme$server");
            assert_eq!(server_role_name("Acme"), "acme$server");
            assert!(validate_username(&server_role_name("acme")).is_err());
            let long = "a".repeat(63);
            let role = server_role_name(&long);
            assert_eq!(role.len(), 63, "{role}");
            assert!(role.starts_with(&format!("{}$server$", "a".repeat(39))));
            assert_ne!(role, build_role_name(&long));
            crate::database::connection::validate_role_name(&role).expect("a role name");
            crate::database::connection::validate_role_name(&server_role_name("acme"))
                .expect("a role name");
        }

        #[test]
        fn a_session_role_name_is_checked() {
            use crate::database::connection::validate_role_name;
            for bad in [
                "",
                "Acme$server",
                "a\"b",
                "a b",
                "1abc",
                "a;b",
                &"a".repeat(64),
            ] {
                assert!(validate_role_name(bad).is_err(), "{bad:?}");
            }
        }

        #[test]
        fn a_build_credential_prints_no_password() {
            let c = BuildCredential {
                role: "acme$build".into(),
                password: "not-for-logs".into(),
            };
            assert!(!format!("{c:?}").contains("not-for-logs"));
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
