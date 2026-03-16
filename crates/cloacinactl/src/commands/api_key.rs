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

//! Implementation of `cloacinactl api-key` subcommands.

use anyhow::{anyhow, Result};
use cloacina::dal::unified::{ApiKeyDAL, TenantDAL};
use cloacina::dal::DAL;
use cloacina::database::universal_types::{UniversalBool, UniversalUuid};
use cloacina::security::api_keys::generate_api_key;

/// Create a new API key.
pub async fn create(
    dal: &DAL,
    tenant: Option<&str>,
    name: Option<&str>,
    read: bool,
    write: bool,
    execute: bool,
    admin: bool,
    patterns: &[String],
) -> Result<()> {
    // Resolve tenant name to tenant_id
    let (tenant_id, tenant_name) = if let Some(tenant_name) = tenant {
        let tenant_dal = TenantDAL::new(dal);
        let tenant_row = tenant_dal
            .get_by_name(tenant_name)
            .await
            .map_err(|e| anyhow!("{}", e))?
            .ok_or_else(|| anyhow!("Tenant '{}' not found", tenant_name))?;
        (Some(tenant_row.id), tenant_name.to_string())
    } else {
        (None, String::new())
    };

    // Default permissions: if none specified, default to read-only (monitor key)
    let (can_read, can_write, can_execute, can_admin) = if !read && !write && !execute && !admin {
        (true, false, false, false)
    } else {
        (read, write, execute, admin)
    };

    // Generate the API key
    let env = "live";
    let (full_key, prefix, hash) = generate_api_key(env, &tenant_name);

    let key_id = UniversalUuid(uuid::Uuid::new_v4());

    let new_key = cloacina::dal::unified::models::NewApiKey {
        id: key_id.clone(),
        tenant_id,
        key_hash: hash,
        key_prefix: prefix.clone(),
        name: name.map(|s| s.to_string()),
        can_read: UniversalBool::from(can_read),
        can_write: UniversalBool::from(can_write),
        can_execute: UniversalBool::from(can_execute),
        can_admin: UniversalBool::from(can_admin),
    };

    let api_key_dal = ApiKeyDAL::new(dal);
    api_key_dal
        .create(new_key)
        .await
        .map_err(|e| anyhow!("{}", e))?;

    // Create workflow patterns if specified
    if !patterns.is_empty() {
        let new_patterns: Vec<cloacina::dal::unified::models::NewWorkflowPattern> = patterns
            .iter()
            .map(|p| cloacina::dal::unified::models::NewWorkflowPattern {
                id: UniversalUuid(uuid::Uuid::new_v4()),
                api_key_id: key_id.clone(),
                pattern: p.clone(),
            })
            .collect();

        api_key_dal
            .create_patterns(new_patterns)
            .await
            .map_err(|e| anyhow!("{}", e))?;
    }

    println!(
        "API Key created. Save this — it won't be shown again:\n\n  {}\n\nKey ID: {}\nPrefix: {}",
        full_key, key_id.0, prefix
    );

    Ok(())
}

/// List API keys.
pub async fn list(dal: &DAL, tenant: Option<&str>) -> Result<()> {
    let api_key_dal = ApiKeyDAL::new(dal);

    let keys = if let Some(tenant_name) = tenant {
        let tenant_dal = TenantDAL::new(dal);
        let tenant_row = tenant_dal
            .get_by_name(tenant_name)
            .await
            .map_err(|e| anyhow!("{}", e))?
            .ok_or_else(|| anyhow!("Tenant '{}' not found", tenant_name))?;
        api_key_dal
            .list_by_tenant(tenant_row.id)
            .await
            .map_err(|e| anyhow!("{}", e))?
    } else {
        api_key_dal.list_all().await.map_err(|e| anyhow!("{}", e))?
    };

    if keys.is_empty() {
        println!("No API keys found.");
        return Ok(());
    }

    println!(
        "{:<38} {:<16} {:<12} {:<12} {:<20} {:<10}",
        "ID", "NAME", "TENANT", "PERMISSIONS", "CREATED", "STATUS"
    );
    for key in &keys {
        let name = key.name.as_deref().unwrap_or("-");
        let tenant_display = match &key.tenant_id {
            Some(t) => &t.0.to_string()[..8],
            None => "global",
        };

        let mut perms = Vec::new();
        if bool::from(key.can_read.clone()) {
            perms.push("R");
        }
        if bool::from(key.can_write.clone()) {
            perms.push("W");
        }
        if bool::from(key.can_execute.clone()) {
            perms.push("X");
        }
        if bool::from(key.can_admin.clone()) {
            perms.push("A");
        }
        let perms_str = perms.join("");

        let created: chrono::DateTime<chrono::Utc> = key.created_at.clone().into();
        let created_str = created.format("%Y-%m-%d %H:%M").to_string();

        let status = if key.revoked_at.is_some() {
            "revoked"
        } else if let Some(ref expires) = key.expires_at {
            let exp: chrono::DateTime<chrono::Utc> = expires.clone().into();
            if exp < chrono::Utc::now() {
                "expired"
            } else {
                "active"
            }
        } else {
            "active"
        };

        println!(
            "{:<38} {:<16} {:<12} {:<12} {:<20} {:<10}",
            key.id.0, name, tenant_display, perms_str, created_str, status
        );
    }

    Ok(())
}

/// Revoke an API key.
pub async fn revoke(dal: &DAL, key_id: &str) -> Result<()> {
    let key_uuid = super::parse_uuid(key_id)?;

    let api_key_dal = ApiKeyDAL::new(dal);
    api_key_dal
        .revoke(key_uuid)
        .await
        .map_err(|e| anyhow!("{}", e))?;

    println!("API key {} revoked.", key_id);

    Ok(())
}

/// Create a global super-admin key (bootstrap command).
pub async fn create_admin(dal: &DAL, name: &str) -> Result<()> {
    create(dal, None, Some(name), true, true, true, true, &[]).await
}
