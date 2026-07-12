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

/// Strip Python `# …` line comments, comment-aware (a `#` inside a string
/// literal is preserved). CLOACI-T-0899: the param/secret/boundary source
/// scanners split on commas and `=` without a real Python lexer, so an inline
/// comment in a `workflow_params(...)` / `workflow_secrets(...)` /
/// `boundary_schema(...)` call — which users naturally write — was parsed AS
/// part of a declaration (e.g. `source=str,  # required` yielded a param named
/// "# required\n    dst"). Stripping comments first makes all three scanners
/// robust in one place. Handles `'…'`, `"…"`, and triple-quoted strings with
/// backslash escapes; newlines are preserved so line structure is intact.
fn strip_py_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    // The active string delimiter (`"`, `'`, `"""`, or `'''`) when inside one.
    let mut quote: Option<&'static str> = None;
    while i < src.len() {
        let rest = &src[i..];
        if let Some(q) = quote {
            if rest.starts_with('\\') {
                out.push('\\');
                i += 1;
                if let Some(c) = src[i..].chars().next() {
                    out.push(c);
                    i += c.len_utf8();
                }
                continue;
            }
            if rest.starts_with(q) {
                out.push_str(q);
                i += q.len();
                quote = None;
                continue;
            }
            let c = rest.chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
            continue;
        }
        for delim in ["\"\"\"", "'''"] {
            if rest.starts_with(delim) {
                quote = Some(delim);
                break;
            }
        }
        if quote.is_some() {
            let q = quote.unwrap();
            out.push_str(q);
            i += q.len();
            continue;
        }
        if rest.starts_with('"') {
            quote = Some("\"");
            out.push('"');
            i += 1;
            continue;
        }
        if rest.starts_with('\'') {
            quote = Some("'");
            out.push('\'');
            i += 1;
            continue;
        }
        if rest.starts_with('#') {
            // Skip to (but keep) the end of line.
            while let Some(c) = src[i..].chars().next() {
                if c == '\n' {
                    break;
                }
                i += c.len_utf8();
            }
            continue;
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out
}

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

/// Parse declared workflow SECRETS from the unpacked package `source_dir`
/// (CLOACI-I-0133 / T-0859) — the Python parity of Rust's
/// `#[workflow(secrets(...))]`. Each `@cloaca.workflow_secrets("a", "b")` name
/// becomes a required **encrypted** [`InputSlot`] (`encrypted: true`) so the
/// manifest carries which secrets the workflow requires. Only `"python"` parses
/// source; Rust gets its secrets from the FFI input-interface entrypoint. Never
/// errors (best-effort, mirroring [`parse_workflow_params`]).
pub fn parse_workflow_secrets(source_dir: &Path, language: &str) -> Vec<InputSlot> {
    if !language.eq_ignore_ascii_case("python") {
        return Vec::new();
    }
    let mut out = Vec::new();
    walk_py_secrets(source_dir, &mut out);
    out
}

fn walk_py_secrets(dir: &Path, out: &mut Vec<InputSlot>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            if path.file_name().map(|n| n == "target").unwrap_or(false) {
                continue;
            }
            walk_py_secrets(&path, out);
        } else if ft.is_file() && path.extension().and_then(|e| e.to_str()) == Some("py") {
            if entry
                .metadata()
                .map(|m| m.len() > MAX_SOURCE_BYTES)
                .unwrap_or(false)
            {
                continue;
            }
            if let Ok(contents) = std::fs::read_to_string(&path) {
                parse_file_secrets(&strip_py_comments(&contents), out);
            }
        }
    }
}

