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

//! CLOACI-T-0752: opinionated "what & why" doc extraction at build time.
//!
//! The compiler already unpacks the full package source to build it. Rather
//! than route docs through the FFI metadata wire format (which is in flux), we
//! parse the author's language-native documentation here — Rust doc-comments
//! (`///`) on `#[task]` functions, Python docstrings on `@task`-decorated
//! functions — and overlay the result onto the persisted per-task metadata.
//!
//! Authoring convention (opinionated, but degrades gracefully):
//! - A `what:` line gives the summary; a `why:` line gives the rationale.
//! - With no markers, the whole doc-comment becomes `what` and `why` is empty.
//!
//! Best-effort: a file that fails to parse contributes no docs rather than
//! failing the build. Keyed by task local id.

use std::collections::HashMap;
use std::path::Path;

use cloacina::registry::loader::package_loader::TaskDocs;

/// Per-file size cap — task source files are small; skip anything pathological.
const MAX_DOC_SOURCE_BYTES: u64 = 1024 * 1024;

/// Parse per-task docs from the unpacked package `source_dir`. `language` is
/// the manifest language (`"python"` selects the Python heuristic; anything
/// else is treated as Rust). Never errors — returns whatever it could parse.
pub fn parse_task_docs(source_dir: &Path, language: &str) -> HashMap<String, TaskDocs> {
    let mut out = HashMap::new();
    let python = language.eq_ignore_ascii_case("python");
    let ext = if python { "py" } else { "rs" };
    collect_and_parse(source_dir, ext, python, &mut out);
    out
}

/// Recursively walk `dir`, parsing every file with the target extension.
fn collect_and_parse(dir: &Path, ext: &str, python: bool, out: &mut HashMap<String, TaskDocs>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            // Skip build output to keep the walk cheap.
            if path.file_name().map(|n| n == "target").unwrap_or(false) {
                continue;
            }
            collect_and_parse(&path, ext, python, out);
        } else if file_type.is_file() && path.extension().and_then(|e| e.to_str()) == Some(ext) {
            if entry
                .metadata()
                .map(|m| m.len() > MAX_DOC_SOURCE_BYTES)
                .unwrap_or(false)
            {
                continue;
            }
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            if python {
                parse_python_file(&contents, out);
            } else {
                parse_rust_file(&contents, out);
            }
        }
    }
}

/// Extract docs from `#[task]`-annotated functions in a Rust source file.
fn parse_rust_file(contents: &str, out: &mut HashMap<String, TaskDocs>) {
    let Ok(file) = syn::parse_file(contents) else {
        return;
    };
    for item in file.items {
        let syn::Item::Fn(func) = item else {
            continue;
        };
        let task_attr = func.attrs.iter().find(|a| a.path().is_ident("task"));
        let Some(task_attr) = task_attr else {
            continue;
        };
        let id = extract_task_id(task_attr).unwrap_or_else(|| func.sig.ident.to_string());
        if let Some(docs) = extract_rust_doc(&func.attrs) {
            out.insert(id, docs);
        }
    }
}

