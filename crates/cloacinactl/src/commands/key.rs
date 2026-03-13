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

//! Implementation of `cloacinactl key` subcommands.

use anyhow::{anyhow, Context, Result};
use cloacina::security::{DbKeyManager, KeyManager};
use tracing::info;

use super::{connect_db, parse_uuid, read_master_key};

/// Generate a new signing keypair.
pub async fn generate(database_url: &str, org_id: &str, name: &str) -> Result<()> {
    let org_uuid = parse_uuid(org_id).context("Invalid organization ID")?;
    let master_key = read_master_key()?;
    let dal = connect_db(database_url)?;

    let key_manager = DbKeyManager::new(dal);
    let key_info = key_manager
        .create_signing_key(org_uuid, name, &master_key)
        .await
        .map_err(|e| anyhow!("{}", e))?;

    info!(
        "Signing key created\n  ID: {}\n  Name: {}\n  Fingerprint: {}",
        key_info.id, key_info.key_name, key_info.fingerprint
    );

    Ok(())
}

/// List signing keys for an organization.
pub async fn list(database_url: &str, org_id: &str) -> Result<()> {
    let org_uuid = parse_uuid(org_id).context("Invalid organization ID")?;
    let dal = connect_db(database_url)?;

    let key_manager = DbKeyManager::new(dal);
    let keys = key_manager
        .list_signing_keys(org_uuid)
        .await
        .map_err(|e| anyhow!("{}", e))?;

    if keys.is_empty() {
        println!("No signing keys found.");
        return Ok(());
    }

    println!(
        "{:<38} {:<20} {:<16} {:<10}",
        "ID", "NAME", "FINGERPRINT", "STATUS"
    );
    for key in &keys {
        let status = if key.is_active() { "active" } else { "revoked" };
        let fp_short = if key.fingerprint.len() > 16 {
            &key.fingerprint[..16]
        } else {
            &key.fingerprint
        };
        println!(
            "{:<38} {:<20} {:<16} {:<10}",
            key.id, key.key_name, fp_short, status
        );
    }

    Ok(())
}

/// Export a public key for distribution.
pub async fn export(database_url: &str, key_id: &str, format: &str) -> Result<()> {
    let key_uuid = parse_uuid(key_id).context("Invalid key ID")?;
    let dal = connect_db(database_url)?;

    let key_manager = DbKeyManager::new(dal);
    let export = key_manager
        .export_public_key(key_uuid)
        .await
        .map_err(|e| anyhow!("{}", e))?;

    match format {
        "pem" => {
            print!("{}", export.public_key_pem);
        }
        "raw" => {
            println!("{}", hex::encode(&export.public_key_raw));
        }
        _ => {
            return Err(anyhow!("Unknown format '{}'. Use 'pem' or 'raw'.", format));
        }
    }

    Ok(())
}

/// Revoke a signing key.
pub async fn revoke(database_url: &str, key_id: &str) -> Result<()> {
    let key_uuid = parse_uuid(key_id).context("Invalid key ID")?;
    let dal = connect_db(database_url)?;

    let key_manager = DbKeyManager::new(dal);
    key_manager
        .revoke_signing_key(key_uuid)
        .await
        .map_err(|e| anyhow!("{}", e))?;

    info!("Signing key {} revoked", key_id);

    Ok(())
}
