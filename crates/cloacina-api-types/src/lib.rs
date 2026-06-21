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

//! Public API contract types for `cloacina-server` (CLOACI-I-0113 / T-0642).
//!
//! This crate is the single source of truth for every type that crosses the
//! wire between `cloacina-server` and its clients: REST request/response
//! DTOs and the WebSocket message protocols. The server consumes these types
//! in its route handlers; the Rust client crate (`cloacina-client`) returns
//! them; the OpenAPI document (T-0643) is generated from them.
//!
//! ## Design constraints
//!
//! - **No server internals.** No diesel, no engine types, no axum. Anything
//!   here is public contract — IDs are `String` (UUID-formatted), timestamps
//!   are `String` (RFC 3339), matching what the server has always emitted.
//! - **Wire-format stable.** Serialized output of these types is
//!   byte-compatible with the ad-hoc `serde_json::json!` responses they
//!   replaced. Changes here are API changes and must ride a release.

pub mod common;
pub mod compiler;
pub mod delivery;
pub mod error;
pub mod executions;
pub mod fleet;
pub mod health;
pub mod input_interface;
pub mod keys;
pub mod operations;
pub mod reactor;
pub mod tenants;
pub mod triggers;
pub mod workflows;

pub use common::{ListResponse, TenantListResponse};
pub use compiler::CompilerStatus;
pub use delivery::{ClientMessage, EnvelopeError, ServerMessage, DELIVERY_PROTOCOL_VERSION};
pub use error::ErrorBody;
pub use executions::{
    ExecuteRequest, ExecuteResponse, ExecutionDetail, ExecutionEvent, ExecutionEventsResponse,
    ExecutionSummary, ExecutionTasksResponse, ListExecutionsQuery, TaskExecutionDetail,
};
pub use fleet::AgentInfo;
pub use health::{
    AccumulatorStatus, GraphStatus, GraphTopology, GraphTopologyEdge, GraphTopologyNode,
    ReactorStatus,
};
pub use input_interface::{DeclaredSurface, InputSlot};
pub use keys::{
    CreateKeyRequest, KeyCreatedResponse, KeyInfo, KeyRevokedResponse, KeyRole, WsTicketResponse,
};
pub use operations::{OpsMetricsEvent, ReconcilerStatus, ServerHealthLite};
pub use reactor::{
    FireMode, FireReactorRequest, FireReactorResponse, InjectAccumulatorRequest,
    InjectAccumulatorResponse, ReactorCommand, ReactorResponse,
};
pub use tenants::{
    CreateTenantRequest, TenantCreatedResponse, TenantRemovedResponse, TenantSummary,
};
pub use triggers::{
    ListTriggersQuery, TriggerDetailResponse, TriggerExecution, TriggerPauseResponse,
    TriggerScheduleInfo, TriggerScheduleSummary,
};
pub use workflows::{
    WorkflowDeletedResponse, WorkflowDetail, WorkflowPauseResponse, WorkflowSourceFile,
    WorkflowSourceResponse, WorkflowSummary, WorkflowTaskNode, WorkflowUploadedResponse,
};
