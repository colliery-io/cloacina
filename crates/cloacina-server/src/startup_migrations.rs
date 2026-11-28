/*
 *  Copyright 2026 Colliery Software
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

//! The migrations of server start (CLOACINA-T-1056).
//!
//! Every schema holds the same tables and its own
//! `__diesel_schema_migrations`: `public`, and one schema for each tenant.
//! A tenant schema is made, with every migration of that server version, when
//! the tenant is created. A migration that a later version adds must then
//! reach each tenant schema that exists. So the start migrates `public` and
//! then each tenant schema.
//!
//! - Each schema is one transaction that first takes the migration lock
//!   (CLOACINA-T-1042). Replicas that start together migrate each schema
//!   once: the others wait for the lock and then find nothing pending.
//! - A tenant whose migration fails is left as it was (its transaction rolls
//!   back). The start goes on to the other tenants, logs each failure with
//!   the tenant's name, and then fails with an error that names every tenant
//!   that failed. The server does not start: a connection of a tenant reads
//!   `<tenant>, public`, so a table that the tenant schema lacks would be
//!   looked up in `public`, where the tenant's server role is refused
//!   (CLOACINA-T-1080).
//! - Each migration run of a tenant schema also gives the tenant's build role
//!   and server role their grants again, and makes them if they are absent.
//! - A tenant with nothing pending costs one short transaction.

use std::time::Instant;

use cloacina::Database;
use cloacina::database::DatabaseAdmin;
use tracing::{error, info};

/// Which tenant schemas the start migrates.
pub enum TenantSchemas<'a> {
    /// Each tenant schema of the database
    /// (`DatabaseAdmin::list_migrated_tenant_schemas`). The server uses this.
    All,
    /// These schemas only. For tests: the test database is shared, and a
    /// test migrates the tenants it made and no other.
    Named(&'a [String]),
}

/// What the start migrated.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Migrated {
    /// The tenant schemas that were looked at.
    pub tenants: usize,
    /// The tenant schemas that had pending migrations, which ran.
    pub tenants_changed: Vec<String>,
}

/// A tenant schema that could not be migrated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantFailure {
    pub tenant: String,
    pub error: String,
}

/// Why the start migrations failed.
#[derive(Debug)]
pub enum StartMigrationError {
    /// The migrations of `public` failed. No tenant schema was tried.
    Public(String),
    /// The tenant schemas could not be listed. No tenant schema was tried.
    ListTenants(String),
    /// Some tenant schemas failed, out of `tenants` that were tried.
    Tenants {
        tenants: usize,
        failures: Vec<TenantFailure>,
    },
}

impl std::fmt::Display for StartMigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public(e) => write!(f, "failed to run migrations: {e}"),
            Self::ListTenants(e) => write!(
                f,
                "failed to list the tenant schemas for their migrations: {e}"
            ),
            Self::Tenants { tenants, failures } => write!(
                f,
                "migrations failed in {} of {tenants} tenant schema(s): {}. Each of them is                  unchanged; the other tenant schemas are migrated. The server does not start                  until each tenant schema is migrated.",
                failures.len(),
                describe(failures)
            ),
        }
    }
}

impl std::error::Error for StartMigrationError {}

fn describe(failures: &[TenantFailure]) -> String {
    failures
        .iter()
        .map(|f| format!("'{}' ({})", f.tenant, f.error))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Migrates one tenant schema when this replica first opens the tenant's
/// database (CLOACINA-T-1093). Returns how many migrations ran.
///
/// The start migrates every tenant schema that exists then. A tenant made
/// later through an older replica, during a rolling upgrade, gets the
/// migrations of that replica only. So each replica also migrates a tenant
/// schema the first time it opens it: before its runner is made and before
/// its routes read it.
///
/// - `public` is migrated at start: nothing to do.
/// - A schema with no `__diesel_schema_migrations` table is not migrated, as
///   at start: it is not a tenant this server made.
/// - Else [`DatabaseAdmin::migrate_tenant_schema`]: one transaction, under
///   the migration lock, so replicas that open the tenant at the same moment
///   migrate it once. Nothing pending is one short transaction.
///
/// A failure is logged with the tenant's name and returned; the schema is
/// unchanged.
pub async fn migrate_tenant_on_first_open(
    admin_database: &Database,
    tenant: &str,
) -> Result<usize, String> {
    if tenant == "public" {
        return Ok(0);
    }
    let admin = DatabaseAdmin::new(admin_database.clone());
    let result = async {
        if !admin.is_migrated_tenant_schema(tenant).await? {
            return Ok(0);
        }
        admin.migrate_tenant_schema(tenant).await
    }
    .await;
    match result {
        Ok(0) => Ok(0),
        Ok(ran) => {
            info!(
                tenant = %tenant,
                migrations = ran,
                "tenant schema migrated on first open"
            );
            Ok(ran)
        }
        Err(e) => {
            error!(
                tenant = %tenant,
                error = %e,
                "tenant schema not migrated on first open; the schema is unchanged and the \
                 tenant is not served on this replica until a try succeeds"
            );
            Err(format!("tenant '{tenant}': {e}"))
        }
    }
}

/// Migrates `public`, then each tenant schema of `tenants`.
pub async fn migrate_at_start(
    database: &Database,
    tenants: TenantSchemas<'_>,
) -> Result<Migrated, StartMigrationError> {
    database
        .run_migrations()
        .await
        .map_err(StartMigrationError::Public)?;

    let admin = DatabaseAdmin::new(database.clone());
    // `public`'s build role and its grants (CLOACINA-T-1063). A tenant
    // schema gets its own with each migration run below.
    admin
        .ensure_build_role("public")
        .await
        .map_err(|e| StartMigrationError::Public(format!("build role of public: {e}")))?;
    let listed;
    let tenants: &[String] = match tenants {
        TenantSchemas::Named(names) => names,
        TenantSchemas::All => {
            listed = admin
                .list_migrated_tenant_schemas()
                .await
                .map_err(|e| StartMigrationError::ListTenants(e.to_string()))?;
            &listed
        }
    };

    let started = Instant::now();
    let mut migrated = Migrated {
        tenants: tenants.len(),
        ..Migrated::default()
    };
    let mut failures = Vec::new();
    for tenant in tenants {
        match admin.migrate_tenant_schema(tenant).await {
            Ok(0) => {}
            Ok(ran) => {
                info!(tenant = %tenant, migrations = ran, "tenant schema migrated");
                migrated.tenants_changed.push(tenant.clone());
            }
            Err(e) => {
                error!(
                    tenant = %tenant,
                    error = %e,
                    "tenant schema not migrated; the schema is unchanged"
                );
                failures.push(TenantFailure {
                    tenant: tenant.clone(),
                    error: e.to_string(),
                });
            }
        }
    }
    info!(
        tenants = migrated.tenants,
        changed = migrated.tenants_changed.len(),
        failed = failures.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "tenant schema migrations checked"
    );

    if failures.is_empty() {
        Ok(migrated)
    } else {
        Err(StartMigrationError::Tenants {
            tenants: migrated.tenants,
            failures,
        })
    }
}
