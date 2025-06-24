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

use cloacina::registry::traits::RegistryStorage;
use cloacina::Database;
use std::sync::Arc;

#[cfg(feature = "postgres")]
use cloacina::dal::PostgresRegistryStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing PostgreSQL storage directly...");

    #[cfg(feature = "postgres")]
    {
        let database = Database::new("postgresql://postgres@localhost/cloacina_test", "public", 5);
        let mut storage = PostgresRegistryStorage::new(database);

        let test_data = b"test binary data".to_vec();
        println!("Storing binary data...");

        match storage.store_binary(test_data).await {
            Ok(id) => {
                println!("Successfully stored binary data with ID: {}", id);

                // Try to retrieve it
                match storage.retrieve_binary(&id).await {
                    Ok(Some(data)) => println!("Successfully retrieved {} bytes", data.len()),
                    Ok(None) => println!("ERROR: Data not found after storage"),
                    Err(e) => println!("ERROR retrieving data: {}", e),
                }
            }
            Err(e) => println!("ERROR storing binary data: {}", e),
        }
    }

    Ok(())
}
