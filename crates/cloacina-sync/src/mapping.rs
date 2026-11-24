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

//! In-flight mapping and schema enforcement: apply a `MappingSpec` to each
//! record between read and write, then coerce it to the stream's typed schema.
//! Bounded ops only, no I/O, no network, no arbitrary code; heavy transforms
//! stay downstream. Operates on row JSON, one object per record. Moved from
//! weir's engine with Sync (COLLIERY-I-0632 / CLOACINA-T-0976).

use cloacina_constructor_contract::sync::{
    CastType, CompareOp, ComputeExpr, FieldType, MappingOp, MappingSpec, StreamSchema,
};
use serde_json::{Map, Value};

/// Outcome of applying a `MappingSpec` to one record.
pub enum Mapped {
    /// Transformed record — keep + write.
    Keep(Value),
    /// Predicate excluded it — drop (counted, not an error).
    Filtered,
    /// A cast failed — dead-letter with this reason (don't write).
    DeadLetter(String),
}

/// Apply `spec`'s ops to a JSON record **in order**. Non-object records pass
/// through unchanged (mapping is field-based).
pub fn apply(spec: &MappingSpec, mut rec: Value) -> Mapped {
    if spec.ops.is_empty() {
        return Mapped::Keep(rec);
    }
    let Some(obj) = rec.as_object_mut() else {
        return Mapped::Keep(rec);
    };
    for op in &spec.ops {
        match op {
            MappingOp::Select { fields } => obj.retain(|k, _| fields.iter().any(|f| f == k)),
            MappingOp::Drop { fields } => obj.retain(|k, _| !fields.iter().any(|f| f == k)),
            MappingOp::Rename { from, to } => {
                if let Some(v) = obj.remove(from) {
                    obj.insert(to.clone(), v);
                }
            }
            MappingOp::Cast { field, to } => {
                if let Some(v) = obj.get(field) {
                    match cast_value(v, *to) {
                        Ok(cv) => {
                            obj.insert(field.clone(), cv);
                        }
                        Err(reason) => return Mapped::DeadLetter(reason),
                    }
                }
            }
            MappingOp::Filter { field, op, value } => {
                let keep = obj.get(field).is_some_and(|v| compare(v, *op, value));
                if !keep {
                    return Mapped::Filtered;
                }
            }
            MappingOp::Compute { field, value } => {
                let computed = eval_compute(value, obj);
                obj.insert(field.clone(), Value::String(computed));
            }
        }
    }
    Mapped::Keep(rec)
}

/// A JSON value as its scalar string form (strings unquoted; others via JSON).
fn as_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn cast_value(v: &Value, to: CastType) -> Result<Value, String> {
    // Null is absence of a value — it passes through any cast unchanged (SQL-nullable semantics),
    // rather than dead-lettering the whole record over one empty optional field.
    if v.is_null() {
        return Ok(Value::Null);
    }
    let fail = || format!("cast: cannot coerce {v} to {to:?}");
    match to {
        CastType::Str => Ok(Value::String(as_str(v))),
        CastType::Integer => match v {
            Value::Number(n) if n.is_i64() || n.is_u64() => Ok(v.clone()),
            Value::Number(n) => n
                .as_f64()
                .map(|f| f as i64)
                .map(Value::from)
                .ok_or_else(fail),
            Value::String(s) => s.trim().parse::<i64>().map(Value::from).map_err(|_| fail()),
            _ => Err(fail()),
        },
        CastType::Float => match v {
            Value::Number(_) => Ok(v.clone()),
            Value::String(s) => s.trim().parse::<f64>().map(Value::from).map_err(|_| fail()),
            _ => Err(fail()),
        },
        CastType::Boolean => match v {
            Value::Bool(_) => Ok(v.clone()),
            Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "true" | "1" => Ok(Value::Bool(true)),
                "false" | "0" => Ok(Value::Bool(false)),
                _ => Err(fail()),
            },
            _ => Err(fail()),
        },
    }
}

