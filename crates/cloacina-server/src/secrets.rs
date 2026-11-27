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

//! Server secrets support (CLOACI-I-0133 / T-0862).
//!
//! Two pieces the tenant secrets subsystem needs, shared by the CRUD routes and
//! the fleet delivery path so they agree on identity:
//!
//! 1. [`tenant_org_id`] — the single tenant(schema-name) → `org_id` mapping.
//!    The [`SecretStore`] keys every row by an `org_id`, but a tenant is
//!    identified server-side by its schema-name string (there is no tenants
//!    table with a UUID PK — a tenant *is* a Postgres schema, see
//!    `routes/tenants.rs`). The function lives in `cloacina::security`
//!    (CLOACINA-A-0018): the secrets routes, [`ServerFleetSecretResolverFactory`]
//!    and the trusted keys of package signing all call it there.
//!
//! 2. [`ServerFleetSecretResolverFactory`] — the concrete
//!    [`FleetSecretResolverFactory`](crate::fleet_executor::FleetSecretResolverFactory)
//!    (T-0861 handoff). Given a `(tenant, package)` scope it builds a tenant-scoped
//!    [`SecretStoreResolver`] over the tenant's schema + derived `org_id` + the
//!    server KEK, so a `$secret`-referencing fleet task actually resolves (it
//!    failed closed while the factory was `None`).

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;

use cloacina::SecretResolver;
use cloacina::database::Database;
use cloacina::security::{SecretAllow, SecretStore, SecretStoreResolver, tenant_org_id};

use crate::TenantDatabaseCache;
use crate::fleet_executor::FleetSecretResolverFactory;

/// Concrete [`FleetSecretResolverFactory`] over the tenant secrets subsystem
/// (CLOACI-T-0862; activates the T-0861 seam).
///
/// Builds a tenant-scoped [`SecretStoreResolver`] on demand: the tenant's own
/// schema (via [`TenantDatabaseCache`]) + its derived `org_id` + the server KEK
/// (`CLOACINA_SECRET_KEK`). `None` when the KEK is unset — the deployment has no
/// secrets configured, so a secret-referencing dispatch fails closed (never a
/// plaintext leak).
///
/// **Grant scope (CLOACINA-T-1081):** the resolver is scoped to the tenant
/// (`org_id`), the outer boundary, and gated with `SecretAllow::List` to the
/// task's grant: the secrets its package declares in the stored metadata
/// document, through the run's bindings. The fleet executor computes the grant
/// and refuses a dispatch that names a secret outside it before it asks here.
#[derive(Clone)]
pub struct ServerFleetSecretResolverFactory {
    /// Admin (public-schema) database — the base `TenantDatabaseCache::resolve`
    /// clones/derives per-tenant schema handles from.
    admin_db: Database,
    /// Shared per-tenant schema-scoped `Database` cache (same one the routes use).
    tenant_databases: Arc<TenantDatabaseCache>,
}

impl ServerFleetSecretResolverFactory {
    pub fn new(admin_db: Database, tenant_databases: Arc<TenantDatabaseCache>) -> Self {
        Self {
            admin_db,
            tenant_databases,
        }
    }
}

/// Tenant-scoped secret resolver for a runner's IN-PROCESS thread executor
/// (CLOACI-T-0890): threaded at runner construction so `context.secret(...)`
/// resolves when tasks execute locally (the `"default"` executor) rather than
/// on the fleet. Returns `None` when `CLOACINA_SECRET_KEK` is unset — the
/// executor then fails closed with the clear "secrets backend not configured"
/// error. Same composition as the fleet factory: tenant DB → SecretStore →
/// tenant-org-scoped SecretStoreResolver. The runner gates it per task to the
/// secrets the task's package declares (`SecretGrantMode::Declared`,
/// CLOACINA-T-1081).
pub fn runner_secret_resolver(tenant: &str, db: &Database) -> Option<Arc<dyn SecretResolver>> {
    let kek = SecretStoreResolver::kek_from_env().ok()?;
    let store = SecretStore::new(cloacina::dal::DAL::new(db.clone()));
    Some(SecretStoreResolver::new(store, tenant_org_id(tenant), kek).into_arc())
}

