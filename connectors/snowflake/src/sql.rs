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

//! The Snowflake connector's SQL API statements (CLOACINA-T-0986), moved from
//! weir. Pure, so the unit lane tests them: `cloacina-sync` compiles this file
//! into its unit tests.
//!
//! Statements use `?` positional binds; [`bindings`] numbers them the way the
//! SQL API expects (`{"1": {"type": ..., "value": ...}, "2": ...}`).
//! Identifiers are uppercased and double-quoted ([`qident`]), so unquoted
//! `SELECT *` still works while the compiled SQL stays injection-safe.

use serde_json::Value;

/// A destination column: name + Snowflake column type + SQL API bind type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Col {
    pub name: String,
    /// Column type in `CREATE TABLE` (`VARCHAR` | `NUMBER` | `DOUBLE` | `BOOLEAN`).
    pub sf_type: &'static str,
    /// SQL API bind type (`TEXT` | `FIXED` | `REAL` | `BOOLEAN`).
    pub bind_type: &'static str,
}

/// Infer the column set from a record's top-level fields (post-mapping records in a
/// stream are homogeneous): string→VARCHAR, integer→NUMBER, float→DOUBLE,
/// bool→BOOLEAN; null and nested object/array → VARCHAR (nested values land as their
/// JSON text in v1 — `VARIANT` via `PARSE_JSON` is a noted follow-up).
pub fn infer_cols(rec: &Value) -> Vec<Col> {
    let Some(obj) = rec.as_object() else {
        return Vec::new();
    };
    obj.iter()
        .map(|(name, v)| {
            let (sf_type, bind_type) = match v {
                Value::Bool(_) => ("BOOLEAN", "BOOLEAN"),
                Value::Number(n) if n.is_i64() || n.is_u64() => ("NUMBER", "FIXED"),
                Value::Number(_) => ("DOUBLE", "REAL"),
                _ => ("VARCHAR", "TEXT"),
            };
            Col {
                name: name.clone(),
                sf_type,
                bind_type,
            }
        })
        .collect()
}

/// Uppercase + double-quote an identifier (embedded quotes doubled).
pub fn qident(name: &str) -> String {
    format!("\"{}\"", name.to_uppercase().replace('"', "\"\""))
}

/// Fully-qualified `"DB"."SCHEMA"."TABLE"`.
pub fn qualified_table(database: &str, schema: &str, table: &str) -> String {
    format!("{}.{}.{}", qident(database), qident(schema), qident(table))
}

/// `CREATE TABLE IF NOT EXISTS <fq> (cols…)`.
pub fn create_table_sql(fq: &str, cols: &[Col]) -> String {
    let defs = cols
        .iter()
        .map(|c| format!("{} {}", qident(&c.name), c.sf_type))
        .collect::<Vec<_>>()
        .join(", ");
    format!("CREATE TABLE IF NOT EXISTS {fq} ({defs})")
}

/// Chunked multi-row append: `INSERT INTO <fq> (cols…) VALUES (?,…),(?,…)…`.
pub fn insert_sql(fq: &str, cols: &[Col], nrows: usize) -> String {
    let names = cols
        .iter()
        .map(|c| qident(&c.name))
        .collect::<Vec<_>>()
        .join(", ");
    let row = format!("({})", vec!["?"; cols.len()].join(", "));
    let values = vec![row; nrows].join(", ");
    format!("INSERT INTO {fq} ({names}) VALUES {values}")
}

/// The `FROM VALUES` source subquery both MERGE and DELETE ride: Snowflake names the
/// tuple columns `COLUMN1…COLUMNn`, aliased back to the real column names.
fn values_source(cols: &[Col], nrows: usize) -> String {
    let selects = cols
        .iter()
        .enumerate()
        .map(|(i, c)| format!("COLUMN{} AS {}", i + 1, qident(&c.name)))
        .collect::<Vec<_>>()
        .join(", ");
    let row = format!("({})", vec!["?"; cols.len()].join(", "));
    let values = vec![row; nrows].join(", ");
    format!("SELECT {selects} FROM VALUES {values}")
}

