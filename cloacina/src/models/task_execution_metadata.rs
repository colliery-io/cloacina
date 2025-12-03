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

//! Task Execution Metadata Module
//!
//! This module defines domain structures for task execution metadata.
//! These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Represents a task execution metadata record (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionMetadata {
    pub id: UniversalUuid,
    pub task_execution_id: UniversalUuid,
    pub pipeline_execution_id: UniversalUuid,
    pub task_name: String,
    pub context_id: Option<UniversalUuid>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

/// Structure for creating new task execution metadata (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTaskExecutionMetadata {
    pub task_execution_id: UniversalUuid,
    pub pipeline_execution_id: UniversalUuid,
    pub task_name: String,
    pub context_id: Option<UniversalUuid>,
}
