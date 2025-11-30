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

//! Context DAL for unified backend support

use super::DAL;

/// Data access layer for context operations.
#[derive(Clone)]
pub struct ContextDAL<'a> {
    dal: &'a DAL,
}

impl<'a> ContextDAL<'a> {
    /// Creates a new ContextDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    // TODO: Implement context operations with backend dispatch
    // Methods will be migrated from postgres_dal/context.rs and sqlite_dal/context.rs
}
