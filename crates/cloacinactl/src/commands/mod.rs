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

//! cloacinactl command implementations.

pub mod api_key;
pub mod cleanup_events;
pub mod daemon;
pub mod key;
pub mod key_trust;
pub mod package;
pub mod serve;

use anyhow::{anyhow, Context, Result};
use cloacina::dal::DAL;
use cloacina::Database;

/// Default connection pool size for CLI operations.
const CLI_POOL_SIZE: u32 = 4;

/// Connect to the database and return a DAL instance.
pub fn connect_db(database_url: &str) -> Result<DAL> {
    let database = Database::try_new_with_schema(database_url, "", CLI_POOL_SIZE, None)
        .context("Failed to connect to database")?;
    Ok(DAL::new(database))
}

/// Read the master encryption key from CLOACINA_MASTER_KEY env var (hex-encoded).
pub fn read_master_key() -> Result<[u8; 32]> {
    let hex_key = std::env::var("CLOACINA_MASTER_KEY").map_err(|_| {
        anyhow!(
            "CLOACINA_MASTER_KEY environment variable is required.\n\
             Set it to a 64-character hex string (32 bytes) for key encryption."
        )
    })?;

    let bytes = hex::decode(&hex_key).context("CLOACINA_MASTER_KEY must be valid hex")?;

    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("CLOACINA_MASTER_KEY must be exactly 32 bytes (64 hex characters)"))?;

    Ok(key)
}

/// Parse a UUID string into a UniversalUuid.
pub fn parse_uuid(s: &str) -> Result<cloacina::database::universal_types::UniversalUuid> {
    let uuid = uuid::Uuid::parse_str(s).context("Invalid UUID format")?;
    Ok(cloacina::database::universal_types::UniversalUuid(uuid))
}
