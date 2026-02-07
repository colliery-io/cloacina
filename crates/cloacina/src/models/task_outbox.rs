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

//! Task Outbox Model
//!
//! This module defines domain structures for the task outbox, which is used for
//! work distribution. The outbox is a transient table - rows are deleted immediately
//! upon claiming by workers.
//!
//! The outbox pattern provides:
//! - Reliable work distribution signaling
//! - Push notifications (Postgres LISTEN/NOTIFY) without polling
//! - Atomic task ready state + notification (single transaction)

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Represents a task outbox entry (domain type).
///
/// The outbox is transient: entries are created when tasks become ready and
/// deleted when workers claim them. This provides a reliable work queue that
/// can be used with push notifications (LISTEN/NOTIFY on Postgres) or polling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutbox {
    /// Auto-incrementing primary key (BIGSERIAL)
    pub id: i64,
    /// The task execution that is ready for processing
    pub task_execution_id: UniversalUuid,
    /// When the outbox entry was created
    pub created_at: UniversalTimestamp,
}

/// Structure for creating new task outbox entries (domain type).
///
/// Only the task_execution_id is required; created_at is set automatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTaskOutbox {
    /// The task execution that is ready for processing
    pub task_execution_id: UniversalUuid,
}
