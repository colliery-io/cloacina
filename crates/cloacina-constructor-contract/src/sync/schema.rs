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

//! Typed stream schemas: the field set of a stream with types and nullability,
//! inferred from a sample or declared, persisted per stream, and enforced on the
//! record path by the map-and-enforce task (COLLIERY-I-0632).

use serde::{Deserialize, Serialize};

/// A field type — the mapping `CastType` set + `timestamp`/`json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Str,
    Integer,
    Float,
    Boolean,
    Timestamp,
    Json,
}

/// One typed field of a stream schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default = "yes")]
    pub nullable: bool,
}

fn yes() -> bool {
    true
}

/// A stream's typed schema — the field set with types + nullability, persisted per stream.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamSchema {
    pub fields: Vec<Field>,
}

fn value_type(v: &serde_json::Value) -> Option<FieldType> {
    match v {
        serde_json::Value::Null => None,
        serde_json::Value::Bool(_) => Some(FieldType::Boolean),
        serde_json::Value::Number(n) => Some(if n.is_i64() || n.is_u64() {
            FieldType::Integer
        } else {
            FieldType::Float
        }),
        serde_json::Value::String(_) => Some(FieldType::Str),
        _ => Some(FieldType::Json), // object / array
    }
}

impl StreamSchema {
    /// Infer a schema from a sample of JSON-object records: a field's type is the
    /// common type of its non-null values (int+float → float; a genuine mix → json); a field that is
    /// ever null or absent-in-some-records is nullable. Field order follows first appearance.
    pub fn infer(sample: &[String]) -> StreamSchema {
        use std::collections::{HashMap, HashSet};
        let n = sample.len();
        let mut order: Vec<String> = Vec::new();
        let mut types: HashMap<String, HashSet<FieldType>> = HashMap::new();
        let mut nullable: HashMap<String, bool> = HashMap::new();
        let mut count: HashMap<String, usize> = HashMap::new();
        for rec in sample {
            let Ok(serde_json::Value::Object(o)) = serde_json::from_str::<serde_json::Value>(rec)
            else {
                continue;
            };
            for (k, v) in &o {
                if !types.contains_key(k) {
                    order.push(k.clone());
                    types.insert(k.clone(), HashSet::new());
                    nullable.insert(k.clone(), false);
                    count.insert(k.clone(), 0);
                }
                *count.get_mut(k).unwrap() += 1;
                match value_type(v) {
                    None => *nullable.get_mut(k).unwrap() = true,
                    Some(t) => {
                        types.get_mut(k).unwrap().insert(t);
                    }
                }
            }
        }
        let fields = order
            .into_iter()
            .map(|k| {
                let ts = &types[&k];
                let field_type = if ts.is_empty() {
                    FieldType::Str // all-null / never-observed → default string
                } else if ts.len() == 1 {
                    *ts.iter().next().unwrap()
                } else if *ts == HashSet::from([FieldType::Integer, FieldType::Float]) {
                    FieldType::Float // numeric widening
                } else {
                    FieldType::Json // genuine mix
                };
                let nullable = nullable[&k] || count[&k] < n; // ever null, or absent in some records
                Field {
                    name: k,
                    field_type,
                    nullable,
                }
            })
            .collect();
        StreamSchema { fields }
    }

    /// Diff `self` (stored) against a freshly-`discovered` schema: a field present in
    /// `discovered` but not stored is **additive**; a shared field whose type changed is **breaking**.
    /// Dropped fields + nullability shifts are intentionally ignored — inference reads them from a
    /// sample, so they're too noisy to gate a run on. Only a concrete type change is a hard break.
    pub fn diff(&self, discovered: &StreamSchema) -> SchemaDiff {
        use std::collections::HashMap;
        let stored: HashMap<&str, &Field> =
            self.fields.iter().map(|f| (f.name.as_str(), f)).collect();
        let mut additive = Vec::new();
        let mut breaking = Vec::new();
        for f in &discovered.fields {
            match stored.get(f.name.as_str()) {
                None => additive.push(f.clone()),
                Some(s) if s.field_type != f.field_type => breaking.push(format!(
                    "field `{}` type changed {:?} → {:?}",
                    f.name, s.field_type, f.field_type
                )),
                Some(_) => {}
            }
        }
        SchemaDiff { additive, breaking }
    }

    /// `self` with `additive` fields appended — accept a purely-additive evolution.
    pub fn merged(&self, additive: &[Field]) -> StreamSchema {
        let mut fields = self.fields.clone();
        fields.extend(additive.iter().cloned());
        StreamSchema { fields }
    }
}

/// The result of [`StreamSchema::diff`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaDiff {
    /// New fields — safe to accept (merge into the stored schema).
    pub additive: Vec<Field>,
    /// Human-readable reasons a change is breaking — surfaced, not auto-applied.
    pub breaking: Vec<String>,
}

#[cfg(test)]
mod schema_tests {
    use super::*;

    #[test]
    fn infers_types_and_nullability() {
        let sample = vec![
            r#"{"id":1,"v":"a","amt":1.5}"#.to_string(),
            r#"{"id":2,"v":"b","amt":2,"x":null}"#.to_string(), // x null; amt int here → widens to float
        ];
        let s = StreamSchema::infer(&sample);
        let by = |n: &str| s.fields.iter().find(|f| f.name == n).cloned().unwrap();
        assert_eq!(by("id").field_type, FieldType::Integer);
        assert!(!by("id").nullable);
        assert_eq!(by("v").field_type, FieldType::Str);
        assert_eq!(by("amt").field_type, FieldType::Float); // int + float → float
        assert!(by("x").nullable); // null in one, absent in the other
    }

    #[test]
    fn mixed_scalar_types_widen_to_json() {
        let sample = vec![r#"{"m":1}"#.to_string(), r#"{"m":"str"}"#.to_string()];
        assert_eq!(
            StreamSchema::infer(&sample).fields[0].field_type,
            FieldType::Json
        );
    }

    #[test]
    fn schema_json_round_trips() {
        let s = StreamSchema {
            fields: vec![Field {
                name: "id".into(),
                field_type: FieldType::Integer,
                nullable: false,
            }],
        };
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<StreamSchema>(&j).unwrap(), s);
        // `type` is the wire key; nullable defaults to true when omitted.
        let parsed: StreamSchema =
            serde_json::from_str(r#"{"fields":[{"name":"x","type":"str"}]}"#).unwrap();
        assert!(parsed.fields[0].nullable);
    }

    #[test]
    fn diff_additive_vs_breaking() {
        let stored = StreamSchema {
            fields: vec![
                Field {
                    name: "id".into(),
                    field_type: FieldType::Integer,
                    nullable: false,
                },
                Field {
                    name: "v".into(),
                    field_type: FieldType::Str,
                    nullable: true,
                },
            ],
        };
        // A new field is additive; merging appends it.
        let plus = stored.merged(&[Field {
            name: "x".into(),
            field_type: FieldType::Boolean,
            nullable: true,
        }]);
        let d = stored.diff(&plus);
        assert!(d.breaking.is_empty());
        assert_eq!(d.additive.len(), 1);
        assert_eq!(d.additive[0].name, "x");
        assert_eq!(stored.merged(&d.additive).fields.len(), 3);

        // A type change on a shared field is breaking.
        let retyped = StreamSchema {
            fields: vec![Field {
                name: "id".into(),
                field_type: FieldType::Str,
                nullable: false,
            }],
        };
        let d2 = stored.diff(&retyped);
        assert_eq!(d2.breaking.len(), 1);
        assert!(d2.breaking[0].contains("id"));
        assert!(d2.additive.is_empty());
    }
}
