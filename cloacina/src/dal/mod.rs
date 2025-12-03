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

//! Data Access Layer with runtime backend selection
//!
//! This module provides storage-specific DAL implementations:
//! - unified: Runtime backend selection (PostgreSQL or SQLite)
//! - filesystem_dal: For filesystem-based storage operations
//!
//! # Architecture
//!
//! The unified DAL uses custom Diesel SQL types (DbUuid, DbTimestamp, DbBool,
//! DbBinary) that work with both PostgreSQL and SQLite backends. Backend
//! selection happens at runtime based on the database connection URL.

// Unified DAL with runtime backend selection
pub mod unified;

// Filesystem DAL is always available
mod filesystem_dal;

// Export unified DAL as the primary DAL
pub use unified::DAL;

// Export CronExecutionStats from the unified module
pub use unified::cron_execution::CronExecutionStats;

// Re-export filesystem DAL
pub use filesystem_dal::FilesystemRegistryStorage;

// Re-export unified DAL types for convenience
pub use unified::UnifiedRegistryStorage;
pub use unified::DAL as UnifiedDAL;
