/*
 *  Copyright 2025 Colliery Software
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

//! Database models for workflow registry storage.
//!
//! This module defines domain structures for workflow registry storage.
//! These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Domain model for a workflow registry entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRegistryEntry {
    pub id: UniversalUuid,
    pub created_at: UniversalTimestamp,
    pub data: Vec<u8>,
}

/// Model for creating new workflow registry entries (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkflowRegistryEntry {
    pub data: Vec<u8>,
}

impl NewWorkflowRegistryEntry {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

/// Model for creating new workflow registry entries with explicit ID and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkflowRegistryEntryWithId {
    pub id: UniversalUuid,
    pub created_at: UniversalTimestamp,
    pub data: Vec<u8>,
}
