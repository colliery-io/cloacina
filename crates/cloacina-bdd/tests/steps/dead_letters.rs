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
//! Dead-letter steps: a stream schema, and the records set aside.

use cloacina_constructor_contract::sync::{Field, FieldType, StreamSchema};
use cucumber::gherkin::Step;
use cucumber::{given, then};
use serde_json::Value;

use crate::support::pg;
use crate::support::world::SyncWorld;

fn field_type(name: &str) -> FieldType {
    match name {
        "text" => FieldType::Str,
        "integer" => FieldType::Integer,
        "float" => FieldType::Float,
        "boolean" => FieldType::Boolean,
        "timestamp" => FieldType::Timestamp,
        "json" => FieldType::Json,
        other => panic!("no field type `{other}`"),
    }
}

/// A table of `field | type | nullable`, `nullable` being `yes` or `no`.
#[given("the stream's schema is")]
async fn schema(w: &mut SyncWorld, step: &Step) {
    let rows = &step.table.as_ref().expect("a schema table").rows;
    let fields = rows
        .iter()
        .skip(1)
        .map(|r| Field {
            name: r[0].clone(),
            field_type: field_type(&r[1]),
            nullable: r[2] == "yes",
        })
        .collect();
    w.schema = Some(StreamSchema { fields });
}

#[then(expr = "the dead-letters hold {int} record(s)")]
async fn dead_letter_count(w: &mut SyncWorld, n: usize) {
    let dead = w.dead_letters().await;
    assert_eq!(dead.len(), n, "dead-letters: {dead:?}");
}

#[then(expr = "a dead-letter holds the record with id {int}, for its field {string}")]
async fn dead_letter_for(w: &mut SyncWorld, id: i64, field: String) {
    let dead = w.dead_letters().await;
    let found = dead.iter().find(|(record, _)| match record.get("id") {
        Some(Value::Number(n)) => n.as_i64() == Some(id),
        Some(Value::String(s)) => s.parse::<i64>().ok() == Some(id),
        _ => false,
    });
    let (_, reason) = found.unwrap_or_else(|| panic!("no dead-letter for id {id}: {dead:?}"));
    assert!(
        reason.contains(&format!("`{field}`")),
        "the reason names `{field}`: {reason}"
    );
}

#[then(expr = "the destination table {string} holds no records")]
async fn holds_none(w: &mut SyncWorld, name: String) {
    let table = w.table(&name);
    assert_eq!(
        pg::count_if_exists(&table).await,
        0,
        "{table} holds records"
    );
}
