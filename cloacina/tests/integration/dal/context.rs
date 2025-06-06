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

#[tokio::test]
async fn test_save_and_load_context() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let context_dal = dal.context();

    // Create and save a context
    let mut context = Context::new();
    context.insert("test", 42).expect("Failed to insert value");
    let id = context_dal
        .create(&context)
        .await
        .expect("Failed to save context")
        .expect("Non-empty context should return Some(uuid)");

    // Load and verify the context
    let loaded = context_dal
        .read::<i32>(id)
        .await
        .expect("Failed to load context");
    assert_eq!(loaded.get("test"), Some(&42));
}

#[tokio::test]
async fn test_update_context() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let context_dal = dal.context();

    // Create and save a context
    let mut context = Context::new();
    context.insert("test", 42).expect("Failed to insert value");
    let id = context_dal
        .create(&context)
        .await
        .expect("Failed to save context")
        .expect("Non-empty context should return Some(uuid)");

    // Update the context
    context.update("test", 43).expect("Failed to update value");
    context_dal
        .update(id, &context)
        .await
        .expect("Failed to update context");

    // Load and verify the update
    let loaded = context_dal
        .read::<i32>(id)
        .await
        .expect("Failed to load context");
    assert_eq!(loaded.get("test"), Some(&43));
}

#[tokio::test]
async fn test_delete_context() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let context_dal = dal.context();

    // Create and save a context
    let mut context = Context::new();
    context.insert("test", 42).expect("Failed to insert value");
    let id = context_dal
        .create(&context)
        .await
        .expect("Failed to save context")
        .expect("Non-empty context should return Some(uuid)");

    // Delete the context
    context_dal
        .delete(id)
        .await
        .expect("Failed to delete context");

    // Verify it's gone
    let result = context_dal.read::<i32>(id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_empty_context_handling() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let context_dal = dal.context();

    // Test completely empty context
    let empty_context = Context::<i32>::new();
    let id = context_dal
        .create(&empty_context)
        .await
        .expect("Empty context should not fail");
    assert_eq!(id, None, "Empty context should return None");
}

#[tokio::test]
async fn test_list_contexts_pagination() {
    let fixture = get_or_init_fixture().await;
    let mut fixture = fixture
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fixture.initialize().await;

    let dal = fixture.get_dal();
    let context_dal = dal.context();

    // Create multiple contexts
    let mut contexts = Vec::new();
    for i in 0..5 {
        let mut context = Context::new();
        context.insert("value", i).expect("Failed to insert value");
        let id = context_dal
            .create(&context)
            .await
            .expect("Failed to create context")
            .expect("Non-empty context should return Some(uuid)");
        contexts.push(id);
    }

    // Test pagination - get total count first to verify our assumption
    let all_contexts = context_dal
        .list::<i32>(10, 0)
        .await
        .expect("Failed to list all contexts");
    let total_count = all_contexts.len();

    // Test pagination
    let first_page = context_dal
        .list::<i32>(3, 0)
        .await
        .expect("Failed to list contexts");
    assert!(
        first_page.len() <= 3,
        "First page should have at most 3 items"
    );

    let second_page = context_dal
        .list::<i32>(3, 3)
        .await
        .expect("Failed to list contexts");
    let expected_second_page_size = if total_count > 3 { total_count - 3 } else { 0 };
    assert_eq!(
        second_page.len(),
        expected_second_page_size,
        "Second page size mismatch. Total: {}, First page: {}",
        total_count,
        first_page.len()
    );
}
