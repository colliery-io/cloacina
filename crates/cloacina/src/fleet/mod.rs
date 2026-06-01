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

pub mod protocol;

pub use protocol::*;
