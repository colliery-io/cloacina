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

use crate::database::{detect_backend_from_url, DatabaseBackend};
use anyhow::{anyhow, Result};
use colored::Colorize;

pub fn validate_backend_compatibility(url: &str) -> Result<()> {
    let detected_backend = detect_backend_from_url(url)?;
    let compiled_backend = get_compiled_backend();

    if detected_backend != compiled_backend {
        print_backend_mismatch_error(url, &detected_backend, &compiled_backend);
        return Err(anyhow!(
            "Database backend mismatch: URL requires {} but this binary supports {}",
            detected_backend,
            compiled_backend
        ));
    }

    Ok(())
}

fn get_compiled_backend() -> DatabaseBackend {
    #[cfg(feature = "postgres")]
    return DatabaseBackend::PostgreSQL;

    #[cfg(feature = "sqlite")]
    return DatabaseBackend::SQLite;

    #[cfg(not(any(feature = "postgres", feature = "sqlite")))]
    compile_error!("Either 'postgres' or 'sqlite' feature must be enabled");
}

fn print_backend_mismatch_error(url: &str, detected: &DatabaseBackend, compiled: &DatabaseBackend) {
    eprintln!();
    eprintln!("{} Database Backend Mismatch", "❌".red());
    eprintln!();
    eprintln!("  {} {}", "Database URL:".bright_white(), url);
    eprintln!(
        "  {} {} {}",
        "Detected:".bright_white(),
        detected,
        "(from URL)".dimmed()
    );
    eprintln!(
        "  {} {} {}",
        "This binary:".bright_white(),
        compiled,
        "(compiled support)".dimmed()
    );
    eprintln!();

    match detected {
        DatabaseBackend::PostgreSQL => {
            eprintln!(
                "{} Use {} for PostgreSQL databases",
                "💡".yellow(),
                "cloacina-ctl-postgres".bright_cyan()
            );
            eprintln!("   Download from: https://github.com/your-org/cloacina/releases");
        }
        DatabaseBackend::SQLite => {
            eprintln!(
                "{} Use {} for SQLite databases",
                "💡".yellow(),
                "cloacina-ctl-sqlite".bright_cyan()
            );
            eprintln!("   Download from: https://github.com/your-org/cloacina/releases");
        }
    }
    eprintln!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "postgres")]
    fn test_postgres_compatibility() {
        assert!(validate_backend_compatibility("postgres://localhost/db").is_ok());
        assert!(validate_backend_compatibility("sqlite:///path/to/db.sqlite").is_err());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn test_sqlite_compatibility() {
        assert!(validate_backend_compatibility("sqlite:///path/to/db.sqlite").is_ok());
        assert!(validate_backend_compatibility("postgres://localhost/db").is_err());
    }
}