/// Find every `workflow_secrets(...)` call and turn each string-literal name into
/// a required encrypted slot (first decl per name wins).
fn parse_file_secrets(contents: &str, out: &mut Vec<InputSlot>) {
    let needle = "workflow_secrets(";
    let mut rest = contents;
    while let Some(pos) = rest.find(needle) {
        let after = &rest[pos + needle.len()..];
        match take_balanced(after) {
            Some((args, consumed)) => {
                for part in split_top_level(args, ',') {
                    let part = part.trim();
                    // Accept only quoted string-literal names.
                    let name = if (part.starts_with('"') && part.ends_with('"') && part.len() >= 2)
                        || (part.starts_with('\'') && part.ends_with('\'') && part.len() >= 2)
                    {
                        &part[1..part.len() - 1]
                    } else {
                        continue;
                    };
                    if name.is_empty() || out.iter().any(|s: &InputSlot| s.name == name) {
                        continue;
                    }
                    out.push(InputSlot::secret(name));
                }
                rest = &after[consumed..];
            }
            None => break,
        }
    }
}

/// Parse `@cloaca.boundary_schema(field=type, ...)` accumulator boundary
/// declarations from Python source (CLOACI-T-0770) — the parity of deriving
/// `schemars::JsonSchema` on a Rust boundary type. Each decorated accumulator
/// yields one `DeclaredSurface` (`kind = "accumulator"`) carrying a single
/// object-typed slot named after the accumulator, so inject/fire-with render
/// typed forms. Non-Python languages get their surfaces from the FFI interface.
/// Best-effort: never errors.
pub fn parse_boundary_schemas(
    source_dir: &Path,
    language: &str,
) -> Vec<cloacina_api_types::DeclaredSurface> {
    if !language.eq_ignore_ascii_case("python") {
        return Vec::new();
    }
    let mut out = Vec::new();
    walk_py_surfaces(source_dir, &mut out);
    out
}

fn walk_py_surfaces(dir: &Path, out: &mut Vec<cloacina_api_types::DeclaredSurface>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            if path.file_name().map(|n| n == "target").unwrap_or(false) {
                continue;
            }
            walk_py_surfaces(&path, out);
        } else if ft.is_file() && path.extension().and_then(|e| e.to_str()) == Some("py") {
            if entry
                .metadata()
                .map(|m| m.len() > MAX_SOURCE_BYTES)
                .unwrap_or(false)
            {
                continue;
            }
            if let Ok(contents) = std::fs::read_to_string(&path) {
                parse_file_surfaces(&strip_py_comments(&contents), out);
            }
        }
    }
}

/// Find every `boundary_schema(...)` decorator, build its object schema, and
/// attach it to the accumulator it decorates (the next `def NAME` below it).
fn parse_file_surfaces(contents: &str, out: &mut Vec<cloacina_api_types::DeclaredSurface>) {
    let needle = "boundary_schema(";
    let mut from = 0usize;
    while let Some(rel) = contents[from..].find(needle) {
        let pos = from + rel;
        let after = &contents[pos + needle.len()..];
        let Some((args, consumed)) = take_balanced(after) else {
            break;
        };
        from = pos + needle.len() + consumed;
        let props = parse_object_props(args);
        if props.is_empty() {
            continue;
        }
        let Some(acc_name) = next_def_name(&after[consumed..]) else {
            continue;
        };
        if out.iter().any(|s| s.name == acc_name) {
            continue;
        }
        let required: Vec<serde_json::Value> = props
            .iter()
            .map(|(k, _)| serde_json::Value::String(k.clone()))
            .collect();
        let mut properties = serde_json::Map::new();
        for (k, v) in &props {
            properties.insert(k.clone(), v.clone());
        }
        let schema = serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required,
        });
        // Optional slot: an operator may inject, but isn't required to.
        let slot = InputSlot::optional(&acc_name, schema, None);
        out.push(cloacina_api_types::DeclaredSurface {
            kind: "accumulator".to_string(),
            name: acc_name,
            slots: vec![slot],
        });
    }
}

