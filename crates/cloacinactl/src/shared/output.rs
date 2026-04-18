/*
 *  Copyright 2026 Colliery Software
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

//! Output rendering: tables / json / yaml / ids, and a `Redacted` newtype for
//! secrets.

use std::io::{self, Write};

use serde::Serialize;

use crate::OutputFormat;

/// Something the CLI can render in any supported `OutputFormat`.
pub trait Renderable {
    /// Render to a writer in the requested format.
    fn render(&self, format: OutputFormat, out: &mut dyn Write) -> io::Result<()>;
}

/// Convenience: render any serializable + table-renderable type using `format`,
/// writing to stdout.
pub fn emit<T: Renderable>(value: &T, format: OutputFormat) -> io::Result<()> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    value.render(format, &mut lock)?;
    Ok(())
}

/// Generic serde-based rendering for `Json` and `Yaml` formats.
///
/// For `Table` and `Id`, types implement `Renderable` directly because those
/// require per-type column knowledge.
pub fn render_serialized<T: Serialize>(
    value: &T,
    format: OutputFormat,
    out: &mut dyn Write,
) -> io::Result<()> {
    match format {
        OutputFormat::Json => {
            let s = serde_json::to_string_pretty(value)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            writeln!(out, "{s}")
        }
        OutputFormat::Yaml => {
            let s = serde_yaml::to_string(value)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            write!(out, "{s}")
        }
        // Table and Id need per-type handling; callers should short-circuit
        // before reaching here.
        OutputFormat::Table | OutputFormat::Id => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "render_serialized called with Table/Id; implement Renderable directly",
        )),
    }
}

/// A string redacted to its first/last 4 chars for human display.
///
/// Scheme-prefixed values (`env:`, `file:`, `keyring:`) pass through unchanged
/// in the short form because they don't contain the actual secret.
#[derive(Debug, Clone)]
pub struct Redacted(pub String);

impl Redacted {
    pub fn short(&self) -> String {
        let raw = &self.0;
        if raw.starts_with("env:") || raw.starts_with("file:") || raw.starts_with("keyring:") {
            return raw.clone();
        }
        if raw.len() <= 8 {
            return "<short>".to_string();
        }
        format!("{}…{}", &raw[..4], &raw[raw.len() - 4..])
    }

    pub fn raw(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Redacted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short())
    }
}

impl Serialize for Redacted {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        // Serialize the raw value in JSON/YAML — the human-only masking lives
        // in `Display` / table rendering.
        s.serialize_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_short_form() {
        assert_eq!(Redacted("abcdefghijkl".to_string()).short(), "abcd…ijkl");
        assert_eq!(Redacted("short".to_string()).short(), "<short>");
        assert_eq!(Redacted("env:FOO".to_string()).short(), "env:FOO");
        assert_eq!(
            Redacted("file:/etc/key".to_string()).short(),
            "file:/etc/key"
        );
    }

    #[test]
    fn redacted_json_is_raw() {
        let v = Redacted("abcdefghijkl".to_string());
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "\"abcdefghijkl\"");
    }
}