/// Pull `id = "..."` out of a `#[task(...)]` attribute's tokens. Token-level
/// scan so other attribute args (lists, ints, paths) never break extraction.
fn extract_task_id(attr: &syn::Attribute) -> Option<String> {
    let syn::Meta::List(list) = &attr.meta else {
        return None;
    };
    let tokens: Vec<proc_macro2::TokenTree> = list.tokens.clone().into_iter().collect();
    let mut i = 0;
    while i + 2 < tokens.len() {
        if let proc_macro2::TokenTree::Ident(ident) = &tokens[i] {
            if *ident == "id" {
                if let proc_macro2::TokenTree::Punct(p) = &tokens[i + 1] {
                    if p.as_char() == '=' {
                        if let proc_macro2::TokenTree::Literal(lit) = &tokens[i + 2] {
                            return Some(unquote(&lit.to_string()));
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Strip surrounding double quotes from a string-literal token rendering.
fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

/// Join the `#[doc = "..."]` lines of an item and split into what/why.
fn extract_rust_doc(attrs: &[syn::Attribute]) -> Option<TaskDocs> {
    let mut lines: Vec<String> = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    lines.push(s.value());
                }
            }
        }
    }
    if lines.is_empty() {
        return None;
    }
    let docs = split_what_why(&lines);
    if docs.what.is_none() && docs.why.is_none() {
        None
    } else {
        Some(docs)
    }
}

/// Split doc lines into structured what/why per the opinionated convention.
/// `what:` / `why:` markers (case-insensitive, line-leading) route subsequent
/// text; with no markers the whole block becomes `what`.
fn split_what_why(lines: &[String]) -> TaskDocs {
    let mut what: Vec<String> = Vec::new();
    let mut why: Vec<String> = Vec::new();
    // 1 = appending to what, 2 = appending to why.
    let mut target = 1u8;

    for raw in lines {
        let line = raw.trim();
        let lower = line.to_ascii_lowercase();
        if let Some(idx) = lower.strip_prefix("what:") {
            target = 1;
            let _ = idx;
            let rest = line[5..].trim();
            if !rest.is_empty() {
                what.push(rest.to_string());
            }
        } else if lower.strip_prefix("why:").is_some() {
            target = 2;
            let rest = line[4..].trim();
            if !rest.is_empty() {
                why.push(rest.to_string());
            }
        } else if target == 2 {
            why.push(line.to_string());
        } else {
            what.push(line.to_string());
        }
    }

    TaskDocs {
        what: join_doc(what),
        why: join_doc(why),
    }
}

/// Join collected lines into a single trimmed paragraph, or `None` if empty.
fn join_doc(lines: Vec<String>) -> Option<String> {
    let joined = lines
        .into_iter()
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let joined = joined.trim().to_string();
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

/// Heuristic Python docstring extraction for `@task` / `@cloaca.task` /
/// `@cloacina.task`-decorated functions. This is deliberately a best-effort
/// line scanner (not a full Python parse — documented limitation, CLOACI-T-0752)
/// covering the common single-function-per-def authoring shape.
fn parse_python_file(contents: &str, out: &mut HashMap<String, TaskDocs>) {
    let lines: Vec<&str> = contents.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        let is_task_decorator = trimmed.starts_with("@task")
            || trimmed.starts_with("@cloaca.task")
            || trimmed.starts_with("@cloacina.task");
        if !is_task_decorator {
            i += 1;
            continue;
        }

        // Collect the (possibly multi-line) decorator + find the `def`.
        let decorator_text = trimmed.to_string();
        let mut j = i + 1;
        while j < lines.len() && !lines[j].trim_start().starts_with("def ") {
            // Stop if we hit a blank/other statement without a def soon.
            if j - i > 10 {
                break;
            }
            j += 1;
        }
        if j >= lines.len() || !lines[j].trim_start().starts_with("def ") {
            i += 1;
            continue;
        }

        let def_line = lines[j].trim_start();
        let fn_name = def_line
            .strip_prefix("def ")
            .and_then(|s| s.split('(').next())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let id = extract_python_id(&decorator_text).unwrap_or(fn_name);

        // Find the docstring: first non-empty body line opening with """ or '''.
        if let Some(doc) = extract_python_docstring(&lines, j + 1) {
            let docs = split_what_why(&doc);
            if docs.what.is_some() || docs.why.is_some() {
                if !id.is_empty() {
                    out.insert(id, docs);
                }
            }
        }
        i = j + 1;
    }
}

/// Pull `id="..."` / `id='...'` from a Python `@task(...)` decorator line.
fn extract_python_id(decorator: &str) -> Option<String> {
    let key = "id";
    let pos = decorator.find(key)?;
    let after = &decorator[pos + key.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix('=')?.trim_start();
    let quote = after.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &after[1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

/// Collect a triple-quoted docstring starting at/after `start`, returning its
/// lines (without the quote delimiters).
fn extract_python_docstring(lines: &[&str], start: usize) -> Option<Vec<String>> {
    let mut k = start;
    // Skip leading blank lines.
    while k < lines.len() && lines[k].trim().is_empty() {
        k += 1;
    }
    if k >= lines.len() {
        return None;
    }
    let first = lines[k].trim_start();
    let delim = if first.starts_with("\"\"\"") {
        "\"\"\""
    } else if first.starts_with("'''") {
        "'''"
    } else {
        return None;
    };

    let mut body: Vec<String> = Vec::new();
    let after_open = &first[delim.len()..];
    // Single-line docstring: """text"""
    if let Some(end) = after_open.find(delim) {
        body.push(after_open[..end].to_string());
        return Some(body);
    }
    if !after_open.trim().is_empty() {
        body.push(after_open.trim().to_string());
    }
    k += 1;
    while k < lines.len() {
        let line = lines[k];
        if let Some(end) = line.find(delim) {
            let pre = &line[..end];
            if !pre.trim().is_empty() {
                body.push(pre.trim().to_string());
            }
            return Some(body);
        }
        body.push(line.trim().to_string());
        k += 1;
    }
    // Unterminated docstring — return what we have.
    Some(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_what_why_markers() {
        let src = r#"
            #[task(id = "validate")]
            /// what: validates the incoming order
            /// why: downstream pricing assumes a clean order
            async fn validate(ctx: Context) -> Result<()> { Ok(()) }
        "#;
        let mut out = HashMap::new();
        parse_rust_file(src, &mut out);
        let d = out.get("validate").expect("validate documented");
        assert_eq!(d.what.as_deref(), Some("validates the incoming order"));
        assert_eq!(
            d.why.as_deref(),
            Some("downstream pricing assumes a clean order")
        );
    }

    #[test]
    fn rust_plain_doc_becomes_what() {
        let src = r#"
            #[task]
            /// Cleans the staging table before load.
            fn cleanup(ctx: Context) -> Result<()> { Ok(()) }
        "#;
        let mut out = HashMap::new();
        parse_rust_file(src, &mut out);
        // No explicit id → falls back to fn name.
        let d = out.get("cleanup").expect("cleanup documented");
        assert_eq!(
            d.what.as_deref(),
            Some("Cleans the staging table before load.")
        );
        assert!(d.why.is_none());
    }

    #[test]
    fn rust_undocumented_task_absent() {
        let src = r#"
            #[task(id = "bare")]
            fn bare(ctx: Context) -> Result<()> { Ok(()) }
        "#;
        let mut out = HashMap::new();
        parse_rust_file(src, &mut out);
        assert!(
            out.get("bare").is_none(),
            "undocumented task contributes nothing"
        );
    }

    #[test]
    fn non_task_fn_ignored() {
        let src = r#"
            /// just a helper
            fn helper() {}
        "#;
        let mut out = HashMap::new();
        parse_rust_file(src, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn python_docstring_markers() {
        let src = "\
@cloaca.task(id=\"extract\")\n\
def extract(ctx):\n\
    \"\"\"\n\
    what: pulls rows from the source\n\
    why: the rest of the graph needs them staged\n\
    \"\"\"\n\
    return ctx\n";
        let mut out = HashMap::new();
        parse_python_file(src, &mut out);
        let d = out.get("extract").expect("extract documented");
        assert_eq!(d.what.as_deref(), Some("pulls rows from the source"));
        assert_eq!(
            d.why.as_deref(),
            Some("the rest of the graph needs them staged")
        );
    }

    #[test]
    fn python_single_line_docstring_to_what() {
        let src = "\
@task\n\
def t(ctx):\n\
    \"\"\"summarize the batch\"\"\"\n\
    return ctx\n";
        let mut out = HashMap::new();
        parse_python_file(src, &mut out);
        let d = out.get("t").expect("documented");
        assert_eq!(d.what.as_deref(), Some("summarize the batch"));
    }
}
