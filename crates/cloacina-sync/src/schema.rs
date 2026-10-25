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

//! A sync stream's captured schema and how it evolves (COLLIERY-I-0635 /
//! CLOACINA-T-1003), as weir does it.
//!
//! Each segment samples its mapped records before enforcement, so a record
//! the schema rejects still shows the drift. Against the stored schema (or
//! the declared one, or the sample's own shape before any is stored):
//!
//! - a field the schema lacks is new: it is merged in, and `broken` clears;
//! - a field whose sampled values no longer fit its type marks the schema
//!   broken, with the reasons, and the schema stays as it was;
//! - a dropped field and a change of nullability are ignored.
//!
//! Nothing pauses on a broken schema: the records that do not fit go to the
//! dead-letters. Weir compared inferred types; a declared schema here may
//! coerce (`"30"` fits an integer), so a field breaks only when a sampled
//! value does not coerce to its type.

use cloacina_constructor_contract::sync::{Field, StreamSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mapping::coerce_field;

/// How many records of a segment the schema is inferred from.
pub const SAMPLE: usize = 200;

/// A stream's stored schema, as the dispatcher hands it to a segment.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StoredSchema {
    pub schema: StreamSchema,
    #[serde(default)]
    pub broken: Option<String>,
}

/// The schema a stream holds after a segment whose mapped records are
/// `sample`, and why it is broken, if it is. `None` when the segment had no
/// records: nothing changes.
pub fn evolve(
    stored: Option<&StreamSchema>,
    declared: Option<&StreamSchema>,
    sample: &[String],
) -> Option<StoredSchema> {
    if sample.is_empty() {
        return None;
    }
    let current = StreamSchema::infer(sample);
    let base = stored
        .or(declared)
        .cloned()
        .unwrap_or_else(|| current.clone());

    let known: std::collections::HashSet<&str> =
        base.fields.iter().map(|f| f.name.as_str()).collect();
    let additive: Vec<Field> = current
        .fields
        .iter()
        .filter(|f| !known.contains(f.name.as_str()))
        .cloned()
        .collect();
    let breaking = misfits(&base, sample);

    Some(if !breaking.is_empty() {
        StoredSchema {
            schema: base,
            broken: Some(breaking.join("; ")),
        }
    } else {
        StoredSchema {
            schema: base.merged(&additive),
            broken: None,
        }
    })
}

/// The fields of `schema` with a sampled value that does not coerce to the
/// field's type, as reasons.
fn misfits(schema: &StreamSchema, sample: &[String]) -> Vec<String> {
    let mut reasons = Vec::new();
    for field in &schema.fields {
        let bad = sample.iter().find_map(|rec| {
            let v: Value = serde_json::from_str(rec).ok()?;
            let value = v.get(&field.name)?;
            if value.is_null() {
                return None;
            }
            coerce_field(value, field.field_type)
                .is_err()
                .then(|| value.clone())
        });
        if let Some(value) = bad {
            reasons.push(format!(
                "field `{}` holds {value}, which is not a {:?}",
                field.name, field.field_type
            ));
        }
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina_constructor_contract::sync::FieldType;

    fn schema(fields: &[(&str, FieldType)]) -> StreamSchema {
        StreamSchema {
            fields: fields
                .iter()
                .map(|(n, t)| Field {
                    name: n.to_string(),
                    field_type: *t,
                    nullable: true,
                })
                .collect(),
        }
    }

    fn rows(r: &[&str]) -> Vec<String> {
        r.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_first_segment_captures_the_sample_shape() {
        let s = evolve(None, None, &rows(&[r#"{"id":1,"name":"a"}"#])).unwrap();
        assert_eq!(s.broken, None);
        let names: Vec<_> = s.schema.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["id", "name"]);
    }

    #[test]
    fn a_new_field_merges_and_clears_broken() {
        let stored = schema(&[("id", FieldType::Integer)]);
        let s = evolve(Some(&stored), None, &rows(&[r#"{"id":1,"email":"x"}"#])).unwrap();
        assert_eq!(s.broken, None);
        assert_eq!(s.schema.fields.len(), 2);
        assert_eq!(s.schema.fields[1].name, "email");
    }

    #[test]
    fn a_value_that_no_longer_fits_marks_the_schema_broken_and_keeps_it() {
        let stored = schema(&[("id", FieldType::Integer)]);
        let s = evolve(Some(&stored), None, &rows(&[r#"{"id":"abc","extra":1}"#])).unwrap();
        assert!(s.broken.as_deref().unwrap().contains("`id`"), "{s:?}");
        assert_eq!(s.schema, stored, "a broken schema is kept as it was");
    }

    #[test]
    fn a_declared_schema_that_coerces_is_not_broken() {
        let declared = schema(&[("id", FieldType::Integer), ("age", FieldType::Integer)]);
        let s = evolve(None, Some(&declared), &rows(&[r#"{"id":"1","age":"30"}"#])).unwrap();
        assert_eq!(s.broken, None, "text that coerces fits");
        assert_eq!(s.schema, declared);
    }

    #[test]
    fn dropped_fields_and_nulls_are_ignored_and_no_records_change_nothing() {
        let stored = schema(&[("id", FieldType::Integer), ("gone", FieldType::Str)]);
        let s = evolve(Some(&stored), None, &rows(&[r#"{"id":null}"#])).unwrap();
        assert_eq!(s.broken, None);
        assert_eq!(s.schema, stored);
        assert_eq!(evolve(Some(&stored), None, &[]), None);
    }
}