/// Enforce a stream's typed schema on one record: coerce each schema field to its
/// type (reusing the `Cast` machinery); a value that won't coerce, or a missing/null **non-nullable**
/// field, dead-letters the record. Extra fields (not in the schema) pass through untouched. Runs
/// **after** mapping, so the schema describes the post-mapping shape. Non-object records pass through.
pub fn enforce(schema: &StreamSchema, mut rec: Value) -> Mapped {
    let Some(obj) = rec.as_object_mut() else {
        return Mapped::Keep(rec);
    };
    for field in &schema.fields {
        // `.cloned()` releases the borrow so we can re-insert the coerced value below.
        match obj.get(&field.name).cloned() {
            None => {
                if !field.nullable {
                    return Mapped::DeadLetter(format!(
                        "schema: missing required field `{}`",
                        field.name
                    ));
                }
            }
            Some(Value::Null) => {
                if !field.nullable {
                    return Mapped::DeadLetter(format!(
                        "schema: required field `{}` is null",
                        field.name
                    ));
                }
            }
            Some(v) => match coerce_field(&v, field.field_type) {
                Ok(Some(cv)) => {
                    obj.insert(field.name.clone(), cv);
                }
                Ok(None) => {} // json: accept any shape as-is
                Err(_) => {
                    return Mapped::DeadLetter(format!(
                        "schema: field `{}` value {v} is not a {:?}",
                        field.name, field.field_type
                    ));
                }
            },
        }
    }
    Mapped::Keep(rec)
}

/// Coerce a non-null value to a `FieldType`. `Ok(Some(v))` = coerced value to store; `Ok(None)` =
/// accept as-is (json); `Err` = uncoercible.
pub(crate) fn coerce_field(v: &Value, to: FieldType) -> Result<Option<Value>, String> {
    let ct = match to {
        FieldType::Str => CastType::Str,
        FieldType::Integer => CastType::Integer,
        FieldType::Float => CastType::Float,
        FieldType::Boolean => CastType::Boolean,
        // A timestamp is carried as text; accept a string/number and normalize to string.
        FieldType::Timestamp => return Ok(Some(Value::String(as_str(v)))),
        // Arbitrary JSON — any shape is valid.
        FieldType::Json => return Ok(None),
    };
    cast_value(v, ct).map(Some)
}

