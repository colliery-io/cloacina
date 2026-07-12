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
//! 1. [`tenant_org_id`] — the single tenant(schema-name) → `org_id`
//!    ([`UniversalUuid`]) mapping. The [`SecretStore`] keys every row by an
//!    `org_id`, but a tenant is identified server-side by its schema-name string
//!    (there is no tenants table with a UUID PK — a tenant *is* a Postgres schema,
//!    see `routes/tenants.rs`). We derive a **stable, deterministic** UUID from
//!    the schema name (UUIDv5 under a fixed namespace) so the value is identical
//!    across processes/restarts without a lookup table. Both the CRUD handlers
//!    and [`ServerFleetSecretResolverFactory`] MUST route through this helper.
//!
//! 2. [`ServerFleetSecretResolverFactory`] — the concrete
//!    [`FleetSecretResolverFactory`](crate::fleet_executor::FleetSecretResolverFactory)
//!    (T-0861 handoff). Given a `(tenant, package)` scope it builds a tenant-scoped
//!    [`SecretStoreResolver`] over the tenant's schema + derived `org_id` + the
//!    server KEK, so a `$secret`-referencing fleet task actually resolves (it
//!    failed closed while the factory was `None`).

use std::sync::Arc;

use async_trait::async_trait;

use cloacina::database::universal_types::UniversalUuid;
use cloacina::database::Database;
use cloacina::security::{SecretStore, SecretStoreResolver};
use cloacina::SecretResolver;

use crate::fleet_executor::FleetSecretResolverFactory;
use crate::TenantDatabaseCache;

/// Fixed namespace for the tenant-schema → `org_id` UUIDv5 derivation. Chosen
/// once and frozen: changing it re-keys every existing secret, so it is a
/// stable constant, never regenerated.
const TENANT_ORG_NAMESPACE: uuid::Uuid =
    uuid::Uuid::from_u128(0xC10AC1A5_0133_5EC0_BABE_000000000000u128);

/// Map a tenant's schema-name to its stable `org_id`.
///
/// Deterministic (UUIDv5 over [`TENANT_ORG_NAMESPACE`] + the schema name): the
/// same tenant name always yields the same `org_id`, in every process, with no
/// lookup table. This is the one mapping the whole secrets subsystem uses so the
/// CRUD write path and the fleet resolve path key rows identically.
pub fn tenant_org_id(tenant_id: &str) -> UniversalUuid {
    UniversalUuid(uuid::Uuid::new_v5(
        &TENANT_ORG_NAMESPACE,
        tenant_id.as_bytes(),
    ))
}

/// Concrete [`FleetSecretResolverFactory`] over the tenant secrets subsystem
/// (CLOACI-T-0862; activates the T-0861 seam).
///
/// Builds a tenant-scoped [`SecretStoreResolver`] on demand: the tenant's own
/// schema (via [`TenantDatabaseCache`]) + its derived `org_id` + the server KEK
/// (`CLOACINA_SECRET_KEK`). `None` when the KEK is unset — the deployment has no
/// secrets configured, so a secret-referencing dispatch fails closed (never a
/// plaintext leak).
///
/// **Grant scope (honest v1 limitation):** the resolver is scoped to the tenant
/// (`org_id`) — the permanent isolation boundary. Per-*package* grant gating
/// (`ResolvedGrants.secrets` → a `SecretAllow::List`) is NOT yet enforced on this
/// path because per-package secret grants are not persisted server-side (they
/// live only in a package's constructor declarations, extracted from the cdylib
/// at reconciler-load time — unavailable at dispatch without loading the
/// artifact). A fleet task therefore resolves any secret *in its own tenant* that
/// it references; cross-tenant resolution is impossible. Tightening to the
/// package's granted allow-list is the follow-up once those grants are persisted.
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
/// tenant-org-scoped SecretStoreResolver.
pub fn runner_secret_resolver(tenant: &str, db: &Database) -> Option<Arc<dyn SecretResolver>> {
    let kek = SecretStoreResolver::kek_from_env().ok()?;
    let store = SecretStore::new(cloacina::dal::DAL::new(db.clone()));
    Some(SecretStoreResolver::new(store, tenant_org_id(tenant), kek).into_arc())
}

#[async_trait]
impl FleetSecretResolverFactory for ServerFleetSecretResolverFactory {
    async fn resolver_for(&self, tenant: &str, package: &str) -> Option<Arc<dyn SecretResolver>> {
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
        // Tenant-scoped resolver (see the grant-scope note on the struct).
        let resolver = SecretStoreResolver::new(store, org_id, kek);
        Some(resolver.into_arc())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_org_id_is_deterministic_and_distinct() {
        // Same name → same id across calls (the load-bearing property: the CRUD
        // write path and the fleet resolve path must agree).
        assert_eq!(tenant_org_id("acme"), tenant_org_id("acme"));
        assert_eq!(tenant_org_id("public"), tenant_org_id("public"));
        // Different names → different ids (tenant isolation is not aliased).
        assert_ne!(tenant_org_id("acme"), tenant_org_id("beta"));
        assert_ne!(tenant_org_id("acme"), tenant_org_id("public"));
    }

    // Full end-to-end coverage of the factory (`resolver_for` resolving a stored
    // secret + tenant isolation) lives in the postgres integration lane
    // (`lib.rs mod tests::secret_factory_*`) since the server crate is
    // postgres-only for tests. The composition the factory performs —
    // `tenant_org_id` → `SecretStore::new` → `SecretStoreResolver::new` — is
    // additionally proven piecewise by the `cloacina` crate's own
    // `SecretStoreResolver` tests (metadata-only reads, tenant isolation, grant
    // gating) under sqlite.
}
