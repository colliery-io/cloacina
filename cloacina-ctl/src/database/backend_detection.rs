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

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseBackend {
    PostgreSQL,
    SQLite,
}

impl std::fmt::Display for DatabaseBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseBackend::PostgreSQL => write!(f, "PostgreSQL"),
            DatabaseBackend::SQLite => write!(f, "SQLite"),
        }
    }
}

pub fn detect_backend_from_url(url: &str) -> Result<DatabaseBackend> {
    let url_lower = url.to_lowercase();

    if url_lower.starts_with("postgres://") || url_lower.starts_with("postgresql://") {
        Ok(DatabaseBackend::PostgreSQL)
    } else if url_lower.starts_with("sqlite://")
        || url_lower.ends_with(".db")
        || url_lower.ends_with(".sqlite")
    {
        Ok(DatabaseBackend::SQLite)
    } else {
        Err(anyhow!(
            "Unable to detect database backend from URL: {}. \
            Supported formats: postgres://, postgresql://, sqlite://, *.db, *.sqlite",
            url
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_detection() {
        assert_eq!(
            detect_backend_from_url("postgres://localhost:5432/mydb").unwrap(),
            DatabaseBackend::PostgreSQL
        );
        assert_eq!(
            detect_backend_from_url("postgresql://user:pass@host/db").unwrap(),
            DatabaseBackend::PostgreSQL
        );
    }

    #[test]
    fn test_sqlite_detection() {
        assert_eq!(
            detect_backend_from_url("sqlite:///path/to/db.sqlite").unwrap(),
            DatabaseBackend::SQLite
        );
        assert_eq!(
            detect_backend_from_url("/path/to/database.db").unwrap(),
            DatabaseBackend::SQLite
        );
        assert_eq!(
            detect_backend_from_url("./local.sqlite").unwrap(),
            DatabaseBackend::SQLite
        );
    }

    #[test]
    fn test_invalid_url() {
        assert!(detect_backend_from_url("mysql://localhost/db").is_err());
        assert!(detect_backend_from_url("invalid-url").is_err());
    }
}