/// Compare a field value to `rhs`: numeric when both parse as f64, else string.
fn compare(v: &Value, op: CompareOp, rhs: &str) -> bool {
    let ord = match (as_f64(v), rhs.trim().parse::<f64>().ok()) {
        (Some(a), Some(b)) => a.partial_cmp(&b),
        _ => Some(as_str(v).cmp(&rhs.to_string())),
    };
    match (op, ord) {
        (CompareOp::Eq, Some(o)) => o.is_eq(),
        (CompareOp::Ne, Some(o)) => !o.is_eq(),
        (CompareOp::Lt, Some(o)) => o.is_lt(),
        (CompareOp::Le, Some(o)) => o.is_le(),
        (CompareOp::Gt, Some(o)) => o.is_gt(),
        (CompareOp::Ge, Some(o)) => o.is_ge(),
        (_, None) => false,
    }
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn eval_compute(expr: &ComputeExpr, obj: &Map<String, Value>) -> String {
    let field = |f: &str| lookup_path(obj, f).map(as_str).unwrap_or_default();
    match expr {
        ComputeExpr::Const(s) => s.clone(),
        ComputeExpr::Field(f) => field(f),
        ComputeExpr::Concat(fs) => fs.iter().map(|f| field(f)).collect(),
        ComputeExpr::Lower(f) => field(f).to_lowercase(),
        ComputeExpr::Upper(f) => field(f).to_uppercase(),
    }
}

/// Resolve a compute field reference: a plain name, or a **dot-path** into nested
/// values where a numeric segment indexes an array —
/// `dimensionValues.0.value` reaches `{"dimensionValues": [{"value": …}]}`. This is
/// what lets a mapping flatten columnar API responses (GA4-style) without a new op.
fn lookup_path<'a>(obj: &'a Map<String, Value>, path: &str) -> Option<&'a Value> {
    let mut segs = path.split('.');
    let mut cur = obj.get(segs.next()?)?;
    for seg in segs {
        cur = match (cur, seg.parse::<usize>()) {
            (Value::Array(a), Ok(i)) => a.get(i)?,
            (v, _) => v.get(seg)?,
        };
    }
    Some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_constructor_contract::sync::Field;

    fn schema(fields: Vec<(&str, FieldType, bool)>) -> StreamSchema {
        StreamSchema {
            fields: fields
                .into_iter()
                .map(|(n, t, nullable)| Field {
                    name: n.into(),
                    field_type: t,
                    nullable,
                })
                .collect(),
        }
    }
    fn enforced(s: &StreamSchema, rec: &str) -> Mapped {
        enforce(s, serde_json::from_str(rec).unwrap())
    }

    #[test]
    fn enforce_coerces_and_dead_letters() {
        let s = schema(vec![
            ("id", FieldType::Integer, false),
            ("v", FieldType::Str, true),
        ]);
        // "5" coerces to integer 5.
        match enforced(&s, r#"{"id":"5","v":"a"}"#) {
            Mapped::Keep(v) => assert_eq!(v["id"], serde_json::json!(5)),
            _ => panic!("expected keep"),
        }
        // Uncoercible value and a missing required field both dead-letter.
        assert!(matches!(
            enforced(&s, r#"{"id":"abc"}"#),
            Mapped::DeadLetter(_)
        ));
        assert!(matches!(
            enforced(&s, r#"{"v":"a"}"#),
            Mapped::DeadLetter(_)
        ));
    }

    #[test]
    fn enforce_nullability_and_extra_fields() {
        let s = schema(vec![
            ("id", FieldType::Integer, false),
            ("opt", FieldType::Str, true),
        ]);
        // Null in a non-nullable field dead-letters.
        assert!(matches!(
            enforced(&s, r#"{"id":null}"#),
            Mapped::DeadLetter(_)
        ));
        // A nullable field may be null; an unschema'd extra field passes through untouched.
        match enforced(&s, r#"{"id":1,"opt":null,"extra":true}"#) {
            Mapped::Keep(v) => {
                assert!(v["opt"].is_null());
                assert_eq!(v["extra"], serde_json::json!(true));
            }
            _ => panic!("expected keep"),
        }
    }

    #[test]
    fn enforce_json_and_timestamp() {
        let s = schema(vec![
            ("meta", FieldType::Json, true),
            ("ts", FieldType::Timestamp, true),
        ]);
        match enforced(&s, r#"{"meta":{"a":1},"ts":"2020-01-01T00:00:00Z"}"#) {
            Mapped::Keep(v) => {
                assert!(v["meta"].is_object()); // json accepts any shape
                assert_eq!(v["ts"], serde_json::json!("2020-01-01T00:00:00Z"));
            }
            _ => panic!("expected keep"),
        }
    }
    use cloacina_constructor_contract::sync::MappingSpec;

    fn rec(j: &str) -> Value {
        serde_json::from_str(j).unwrap()
    }
    fn spec(ops: Vec<MappingOp>) -> MappingSpec {
        MappingSpec { ops }
    }
    fn kept(m: Mapped) -> Value {
        match m {
            Mapped::Keep(v) => v,
            Mapped::Filtered => panic!("filtered"),
            Mapped::DeadLetter(r) => panic!("dead-letter: {r}"),
        }
    }

    #[test]
    fn select_drop_rename() {
        let v = kept(apply(
            &spec(vec![
                MappingOp::Rename {
                    from: "a".into(),
                    to: "x".into(),
                },
                MappingOp::Drop {
                    fields: vec!["b".into()],
                },
            ]),
            rec(r#"{"a":1,"b":2,"c":3}"#),
        ));
        assert_eq!(v, rec(r#"{"x":1,"c":3}"#));

        let v = kept(apply(
            &spec(vec![MappingOp::Select {
                fields: vec!["c".into()],
            }]),
            rec(r#"{"a":1,"b":2,"c":3}"#),
        ));
        assert_eq!(v, rec(r#"{"c":3}"#));
    }

    #[test]
    fn cast_ok_and_deadletter() {
        let v = kept(apply(
            &spec(vec![MappingOp::Cast {
                field: "n".into(),
                to: CastType::Integer,
            }]),
            rec(r#"{"n":"42"}"#),
        ));
        assert_eq!(v, rec(r#"{"n":42}"#));

        match apply(
            &spec(vec![MappingOp::Cast {
                field: "n".into(),
                to: CastType::Integer,
            }]),
            rec(r#"{"n":"notnum"}"#),
        ) {
            Mapped::DeadLetter(_) => {}
            _ => panic!("expected dead-letter on bad cast"),
        }
    }

    #[test]
    fn filter_numeric_and_string() {
        // numeric: amount > 100
        assert!(matches!(
            apply(
                &spec(vec![MappingOp::Filter {
                    field: "amount".into(),
                    op: CompareOp::Gt,
                    value: "100".into(),
                }]),
                rec(r#"{"amount":50}"#),
            ),
            Mapped::Filtered
        ));
        let v = kept(apply(
            &spec(vec![MappingOp::Filter {
                field: "amount".into(),
                op: CompareOp::Gt,
                value: "100".into(),
            }]),
            rec(r#"{"amount":150}"#),
        ));
        assert_eq!(v, rec(r#"{"amount":150}"#));
        // string eq
        assert!(matches!(
            apply(
                &spec(vec![MappingOp::Filter {
                    field: "status".into(),
                    op: CompareOp::Eq,
                    value: "active".into(),
                }]),
                rec(r#"{"status":"inactive"}"#),
            ),
            Mapped::Filtered
        ));
    }

    #[test]
    fn compute_variants() {
        let v = kept(apply(
            &spec(vec![
                MappingOp::Compute {
                    field: "full".into(),
                    value: ComputeExpr::Concat(vec!["a".into(), "b".into()]),
                },
                MappingOp::Compute {
                    field: "up".into(),
                    value: ComputeExpr::Upper("a".into()),
                },
                MappingOp::Compute {
                    field: "tag".into(),
                    value: ComputeExpr::Const("x".into()),
                },
            ]),
            rec(r#"{"a":"hi","b":"there"}"#),
        ));
        assert_eq!(v["full"], Value::String("hithere".into()));
        assert_eq!(v["up"], Value::String("HI".into()));
        assert_eq!(v["tag"], Value::String("x".into()));
    }

    // ---- : edge cases that define (not incidentally leave) behaviour ----

    #[test]
    fn absent_fields_are_noops_not_errors() {
        // Rename / Drop / Select / Cast that name a missing field leave the record intact.
        let v = kept(apply(
            &spec(vec![
                MappingOp::Rename {
                    from: "missing".into(),
                    to: "x".into(),
                },
                MappingOp::Drop {
                    fields: vec!["gone".into()],
                },
                MappingOp::Cast {
                    field: "absent".into(),
                    to: CastType::Integer,
                },
            ]),
            rec(r#"{"a":1}"#),
        ));
        assert_eq!(
            v,
            rec(r#"{"a":1}"#),
            "no-op on absent fields, no dead-letter"
        );
        // Select naming only-absent fields empties the object (kept only what matched).
        let v = kept(apply(
            &spec(vec![MappingOp::Select {
                fields: vec!["nope".into()],
            }]),
            rec(r#"{"a":1,"b":2}"#),
        ));
        assert_eq!(v, rec("{}"));
    }

    #[test]
    fn cast_null_passes_through_but_unrepresentable_dead_letters() {
        // Null is absence → passes through any cast (not a dead-letter).
        let v = kept(apply(
            &spec(vec![MappingOp::Cast {
                field: "age".into(),
                to: CastType::Integer,
            }]),
            rec(r#"{"age":null,"name":"x"}"#),
        ));
        assert_eq!(v, rec(r#"{"age":null,"name":"x"}"#));
        // A structurally-unrepresentable value (object/array) still dead-letters.
        assert!(matches!(
            apply(
                &spec(vec![MappingOp::Cast {
                    field: "o".into(),
                    to: CastType::Integer
                }]),
                rec(r#"{"o":{"nested":1}}"#),
            ),
            Mapped::DeadLetter(_)
        ));
    }

    #[test]
    fn cast_between_scalar_types() {
        // int→str, str→float, str→bool, and the whole-number-float→int narrowing.
        let v = kept(apply(
            &spec(vec![
                MappingOp::Cast {
                    field: "i".into(),
                    to: CastType::Str,
                },
                MappingOp::Cast {
                    field: "f".into(),
                    to: CastType::Float,
                },
                MappingOp::Cast {
                    field: "b".into(),
                    to: CastType::Boolean,
                },
            ]),
            rec(r#"{"i":7,"f":"3.5","b":"TRUE"}"#),
        ));
        assert_eq!(v["i"], Value::String("7".into()));
        assert_eq!(v["f"], Value::from(3.5));
        assert_eq!(v["b"], Value::Bool(true));
    }

    #[test]
    fn filter_on_absent_field_drops_the_record() {
        // Can't satisfy a predicate on a field that isn't there → excluded (counted, not error).
        assert!(matches!(
            apply(
                &spec(vec![MappingOp::Filter {
                    field: "missing".into(),
                    op: CompareOp::Gt,
                    value: "0".into(),
                }]),
                rec(r#"{"a":1}"#),
            ),
            Mapped::Filtered
        ));
    }

    #[test]
    fn unmapped_fields_pass_through_and_ops_apply_in_order() {
        // Non-Select ops leave untouched fields alone; ops compose left-to-right.
        let v = kept(apply(
            &spec(vec![
                MappingOp::Cast {
                    field: "n".into(),
                    to: CastType::Integer,
                },
                MappingOp::Rename {
                    from: "n".into(),
                    to: "id".into(),
                },
            ]),
            rec(r#"{"n":"5","keep":"me"}"#),
        ));
        assert_eq!(
            v,
            rec(r#"{"id":5,"keep":"me"}"#),
            "cast then rename; extra field passes through"
        );
    }

    #[test]
    fn non_object_records_pass_through_unchanged() {
        // Mapping is field-based; a scalar/array record is returned as-is.
        let v = kept(apply(
            &spec(vec![MappingOp::Drop {
                fields: vec!["x".into()],
            }]),
            rec("[1,2,3]"),
        ));
        assert_eq!(v, rec("[1,2,3]"));
    }

    /// Dot-path Compute: flatten a GA4-style columnar record —
    /// numeric segments index arrays; flat names still work; missing paths → empty.
    #[test]
    fn compute_field_resolves_dot_paths() {
        let columnar = rec(
            r#"{"dimensionValues":[{"value":"20260701"},{"value":"Direct"}],
                "metricValues":[{"value":"12"}],"plain":"x"}"#,
        );
        let v = kept(apply(
            &spec(vec![
                MappingOp::Compute {
                    field: "date".into(),
                    value: ComputeExpr::Field("dimensionValues.0.value".into()),
                },
                MappingOp::Compute {
                    field: "channel".into(),
                    value: ComputeExpr::Field("dimensionValues.1.value".into()),
                },
                MappingOp::Compute {
                    field: "sessions".into(),
                    value: ComputeExpr::Field("metricValues.0.value".into()),
                },
                MappingOp::Compute {
                    field: "still_flat".into(),
                    value: ComputeExpr::Field("plain".into()),
                },
                MappingOp::Compute {
                    field: "missing".into(),
                    value: ComputeExpr::Field("metricValues.9.value".into()),
                },
                MappingOp::Drop {
                    fields: vec!["dimensionValues".into(), "metricValues".into()],
                },
            ]),
            columnar,
        ));
        assert_eq!(v["date"], "20260701");
        assert_eq!(v["channel"], "Direct");
        assert_eq!(v["sessions"], "12");
        assert_eq!(v["still_flat"], "x");
        assert_eq!(v["missing"], "");
        assert!(v.get("dimensionValues").is_none());
    }
}
