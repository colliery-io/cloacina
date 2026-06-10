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

//! Error response body.

use serde::{Deserialize, Serialize};

/// Standardized error response body. Every non-2xx response from
/// `cloacina-server` carries this shape; the request correlation ID is in
/// the `x-request-id` response header, not the body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    /// Human-readable error message.
    pub error: String,
    /// Stable machine-readable error code (e.g. `invalid_pagination`,
    /// `workflow_not_found`).
    pub code: String,
}
