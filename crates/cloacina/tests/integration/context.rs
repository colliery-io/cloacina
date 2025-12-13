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

// Context tests run against both PostgreSQL and SQLite to verify consistent
// behavior across backends. These tests use the DAL abstraction layer.

#[cfg(feature = "postgres")]
mod postgres_tests {
    use crate::fixtures::get_or_init_postgres_fixture;
    use cloacina::context::Context;
    use cloacina::dal::DAL;
    use serial_test::serial;
    use tracing::debug;

    #[tokio::test]
    #[serial]
    async fn test_context_db_operations() {
        // Get test fixture and initialize it
        let fixture = get_or_init_postgres_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.initialize().await;

        // Get database and DAL
        let database = fixture.get_database();
        let dal = DAL::new(database);

        // Create a test context with some data
        let mut context = Context::<i32>::new();
        context.insert("test_key", 42).unwrap();
        context.insert("another_key", 100).unwrap();

        // Save context using DAL - returns the ID
        let context_id = dal
            .context()
            .create(&context)
            .await
            .unwrap()
            .expect("Context should not be empty");
        debug!("Created context with id: {:?}", context_id);

        // Load context from database
        let loaded_context: Context<i32> = dal.context().read(context_id).await.unwrap();

        // Verify data matches
        assert_eq!(loaded_context.get("test_key"), Some(&42));
        assert_eq!(loaded_context.get("another_key"), Some(&100));

        // Test updating the context
        let mut updated_context = loaded_context.clone_data();
        updated_context.update("test_key", 43).unwrap();

        // Update in database
        dal.context()
            .update(context_id, &updated_context)
            .await
            .unwrap();

        // Load updated context
        let final_context: Context<i32> = dal.context().read(context_id).await.unwrap();

        // Verify updates
        assert_eq!(final_context.get("test_key"), Some(&43));
        assert_eq!(final_context.get("another_key"), Some(&100));

        // Clean up
        dal.context().delete(context_id).await.unwrap();
    }
}

#[cfg(feature = "sqlite")]
mod sqlite_tests {
    use crate::fixtures::get_or_init_sqlite_fixture;
    use cloacina::context::Context;
    use cloacina::dal::DAL;
    use serial_test::serial;
    use tracing::debug;

    #[tokio::test]
    #[serial]
    async fn test_context_db_operations() {
        // Get test fixture and initialize it
        let fixture = get_or_init_sqlite_fixture().await;
        let mut fixture = fixture.lock().unwrap_or_else(|e| e.into_inner());
        fixture.initialize().await;

        // Get database and DAL
        let database = fixture.get_database();
        let dal = DAL::new(database);

        // Create a test context with some data
        let mut context = Context::<i32>::new();
        context.insert("test_key", 42).unwrap();
        context.insert("another_key", 100).unwrap();

        // Save context using DAL - returns the ID
        let context_id = dal
            .context()
            .create(&context)
            .await
            .unwrap()
            .expect("Context should not be empty");
        debug!("Created context with id: {:?} (SQLite)", context_id);

        // Load context from database
        let loaded_context: Context<i32> = dal.context().read(context_id).await.unwrap();

        // Verify data matches
        assert_eq!(loaded_context.get("test_key"), Some(&42));
        assert_eq!(loaded_context.get("another_key"), Some(&100));

        // Test updating the context
        let mut updated_context = loaded_context.clone_data();
        updated_context.update("test_key", 43).unwrap();

        // Update in database
        dal.context()
            .update(context_id, &updated_context)
            .await
            .unwrap();

        // Load updated context
        let final_context: Context<i32> = dal.context().read(context_id).await.unwrap();

        // Verify updates
        assert_eq!(final_context.get("test_key"), Some(&43));
        assert_eq!(final_context.get("another_key"), Some(&100));

        // Clean up
        dal.context().delete(context_id).await.unwrap();
    }
}
