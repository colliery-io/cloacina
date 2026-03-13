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

//! Implementation of `cloacinactl key trust` subcommands.

use anyhow::{anyhow, Context, Result};
use cloacina::security::{DbKeyManager, KeyManager};
use tracing::info;

use super::{connect_db, parse_uuid};

/// Add a trusted public key from a PEM file.
pub async fn add(
    database_url: &str,
    org_id: &str,
    key_file: &str,
    name: Option<&str>,
) -> Result<()> {
    let org_uuid = parse_uuid(org_id).context("Invalid organization ID")?;
    let dal = connect_db(database_url)?;

    let pem = std::fs::read_to_string(key_file)
        .with_context(|| format!("Failed to read key file: {}", key_file))?;

    let key_manager = DbKeyManager::new(dal);
    let trust_info = key_manager
        .trust_public_key_pem(org_uuid, &pem, name)
        .await
        .map_err(|e| anyhow!("{}", e))?;

    info!(
        "Trusted key added\n  ID: {}\n  Fingerprint: {}\n  Name: {}",
        trust_info.id,
        trust_info.fingerprint,
        trust_info.key_name.as_deref().unwrap_or("(none)")
    );

    Ok(())
}

/// List trusted public keys for an organization.
pub async fn list(database_url: &str, org_id: &str) -> Result<()> {
    let org_uuid = parse_uuid(org_id).context("Invalid organization ID")?;
    let dal = connect_db(database_url)?;

    let key_manager = DbKeyManager::new(dal);
    let keys = key_manager
        .list_trusted_keys(org_uuid)
        .await
        .map_err(|e| anyhow!("{}", e))?;

    if keys.is_empty() {
        println!("No trusted keys found.");
        return Ok(());
    }

    println!(
        "{:<38} {:<20} {:<16} {:<10}",
        "ID", "NAME", "FINGERPRINT", "STATUS"
    );
    for key in &keys {
        let status = if key.is_active() {
            "trusted"
        } else {
            "revoked"
        };
        let name = key.key_name.as_deref().unwrap_or("(unnamed)");
        let fp_short = if key.fingerprint.len() > 16 {
            &key.fingerprint[..16]
        } else {
            &key.fingerprint
        };
        println!(
            "{:<38} {:<20} {:<16} {:<10}",
            key.id, name, fp_short, status
        );
    }

    Ok(())
}

/// Revoke a trusted public key.
pub async fn revoke(database_url: &str, key_id: &str) -> Result<()> {
    let key_uuid = parse_uuid(key_id).context("Invalid key ID")?;
    let dal = connect_db(database_url)?;

    let key_manager = DbKeyManager::new(dal);
    key_manager
        .revoke_trusted_key(key_uuid)
        .await
        .map_err(|e| anyhow!("{}", e))?;

    info!("Trusted key {} revoked", key_id);

    Ok(())
}
