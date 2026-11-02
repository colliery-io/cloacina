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
//! Schema steps: a source whose columns change, and the stream's captured
//! schema.

use cucumber::{given, then, when};

use crate::support::pg;
use crate::support::world::{Side, SyncWorld};

#[given("an echo destination")]
async fn echo_destination(w: &mut SyncWorld) {
    w.destination = Some(Side::echo());
}

#[given(expr = "the source table {string} gains the text column {string}")]
#[when(expr = "the source table {string} gains the text column {string}")]
async fn gains_column(w: &mut SyncWorld, name: String, column: String) {
    let table = w.table(&name);
    pg::execute(&format!(
        "ALTER TABLE {} ADD COLUMN {} TEXT",
        pg::quoted(&table),
        pg::quoted(&column)
    ))
    .await;
}

#[given(expr = "the column {string} of the source table {string} becomes text")]
#[when(expr = "the column {string} of the source table {string} becomes text")]
async fn becomes_text(w: &mut SyncWorld, column: String, name: String) {
    let table = w.table(&name);
    let c = pg::quoted(&column);
    pg::execute(&format!(
        "ALTER TABLE {} ALTER COLUMN {c} TYPE TEXT USING {c}::text",
        pg::quoted(&table)
    ))
    .await;
}

#[when("the stream's new shape is accepted")]
async fn accepted(w: &mut SyncWorld) {
    w.accept_schema().await;
}

#[then(expr = "the stream's schema has the field {string} as {word}")]
async fn has_field(w: &mut SyncWorld, field: String, ty: String) {
    let (schema, _) = w.stream_schema().await.expect("a captured schema");
    let f = schema
        .fields
        .iter()
        .find(|f| f.name == field)
        .unwrap_or_else(|| panic!("no field `{field}` in {schema:?}"));
    assert_eq!(format!("{:?}", f.field_type), ty, "{schema:?}");
}

#[then("the stream's schema is not broken")]
async fn not_broken(w: &mut SyncWorld) {
    let (_, broken) = w.stream_schema().await.expect("a captured schema");
    assert_eq!(broken, None);
}

#[then(expr = "the stream's schema is broken, naming {string}")]
async fn broken(w: &mut SyncWorld, field: String) {
    let (_, broken) = w.stream_schema().await.expect("a captured schema");
    let reason = broken.expect("the schema is broken");
    assert!(reason.contains(&format!("`{field}`")), "{reason}");
}
