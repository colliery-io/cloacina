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

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub pool_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenancyConfig {
    pub isolation_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub tenancy: TenancyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultConfig {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
}

/// Get the default configuration for the compiled database backend
pub fn get_backend_defaults() -> DefaultConfig {
    #[cfg(feature = "postgres")]
    return get_postgres_defaults();

    #[cfg(feature = "sqlite")]
    return get_sqlite_defaults();

    #[cfg(not(any(feature = "postgres", feature = "sqlite")))]
    compile_error!("Either 'postgres' or 'sqlite' feature must be enabled");
}

fn get_postgres_defaults() -> DefaultConfig {
    DefaultConfig {
        database: DatabaseConfig { pool_size: 10 },
        server: ServerConfig {
            tenancy: TenancyConfig {
                isolation_method: "schema".to_string(),
            },
        },
    }
}

fn get_sqlite_defaults() -> DefaultConfig {
    DefaultConfig {
        database: DatabaseConfig { pool_size: 1 },
        server: ServerConfig {
            tenancy: TenancyConfig {
                isolation_method: "file".to_string(),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "postgres")]
    fn test_postgres_defaults() {
        let defaults = get_backend_defaults();
        assert_eq!(defaults.database.pool_size, 10);
        assert_eq!(defaults.server.tenancy.isolation_method, "schema");
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn test_sqlite_defaults() {
        let defaults = get_backend_defaults();
        assert_eq!(defaults.database.pool_size, 1);
        assert_eq!(defaults.server.tenancy.isolation_method, "file");
    }
}
