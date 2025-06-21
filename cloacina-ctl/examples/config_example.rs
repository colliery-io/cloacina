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

//! Example demonstrating configuration loading and validation
//!
//! Run with:
//! cargo run --example config_example --features postgres
//! CLOACINA_DATABASE_URL=postgresql://custom/db cargo run --example config_example --features postgres

use cloacina_ctl::config::{generate_default_config_yaml, CloacinaConfig, ConfigLoader, Validate};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cloacina Configuration Example ===\n");

    // 1. Generate and display default configuration
    println!("1. Default configuration (YAML):");
    let default_yaml = generate_default_config_yaml()?;
    println!("{}\n", default_yaml);

    // 2. Load configuration using the loader
    println!("2. Loading configuration...");
    let loader = ConfigLoader::new();

    println!("Search paths:");
    for path in loader.get_search_paths() {
        println!("  - {}", path.display());
    }
    println!();

    // Try to load config (will use defaults if no file found)
    match loader.load_config(None) {
        Ok(config) => {
            println!("✅ Configuration loaded successfully!");
            println!("Database URL: {}", config.database.url);
            println!("Pool size: {}", config.database.pool_size);
            println!("Log level: {}", config.server.log_level);

            // 3. Validate the configuration
            println!("\n3. Validating configuration...");
            match config.validate() {
                Ok(()) => println!("✅ Configuration is valid!"),
                Err(e) => println!("❌ Configuration validation failed: {}", e),
            }
        }
        Err(e) => {
            println!("ℹ️  No configuration file found, using defaults: {}", e);

            // Show how defaults work
            let default_config = CloacinaConfig::default();
            println!("Default database URL: {}", default_config.database.url);

            // 4. Demonstrate environment variable substitution
            println!("\n4. Testing environment variable substitution...");

            // Set an environment variable
            env::set_var("CLOACINA_DATABASE_URL", "postgresql://example.com/mydb");

            let config_with_env = CloacinaConfig::default();
            println!(
                "With CLOACINA_DATABASE_URL set: {}",
                config_with_env.database.url
            );

            // Clean up
            env::remove_var("CLOACINA_DATABASE_URL");
        }
    }

    println!("\n=== Example complete ===");
    Ok(())
}