#[async_trait]
impl FleetSecretResolverFactory for ServerFleetSecretResolverFactory {
    async fn resolver_for(
        &self,
        tenant: &str,
        package: &str,
        allowed: &BTreeSet<String>,
    ) -> Option<Arc<dyn SecretResolver>> {
        // No KEK ⇒ secrets are not configured on this deployment → fail closed.
        let kek = match SecretStoreResolver::kek_from_env() {
            Ok(kek) => kek,
            Err(e) => {
                tracing::warn!(
                    tenant = %tenant,
                    package = %package,
                    error = %e,
                    "fleet secret resolver: server KEK unavailable — cannot resolve secrets \
                     for this dispatch (failing closed, no plaintext fallback)"
                );
                return None;
            }
        };

        // Resolve the tenant's own schema-scoped database (tenant isolation).
        let tenant_db = match self.tenant_databases.resolve(tenant, &self.admin_db).await {
            Ok(db) => db,
            Err(e) => {
                tracing::warn!(
                    tenant = %tenant,
                    error = %e,
                    "fleet secret resolver: could not open tenant database — failing closed"
                );
                return None;
            }
        };

        let store = SecretStore::new(cloacina::dal::DAL::new(tenant_db));
        let org_id = tenant_org_id(tenant);
        // Tenant-scoped and gated to the task's grant (see the note on the
        // struct).
        let allow = SecretAllow::List(allowed.iter().cloned().collect());
        let resolver = SecretStoreResolver::new_gated(store, org_id, kek, allow);
        Some(resolver.into_arc())
    }
}

/// The audit action of a refused request for a secret (CLOACINA-T-1081).
pub const ACTION_SECRET_REFUSED: &str = "secret.refused";

/// Writes an audit row for each secret a task asked for and its package does
/// not declare (CLOACINA-T-1081). Registered once at server start with
/// [`cloacina::security::register_secret_refusal_recorder`]. The row names
/// the secret, never a value.
pub struct AuditSecretRefusalRecorder {
    /// Admin (public-schema) database: `audit_events` lives there.
    admin_db: Database,
}

impl AuditSecretRefusalRecorder {
    pub fn new(admin_db: Database) -> Self {
        Self { admin_db }
    }
}

/// The audit row of a refusal.
pub fn refusal_audit_row(
    refusal: &cloacina::security::SecretRefusal,
) -> cloacina::dal::unified::audit_events::NewAuditEvent {
    cloacina::dal::unified::audit_events::NewAuditEvent {
        tenant_id: Some(refusal.tenant.clone()),
        actor_key_id: "server".to_string(),
        actor_key_name: "server".to_string(),
        actor_identity: None,
        action: ACTION_SECRET_REFUSED.to_string(),
        target_kind: "secret".to_string(),
        target: Some(refusal.secret.clone()),
        outcome: cloacina::dal::unified::audit_events::OUTCOME_DENIED.to_string(),
        detail: serde_json::json!({
            "package": refusal.package,
            "version": refusal.version,
            "task": refusal.task,
            "workflow_execution_id": refusal.workflow_execution_id,
            "task_execution_id": refusal.task_execution_id,
            "path": refusal.path,
            "reason": refusal.message,
        }),
    }
}

#[async_trait]
impl cloacina::security::SecretRefusalRecorder for AuditSecretRefusalRecorder {
    async fn record(&self, refusal: &cloacina::security::SecretRefusal) {
        let dal = cloacina::dal::DAL::new(self.admin_db.clone());
        if let Err(e) = dal.audit_events().insert(refusal_audit_row(refusal)).await {
            metrics::counter!("cloacina_audit_write_errors_total").increment(1);
            tracing::warn!(
                tenant = %refusal.tenant,
                package = %refusal.package,
                error = %e,
                "audit: the row of a refused secret could not be written"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    // Full end-to-end coverage of the factory (`resolver_for` resolving a stored
    // secret + tenant isolation) lives in the postgres integration lane
    // (`lib.rs mod tests::secret_factory_*`) since the server crate is
    // postgres-only for tests. The composition the factory performs —
    // `tenant_org_id` → `SecretStore::new` → `SecretStoreResolver::new` — is
    // additionally proven piecewise by the `cloacina` crate's own
    // `SecretStoreResolver` tests (metadata-only reads, tenant isolation, grant
    // gating) under sqlite.

    use super::*;

    /// CLOACINA-T-1081: the audit row of a refused secret names the secret,
    /// the package and the task, in the tenant, with outcome `denied`.
    #[test]
    fn a_refused_secret_has_an_audit_row_with_names_only() {
        let refusal = cloacina::security::SecretRefusal {
            tenant: "acme".into(),
            package: "pkg_a".into(),
            version: Some("1.0.0".into()),
            task: "acme::pkg_a::wf::t1".into(),
            workflow_execution_id: "run-1".into(),
            task_execution_id: "task-1".into(),
            secret: "b_secret".into(),
            path: "fleet",
            message: "b_secret: package `pkg_a` does not declare it".into(),
        };
        let row = refusal_audit_row(&refusal);
        assert_eq!(row.tenant_id.as_deref(), Some("acme"));
        assert_eq!(row.action, ACTION_SECRET_REFUSED);
        assert_eq!(row.target_kind, "secret");
        assert_eq!(row.target.as_deref(), Some("b_secret"));
        assert_eq!(row.outcome, "denied");
        assert_eq!(row.actor_key_id, "server");
        assert_eq!(row.detail["package"], "pkg_a");
        assert_eq!(row.detail["version"], "1.0.0");
        assert_eq!(row.detail["task"], "acme::pkg_a::wf::t1");
        assert_eq!(row.detail["path"], "fleet");
    }
}