/// Replay-idempotent upsert: `MERGE INTO <fq> USING (SELECT … FROM VALUES …) ON keys…`.
/// Errors if a key isn't in the column set. Callers must de-duplicate source rows by
/// key first ([`dedup_by_keys`]) — Snowflake rejects nondeterministic merges.
pub fn merge_sql(fq: &str, cols: &[Col], keys: &[String], nrows: usize) -> Result<String, String> {
    if keys.is_empty() {
        return Err("upsert requires business keys".to_string());
    }
    for k in keys {
        if !cols.iter().any(|c| c.name.eq_ignore_ascii_case(k)) {
            return Err(format!("upsert key `{k}` is not a record field"));
        }
    }
    let on = keys
        .iter()
        .map(|k| format!("t.{k} = s.{k}", k = qident(k)))
        .collect::<Vec<_>>()
        .join(" AND ");
    let non_key: Vec<&Col> = cols
        .iter()
        .filter(|c| !keys.iter().any(|k| c.name.eq_ignore_ascii_case(k)))
        .collect();
    let update = if non_key.is_empty() {
        String::new()
    } else {
        let sets = non_key
            .iter()
            .map(|c| format!("t.{n} = s.{n}", n = qident(&c.name)))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" WHEN MATCHED THEN UPDATE SET {sets}")
    };
    let names = cols
        .iter()
        .map(|c| qident(&c.name))
        .collect::<Vec<_>>()
        .join(", ");
    let sources = cols
        .iter()
        .map(|c| format!("s.{}", qident(&c.name)))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "MERGE INTO {fq} AS t USING ({src}) AS s ON {on}{update} \
         WHEN NOT MATCHED THEN INSERT ({names}) VALUES ({sources})",
        src = values_source(cols, nrows),
    ))
}

/// CDC delete by key: `DELETE FROM <fq> USING (SELECT … FROM VALUES …) WHERE keys…`.
pub fn delete_sql(fq: &str, key_cols: &[Col], nrows: usize) -> String {
    let on = key_cols
        .iter()
        .map(|c| format!("{fq}.{k} = s.{k}", k = qident(&c.name)))
        .collect::<Vec<_>>()
        .join(" AND ");
    format!(
        "DELETE FROM {fq} USING ({src}) AS s WHERE {on}",
        src = values_source(key_cols, nrows),
    )
}

/// A record value in SQL API bind form: everything is carried as its string form
/// (numbers/bools stringified, nested values as JSON text); JSON `null` stays null.
pub fn bind_value(v: &Value) -> Value {
    match v {
        Value::Null => Value::Null,
        Value::String(s) => Value::String(s.clone()),
        Value::Bool(b) => Value::String(b.to_string()),
        Value::Number(n) => Value::String(n.to_string()),
        nested => Value::String(nested.to_string()),
    }
}

/// Number the binds sequentially across `rows` × `cols` the way the SQL API expects:
/// `{"1": {"type": "TEXT", "value": …}, "2": …}`, row-major.
pub fn bindings(rows: &[&Value], cols: &[Col]) -> serde_json::Map<String, Value> {
    let mut map = serde_json::Map::new();
    let mut i = 0usize;
    for row in rows {
        for col in cols {
            i += 1;
            let v = row.get(&col.name).map(bind_value).unwrap_or(Value::Null);
            map.insert(
                i.to_string(),
                serde_json::json!({ "type": col.bind_type, "value": v }),
            );
        }
    }
    map
}

/// Incremental source read: wrap the base SELECT with a bound cursor
/// lower-bound + deterministic order. Subquery-wrapping keeps it valid for both a bare
/// table SELECT and an arbitrary configured query.
pub fn incremental_select(base: &str, cursor_col: &str) -> String {
    let c = qident(cursor_col);
    format!("SELECT * FROM ({base}) WHERE {c} > ? ORDER BY {c}")
}

