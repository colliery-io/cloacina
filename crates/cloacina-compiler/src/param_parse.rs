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

//! CLOACI-T-0760: Python declared-param extraction at build time.
//!
//! Python parity for `#[workflow(params(...))]` (CLOACI-I-0128). Python packages
//! build to an empty artifact, so there is no FFI `get_input_interface` to read.
//! Instead the author declares params with a `cloaca.workflow_params(...)`
//! decorator and the compiler parses it straight from source here — the same
//! "parse the author's language-native declaration at build time" approach as
//! the doc-comment extractor ([`crate::doc_parse`]).
//!
//! Authoring convention:
//! ```python
//! @cloaca.workflow_params(
//!     source_id=str,            # required
//!     batch_size=(int, 500),    # optional, with default
//! )
//! @cloaca.task(id="prepare")
//! def prepare(context): ...
//! ```
//! Supported scalar types map to JSON Schema: `str`→string, `int`→integer,
//! `float`→number, `bool`→boolean, `list`→array, `dict`→object. Unknown types
//! are skipped (the param is dropped rather than failing the build). This mirrors
//! the Rust v1 scope (top-level scalar typing); richer types are a follow-up.
//!
//! Best-effort: anything that fails to parse contributes no params rather than
//! failing the build.

use std::path::Path;

use cloacina::input_interface::InputSlot;

/// Per-file size cap — workflow source files are small; skip anything
/// pathological (mirrors `doc_parse`).
const MAX_SOURCE_BYTES: u64 = 1024 * 1024;

/// Parse declared workflow params from the unpacked package `source_dir`.
/// `language` selects the strategy — only `"python"` parses source (Rust gets
/// its params from the FFI input-interface entrypoint). Never errors.
pub fn parse_workflow_params(source_dir: &Path, language: &str) -> Vec<InputSlot> {
    if !language.eq_ignore_ascii_case("python") {
        return Vec::new();
    }
    let mut out = Vec::new();
    walk_py(source_dir, &mut out);
    out
}

fn walk_py(dir: &Path, out: &mut Vec<InputSlot>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_dir() {
            if path.file_name().map(|n| n == "target").unwrap_or(false) {
                continue;
            }
            walk_py(&path, out);
        } else if ft.is_file() && path.extension().and_then(|e| e.to_str()) == Some("py") {
            if entry
                .metadata()
                .map(|m| m.len() > MAX_SOURCE_BYTES)
                .unwrap_or(false)
            {
                continue;
            }
            if let Ok(contents) = std::fs::read_to_string(&path) {
                parse_file(&contents, out);
            }
        }
    }
}

/// Find every `workflow_params(...)` call in a file and parse its arguments.
fn parse_file(contents: &str, out: &mut Vec<InputSlot>) {
    let needle = "workflow_params(";
    let mut rest = contents;
    while let Some(pos) = rest.find(needle) {
        let after = &rest[pos + needle.len()..];
        match take_balanced(after) {
            Some((args, consumed)) => {
                parse_args(args, out);
                rest = &after[consumed..];
            }
            None => break,
        }
    }
}

/// Given text immediately after an opening `(`, return `(inner, consumed)` where
/// `inner` is the content up to the matching `)` and `consumed` includes it.
/// Quote-aware so parens inside string literals don't unbalance the scan.
fn take_balanced(s: &str) -> Option<(&str, usize)> {
    let mut depth = 1i32;
    let mut in_str: Option<char> = None;
    for (i, c) in s.char_indices() {
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => in_str = Some(c),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&s[..i], i + c.len_utf8()));
                }
            }
            _ => {}
        }
    }
    None
}

/// Parse `name=type, name=(type, default), …` into slots (first decl per name wins).
fn parse_args(args: &str, out: &mut Vec<InputSlot>) {
    for part in split_top_level(args, ',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some(eq) = part.find('=') else {
            continue;
        };
        let name = part[..eq].trim();
        let value = part[eq + 1..].trim();
        if name.is_empty() {
            continue;
        }
        if out.iter().any(|s: &InputSlot| s.name == name) {
            continue;
        }
        if let Some(slot) = parse_param(name, value) {
            out.push(slot);
        }
    }
}

