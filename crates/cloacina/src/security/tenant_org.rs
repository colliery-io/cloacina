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

//! The organization id of a tenant (CLOACINA-A-0018).
//!
//! A tenant is a schema name; there is no table of tenants or of
//! organizations. The tables that key rows by an `org_id` (the secrets tables,
//! `trusted_keys`) use the id this module derives from the tenant's name.
//! Secrets and package signing both call [`tenant_org_id`], so they agree.

use crate::database::universal_types::UniversalUuid;

/// Fixed namespace of the tenant name to `org_id` UUIDv5 derivation. Frozen:
/// a change gives every tenant a new org id, which orphans its secrets and
/// its trusted keys.
const TENANT_ORG_NAMESPACE: uuid::Uuid =
    uuid::Uuid::from_u128(0xC10AC1A5_0133_5EC0_BABE_000000000000u128);

/// The `org_id` of a tenant, from its schema name.
///
/// Deterministic (UUIDv5 of the name under [`TENANT_ORG_NAMESPACE`]): the same
/// name gives the same id in every process, with no lookup table.
pub fn tenant_org_id(tenant_id: &str) -> UniversalUuid {
    UniversalUuid(uuid::Uuid::new_v5(
        &TENANT_ORG_NAMESPACE,
        tenant_id.as_bytes(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_org_id_is_deterministic_and_distinct() {
        assert_eq!(tenant_org_id("acme"), tenant_org_id("acme"));
        assert_eq!(tenant_org_id("public"), tenant_org_id("public"));
        assert_ne!(tenant_org_id("acme"), tenant_org_id("beta"));
        assert_ne!(tenant_org_id("acme"), tenant_org_id("public"));
    }

    /// The ids are stored in the secrets tables and in `trusted_keys`: the
    /// derivation must not change. This is the id of `public` since the
    /// function was first written.
    #[test]
    fn tenant_org_id_of_a_name_is_fixed() {
        assert_eq!(
            tenant_org_id("public").0.to_string(),
            "4c7dea93-767a-5ad9-8519-aa870631cb8e"
        );
    }
}
