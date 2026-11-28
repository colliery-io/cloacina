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

//! Execution-agent fleet protocol (CLOACI-I-0114).
//!
//! Pure protocol types — no diesel, no engine internals — shared by
//! `cloacina-server` (the `FleetExecutor` and agent endpoints) and the
//! `cloacina-agent` binary (T-0632), plus any future SDK. OQ-E (physical
//! share with the [[CLOACI-I-0113]] SDK crate) can move these without
//! behavior change.

pub mod poll;
pub mod protocol;

pub use poll::{
    ACCUMULATOR_POLL_CAPABILITY, POLL_JOB_CAPABILITY, POLL_PACKET_KIND, PollFailure,
    PollWorkPacket, accumulator_poll_from_outcome, accumulator_poll_outcome, poll_outcome,
    poll_result_from_outcome,
};
pub use protocol::*;

// CLOACINA-T-1086: the release rules between a server and its agents.
pub use cloacina_api_types::version;

// CLOACINA-T-1027: the quantity parsers an agent reads its `--cpu` and
// `--memory` totals with, and a task's placement.
pub use cloacina_api_types::resources::{TaskPlacement, parse_cpu_millis, parse_memory_bytes};

// CLOACINA-T-1028: the default of the agent's own share of its memory.
pub use cloacina_api_types::fleet::{
    DEFAULT_AGENT_RESERVED_MEMORY, DEFAULT_AGENT_RESERVED_MEMORY_BYTES,
};
