/*
 *  Copyright 2025 Colliery Software
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

use crate::fixtures::get_or_init_fixture;
use cloacina::context::Context;
use cloacina::database::schema::contexts::dsl::*;
use cloacina::models::context::DbContext;
use diesel::prelude::*;
use serial_test::serial;
use tracing::debug;

#[tokio::test]
#[serial]
async fn test_context_db_operations() {
    // Get test fixture and initialize it
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap();
    fixture.initialize().await;

    // Get database connection
    let conn = fixture.get_connection();

    // Create a test context with some data
    let mut context = Context::<i32>::new();
    context.insert("test_key", 42).unwrap();
    context.insert("another_key", 100).unwrap();

    // Convert to new DB record and insert
    let new_record = context.to_new_db_record().unwrap();
    debug!("New record to insert: {:?}", new_record);

    let db_context: DbContext = diesel::insert_into(contexts)
        .values(&new_record)
        .get_result(conn)
        .unwrap();

    // Load from database
    let loaded_context = Context::<i32>::from_db_record(&db_context).unwrap();

    // Verify data matches
    assert_eq!(loaded_context.get("test_key"), Some(&42));
    assert_eq!(loaded_context.get("another_key"), Some(&100));

    // Test updating the context
    let mut updated_context = loaded_context;
    updated_context.update("test_key", 43).unwrap();

    // Convert to full DB record and update
    let updated_db_record = updated_context.to_db_record(db_context.id.into()).unwrap();
    debug!("Updated record: {:?}", updated_db_record);

    let updated_db_context: DbContext = diesel::update(contexts)
        .filter(id.eq(db_context.id))
        .set((
            value.eq(updated_db_record.value),
            updated_at.eq(updated_db_record.updated_at),
        ))
        .get_result(conn)
        .unwrap();

    // Load updated context
    let final_context = Context::<i32>::from_db_record(&updated_db_context).unwrap();

    // Verify updates
    assert_eq!(final_context.get("test_key"), Some(&43));
    assert_eq!(final_context.get("another_key"), Some(&100));

    // Clean up
    diesel::delete(contexts)
        .filter(id.eq(db_context.id))
        .execute(conn)
        .unwrap();
}
