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

//! Shared response envelopes.

use serde::{Deserialize, Serialize};

/// Unified list envelope (CLOACI-T-0594 / API-03): every list endpoint
/// returns `{items, total}`. `total` is best-effort — it equals the
/// returned page size when the server doesn't run a separate COUNT.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
}

impl<T> ListResponse<T> {
    /// Build the envelope with `total` set to the page size.
    pub fn new(items: Vec<T>) -> Self {
        let total = items.len();
        Self { items, total }
    }
}

/// List envelope variant that retains a top-level `tenant_id`, used by
/// tenant-scoped list endpoints for backward compatibility with operator
/// dashboards that key off it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TenantListResponse<T> {
    pub tenant_id: String,
    pub items: Vec<T>,
    pub total: usize,
}

impl<T> TenantListResponse<T> {
    /// Build the envelope with `total` set to the page size.
    pub fn new(tenant_id: impl Into<String>, items: Vec<T>) -> Self {
        let total = items.len();
        Self {
            tenant_id: tenant_id.into(),
            items,
            total,
        }
    }
}
