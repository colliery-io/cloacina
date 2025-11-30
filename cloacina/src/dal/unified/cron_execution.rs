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

//! Cron execution DAL for unified backend support

use super::DAL;

/// Data access layer for cron execution operations.
#[derive(Clone)]
pub struct CronExecutionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> CronExecutionDAL<'a> {
    /// Creates a new CronExecutionDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    // TODO: Implement cron execution operations with backend dispatch
}
