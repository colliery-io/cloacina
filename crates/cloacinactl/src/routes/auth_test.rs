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

//! Test endpoint for auth middleware validation.
//! Returns the authenticated context as JSON. Only available in debug builds.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use serde::Serialize;

use crate::auth::context::AuthContext;

/// Response from /auth-test — echoes the authenticated context.
#[derive(Debug, Serialize)]
pub struct AuthTestResponse {
    pub key_id: String,
    pub tenant_id: Option<String>,
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_admin: bool,
    pub is_global: bool,
    pub workflow_patterns: Vec<String>,
}

/// GET /auth-test — returns the authenticated context (protected endpoint).
pub async fn auth_test(Extension(auth): Extension<AuthContext>) -> impl IntoResponse {
    let response = AuthTestResponse {
        key_id: auth.key_id.to_string(),
        tenant_id: auth.tenant_id.map(|t| t.to_string()),
        can_read: auth.can_read,
        can_write: auth.can_write,
        can_execute: auth.can_execute,
        can_admin: auth.can_admin,
        is_global: auth.is_global(),
        workflow_patterns: auth.workflow_patterns.clone(),
    };

    (StatusCode::OK, Json(response))
}