/// First incremental run (no state yet): full read, but ordered by the cursor so the
/// checkpoint is deterministic.
pub fn ordered_select(base: &str, cursor_col: &str) -> String {
    let c = qident(cursor_col);
    format!("SELECT * FROM ({base}) ORDER BY {c}")
}

/// Zip the SQL API's array rows (`data: [[v1, v2], …]`) with the result's `rowType`
/// column names into JSON object records. Field names are **lowercased**:
/// Snowflake stores identifiers uppercase, but records, mappings, upsert keys
/// and cursor fields read friendlier lowercase names.
pub fn rows_to_objects(names: &[String], rows: &[Value]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| row.as_array())
        .map(|cells| {
            let obj: serde_json::Map<String, Value> = names
                .iter()
                .zip(cells.iter())
                .map(|(n, v)| (n.to_lowercase(), v.clone()))
                .collect();
            Value::Object(obj).to_string()
        })
        .collect()
}

/// De-duplicate rows by their key tuple, **last wins** (replayed/duplicated keys in one
/// batch would make MERGE nondeterministic). Order of the survivors is preserved.
pub fn dedup_by_keys<'a>(rows: &[&'a Value], keys: &[String]) -> Vec<&'a Value> {
    let mut seen: std::collections::HashMap<String, usize> = Default::default();
    let mut out: Vec<Option<&Value>> = Vec::with_capacity(rows.len());
    for row in rows {
        let key = keys
            .iter()
            .map(|k| {
                row.get(k)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "\u{0}".to_string())
            })
            .collect::<Vec<_>>()
            .join("\u{1}");
        if let Some(&idx) = seen.get(&key) {
            out[idx] = None; // superseded — last wins
        }
        seen.insert(key, out.len());
        out.push(Some(row));
    }
    out.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cols() -> Vec<Col> {
        infer_cols(&json!({"email": "a@x.com", "age": 41, "score": 1.5, "active": true}))
    }

    #[test]
    fn infers_types_from_json_kinds() {
        let c = cols();
        let by_name: std::collections::HashMap<_, _> =
            c.iter().map(|c| (c.name.as_str(), c)).collect();
        assert_eq!(by_name["email"].sf_type, "VARCHAR");
        assert_eq!(by_name["age"].sf_type, "NUMBER");
        assert_eq!(by_name["age"].bind_type, "FIXED");
        assert_eq!(by_name["score"].sf_type, "DOUBLE");
        assert_eq!(by_name["active"].sf_type, "BOOLEAN");
        // Nested values land as JSON text in a VARCHAR.
        let n = infer_cols(&json!({"tags": ["a"], "meta": {"x": 1}, "gone": null}));
        assert!(n.iter().all(|c| c.sf_type == "VARCHAR"));
    }

    #[test]
    fn create_and_insert_compile() {
        let fq = qualified_table("demo_db", "public", "contacts");
        assert_eq!(fq, r#""DEMO_DB"."PUBLIC"."CONTACTS""#);
        let c = cols();
        assert_eq!(
            create_table_sql(&fq, &c),
            r#"CREATE TABLE IF NOT EXISTS "DEMO_DB"."PUBLIC"."CONTACTS" ("ACTIVE" BOOLEAN, "AGE" NUMBER, "EMAIL" VARCHAR, "SCORE" DOUBLE)"#
        );
        let sql = insert_sql(&fq, &c, 3);
        assert!(sql.starts_with(r#"INSERT INTO "DEMO_DB"."PUBLIC"."CONTACTS" ("ACTIVE", "AGE", "EMAIL", "SCORE") VALUES"#));
        assert_eq!(sql.matches("(?, ?, ?, ?)").count(), 3, "one tuple per row");
    }

    #[test]
    fn merge_compiles_keyed_and_rejects_unknown_key() {
        let fq = qualified_table("d", "s", "t");
        let c = cols();
        let sql = merge_sql(&fq, &c, &["email".into()], 2).expect("merge");
        assert!(sql.contains(r#"MERGE INTO "D"."S"."T" AS t USING (SELECT COLUMN1 AS "ACTIVE", COLUMN2 AS "AGE", COLUMN3 AS "EMAIL", COLUMN4 AS "SCORE" FROM VALUES (?, ?, ?, ?), (?, ?, ?, ?)) AS s ON t."EMAIL" = s."EMAIL""#));
        assert!(sql.contains(r#"WHEN MATCHED THEN UPDATE SET t."ACTIVE" = s."ACTIVE", t."AGE" = s."AGE", t."SCORE" = s."SCORE""#));
        assert!(sql.contains(r#"WHEN NOT MATCHED THEN INSERT ("ACTIVE", "AGE", "EMAIL", "SCORE") VALUES (s."ACTIVE", s."AGE", s."EMAIL", s."SCORE")"#));
        assert!(merge_sql(&fq, &c, &["nope".into()], 1).is_err());
        assert!(merge_sql(&fq, &c, &[], 1).is_err());
    }

    #[test]
    fn delete_compiles_by_key_tuple() {
        let key_cols = vec![Col {
            name: "id".into(),
            sf_type: "VARCHAR",
            bind_type: "TEXT",
        }];
        let sql = delete_sql(r#""D"."S"."T""#, &key_cols, 2);
        assert_eq!(
            sql,
            r#"DELETE FROM "D"."S"."T" USING (SELECT COLUMN1 AS "ID" FROM VALUES (?), (?)) AS s WHERE "D"."S"."T"."ID" = s."ID""#
        );
    }

    #[test]
    fn bindings_number_sequentially_row_major() {
        let c = infer_cols(&json!({"id": 1, "name": "a"}));
        let r1 = json!({"id": 1, "name": "a"});
        let r2 = json!({"id": 2, "name": null});
        let b = bindings(&[&r1, &r2], &c);
        assert_eq!(b.len(), 4);
        assert_eq!(b["1"], json!({"type": "FIXED", "value": "1"}));
        assert_eq!(b["2"], json!({"type": "TEXT", "value": "a"}));
        assert_eq!(b["3"], json!({"type": "FIXED", "value": "2"}));
        assert_eq!(b["4"], json!({"type": "TEXT", "value": null}));
    }

    #[test]
    fn incremental_select_wraps_and_binds_the_cursor() {
        assert_eq!(
            incremental_select(r#"SELECT * FROM "D"."S"."T""#, "updated_at"),
            r#"SELECT * FROM (SELECT * FROM "D"."S"."T") WHERE "UPDATED_AT" > ? ORDER BY "UPDATED_AT""#
        );
        assert_eq!(
            ordered_select("SELECT a, b FROM x", "a"),
            r#"SELECT * FROM (SELECT a, b FROM x) ORDER BY "A""#
        );
    }

    #[test]
    fn rows_zip_into_lowercased_object_records() {
        let names = vec!["EMAIL".to_string(), "UPDATED_AT".to_string()];
        let rows = vec![
            json!(["a@x.com", "2026-01-01"]),
            json!(["b@x.com", null]),
            json!("not-an-array-row"),
        ];
        let out = rows_to_objects(&names, &rows);
        assert_eq!(out.len(), 2, "non-array rows are dropped");
        assert_eq!(out[0], r#"{"email":"a@x.com","updated_at":"2026-01-01"}"#);
        assert_eq!(out[1], r#"{"email":"b@x.com","updated_at":null}"#);
    }

    #[test]
    fn dedup_keeps_the_last_occurrence_per_key() {
        let a1 = json!({"id": 1, "v": "old"});
        let b = json!({"id": 2, "v": "b"});
        let a2 = json!({"id": 1, "v": "new"});
        let out = dedup_by_keys(&[&a1, &b, &a2], &["id".into()]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["id"], 2);
        assert_eq!(out[1]["v"], "new", "last write for id=1 wins");
    }
}