fn parse_param(name: &str, value: &str) -> Option<InputSlot> {
    if let Some(inner) = value.strip_prefix('(').and_then(|v| v.strip_suffix(')')) {
        // (type, default) → optional slot.
        let mut it = split_top_level(inner, ',').into_iter();
        let ty = it.next()?.trim().to_string();
        let schema = py_type_to_schema(&ty)?;
        let default = it.next().and_then(|d| parse_default(d.trim()));
        Some(InputSlot::optional(name, schema, default))
    } else {
        // bare type → required slot.
        let schema = py_type_to_schema(value)?;
        Some(InputSlot::required(name, schema))
    }
}

/// Map a Python scalar type name to a JSON-Schema type fragment.
fn py_type_to_schema(ty: &str) -> Option<serde_json::Value> {
    let t = match ty {
        "str" => "string",
        "int" => "integer",
        "float" => "number",
        "bool" => "boolean",
        "list" => "array",
        "dict" => "object",
        _ => return None,
    };
    Some(serde_json::json!({ "type": t }))
}

/// Parse a Python literal default into a JSON value (scalars + simple strings).
fn parse_default(d: &str) -> Option<serde_json::Value> {
    match d {
        "True" => return Some(serde_json::Value::Bool(true)),
        "False" => return Some(serde_json::Value::Bool(false)),
        "None" => return Some(serde_json::Value::Null),
        _ => {}
    }
    if let Ok(i) = d.parse::<i64>() {
        return Some(serde_json::json!(i));
    }
    if let Ok(f) = d.parse::<f64>() {
        return Some(serde_json::json!(f));
    }
    if (d.starts_with('"') && d.ends_with('"') && d.len() >= 2)
        || (d.starts_with('\'') && d.ends_with('\'') && d.len() >= 2)
    {
        return Some(serde_json::Value::String(d[1..d.len() - 1].to_string()));
    }
    None
}

/// Split `s` on top-level `delim`, respecting nested brackets and string
/// literals (so `(int, 500)` and `"a,b"` aren't split mid-group).
fn split_top_level(s: &str, delim: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut in_str: Option<char> = None;
    let mut cur = String::new();
    for c in s.chars() {
        if let Some(q) = in_str {
            cur.push(c);
            if c == q {
                in_str = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                in_str = Some(c);
                cur.push(c);
            }
            '(' | '[' | '{' => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                cur.push(c);
            }
            _ if c == delim && depth == 0 => {
                parts.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slots(src: &str) -> Vec<InputSlot> {
        let mut out = Vec::new();
        parse_file(src, &mut out);
        out
    }

    #[test]
    fn parses_required_and_optional_with_default() {
        let src = r#"
@cloaca.workflow_params(
    source_id=str,
    batch_size=(int, 500),
)
@cloaca.task(id="prepare")
def prepare(context):
    return context
"#;
        let s = slots(src);
        assert_eq!(s.len(), 2);
        let sid = s.iter().find(|x| x.name == "source_id").unwrap();
        assert!(sid.required);
        assert_eq!(sid.schema["type"], serde_json::json!("string"));
        let bs = s.iter().find(|x| x.name == "batch_size").unwrap();
        assert!(!bs.required);
        assert_eq!(bs.schema["type"], serde_json::json!("integer"));
        assert_eq!(bs.default, Some(serde_json::json!(500)));
    }

    #[test]
    fn maps_scalar_types_and_string_default() {
        let s = slots(r#"cloaca.workflow_params(flag=bool, rate=float, label=(str, "hi"))"#);
        assert_eq!(s.len(), 3);
        assert_eq!(
            s.iter().find(|x| x.name == "flag").unwrap().schema["type"],
            serde_json::json!("boolean")
        );
        assert_eq!(
            s.iter().find(|x| x.name == "rate").unwrap().schema["type"],
            serde_json::json!("number")
        );
        let label = s.iter().find(|x| x.name == "label").unwrap();
        assert_eq!(label.default, Some(serde_json::json!("hi")));
    }

    #[test]
    fn unknown_type_is_skipped() {
        let s = slots("cloaca.workflow_params(weird=SomeClass, ok=str)");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "ok");
    }

    #[test]
    fn no_decl_yields_empty() {
        assert!(slots("def prepare(context): return context").is_empty());
    }

    #[test]
    fn non_python_language_returns_empty() {
        assert!(parse_workflow_params(Path::new("/nonexistent"), "rust").is_empty());
    }
}
