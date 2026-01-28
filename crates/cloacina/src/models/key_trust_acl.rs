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

//! Domain models for key trust ACLs.
//!
//! Key trust ACLs define explicit trust relationships between organizations.
//! When a parent org grants trust to a child org, the parent implicitly
//! trusts packages signed by the child org's trusted keys.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Domain model for a key trust ACL (Access Control List).
///
/// Represents an explicit trust relationship where `parent_org_id` trusts
/// packages signed by keys trusted by `child_org_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyTrustAcl {
    pub id: UniversalUuid,
    /// The organization granting trust
    pub parent_org_id: UniversalUuid,
    /// The organization being trusted
    pub child_org_id: UniversalUuid,
    pub granted_at: UniversalTimestamp,
    /// None if active, Some if revoked
    pub revoked_at: Option<UniversalTimestamp>,
}

impl KeyTrustAcl {
    /// Check if this trust relationship is currently active
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }

    /// Check if this trust relationship has been revoked
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
}

/// Model for creating a new key trust ACL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewKeyTrustAcl {
    pub parent_org_id: UniversalUuid,
    pub child_org_id: UniversalUuid,
}

impl NewKeyTrustAcl {
    pub fn new(parent_org_id: UniversalUuid, child_org_id: UniversalUuid) -> Self {
        Self {
            parent_org_id,
            child_org_id,
        }
    }
}
