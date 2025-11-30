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

//! Cron schedule DAL for unified backend support

use super::DAL;

/// Data access layer for cron schedule operations.
#[derive(Clone)]
pub struct CronScheduleDAL<'a> {
    dal: &'a DAL,
}

impl<'a> CronScheduleDAL<'a> {
    /// Creates a new CronScheduleDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    // TODO: Implement cron schedule operations with backend dispatch
}