/// `field=type, …` → ordered (name, type-schema) pairs (first decl per name wins).
fn parse_object_props(args: &str) -> Vec<(String, serde_json::Value)> {
    let mut out: Vec<(String, serde_json::Value)> = Vec::new();
    for part in split_top_level(args, ',') {
        let part = part.trim();
        let Some(eq) = part.find('=') else { continue };
        let name = part[..eq].trim();
        let ty = part[eq + 1..].trim();
        if name.is_empty() || out.iter().any(|(n, _)| n == name) {
            continue;
        }
        if let Some(schema) = py_type_to_schema(ty) {
            out.push((name.to_string(), schema));
        }
    }
    out
}

/// The name in the first `def NAME(` / `async def NAME(` after `s` (the
/// accumulator the decorator is applied to). Decorator lines in between have no
/// `def`, so the first hit is the right function.
fn next_def_name(s: &str) -> Option<String> {
    let mut rest = s;
    while let Some(p) = rest.find("def ") {
        // Require `def` to start a token (preceded by start/space/newline).
        let prev_ok = rest[..p]
            .chars()
            .next_back()
            .map(|c| c.is_whitespace())
            .unwrap_or(true);
        let tail = &rest[p + 4..];
        if prev_ok {
            let name: String = tail
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
        rest = tail;
    }
    None
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
                parse_file(&strip_py_comments(&contents), out);
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

    fn surfaces(src: &str) -> Vec<cloacina_api_types::DeclaredSurface> {
        let mut out = Vec::new();
        parse_file_surfaces(src, &mut out);
        out
    }

    #[test]
    fn boundary_schema_builds_accumulator_surface() {
        let src = r#"
@cloaca.boundary_schema(bid=float, ask=float)
@cloaca.passthrough_accumulator
def py_alpha(event):
    return event
"#;
        let s = surfaces(src);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].kind, "accumulator");
        assert_eq!(s[0].name, "py_alpha");
        assert_eq!(s[0].slots.len(), 1);
        let slot = &s[0].slots[0];
        assert_eq!(slot.name, "py_alpha");
        assert_eq!(slot.schema["type"], serde_json::json!("object"));
        assert_eq!(
            slot.schema["properties"]["bid"]["type"],
            serde_json::json!("number")
        );
        assert_eq!(
            slot.schema["properties"]["ask"]["type"],
            serde_json::json!("number")
        );
    }

    #[test]
    fn boundary_schema_skips_decorator_between_it_and_def() {
        // A decorator with its own parens sits between the schema and the def.
        let src = r#"
@cloaca.boundary_schema(value=int)
@cloaca.state_accumulator(capacity=5)
def py_window(event):
    return event
"#;
        let s = surfaces(src);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "py_window");
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

    // CLOACI-T-0859: `workflow_secrets("a", "b")` → required encrypted slots.
    fn secret_slots(src: &str) -> Vec<InputSlot> {
        let mut out = Vec::new();
        parse_file_secrets(src, &mut out);
        out
    }

    #[test]
    fn parses_workflow_secrets_as_encrypted_slots() {
        let src = r#"
@cloaca.workflow_secrets("db_prod", 'stripe_key')
@cloaca.task(id="reach")
def reach(context):
    return context
"#;
        let s = secret_slots(src);
        assert_eq!(s.len(), 2);
        for slot in &s {
            assert!(slot.encrypted, "secret slot should be encrypted: {slot:?}");
            assert!(slot.required);
            assert!(slot.default.is_none());
        }
        assert!(s.iter().any(|x| x.name == "db_prod"));
        assert!(s.iter().any(|x| x.name == "stripe_key"));
    }

    #[test]
    fn workflow_secrets_dedups_and_skips_non_python() {
        let s = secret_slots(r#"cloaca.workflow_secrets("a", "a", "b")"#);
        assert_eq!(s.len(), 2);
        assert!(parse_workflow_secrets(Path::new("/nonexistent"), "rust").is_empty());
    }

    #[test]
    fn non_python_language_returns_empty() {
        assert!(parse_workflow_params(Path::new("/nonexistent"), "rust").is_empty());
    }
}
