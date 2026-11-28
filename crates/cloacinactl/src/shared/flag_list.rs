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

//! Test helper (CLOACINA-T-1106): the long flags of a clap `Parser` struct,
//! read from the source of another binary. `cloacinactl` does not depend on
//! `cloacina-server` or `cloacina-compiler`, so the tests that compare the
//! flags of `server start` and `compiler start` with those binaries read
//! their `main.rs` as text.

use std::collections::BTreeSet;

/// The long flags (`--name`) of the fields of `struct <name> { … }` in
/// `source`. A field is a line indented by four spaces that starts with a
/// letter. Its flag is `long = "…"` of its attribute when given, else the
/// field name in kebab case; a field whose attribute has no `long` (a
/// subcommand, a positional) has none.
pub fn long_flags(source: &str, struct_name: &str) -> BTreeSet<String> {
    let header = format!("struct {struct_name} {{");
    let start = source
        .find(&header)
        .unwrap_or_else(|| panic!("no `{header}` in the source"));
    let body = &source[start + header.len()..];
    let body = &body[..body.find("\n}").expect("the struct has no end")];

    let mut flags = BTreeSet::new();
    let mut attribute = String::new();
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.is_empty() {
            continue;
        }
        let is_field = line.starts_with("    ")
            && line[4..].starts_with(|c: char| c.is_ascii_lowercase())
            && trimmed.contains(':');
        if !is_field {
            attribute.push_str(trimmed);
            attribute.push(' ');
            continue;
        }
        let field = trimmed
            .trim_start_matches("pub ")
            .split(':')
            .next()
            .unwrap()
            .trim();
        if let Some(flag) = flag_of(&attribute, field) {
            flags.insert(flag);
        }
        attribute.clear();
    }
    flags
}

fn flag_of(attribute: &str, field: &str) -> Option<String> {
    if let Some(at) = attribute.find("long = \"") {
        let rest = &attribute[at + "long = \"".len()..];
        return Some(format!("--{}", &rest[..rest.find('"')?]));
    }
    let has_long = attribute
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|token| token == "long");
    has_long.then(|| format!("--{}", field.replace('_', "-")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_long_flags_and_skips_the_rest() {
        let source = r#"
#[derive(Parser)]
struct Cli {
    /// Doc with the word long in it.
    #[arg(short, long)]
    verbose: bool,

    #[arg(
        long,
        env = "X",
        default_value_t = 3
    )]
    some_limit_s: u64,

    #[arg(long = "cargo-flag")]
    cargo_flags: Vec<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

struct Other {
    #[arg(long)]
    not_this: bool,
}
"#;
        let flags: Vec<String> = long_flags(source, "Cli").into_iter().collect();
        assert_eq!(flags, ["--cargo-flag", "--some-limit-s", "--verbose"]);
    }
}
