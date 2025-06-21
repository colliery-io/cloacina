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

//! Example demonstrating database backend validation
//!
//! Run with:
//! cargo run --example database_validation --features postgres -- postgres://localhost/mydb
//! cargo run --example database_validation --features sqlite -- sqlite:///path/to/db.sqlite

use cloacina_ctl::database::{get_backend_defaults, validate_backend_compatibility};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <database_url>", args[0]);
        eprintln!("Example: {} postgres://localhost/mydb", args[0]);
        std::process::exit(1);
    }

    let database_url = &args[1];

    println!("Validating database URL: {}", database_url);
    println!();

    match validate_backend_compatibility(database_url) {
        Ok(()) => {
            println!("✅ Database backend is compatible with this binary!");
            println!();

            let defaults = get_backend_defaults();
            println!("Default configuration for this backend:");
            println!("  Database pool size: {}", defaults.database.pool_size);
            println!(
                "  Tenancy isolation: {}",
                defaults.server.tenancy.isolation_method
            );
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
