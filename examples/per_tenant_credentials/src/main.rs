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

//! Per-tenant credentials example
//!
//! This example demonstrates how to use Cloacina's DatabaseAdmin to create
//! isolated tenant users with their own database credentials and schemas.

use cloacina::database::{Database, DatabaseAdmin, TenantConfig};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use std::env;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("per_tenant_credentials=info,cloacina=info")
        .init();

    // Get admin database URL from environment or use default Docker PostgreSQL
    let admin_database_url = env::var("ADMIN_DATABASE_URL").unwrap_or_else(|_| {
        warn!("ADMIN_DATABASE_URL not set, using default Docker PostgreSQL connection");
        "postgresql://cloacina:cloacina@localhost:5432/cloacina".to_string()
    });

    info!("Starting per-tenant credentials example");

    // Step 1: Admin sets up database connection and creates tenants
    demonstrate_admin_tenant_creation(&admin_database_url).await?;

    // Step 2: Applications use tenant-specific credentials
    demonstrate_tenant_isolation(&admin_database_url).await?;

    info!("Per-tenant credentials example completed successfully");
    Ok(())
}

async fn demonstrate_admin_tenant_creation(
    admin_database_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Admin Tenant Creation Demo ===");

    // Step 1: Admin sets up database connection
    info!("Setting up admin database connection...");
    let admin_db = Database::new(admin_database_url, "cloacina", 10);
    let admin = DatabaseAdmin::new(admin_db);

    // Scenario A: Admin provides password
    info!("Creating tenant 'acme_corp' with admin-provided password...");
    let tenant_a_result = admin.create_tenant(TenantConfig {
        schema_name: "tenant_acme".to_string(),
        username: "acme_user".to_string(),
        password: "admin_chosen_password".to_string(),
    });

    match tenant_a_result {
        Ok(tenant_a_creds) => {
            info!("✓ Tenant A created successfully!");
            info!("  Username: {}", tenant_a_creds.username);
            info!("  Password: {} (admin-provided)", tenant_a_creds.password);
            info!("  Schema: {}", tenant_a_creds.schema_name);
            info!(
                "  Connection: {}",
                mask_password(&tenant_a_creds.connection_string)
            );
        }
        Err(e) => {
            error!("✗ Failed to create tenant A: {}", e);
            info!("This might be expected if you don't have admin privileges or PostgreSQL isn't running");
            return Ok(()); // Continue with the demo
        }
    }

    // Scenario B: Auto-generated secure password
    info!("Creating tenant 'globex_inc' with auto-generated password...");
    let tenant_b_result = admin.create_tenant(TenantConfig {
        schema_name: "tenant_globex".to_string(),
        username: "globex_user".to_string(),
        password: "".to_string(), // Empty = auto-generate
    });

    match tenant_b_result {
        Ok(tenant_b_creds) => {
            info!("✓ Tenant B created successfully!");
            info!("  Username: {}", tenant_b_creds.username);
            info!(
                "  Password: {} (auto-generated, 32 chars)",
                mask_password(&tenant_b_creds.password)
            );
            info!("  Schema: {}", tenant_b_creds.schema_name);
            info!(
                "  Connection: {}",
                mask_password(&tenant_b_creds.connection_string)
            );
        }
        Err(e) => {
            error!("✗ Failed to create tenant B: {}", e);
            info!("This might be expected if you don't have admin privileges or PostgreSQL isn't running");
        }
    }

    info!("Admin tenant creation completed");
    Ok(())
}

async fn demonstrate_tenant_isolation(
    admin_database_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Tenant Isolation Demo ===");

    // For demonstration purposes, we'll use the shared credentials
    // In a real scenario, you would use the tenant-specific credentials returned above
    info!("Note: In production, you would use the specific tenant credentials");
    info!("      returned by DatabaseAdmin::create_tenant() for each tenant");

    // Example of how tenant applications would connect:
    info!("Creating runner with shared credentials for demonstration...");
    let tenant_runner_result =
        DefaultRunner::with_schema(admin_database_url, "demo_tenant").await;

    match tenant_runner_result {
        Ok(tenant_runner) => {
            info!("✓ Tenant runner created successfully");
            info!("  - Schema isolation: ✓ (each tenant has separate schema)");
            info!("  - Migration isolation: ✓ (migrations run per-schema)");
            info!("  - Data isolation: ✓ (tenant data stored in separate schema)");

            // In production with per-tenant credentials:
            info!("  - Database user isolation: ✓ (when using per-tenant credentials)");
            info!("  - Permission isolation: ✓ (tenant users can only access their schema)");
            info!(
                "  - Audit trail: ✓ (PostgreSQL logs show which tenant user performed operations)"
            );

            tenant_runner.shutdown().await?;
        }
        Err(e) => {
            error!("✗ Failed to create tenant runner: {}", e);
            info!("This might be expected if PostgreSQL isn't running");
        }
    }

    // Show the API pattern
    info!("=== API Usage Pattern ===");
    info!("The same DefaultRunner::with_schema() API works for both:");
    info!("");
    info!("// Shared credentials (current approach)");
    info!("let runner = DefaultRunner::with_schema(");
    info!("    \"postgresql://shared_user:shared_pw@host/db\",");
    info!("    \"tenant_acme\"");
    info!(").await?;");
    info!("");
    info!("// Per-tenant credentials (enhanced security)");
    info!("let runner = DefaultRunner::with_schema(");
    info!("    \"postgresql://acme_user:tenant_pw@host/db\",");
    info!("    \"tenant_acme\"");
    info!(").await?;");
    info!("");
    info!("Zero API changes required - just different connection strings!");

    Ok(())
}

/// Masks passwords in connection strings for safe logging
fn mask_password(connection_string: &str) -> String {
    if let Some(at_pos) = connection_string.find('@') {
        if let Some(colon_pos) = connection_string[..at_pos].rfind(':') {
            let mut masked = connection_string.to_string();
            let password_start = colon_pos + 1;
            let password_end = at_pos;
            masked.replace_range(password_start..password_end, "***");
            return masked;
        }
    }
    connection_string.to_string()
}
