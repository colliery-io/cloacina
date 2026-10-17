//! Postgres CDC helpers: parsing `test_decoding` output and the SQL shapes a
//! CDC destination applies. Pure functions, unit-tested in the host-side test
//! `cdc_parsing` of the connector's tests through a copy, since a fidius guest
//! crate cannot run `cargo test` itself. Moved from weir's connector types.

use cloacina_constructor_contract::sync::ChangeOp;


/// Parse one Postgres `test_decoding` line → `(op, row-JSON)`. `None` for
/// `BEGIN`/`COMMIT` framing (they don't start with `table `). Format:
/// `table <schema>.<t>: INSERT|UPDATE|DELETE: <col>[<type>]:<val> …` (UPDATE under REPLICA IDENTITY
/// FULL prints `old-key: … new-tuple: …` — we take the new tuple). Pure + host-testable; the guest
/// connector maps `ChangeOp` to its own WIT enum.
pub fn parse_test_decoding(line: &str) -> Option<(String, ChangeOp, String)> {
    let rest = line.strip_prefix("table ")?;
    let colon = rest.find(": ")?;
    // `<schema>.<table>` → the bare table name (strip schema + any quotes) for filtering.
    let table = rest[..colon]
        .rsplit('.')
        .next()
        .unwrap_or(&rest[..colon])
        .trim_matches('"')
        .to_string();
    let after_table = &rest[colon + 2..];
    let (op_str, cols) = after_table.split_once(": ")?;
    let op = match op_str {
        "INSERT" => ChangeOp::Insert,
        "UPDATE" => ChangeOp::Update,
        "DELETE" => ChangeOp::Delete,
        _ => return None,
    };
    Some((table, op, parse_td_columns(cols)))
}

/// `<col>[<type>]:<val> …` → a JSON object (as text). `'quoted'` (with `''` escapes) → string;
/// `null` → null; numeric/bool coerced; else string. For UPDATE, only the `new-tuple:` part.
fn parse_td_columns(cols: &str) -> String {
    let cols = match cols.split_once("new-tuple: ") {
        Some((_, new)) => new,
        None => cols.strip_prefix("old-key: ").unwrap_or(cols),
    };
    let bytes = cols.as_bytes();
    let mut obj = serde_json::Map::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let name_start = i;
        while i < bytes.len() && bytes[i] != b'[' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let name = cols[name_start..i].to_string();
        while i < bytes.len() && bytes[i] != b']' {
            i += 1;
        }
        i += 1; // past ']'
        if i >= bytes.len() || bytes[i] != b':' {
            break;
        }
        i += 1; // past ':'
        let val = if i < bytes.len() && bytes[i] == b'\'' {
            i += 1;
            let vstart = i;
            loop {
                while i < bytes.len() && bytes[i] != b'\'' {
                    i += 1;
                }
                if i + 1 < bytes.len() && bytes[i] == b'\'' && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                break;
            }
            let raw = cols[vstart..i].replace("''", "'");
            i += 1; // past closing quote
            serde_json::Value::String(raw)
        } else {
            let vstart = i;
            while i < bytes.len() && bytes[i] != b' ' {
                i += 1;
            }
            let raw = &cols[vstart..i];
            if raw == "null" {
                serde_json::Value::Null
            } else if raw == "true" {
                serde_json::Value::Bool(true)
            } else if raw == "false" {
                serde_json::Value::Bool(false)
            } else if let Ok(n) = raw.parse::<i64>() {
                serde_json::Value::from(n)
            } else if let Ok(f) = raw.parse::<f64>() {
                serde_json::Value::from(f)
            } else {
                serde_json::Value::String(raw.to_string())
            }
        };
        obj.insert(name, val);
    }
    serde_json::Value::Object(obj).to_string()
}

/// Postgres CDC-destination SQL builders. Inputs are already quoted/escaped by the
/// caller (the connector's `quote_ident`/`lit`); these assemble the statement shape so it's unit-testable
/// off the wire (the connector's fidius macros preclude `cargo test`).
pub mod pg_cdc {
    /// Upsert a change's row by primary key (`pk` = comma-joined quoted key idents).
    pub fn upsert(table: &str, pk: &str, key_lits: &str, data_lit: &str) -> String {
        format!(
            "INSERT INTO {table} ({pk}, data) VALUES ({key_lits}, {data_lit}::jsonb) \
             ON CONFLICT ({pk}) DO UPDATE SET data = EXCLUDED.data"
        )
    }

    /// Hard delete by key (`where_clause` = `k1 = 'v1' AND …`).
    pub fn delete(table: &str, where_clause: &str) -> String {
        format!("DELETE FROM {table} WHERE {where_clause}")
    }

    /// Soft tombstone: stamp the tombstone column (already quoted) with the delete time.
    pub fn tombstone(table: &str, tombstone_col: &str, where_clause: &str) -> String {
        format!("UPDATE {table} SET {tombstone_col} = now()::text WHERE {where_clause}")
    }
}

