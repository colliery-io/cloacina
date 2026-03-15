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

//! Continuous reactive scheduling for Cloacina.
//!
//! This module implements data-driven DAG execution where a persistent graph
//! of compute tasks reacts automatically to data changes, re-executing only
//! the affected subgraph. Analogous to `make` but applied to a live,
//! continuously-running computation graph.
//!
//! See CLOACI-S-0001 for the full specification.

pub mod accumulator;
pub mod boundary;
pub mod connections;
pub mod datasource;
pub mod detector;
pub mod graph;
pub mod ledger;
pub mod ledger_trigger;
pub mod scheduler;
pub mod state_management;
pub mod trigger_policy;
pub mod watermark;
