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

//! Workflow registry DAL for unified backend support

use super::DAL;

/// Data access layer for workflow registry operations.
#[derive(Clone)]
pub struct WorkflowRegistryDAL<'a> {
    dal: &'a DAL,
}

impl<'a> WorkflowRegistryDAL<'a> {
    /// Creates a new WorkflowRegistryDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    // TODO: Implement workflow registry operations with backend dispatch
}
